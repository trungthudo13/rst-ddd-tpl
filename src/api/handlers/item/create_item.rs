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
        item::{
            CreateItemRequest,
            ItemView,
        },
    },
};

#[utoipa::path(
    tag = "item",
    description = "Create an item.",
    post,
    path = "/items",
    request_body = CreateItemRequest,
    responses(
        (status = 201, description = "Item created", body = ItemView),
        (status = 400, description = "Invalid item", body = ErrorView),
        (status = 500, description = "Internal server error", body = ErrorView),
    ),
)]
pub async fn create_item(
    State(state): State<ItemHandlers>,
    Json(request): Json<CreateItemRequest>,
) -> Result<(StatusCode, Json<ItemView>), ApiError> {
    let command = mapper::to_create(request);
    let item = state.create_item.execute(command).await?;
    Ok((StatusCode::CREATED, Json(mapper::to_view(&item))))
}
