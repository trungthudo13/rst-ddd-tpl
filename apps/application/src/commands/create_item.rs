use std::{
    sync::Arc,
    time::{
        SystemTime,
        UNIX_EPOCH,
    },
};

use domain::Item;
use shared::{
    ApplicationError,
    ItemRepository,
};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateItemCommand {
    pub title: String,
}

pub struct CreateItemHandler<R: ?Sized = dyn ItemRepository<Entity = Item>> {
    repository: Arc<R>,
}

impl<R: ?Sized> CreateItemHandler<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self {
            repository,
        }
    }
}

impl<R: ?Sized> Clone for CreateItemHandler<R> {
    fn clone(&self) -> Self {
        Self {
            repository: Arc::clone(&self.repository),
        }
    }
}

impl<R: ItemRepository<Entity = Item> + ?Sized> CreateItemHandler<R> {
    pub async fn execute(
        &self,
        command: CreateItemCommand,
    ) -> Result<Item, ApplicationError> {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or_default();
        let item = Item::new(Uuid::new_v4(), command.title, created_at)?;
        self.repository.save(&item).await?;
        Ok(item)
    }
}
