use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct CreateItemRequest {
    pub title: String,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateItemRequest {
    pub title: String,
}

#[derive(Deserialize, ToSchema)]
pub struct DeleteItemsRequest {
    pub ids: Vec<String>,
}
