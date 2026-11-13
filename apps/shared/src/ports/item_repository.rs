use async_trait::async_trait;
use uuid::Uuid;

use crate::RepositoryError;

/// Item persistence contract shared by use cases and infrastructure adapters.
///
/// The implementer supplies the entity type so this contract does not depend on
/// a domain crate, database, or transport. The demo binds it to `domain::Item`.
#[async_trait]
pub trait ItemRepository: Send + Sync {
    type Entity: Send + Sync;

    /// Insert a new item. Existing identities must not be overwritten.
    async fn save(
        &self,
        item: &Self::Entity,
    ) -> Result<(), RepositoryError>;
    /// Persist a changed item without inserting a missing identity.
    /// Returns `RepositoryError::NotFound` if the item no longer exists.
    async fn update(
        &self,
        item: &Self::Entity,
    ) -> Result<(), RepositoryError>;
    /// Remove an existing item, or return `RepositoryError::NotFound`.
    async fn delete(
        &self,
        id: Uuid,
    ) -> Result<(), RepositoryError>;
    /// Remove the requested identities; missing IDs and an empty list are ignored.
    async fn delete_many(
        &self,
        ids: &[Uuid],
    ) -> Result<(), RepositoryError>;
    async fn find(
        &self,
        id: Uuid,
    ) -> Result<Option<Self::Entity>, RepositoryError>;
    async fn list(&self) -> Result<Vec<Self::Entity>, RepositoryError>;
}
