use std::sync::Arc;

use domain::Item;
use shared::{
    ApplicationError,
    ItemRepository,
};
use uuid::Uuid;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeleteItemsCommand {
    pub ids: Vec<Uuid>,
}

pub struct DeleteItemsHandler<R: ?Sized = dyn ItemRepository<Entity = Item>> {
    repository: Arc<R>,
}

impl<R: ?Sized> DeleteItemsHandler<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self {
            repository,
        }
    }
}

impl<R: ?Sized> Clone for DeleteItemsHandler<R> {
    fn clone(&self) -> Self {
        Self {
            repository: Arc::clone(&self.repository),
        }
    }
}

impl<R: ItemRepository<Entity = Item> + ?Sized> DeleteItemsHandler<R> {
    pub async fn execute(
        &self,
        command: DeleteItemsCommand,
    ) -> Result<(), ApplicationError> {
        self.repository.delete_many(&command.ids).await?;
        Ok(())
    }
}
