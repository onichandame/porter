use std::time;

use model::{ActiveModelTrait, EntityTrait, ModelTrait, Set, Unchanged};
use proxy::{self};
use tokio::{self, runtime, sync::mpsc, task::JoinHandle};

use crate::{
    event::Event,
    types::{CreateServiceInput, Error, IntoActiveModel, UpdateServiceInput},
    CreateGateInput, UpdateGateInput,
};

/// a porter should live for as long as the main program and be used in the following order
///
/// program start => new() => start() => program end
pub struct Porter {
    proxy_manager: proxy::ProxyManager,
    db: model::Database,
}

impl Porter {
    /// create a new porter
    pub fn new() -> Self {
        let async_handler = runtime::Handle::current();
        let proxy_manager = proxy::ProxyManager::new();
        async_handler
            .block_on(tokio::time::timeout(
                time::Duration::from_secs(3),
                proxy_manager.wait_for_ready(),
            ))
            .unwrap();
        Self {
            proxy_manager,
            db: async_handler.block_on(model::new_database()),
        }
    }

    pub async fn list_service(&self) -> Result<Vec<model::service::Model>, Error> {
        Ok(model::service::Entity::find().all(self.get_db()?).await?)
    }

    pub async fn get_service(&self, id: i32) -> Result<model::service::Model, Error> {
        Ok(model::service::Entity::find_by_id(id)
            .one(self.get_db()?)
            .await?
            .ok_or(format!("service {} not found", id))?)
    }

    pub async fn create_service(
        &self,
        input: CreateServiceInput,
    ) -> Result<model::service::Model, Error> {
        Ok(input.into_active_model().insert(self.get_db()?).await?)
    }

    pub async fn update_service(
        &self,
        id: i32,
        update: UpdateServiceInput,
    ) -> Result<model::service::Model, Error> {
        let mut update = update.into_active_model();
        update.id = Unchanged(id);
        update.updated_at = Set(Some(chrono::Utc::now().naive_utc()));
        Ok(update.update(self.get_db()?).await?)
    }

    pub async fn delete_service(&self, id: i32) -> Result<(), Error> {
        Ok(model::service::Entity::find_by_id(id)
            .one(self.get_db()?)
            .await?
            .ok_or("service not found")?
            .delete(self.get_db()?)
            .await
            .map(|_| ())?)
    }

    pub async fn list_gate(&self) -> Result<Vec<model::gate::Model>, Error> {
        let gates = model::gate::Entity::find()
            .all(self.get_db()?)
            .await?
            .iter_mut()
            .map(|gate| {
                gate.status = self.get_gate_status(gate);
                gate.to_owned()
            })
            .collect();
        Ok(gates)
    }

    pub async fn get_gate(&self, id: i32) -> Result<model::gate::Model, Error> {
        let mut gate = model::gate::Entity::find_by_id(id)
            .one(self.get_db()?)
            .await?
            .ok_or(format!("gate {} not found", id))?;
        gate.status = self.get_gate_status(&gate);
        Ok(gate)
    }

    pub async fn create_gate(
        &mut self,
        input: CreateGateInput,
    ) -> Result<model::gate::Model, Error> {
        let gate = input.into_active_model().insert(self.get_db()?).await?;
        let service = gate
            .find_related(model::service::Entity)
            .one(self.get_db()?)
            .await?
            .ok_or(format!("service for gate {} not found", gate.service_id))?;
        self.proxy_manager.create_proxy(
            gate.id,
            &gate.host,
            gate.port,
            &format!("{}:{}", &service.host, service.port),
        );
        Ok(gate)
    }

    pub async fn update_gate(
        &mut self,
        id: i32,
        update: UpdateGateInput,
    ) -> Result<model::gate::Model, Error> {
        let mut update = update.into_active_model();
        update.id = Unchanged(id);
        update.updated_at = Set(Some(chrono::Utc::now().naive_utc()));
        let gate = update.update(self.get_db()?).await?;
        let service = gate
            .find_related(model::service::Entity)
            .one(self.get_db()?)
            .await?
            .ok_or(format!("service for gate {} not found", id))?;
        self.proxy_manager.create_proxy(
            gate.host.clone(),
            gate.port,
            format!("{}:{}", &service.host, service.port),
        );
        Ok(gate)
    }

    pub async fn delete_gate(&mut self, id: i32) -> Result<(), Error> {
        let gate = model::gate::Entity::find_by_id(id)
            .one(self.get_db()?)
            .await?
            .ok_or("gate not found")?;
        self.proxy_manager.delete_proxy(gate.id).await?;
        Ok(gate.delete(self.get_db()?).await.map(|_| ())?)
    }

    async fn get_gate_status(&self, gate: &model::gate::Model) -> model::gate::Status {
        if self.proxy_manager.proxy_ready(gate.port).await {
            model::gate::Status::Active
        } else {
            model::gate::Status::InActive
        }
    }

    fn get_db(&self) -> Result<&model::Database, Error> {
        Ok(&self.db)
    }
}
