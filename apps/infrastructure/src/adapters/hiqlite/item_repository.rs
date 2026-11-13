use async_trait::async_trait;
use domain::Item;
use hiqlite::{
    Client,
    Param,
};
use shared::{
    ItemRepository,
    RepositoryError,
};
use uuid::Uuid;

use crate::models::item::ItemRow;

#[derive(Clone)]
pub struct HiqliteItemRepository {
    client: Client,
}

impl HiqliteItemRepository {
    pub fn new(client: Client) -> Self {
        Self {
            client,
        }
    }
}

fn from_row(row: ItemRow) -> Result<Item, RepositoryError> {
    let id = Uuid::parse_str(&row.id)
        .map_err(|error| RepositoryError::Backend(format!("invalid item id: {error}")))?;
    Item::new(id, row.title, row.created_at)
        .map_err(|error| RepositoryError::Backend(error.to_string()))
}

fn backend(error: hiqlite::Error) -> RepositoryError {
    RepositoryError::Backend(error.to_string())
}

fn mutation_error(
    error: hiqlite::Error,
    id: Uuid,
) -> RepositoryError {
    match error {
        | hiqlite::Error::QueryReturnedNoRows(_) => RepositoryError::NotFound(id),
        | error => backend(error),
    }
}

#[async_trait]
impl ItemRepository for HiqliteItemRepository {
    type Entity = Item;

    async fn save(
        &self,
        item: &Item,
    ) -> Result<(), RepositoryError> {
        let affected = self
            .client
            .execute(
                "INSERT INTO items (id, title, created_at) VALUES ($1, $2, $3)",
                vec![
                    Param::from(item.id().to_string()),
                    Param::from(item.title().to_owned()),
                    Param::from(item.created_at()),
                ],
            )
            .await
            .map_err(backend)?;
        if affected != 1 {
            return Err(RepositoryError::Backend(format!(
                "expected one inserted row, got {affected}"
            )));
        }
        Ok(())
    }

    async fn update(
        &self,
        item: &Item,
    ) -> Result<(), RepositoryError> {
        // RETURNING identifies the matched row without relying on affected-row counts.
        self.client
            .execute_returning_one(
                "UPDATE items SET title = $1 WHERE id = $2 RETURNING id",
                vec![Param::from(item.title().to_owned()), Param::from(item.id().to_string())],
            )
            .await
            .map_err(|error| mutation_error(error, item.id()))?;
        Ok(())
    }

    async fn delete(
        &self,
        id: Uuid,
    ) -> Result<(), RepositoryError> {
        self.client
            .execute_returning_one("DELETE FROM items WHERE id = $1 RETURNING id", vec![
                Param::from(id.to_string()),
            ])
            .await
            .map_err(|error| mutation_error(error, id))?;
        Ok(())
    }

    async fn delete_many(
        &self,
        ids: &[Uuid],
    ) -> Result<(), RepositoryError> {
        if ids.is_empty() {
            return Ok(());
        }
        // Only placeholder names are generated; all IDs remain bound parameters.
        let placeholders =
            (1..=ids.len()).map(|index| format!("${index}")).collect::<Vec<_>>().join(", ");
        let sql = format!("DELETE FROM items WHERE id IN ({placeholders})");
        let params = ids.iter().map(|id| Param::from(id.to_string())).collect();
        self.client.execute(sql, params).await.map_err(backend)?;
        Ok(())
    }

    async fn find(
        &self,
        id: Uuid,
    ) -> Result<Option<Item>, RepositoryError> {
        let row = self
            .client
            .query_as_optional::<ItemRow, _>(
                "SELECT id, title, created_at FROM items WHERE id = $1",
                vec![Param::from(id.to_string())],
            )
            .await
            .map_err(backend)?;
        row.map(from_row).transpose()
    }

    async fn list(&self) -> Result<Vec<Item>, RepositoryError> {
        let rows = self
            .client
            .query_as::<ItemRow, _>(
                "SELECT id, title, created_at FROM items ORDER BY created_at, id",
                Vec::new(),
            )
            .await
            .map_err(backend)?;
        rows.into_iter().map(from_row).collect()
    }
}
