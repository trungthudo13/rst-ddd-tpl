use std::time::Duration;

use axum::{
    Router,
    http::StatusCode,
};
use tower::ServiceBuilder;
use tower_http::{
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

pub(crate) fn apply(router: Router) -> Router {
    router.layer(
        ServiceBuilder::new().layer(TraceLayer::new_for_http()).layer(
            TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, Duration::from_secs(5)),
        ),
    )
}
