use application::DeleteItemCommand;
use axum::{
    extract::{
        Path,
        State,
    },
    http::StatusCode,
};
use uuid::Uuid;

use crate::ItemHandlers;
use crate::api::{
    error::ApiError,
    schemas::ErrorView,
};

/// Delete an item.
#[utoipa::path(
    delete,
    path = "/items/{id}",
    params(("id" = String, Path, description = "Item UUID")),
    responses(
        (status = 204, description = "Item deleted"),
        (status = 400, description = "Invalid item ID", body = ErrorView),
        (status = 404, description = "Item not found", body = ErrorView),
        (status = 500, description = "Internal server error", body = ErrorView),
    ),
)]
pub async fn delete_item(
    State(state): State<ItemHandlers>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = Uuid::parse_str(&id).map_err(|_| ApiError::invalid_id())?;
    let command = DeleteItemCommand {
        id,
    };
    state.delete_item.execute(command).await?;
    Ok(StatusCode::NO_CONTENT)
}
