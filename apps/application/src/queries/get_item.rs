use std::sync::Arc;

use domain::Item;
use shared::{
    Error,
    ItemRepository,
};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetItemQuery {
    pub id: Uuid,
}

pub struct GetItemHandler<R: ?Sized = dyn ItemRepository<Entity = Item>> {
    repository: Arc<R>,
}

impl<R: ?Sized> GetItemHandler<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self {
            repository,
        }
    }
}

impl<R: ?Sized> Clone for GetItemHandler<R> {
    fn clone(&self) -> Self {
        Self {
            repository: Arc::clone(&self.repository),
        }
    }
}

impl<R: ItemRepository<Entity = Item> + ?Sized> GetItemHandler<R> {
    pub async fn execute(
        &self,
        query: GetItemQuery,
    ) -> Result<Item, Error> {
        self.repository.find(query.id).await?.ok_or(Error::NotFound(query.id))
    }
}
