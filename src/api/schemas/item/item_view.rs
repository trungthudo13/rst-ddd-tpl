use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct ItemView {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) created_at: i64,
}
