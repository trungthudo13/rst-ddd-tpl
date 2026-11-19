use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct DeleteItemsRequest {
    pub ids: Vec<String>,
}
