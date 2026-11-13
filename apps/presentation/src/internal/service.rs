use application::{
    CreateItemHandler,
    DeleteItemHandler,
    DeleteItemsHandler,
    GetItemHandler,
    GetItemsHandler,
    UpdateItemHandler,
};

use super::pb::item::v1::item_internal_server::ItemInternalServer;
use crate::ItemHandlers;

#[derive(Clone)]
pub struct ItemGrpcService {
    handlers: ItemHandlers,
}

impl ItemGrpcService {
    pub(crate) fn create_item(&self) -> &CreateItemHandler {
        &self.handlers.create_item
    }
    pub(crate) fn update_item(&self) -> &UpdateItemHandler {
        &self.handlers.update_item
    }
    pub(crate) fn delete_item(&self) -> &DeleteItemHandler {
        &self.handlers.delete_item
    }
    pub(crate) fn delete_items(&self) -> &DeleteItemsHandler {
        &self.handlers.delete_items
    }
    pub(crate) fn get_item(&self) -> &GetItemHandler {
        &self.handlers.get_item
    }
    pub(crate) fn get_items(&self) -> &GetItemsHandler {
        &self.handlers.get_items
    }
}

pub fn server(handlers: ItemHandlers) -> ItemInternalServer<ItemGrpcService> {
    ItemInternalServer::new(ItemGrpcService {
        handlers,
    })
}
