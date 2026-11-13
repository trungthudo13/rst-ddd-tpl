use std::{
    net::TcpListener,
    time::Duration,
};

use domain::Item;
use hiqlite::{
    AppliedMigration,
    Node,
    NodeConfig,
    Params,
};
use shared::{
    ItemRepository,
    RepositoryError,
};
use tempfile::tempdir;
use uuid::Uuid;

use crate::{
    Database,
    HiqliteItemRepository,
};

fn config(data_dir: String) -> NodeConfig {
    // Keep both reservations alive until the addresses have been selected.
    let raft = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let api = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    NodeConfig {
        node_id: 1,
        nodes: vec![Node {
            id: 1,
            addr_raft: raft.local_addr().unwrap().to_string(),
            addr_api: api.local_addr().unwrap().to_string(),
        }],
        listen_addr_api: "127.0.0.1".into(),
        listen_addr_raft: "127.0.0.1".into(),
        data_dir: data_dir.into(),
        filename_db: "hiqlite.db".into(),
        secret_raft: "test-raft-secret-at-least-16".into(),
        secret_api: "test-api-secret-at-least-16".into(),
        ..Default::default()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hiqlite_applies_embedded_migrations() {
    let data_dir = tempdir().unwrap();
    let database = tokio::time::timeout(
        Duration::from_secs(20),
        Database::connect(config(data_dir.path().to_string_lossy().into_owned())),
    )
    .await
    .expect("Hiqlite startup timed out")
    .expect("Hiqlite startup failed");

    let migrations: Vec<AppliedMigration> =
        database.client().query_map("SELECT * FROM _migrations", Params::new()).await.unwrap();
    assert_eq!(migrations.len(), 1);
    assert_eq!(migrations[0].name, "init");

    let tables: Vec<TableName> = database
        .client()
        .query_as(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name IN ('items', \
             '_migrations') ORDER BY name",
            Vec::new(),
        )
        .await
        .unwrap();
    let tables: Vec<_> = tables.into_iter().map(|table| table.name).collect();
    assert_eq!(tables, ["_migrations", "items"]);

    database.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hiqlite_repository_crud_respects_missing_records() {
    let data_dir = tempdir().unwrap();
    let database = tokio::time::timeout(
        Duration::from_secs(20),
        Database::connect(config(data_dir.path().to_string_lossy().into_owned())),
    )
    .await
    .expect("Hiqlite startup timed out")
    .expect("Hiqlite startup failed");
    let repository = HiqliteItemRepository::new(database.client());
    let item = Item::new(Uuid::new_v4(), "original".into(), 1).unwrap();
    assert!(repository.list().await.unwrap().is_empty());
    assert_eq!(repository.find(item.id()).await.unwrap(), None);
    repository.save(&item).await.unwrap();
    assert!(repository.save(&item).await.is_err());
    assert_eq!(
        repository.find(item.id()).await.unwrap(),
        Some(item.clone())
    );

    let updated = Item::new(item.id(), "updated".into(), item.created_at()).unwrap();
    repository.update(&updated).await.unwrap();

    repository.update(&updated).await.unwrap();
    assert_eq!(
        repository.find(item.id()).await.unwrap(),
        Some(updated.clone())
    );
    repository.delete(item.id()).await.unwrap();
    assert_eq!(repository.find(item.id()).await.unwrap(), None);
    assert!(matches!(
        repository.update(&updated).await,
        Err(RepositoryError::NotFound(id)) if id == item.id()
    ));
    assert!(matches!(
        repository.delete(item.id()).await,
        Err(RepositoryError::NotFound(id)) if id == item.id()
    ));
    assert_eq!(repository.find(item.id()).await.unwrap(), None);

    let other = Item::new(Uuid::new_v4(), "other".into(), 2).unwrap();
    repository.save(&other).await.unwrap();
    repository.delete_many(&[]).await.unwrap();
    assert_eq!(repository.list().await.unwrap(), vec![other.clone()]);
    repository.delete_many(&[other.id(), other.id(), Uuid::new_v4()]).await.unwrap();
    assert!(repository.list().await.unwrap().is_empty());

    database.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hiqlite_retains_items_and_migrations_after_restart() {
    let data_dir = tempdir().unwrap();
    let config = config(data_dir.path().to_string_lossy().into_owned());
    let restart_config = NodeConfig {
        nodes: config
            .nodes
            .iter()
            .map(|node| {
                Node {
                    id: node.id,
                    addr_raft: node.addr_raft.clone(),
                    addr_api: node.addr_api.clone(),
                }
            })
            .collect(),
        ..self::config(data_dir.path().to_string_lossy().into_owned())
    };
    let database = tokio::time::timeout(Duration::from_secs(20), Database::connect(config))
        .await
        .unwrap()
        .unwrap();
    let item = Item::new(Uuid::new_v4(), "persisted".into(), 42).unwrap();
    HiqliteItemRepository::new(database.client()).save(&item).await.unwrap();
    database.shutdown().await.unwrap();
    drop(database);

    let database = tokio::time::timeout(Duration::from_secs(20), Database::connect(restart_config))
        .await
        .unwrap()
        .unwrap();
    let repository = HiqliteItemRepository::new(database.client());
    assert_eq!(repository.find(item.id()).await.unwrap(), Some(item));
    let migrations: Vec<AppliedMigration> =
        database.client().query_map("SELECT * FROM _migrations", Params::new()).await.unwrap();
    assert_eq!(migrations.len(), 1);
    database.shutdown().await.unwrap();
}

#[derive(serde::Deserialize)]
struct TableName {
    name: String,
}
