use model::{self, ActiveValue, NotSet, Set, Value};
use std::error::Error as StdError;

pub type Error = Box<dyn StdError + Send + Sync>;

trait IntoActiveValue<T> {
    fn into_active_value(self) -> T;
}

pub trait IntoActiveModel<T> {
    fn into_active_model(self) -> T;
}

pub struct CreateServiceInput {
    pub host: String,
    pub port: i32,
}

pub struct UpdateServiceInput {
    pub host: Option<String>,
    pub port: Option<i32>,
}

pub struct CreateGateInput {
    pub service_id: i32,
    pub host: String,
    pub port: i32,
}

pub struct UpdateGateInput {
    pub service_id: Option<i32>,
    pub host: Option<String>,
    pub port: Option<i32>,
}

impl IntoActiveModel<model::service::ActiveModel> for CreateServiceInput {
    fn into_active_model(self) -> model::service::ActiveModel {
        model::service::ActiveModel {
            host: self.host.into_active_value(),
            port: self.port.into_active_value(),
            ..Default::default()
        }
    }
}

impl IntoActiveModel<model::service::ActiveModel> for UpdateServiceInput {
    fn into_active_model(self) -> model::service::ActiveModel {
        model::service::ActiveModel {
            host: self.host.into_active_value(),
            port: self.port.into_active_value(),
            ..Default::default()
        }
    }
}

impl IntoActiveModel<model::gate::ActiveModel> for CreateGateInput {
    fn into_active_model(self) -> model::gate::ActiveModel {
        model::gate::ActiveModel {
            service_id: self.service_id.into_active_value(),
            host: self.host.into_active_value(),
            port: self.port.into_active_value(),
            ..Default::default()
        }
    }
}

impl IntoActiveModel<model::gate::ActiveModel> for UpdateGateInput {
    fn into_active_model(self) -> model::gate::ActiveModel {
        model::gate::ActiveModel {
            service_id: self.service_id.into_active_value(),
            host: self.host.into_active_value(),
            port: self.port.into_active_value(),
            ..Default::default()
        }
    }
}

impl<T: Into<Value>> IntoActiveValue<ActiveValue<T>> for T {
    fn into_active_value(self) -> ActiveValue<T> {
        Set(self)
    }
}

impl<T: Into<Value>> IntoActiveValue<ActiveValue<T>> for Option<T> {
    fn into_active_value(self) -> ActiveValue<T> {
        match self {
            Some(v) => Set(v),
            None => NotSet,
        }
    }
}
