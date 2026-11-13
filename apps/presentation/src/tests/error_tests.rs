use application::ApplicationError;
use domain::DomainError;

use crate::api::error::ApiError;

#[test]
fn application_error_converts_to_api_error() {
    let error = ApplicationError::Domain(DomainError::EmptyTitle);

    assert!(matches!(ApiError::from(error), ApiError::Application(_)));
}
