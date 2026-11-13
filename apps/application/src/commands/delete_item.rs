use std::sync::Arc;

use domain::Item;
use shared::{
    ApplicationError,
    ItemRepository,
};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteItemCommand {
    pub id: Uuid,
}

pub struct DeleteItemHandler<R: ?Sized = dyn ItemRepository<Entity = Item>> {
    repository: Arc<R>,
}

impl<R: ?Sized> DeleteItemHandler<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self {
            repository,
        }
    }
}

impl<R: ?Sized> Clone for DeleteItemHandler<R> {
    fn clone(&self) -> Self {
        Self {
            repository: Arc::clone(&self.repository),
        }
    }
}

impl<R: ItemRepository<Entity = Item> + ?Sized> DeleteItemHandler<R> {
    pub async fn execute(
        &self,
        command: DeleteItemCommand,
    ) -> Result<(), ApplicationError> {
        self.repository.delete(command.id).await?;
        Ok(())
    }
}
