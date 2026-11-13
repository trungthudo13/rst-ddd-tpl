use domain::Item;

use super::pb::item::v1::ItemReply;

impl From<Item> for ItemReply {
    fn from(item: Item) -> Self {
        Self {
            id: item.id().to_string(),
            title: item.title().to_owned(),
            created_at: item.created_at(),
        }
    }
}
