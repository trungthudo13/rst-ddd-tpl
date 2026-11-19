use async_trait::async_trait;
use uuid::Uuid;

use crate::Error;

#[async_trait]
pub trait ItemRepository: Send + Sync {
    type Entity: Send + Sync;

    async fn save(
        &self,
        item: &Self::Entity,
    ) -> Result<(), Error>;
    async fn update(
        &self,
        item: &Self::Entity,
    ) -> Result<(), Error>;
    async fn delete(
        &self,
        id: Uuid,
    ) -> Result<(), Error>;
    async fn delete_many(
        &self,
        ids: &[Uuid],
    ) -> Result<(), Error>;
    async fn find(
        &self,
        id: Uuid,
    ) -> Result<Option<Self::Entity>, Error>;
    async fn list(&self) -> Result<Vec<Self::Entity>, Error>;
}
