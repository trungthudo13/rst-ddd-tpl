use domain::Item;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct ItemView {
    id: String,
    title: String,
    created_at: i64,
}

impl From<Item> for ItemView {
    fn from(item: Item) -> Self {
        Self {
            id: item.id().to_string(),
            title: item.title().to_owned(),
            created_at: item.created_at(),
        }
    }
}
