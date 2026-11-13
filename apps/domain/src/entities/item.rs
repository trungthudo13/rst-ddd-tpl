use shared::DomainError;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    id: Uuid,
    title: String,
    created_at: i64,
}

impl Item {
    pub fn new(
        id: Uuid,
        title: String,
        created_at: i64,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            id,
            title: Self::normalize_title(&title)?,
            created_at,
        })
    }

    pub fn rename(
        &mut self,
        title: String,
    ) -> Result<(), DomainError> {
        self.title = Self::normalize_title(&title)?;
        Ok(())
    }

    fn normalize_title(title: &str) -> Result<String, DomainError> {
        let title = title.trim();
        if title.is_empty() {
            return Err(DomainError::EmptyTitle);
        }
        Ok(title.to_owned())
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn created_at(&self) -> i64 {
        self.created_at
    }
}
