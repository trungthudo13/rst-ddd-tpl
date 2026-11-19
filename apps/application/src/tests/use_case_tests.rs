use std::sync::{
    Arc,
    Mutex,
};

use async_trait::async_trait;
use domain::Item;
use shared::{
    Error,
    ItemRepository,
};
use uuid::Uuid;

use crate::{
    CreateItemCommand,
    CreateItemHandler,
    DeleteItemCommand,
    DeleteItemHandler,
    DeleteItemsCommand,
    DeleteItemsHandler,
    GetItemHandler,
    GetItemQuery,
    GetItemsHandler,
    GetItemsQuery,
    UpdateItemCommand,
    UpdateItemHandler,
};

#[derive(Clone, Default)]
struct InMemoryRepository {
    items: Arc<Mutex<Vec<Item>>>,
    missing_on_update: bool,
}

#[async_trait]
impl ItemRepository for InMemoryRepository {
    type Entity = Item;

    async fn save(
        &self,
        item: &Item,
    ) -> Result<(), Error> {
        self.items.lock().unwrap().push(item.clone());
        Ok(())
    }

    async fn update(
        &self,
        item: &Item,
    ) -> Result<(), Error> {
        let mut items = self.items.lock().unwrap();
        if self.missing_on_update {
            items.retain(|existing| existing.id() != item.id());
            return Err(Error::NotFound(item.id()));
        }
        let existing = items
            .iter_mut()
            .find(|existing| existing.id() == item.id())
            .ok_or(Error::NotFound(item.id()))?;
        *existing = item.clone();
        Ok(())
    }

    async fn delete(
        &self,
        id: Uuid,
    ) -> Result<(), Error> {
        let mut items = self.items.lock().unwrap();
        let index = items.iter().position(|item| item.id() == id).ok_or(Error::NotFound(id))?;
        items.remove(index);
        Ok(())
    }

    async fn delete_many(
        &self,
        ids: &[Uuid],
    ) -> Result<(), Error> {
        self.items.lock().unwrap().retain(|item| !ids.contains(&item.id()));
        Ok(())
    }

    async fn find(
        &self,
        id: Uuid,
    ) -> Result<Option<Item>, Error> {
        Ok(self.items.lock().unwrap().iter().find(|item| item.id() == id).cloned())
    }

    async fn list(&self) -> Result<Vec<Item>, Error> {
        Ok(self.items.lock().unwrap().clone())
    }
}

struct Handlers {
    create_item: CreateItemHandler<InMemoryRepository>,
    delete_item: DeleteItemHandler<InMemoryRepository>,
    delete_items: DeleteItemsHandler<InMemoryRepository>,
    update_item: UpdateItemHandler<InMemoryRepository>,
    get_item: GetItemHandler<InMemoryRepository>,
    get_items: GetItemsHandler<InMemoryRepository>,
}

fn build_handlers(repository: Arc<InMemoryRepository>) -> Handlers {
    Handlers {
        create_item: CreateItemHandler::new(Arc::clone(&repository)),
        delete_item: DeleteItemHandler::new(Arc::clone(&repository)),
        delete_items: DeleteItemsHandler::new(Arc::clone(&repository)),
        update_item: UpdateItemHandler::new(Arc::clone(&repository)),
        get_item: GetItemHandler::new(Arc::clone(&repository)),
        get_items: GetItemsHandler::new(repository),
    }
}

#[tokio::test]
async fn use_cases_accept_an_implementation_through_the_shared_interface() {
    let repository: Arc<dyn ItemRepository<Entity = Item>> =
        Arc::new(InMemoryRepository::default());
    let create = CreateItemHandler::new(Arc::clone(&repository));
    let get = GetItemHandler::new(repository);

    let create_item_command = CreateItemCommand {
        title: "injected".into(),
    };
    let item = create.execute(create_item_command).await.unwrap();
    let get_item_query = GetItemQuery {
        id: item.id(),
    };
    let found = get.execute(get_item_query).await.unwrap();

    assert_eq!(found, item);
}

#[tokio::test]
async fn create_get_and_list_use_repository_port() {
    let repository = Arc::new(InMemoryRepository::default());
    let handlers = build_handlers(repository);

    let create_item_command = CreateItemCommand {
        title: "  first item  ".into(),
    };
    let item = handlers.create_item.execute(create_item_command).await.unwrap();

    assert_eq!(item.title(), "first item");
    let get_item_query = GetItemQuery {
        id: item.id(),
    };
    assert_eq!(
        handlers.get_item.execute(get_item_query).await.unwrap(),
        item
    );
    assert_eq!(
        handlers.get_items.execute(GetItemsQuery).await.unwrap(),
        vec![item]
    );
}

