use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemSnapshot {
    pub id: Uuid,
    pub title: String,
    pub created_at: i64,
}
