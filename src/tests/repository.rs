use std::{
    collections::HashMap,
    sync::Mutex,
};

use async_trait::async_trait;
use domain::Item;
use shared::{
    ItemRepository,
    RepositoryError,
};
use uuid::Uuid;

#[derive(Default)]
pub(super) struct TestRepository {
    items: Mutex<HashMap<Uuid, Item>>,
    unavailable: bool,
}

impl TestRepository {
    pub(super) fn unavailable() -> Self {
        Self {
            unavailable: true,
            ..Default::default()
        }
    }

    fn ensure_available(&self) -> Result<(), RepositoryError> {
        if self.unavailable {
            return Err(RepositoryError::Backend(
                "private provider diagnostic".into(),
            ));
        }
        Ok(())
    }
}

#[async_trait]
impl ItemRepository for TestRepository {
    type Entity = Item;

    async fn save(
        &self,
        item: &Item,
    ) -> Result<(), RepositoryError> {
        self.ensure_available()?;
        let mut items = self.items.lock().unwrap();
        if items.contains_key(&item.id()) {
            return Err(RepositoryError::Backend("duplicate identity".into()));
        }
        items.insert(item.id(), item.clone());
        Ok(())
    }

    async fn update(
        &self,
        item: &Item,
    ) -> Result<(), RepositoryError> {
        self.ensure_available()?;
        let mut items = self.items.lock().unwrap();
        let existing = items.get_mut(&item.id()).ok_or(RepositoryError::NotFound(item.id()))?;
        *existing = item.clone();
        Ok(())
    }

    async fn delete(
        &self,
        id: Uuid,
    ) -> Result<(), RepositoryError> {
        self.ensure_available()?;
        self.items.lock().unwrap().remove(&id).ok_or(RepositoryError::NotFound(id))?;
        Ok(())
    }

    async fn delete_many(
        &self,
        ids: &[Uuid],
    ) -> Result<(), RepositoryError> {
        self.ensure_available()?;
        let mut items = self.items.lock().unwrap();
        for id in ids {
            items.remove(id);
        }
        Ok(())
    }

    async fn find(
        &self,
        id: Uuid,
    ) -> Result<Option<Item>, RepositoryError> {
        self.ensure_available()?;
        Ok(self.items.lock().unwrap().get(&id).cloned())
    }

    async fn list(&self) -> Result<Vec<Item>, RepositoryError> {
        self.ensure_available()?;
        let mut items: Vec<_> = self.items.lock().unwrap().values().cloned().collect();
        items.sort_by_key(|item| (item.created_at(), item.id()));
        Ok(items)
    }
}
