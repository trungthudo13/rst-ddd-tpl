use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct ItemRow {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) created_at: i64,
}
