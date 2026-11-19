use axum::{
    Json,
    extract::State,
    http::StatusCode,
};

use crate::ItemHandlers;
use crate::api::mappers::item as mapper;
use crate::api::{
    error::ApiError,
    schemas::{
        ErrorView,
        item::DeleteItemsRequest,
    },
};

#[utoipa::path(
    tag = "item",
    description = "Delete a list of items.",
    post,
    path = "/items/bulk-delete",
    request_body = DeleteItemsRequest,
    responses(
        (status = 204, description = "Items deleted"),
        (status = 400, description = "Invalid item ID", body = ErrorView),
        (status = 500, description = "Internal server error", body = ErrorView),
    ),
)]
pub async fn delete_items(
    State(state): State<ItemHandlers>,
    Json(request): Json<DeleteItemsRequest>,
) -> Result<StatusCode, ApiError> {
    let command = mapper::to_delete_many(request)?;
    state.delete_items.execute(command).await?;
    Ok(StatusCode::NO_CONTENT)
}
