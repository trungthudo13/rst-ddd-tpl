use presentation::api::{
    handlers,
    schemas::{
        ErrorView,
        item::{
            CreateItemRequest,
            DeleteItemsRequest,
            ItemView,
            UpdateItemRequest,
        },
    },
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::health::health,
        handlers::item::create_item::create_item,
        handlers::item::delete_item::delete_item,
        handlers::item::delete_items::delete_items,
        handlers::item::get_item::get_item,
        handlers::item::list_items::list_items,
        handlers::item::update_item::update_item,
    ),
    components(schemas(
        CreateItemRequest,
        UpdateItemRequest,
        DeleteItemsRequest,
        ItemView,
        ErrorView
    ))
)]
pub(crate) struct ApiDoc;
