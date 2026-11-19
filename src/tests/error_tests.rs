use application::Error;
use axum::{
    body::to_bytes,
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;
use tonic::Code;
use uuid::Uuid;

use crate::{
    api::error::ApiError,
    internal::mappers::errors::map_error,
};

#[tokio::test]
async fn every_service_error_has_a_safe_http_response() {
    let cases = [
        (Error::InvalidId, StatusCode::BAD_REQUEST, "invalid item id"),
        (
            Error::EmptyTitle,
            StatusCode::BAD_REQUEST,
            "item title cannot be empty",
        ),
        (
            Error::NotFound(Uuid::nil()),
            StatusCode::NOT_FOUND,
            "item not found",
        ),
        (
            Error::Unexpected("private database diagnostic".into()),
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal server error",
        ),
    ];
    for (error, status, message) in cases {
        let response = ApiError::from(error).into_response();
        assert_eq!(response.status(), status);
        assert_eq!(response.headers()["content-type"], "application/json");
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body, json!({"error": message}));
    }
}

#[test]
fn every_service_error_has_a_safe_grpc_status() {
    let cases = [
        (Error::InvalidId, Code::InvalidArgument, "invalid item id"),
        (
            Error::EmptyTitle,
            Code::InvalidArgument,
            "item title cannot be empty",
        ),
        (
            Error::NotFound(Uuid::nil()),
            Code::NotFound,
            "item not found",
        ),
        (
            Error::Unexpected("private database diagnostic".into()),
            Code::Internal,
            "internal server error",
        ),
    ];
    for (error, code, message) in cases {
        let status = map_error(error);
        assert_eq!(status.code(), code);
        assert_eq!(status.message(), message);
    }
}
