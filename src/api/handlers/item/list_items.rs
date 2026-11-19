use application::GetItemsQuery;
use axum::{
    Json,
    extract::State,
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
    description = "List all items.",
    get,
    path = "/items",
    responses(
        (status = 200, description = "Items returned", body = Vec<ItemView>),
        (status = 500, description = "Internal server error", body = ErrorView),
    ),
)]
pub async fn list_items(
    State(state): State<ItemHandlers>
) -> Result<Json<Vec<ItemView>>, ApiError> {
    let items = state.get_items.execute(GetItemsQuery).await?;
    Ok(Json(items.iter().map(mapper::to_view).collect()))
}
