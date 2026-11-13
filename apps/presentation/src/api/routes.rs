use axum::{
    Router,
    routing::{
        get,
        post,
    },
};

use super::handlers;
use crate::ItemHandlers;

pub fn health() -> Router<ItemHandlers> {
    Router::new().route("/health", get(handlers::health::health))
}

pub fn item() -> Router<ItemHandlers> {
    Router::new()
        .route(
            "/items",
            post(handlers::item::create_item).get(handlers::item::list_items),
        )
        .route("/items/bulk-delete", post(handlers::item::delete_items))
        .route(
            "/items/{id}",
            get(handlers::item::get_item)
                .patch(handlers::item::update_item)
                .delete(handlers::item::delete_item),
        )
}
