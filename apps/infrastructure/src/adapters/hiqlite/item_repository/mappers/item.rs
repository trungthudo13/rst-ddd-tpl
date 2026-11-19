use domain::{
    Item,
    ItemSnapshot,
    mappers::item as domain_item,
};
use hiqlite::Param;
use shared::Error;
use uuid::Uuid;

use crate::models::item::ItemRow;

pub(crate) fn from_row(row: ItemRow) -> Result<Item, Error> {
    let id = Uuid::parse_str(&row.id)
        .map_err(|error| Error::Unexpected(format!("invalid stored item id: {error}")))?;
    let snapshot = ItemSnapshot {
        id,
        title: row.title,
        created_at: row.created_at,
    };
    domain_item::from_snapshot(snapshot)
        .map_err(|error| Error::Unexpected(format!("invalid stored item: {error}")))
}

pub(crate) fn to_row(item: &Item) -> ItemRow {
    let snapshot = domain_item::to_snapshot(item);
    ItemRow {
        id: snapshot.id.to_string(),
        title: snapshot.title,
        created_at: snapshot.created_at,
    }
}

pub(crate) fn insert_parameters(item: &Item) -> Vec<Param> {
    let row = to_row(item);
    vec![Param::from(row.id), Param::from(row.title), Param::from(row.created_at)]
}

pub(crate) fn update_parameters(item: &Item) -> Vec<Param> {
    let row = to_row(item);
    vec![Param::from(row.title), Param::from(row.id)]
}
