use application::GetItemQuery;
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
        item::ItemView,
    },
};

/// Get an item by ID.
#[utoipa::path(
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
    let id = Uuid::parse_str(&id).map_err(|_| ApiError::invalid_id())?;
    Ok(Json(
        state
            .get_item
            .execute(GetItemQuery {
                id,
            })
            .await?
            .into(),
    ))
}
