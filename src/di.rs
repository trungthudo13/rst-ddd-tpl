use std::sync::Arc;

use application::{
    CreateItemHandler,
    DeleteItemHandler,
    DeleteItemsHandler,
    GetItemHandler,
    GetItemsHandler,
    UpdateItemHandler,
};
use domain::Item;
use presentation::ItemHandlers;
use shared::ItemRepository;

pub(crate) fn item_handlers(repository: Arc<dyn ItemRepository<Entity = Item>>) -> ItemHandlers {
    ItemHandlers {
        create_item: CreateItemHandler::new(Arc::clone(&repository)),
        update_item: UpdateItemHandler::new(Arc::clone(&repository)),
        delete_item: DeleteItemHandler::new(Arc::clone(&repository)),
        delete_items: DeleteItemsHandler::new(Arc::clone(&repository)),
        get_item: GetItemHandler::new(Arc::clone(&repository)),
        get_items: GetItemsHandler::new(repository),
    }
}