#[tokio::test]
async fn update_changes_title_and_preserves_created_at() {
    let repository = Arc::new(InMemoryRepository::default());
    let handlers = build_handlers(repository);
    let create_item_command = CreateItemCommand {
        title: "first item".into(),
    };
    let item = handlers.create_item.execute(create_item_command).await.unwrap();

    let update_item_command = UpdateItemCommand {
        id: item.id(),
        title: "  updated item  ".into(),
    };
    let updated = handlers.update_item.execute(update_item_command).await.unwrap();

    assert_eq!(updated.title(), "updated item");
    assert_eq!(updated.created_at(), item.created_at());
    let get_item_query = GetItemQuery {
        id: item.id(),
    };
    assert_eq!(
        handlers.get_item.execute(get_item_query).await.unwrap(),
        updated
    );

    let update_item_command = UpdateItemCommand {
        id: item.id(),
        title: " ".into(),
    };
    assert!(matches!(
        handlers.update_item.execute(update_item_command).await,
        Err(Error::EmptyTitle)
    ));
    let get_item_query = GetItemQuery {
        id: item.id(),
    };
    assert_eq!(
        handlers.get_item.execute(get_item_query).await.unwrap(),
        updated
    );
}

#[tokio::test]
async fn update_reports_an_item_deleted_after_it_was_loaded() {
    let in_memory_repository = InMemoryRepository {
        missing_on_update: true,
        ..Default::default()
    };
    let repository = Arc::new(in_memory_repository);
    let handlers = build_handlers(repository);
    let create_item_command = CreateItemCommand {
        title: "first".into(),
    };
    let item = handlers.create_item.execute(create_item_command).await.unwrap();

    let update_item_command = UpdateItemCommand {
        id: item.id(),
        title: "updated".into(),
    };
    assert!(matches!(
        handlers.update_item.execute(update_item_command).await,
        Err(Error::NotFound(id)) if id == item.id()
    ));
    assert!(handlers.get_items.execute(GetItemsQuery).await.unwrap().is_empty());
}

#[tokio::test]
async fn rejects_blank_title_and_reports_missing_item() {
    let repository = Arc::new(InMemoryRepository::default());
    let handlers = build_handlers(repository);

    let create_item_command = CreateItemCommand {
        title: "  ".into(),
    };
    assert!(matches!(
        handlers.create_item.execute(create_item_command).await,
        Err(Error::EmptyTitle)
    ));
    let get_item_query = GetItemQuery {
        id: Uuid::new_v4(),
    };
    assert!(matches!(
        handlers.get_item.execute(get_item_query).await,
        Err(Error::NotFound(_))
    ));
    let update_item_command = UpdateItemCommand {
        id: Uuid::new_v4(),
        title: "missing".into(),
    };
    assert!(matches!(
        handlers.update_item.execute(update_item_command).await,
        Err(Error::NotFound(_))
    ));
    let delete_item_command = DeleteItemCommand {
        id: Uuid::new_v4(),
    };
    assert!(matches!(
        handlers.delete_item.execute(delete_item_command).await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn deletes_one_item_and_bulk_items() {
    let repository = Arc::new(InMemoryRepository::default());
    let handlers = build_handlers(repository);
    let create_item_command = CreateItemCommand {
        title: "first".into(),
    };
    let first = handlers.create_item.execute(create_item_command).await.unwrap();
    let create_item_command = CreateItemCommand {
        title: "second".into(),
    };
    let second = handlers.create_item.execute(create_item_command).await.unwrap();

    let delete_item_command = DeleteItemCommand {
        id: first.id(),
    };
    handlers.delete_item.execute(delete_item_command).await.unwrap();
    let second_id = second.id();
    assert_eq!(
        handlers.get_items.execute(GetItemsQuery).await.unwrap(),
        vec![second]
    );

    let remaining_ids = [second_id];
    let delete_items_command = DeleteItemsCommand {
        ids: remaining_ids.to_vec(),
    };
    handlers.delete_items.execute(delete_items_command).await.unwrap();
    assert!(handlers.get_items.execute(GetItemsQuery).await.unwrap().is_empty());
}
