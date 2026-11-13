use application::GetItemsQuery;
use axum::{
    Json,
    extract::State,
};

use crate::ItemHandlers;
use crate::api::{
    error::ApiError,
    schemas::{
        ErrorView,
        item::ItemView,
    },
};

/// List all items.
#[utoipa::path(
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
    Ok(Json(items.into_iter().map(ItemView::from).collect()))
}
