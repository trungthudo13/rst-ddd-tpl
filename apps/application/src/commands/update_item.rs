use std::sync::Arc;

use domain::Item;
use shared::{
    ApplicationError,
    ItemRepository,
};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateItemCommand {
    pub id: Uuid,
    pub title: String,
}

pub struct UpdateItemHandler<R: ?Sized = dyn ItemRepository<Entity = Item>> {
    repository: Arc<R>,
}

impl<R: ?Sized> UpdateItemHandler<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self {
            repository,
        }
    }
}

impl<R: ?Sized> Clone for UpdateItemHandler<R> {
    fn clone(&self) -> Self {
        Self {
            repository: Arc::clone(&self.repository),
        }
    }
}

impl<R: ItemRepository<Entity = Item> + ?Sized> UpdateItemHandler<R> {
    pub async fn execute(
        &self,
        command: UpdateItemCommand,
    ) -> Result<Item, ApplicationError> {
        let mut item = self
            .repository
            .find(command.id)
            .await?
            .ok_or(ApplicationError::NotFound(command.id))?;
        item.rename(command.title)?;
        self.repository.update(&item).await?;
        Ok(item)
    }
}
