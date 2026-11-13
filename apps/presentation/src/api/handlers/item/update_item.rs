use application::UpdateItemCommand;
use axum::{
    Json,
    extract::{
        Path,
        State,
    },
};
use uuid::Uuid;

use crate::ItemHandlers;
use crate::api::{
    error::ApiError,
    schemas::{
        ErrorView,
        item::{
            ItemView,
            UpdateItemRequest,
        },
    },
};

/// Update an item's title.
#[utoipa::path(
    patch,
    path = "/items/{id}",
    params(("id" = String, Path, description = "Item UUID")),
    request_body = UpdateItemRequest,
    responses(
        (status = 200, description = "Item updated", body = ItemView),
        (status = 400, description = "Invalid item or ID", body = ErrorView),
        (status = 404, description = "Item not found", body = ErrorView),
        (status = 500, description = "Internal server error", body = ErrorView),
    ),
)]
pub async fn update_item(
    State(state): State<ItemHandlers>,
    Path(id): Path<String>,
    Json(request): Json<UpdateItemRequest>,
) -> Result<Json<ItemView>, ApiError> {
    let id = Uuid::parse_str(&id).map_err(|_| ApiError::invalid_id())?;
    let command = UpdateItemCommand {
        id,
        title: request.title,
    };
    let item = state.update_item.execute(command).await?;
    Ok(Json(item.into()))
}
