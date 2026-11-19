use std::{
    collections::HashMap,
    sync::Mutex,
};

use async_trait::async_trait;
use domain::Item;
use shared::{
    Error,
    ItemRepository,
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

    fn ensure_available(&self) -> Result<(), Error> {
        if self.unavailable {
            return Err(Error::Unexpected("private provider diagnostic".into()));
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
    ) -> Result<(), Error> {
        self.ensure_available()?;
        let mut items = self.items.lock().unwrap();
        if items.contains_key(&item.id()) {
            return Err(Error::Unexpected("duplicate identity".into()));
        }
        items.insert(item.id(), item.clone());
        Ok(())
    }

    async fn update(
        &self,
        item: &Item,
    ) -> Result<(), Error> {
        self.ensure_available()?;
        let mut items = self.items.lock().unwrap();
        let existing = items.get_mut(&item.id()).ok_or(Error::NotFound(item.id()))?;
        *existing = item.clone();
        Ok(())
    }

    async fn delete(
        &self,
        id: Uuid,
    ) -> Result<(), Error> {
        self.ensure_available()?;
        self.items.lock().unwrap().remove(&id).ok_or(Error::NotFound(id))?;
        Ok(())
    }

    async fn delete_many(
        &self,
        ids: &[Uuid],
    ) -> Result<(), Error> {
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
    ) -> Result<Option<Item>, Error> {
        self.ensure_available()?;
        Ok(self.items.lock().unwrap().get(&id).cloned())
    }

    async fn list(&self) -> Result<Vec<Item>, Error> {
        self.ensure_available()?;
        let mut items: Vec<_> = self.items.lock().unwrap().values().cloned().collect();
        items.sort_by_key(|item| (item.created_at(), item.id()));
        Ok(items)
    }
}
