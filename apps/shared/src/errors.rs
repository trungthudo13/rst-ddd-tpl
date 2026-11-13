use uuid::Uuid;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("item title cannot be empty")]
    EmptyTitle,
}

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("item {0} was not found")]
    NotFound(Uuid),
    #[error("repository backend failed: {0}")]
    Backend(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Repository(RepositoryError),
    #[error("item {0} was not found")]
    NotFound(Uuid),
}

impl From<RepositoryError> for ApplicationError {
    fn from(error: RepositoryError) -> Self {
        match error {
            | RepositoryError::NotFound(id) => Self::NotFound(id),
            | error => Self::Repository(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_errors_keep_missing_records_distinct_from_backend_failures() {
        let id = Uuid::nil();
        assert!(matches!(
            ApplicationError::from(RepositoryError::NotFound(id)),
            ApplicationError::NotFound(missing) if missing == id
        ));
        assert!(matches!(
            ApplicationError::from(RepositoryError::Backend("offline".into())),
            ApplicationError::Repository(RepositoryError::Backend(_))
        ));
    }
}
