use async_graphql::{Context, Object, Result};
use porter_core::Porter;

use crate::dto::ServiceDTO;

use self::dto::ServiceInputDTO;

mod dto;

#[derive(Default)]
pub struct ServiceQuery;
#[Object]
impl ServiceQuery {
    async fn create_service(
        &self,
        ctx: &Context<'_>,
        input: ServiceInputDTO,
    ) -> Result<ServiceDTO> {
        let porter = ctx.data::<Porter>()?;
        Ok(ServiceDTO::from(porter.create_service(input.into_active_model()).await?))
    }
}
