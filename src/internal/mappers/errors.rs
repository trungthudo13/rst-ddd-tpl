use application::Error;
use tonic::Status;

pub fn map_error(error: Error) -> Status {
    match error {
        | Error::InvalidId => Status::invalid_argument("invalid item id"),
        | Error::EmptyTitle => Status::invalid_argument("item title cannot be empty"),
        | Error::NotFound(_) => Status::not_found("item not found"),
        | Error::Unexpected(diagnostic) => {
            tracing::error!(%diagnostic, "request failed");
            Status::internal("internal server error")
        }
    }
}
