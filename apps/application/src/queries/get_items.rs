use std::sync::Arc;

use domain::Item;
use shared::{
    ApplicationError,
    ItemRepository,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GetItemsQuery;

pub struct GetItemsHandler<R: ?Sized = dyn ItemRepository<Entity = Item>> {
    repository: Arc<R>,
}

impl<R: ?Sized> GetItemsHandler<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self {
            repository,
        }
    }
}

impl<R: ?Sized> Clone for GetItemsHandler<R> {
    fn clone(&self) -> Self {
        Self {
            repository: Arc::clone(&self.repository),
        }
    }
}

impl<R: ItemRepository<Entity = Item> + ?Sized> GetItemsHandler<R> {
    pub async fn execute(
        &self,
        _query: GetItemsQuery,
    ) -> Result<Vec<Item>, ApplicationError> {
        Ok(self.repository.list().await?)
    }
}
