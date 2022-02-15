use async_graphql::SimpleObject;
use chrono::NaiveDateTime;

#[derive(SimpleObject)]
#[graphql(name = "Service")]
pub struct ServiceDTO {
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
    pub deleted_at: Option<NaiveDateTime>,
    pub host: String,
    pub port: i32,
}

impl From<&model::service::Model> for ServiceDTO {
    fn from(service: &model::service::Model) -> Self {
        Self {
            id: service.id.clone(),
            created_at: service.created_at.clone(),
            updated_at: service.deleted_at.clone(),
            deleted_at: service.deleted_at.clone(),
            host: service.host.clone(),
            port: service.port.clone(),
        }
    }
}

impl From<model::service::Model> for ServiceDTO {
    fn from(service: model::service::Model) -> Self {
        Self::from(&service)
    }
}
