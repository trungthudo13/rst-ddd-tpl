use crate::{
    Error,
    Item,
    ItemSnapshot,
};

pub fn to_snapshot(item: &Item) -> ItemSnapshot {
    ItemSnapshot {
        id: item.id(),
        title: item.title().to_owned(),
        created_at: item.created_at(),
    }
}

pub fn from_snapshot(snapshot: ItemSnapshot) -> Result<Item, Error> {
    Item::new(snapshot.id, snapshot.title, snapshot.created_at)
}
