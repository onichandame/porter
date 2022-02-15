use async_graphql::InputObject;

#[derive(InputObject)]
#[graphql(name = "ServiceInput")]
pub struct ServiceInputDTO {
    pub host: String,
    pub port: i32,
}

impl ServiceInputDTO {
    pub fn into_active_model(self) -> model::service::ActiveModel {
        model::service::ActiveModel {
            ..Default::default()
        }
    }
}
