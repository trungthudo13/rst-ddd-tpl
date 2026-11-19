use shared::Error;
use uuid::Uuid;

pub(crate) fn backend(error: hiqlite::Error) -> Error {
    Error::Unexpected(error.to_string())
}

pub(crate) fn mutation(id: Uuid) -> impl FnOnce(hiqlite::Error) -> Error {
    move |error| {
        match error {
            | hiqlite::Error::QueryReturnedNoRows(_) => Error::NotFound(id),
            | error => backend(error),
        }
    }
}
