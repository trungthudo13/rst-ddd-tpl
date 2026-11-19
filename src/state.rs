use application::{
    CreateItemHandler,
    DeleteItemHandler,
    DeleteItemsHandler,
    GetItemHandler,
    GetItemsHandler,
    UpdateItemHandler,
};

pub struct ItemHandlers {
    pub create_item: CreateItemHandler,
    pub update_item: UpdateItemHandler,
    pub delete_item: DeleteItemHandler,
    pub delete_items: DeleteItemsHandler,
    pub get_item: GetItemHandler,
    pub get_items: GetItemsHandler,
}

impl Clone for ItemHandlers {
    fn clone(&self) -> Self {
        Self {
            create_item: self.create_item.clone(),
            update_item: self.update_item.clone(),
            delete_item: self.delete_item.clone(),
            delete_items: self.delete_items.clone(),
            get_item: self.get_item.clone(),
            get_items: self.get_items.clone(),
        }
    }
}
