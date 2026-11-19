use application::Error;
use axum::{
    Json,
    http::StatusCode,
    response::{
        IntoResponse,
        Response,
    },
};

use super::schemas::ErrorView;

pub struct ApiError(pub Error);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self.0 {
            | Error::InvalidId => (StatusCode::BAD_REQUEST, "invalid item id"),
            | Error::EmptyTitle => (StatusCode::BAD_REQUEST, "item title cannot be empty"),
            | Error::NotFound(_) => (StatusCode::NOT_FOUND, "item not found"),
            | Error::Unexpected(diagnostic) => {
                tracing::error!(%diagnostic, "request failed");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
            }
        };
        let view = ErrorView {
            error: message.to_owned(),
        };
        (status, Json(view)).into_response()
    }
}

impl From<Error> for ApiError {
    fn from(error: Error) -> Self {
        Self(error)
    }
}
