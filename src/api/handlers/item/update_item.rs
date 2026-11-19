use axum::{
    Json,
    extract::{
        Path,
        State,
    },
};

use crate::ItemHandlers;
use crate::api::mappers::item as mapper;
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

#[utoipa::path(
    tag = "item",
    description = "Update an item's title.",
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
    let command = mapper::to_update(&id, request)?;
    let item = state.update_item.execute(command).await?;
    Ok(Json(mapper::to_view(&item)))
}
