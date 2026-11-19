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
        item::ItemView,
    },
};

#[utoipa::path(
    tag = "item",
    description = "Get an item by ID.",
    get,
    path = "/items/{id}",
    params(("id" = String, Path, description = "Item UUID")),
    responses(
        (status = 200, description = "Item found", body = ItemView),
        (status = 400, description = "Invalid item ID", body = ErrorView),
        (status = 404, description = "Item not found", body = ErrorView),
        (status = 500, description = "Internal server error", body = ErrorView),
    ),
)]
pub async fn get_item(
    State(state): State<ItemHandlers>,
    Path(id): Path<String>,
) -> Result<Json<ItemView>, ApiError> {
    let query = mapper::to_get(&id)?;
    let item = state.get_item.execute(query).await?;
    Ok(Json(mapper::to_view(&item)))
}
