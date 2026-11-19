use async_trait::async_trait;
use domain::Item;
use hiqlite::{
    Client,
    Param,
};
use shared::{
    Error,
    ItemRepository,
};
use uuid::Uuid;

use crate::models::item::ItemRow;

pub(crate) mod mappers;

use mappers::item as mapper;

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

#[async_trait]
impl ItemRepository for HiqliteItemRepository {
    type Entity = Item;

    async fn save(
        &self,
        item: &Item,
    ) -> Result<(), Error> {
        let affected = self
            .client
            .execute(
                "INSERT INTO items (id, title, created_at) VALUES ($1, $2, $3)",
                mapper::insert_parameters(item),
            )
            .await
            .map_err(mappers::errors::backend)?;
        if affected != 1 {
            return Err(Error::Unexpected(format!(
                "expected one inserted row, got {affected}"
            )));
        }
        Ok(())
    }

    async fn update(
        &self,
        item: &Item,
    ) -> Result<(), Error> {
        self.client
            .execute_returning_one(
                "UPDATE items SET title = $1 WHERE id = $2 RETURNING id",
                mapper::update_parameters(item),
            )
            .await
            .map_err(mappers::errors::mutation(item.id()))?;
        Ok(())
    }

    async fn delete(
        &self,
        id: Uuid,
    ) -> Result<(), Error> {
        self.client
            .execute_returning_one("DELETE FROM items WHERE id = $1 RETURNING id", vec![
                Param::from(id.to_string()),
            ])
            .await
            .map_err(mappers::errors::mutation(id))?;
        Ok(())
    }

    async fn delete_many(
        &self,
        ids: &[Uuid],
    ) -> Result<(), Error> {
        if ids.is_empty() {
            return Ok(());
        }
        let placeholders =
            (1..=ids.len()).map(|index| format!("${index}")).collect::<Vec<_>>().join(", ");
        let sql = format!("DELETE FROM items WHERE id IN ({placeholders})");
        let params = ids.iter().map(|id| Param::from(id.to_string())).collect();
        self.client.execute(sql, params).await.map_err(mappers::errors::backend)?;
        Ok(())
    }

    async fn find(
        &self,
        id: Uuid,
    ) -> Result<Option<Item>, Error> {
        let row = self
            .client
            .query_as_optional::<ItemRow, _>(
                "SELECT id, title, created_at FROM items WHERE id = $1",
                vec![Param::from(id.to_string())],
            )
            .await
            .map_err(mappers::errors::backend)?;
        row.map(mapper::from_row).transpose()
    }

    async fn list(&self) -> Result<Vec<Item>, Error> {
        let rows = self
            .client
            .query_as::<ItemRow, _>(
                "SELECT id, title, created_at FROM items ORDER BY created_at, id",
                Vec::new(),
            )
            .await
            .map_err(mappers::errors::backend)?;
        rows.into_iter().map(mapper::from_row).collect()
    }
}
