use domain::{
    Item,
    ItemSnapshot,
    mappers::item as domain_item,
};
use uuid::Uuid;

use crate::{
    CreateItemCommand,
    Error,
};

pub fn from_create(
    command: CreateItemCommand,
    id: Uuid,
    created_at: i64,
) -> Result<Item, Error> {
    let snapshot = ItemSnapshot {
        id,
        title: command.title,
        created_at,
    };
    domain_item::from_snapshot(snapshot)
}

pub fn to_create(item: &Item) -> CreateItemCommand {
    CreateItemCommand {
        title: item.title().to_owned(),
    }
}
