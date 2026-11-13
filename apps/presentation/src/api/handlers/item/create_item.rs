use application::CreateItemCommand;
use axum::{
    Json,
    extract::State,
    http::StatusCode,
};

use crate::ItemHandlers;
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

/// Create an item.
#[utoipa::path(
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
    let command = CreateItemCommand {
        title: request.title,
    };
    let item = state.create_item.execute(command).await?;
    Ok((StatusCode::CREATED, Json(item.into())))
}
