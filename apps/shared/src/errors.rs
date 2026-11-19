use uuid::Uuid;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("item title cannot be empty")]
    EmptyTitle,
    #[error("invalid item id")]
    InvalidId,
    #[error("item {0} was not found")]
    NotFound(Uuid),
    #[error("unexpected failure: {0}")]
    Unexpected(String),
}
