use application::ApplicationError;
use axum::{
    Json,
    http::StatusCode,
    response::{
        IntoResponse,
        Response,
    },
};
use domain::DomainError;

use super::schemas::ErrorView;

pub enum ApiError {
    InvalidId,
    Application(ApplicationError),
}

impl ApiError {
    pub fn invalid_id() -> Self {
        Self::InvalidId
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            | Self::InvalidId => (StatusCode::BAD_REQUEST, "invalid item id".to_owned()),
            | Self::Application(ApplicationError::Domain(DomainError::EmptyTitle)) => {
                (
                    StatusCode::BAD_REQUEST,
                    "item title cannot be empty".to_owned(),
                )
            }
            | Self::Application(ApplicationError::NotFound(_)) => {
                (StatusCode::NOT_FOUND, "item not found".to_owned())
            }
            | Self::Application(ApplicationError::Repository(error)) => {
                tracing::error!(%error, "repository request failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_owned(),
                )
            }
        };

        (
            status,
            Json(ErrorView {
                error: message,
            }),
        )
            .into_response()
    }
}

impl From<ApplicationError> for ApiError {
    fn from(error: ApplicationError) -> Self {
        Self::Application(error)
    }
}
