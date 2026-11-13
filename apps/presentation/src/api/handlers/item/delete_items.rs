use application::DeleteItemsCommand;
use axum::{
    Json,
    extract::State,
    http::StatusCode,
};
use uuid::Uuid;

use crate::ItemHandlers;
use crate::api::{
    error::ApiError,
    schemas::{
        ErrorView,
        item::DeleteItemsRequest,
    },
};

/// Delete a list of items.
#[utoipa::path(
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
    let ids = request
        .ids
        .iter()
        .map(|id| Uuid::parse_str(id).map_err(|_| ApiError::invalid_id()))
        .collect::<Result<Vec<_>, _>>()?;
    let command = DeleteItemsCommand {
        ids,
    };
    state.delete_items.execute(command).await?;
    Ok(StatusCode::NO_CONTENT)
}
