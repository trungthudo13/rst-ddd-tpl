use application::ApplicationError;
use domain::DomainError;
use tonic::Status;

pub(crate) fn map_error(error: ApplicationError) -> Status {
    match &error {
        | ApplicationError::Domain(DomainError::EmptyTitle) => {
            Status::invalid_argument("item title cannot be empty")
        }
        | ApplicationError::NotFound(_) => Status::not_found("item not found"),
        | ApplicationError::Repository(error) => {
            tracing::error!(%error, "repository request failed");
            Status::internal("internal server error")
        }
    }
}
