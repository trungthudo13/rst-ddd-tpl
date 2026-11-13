use std::time::Duration;

use axum::{
    Router,
    http::StatusCode,
};
use presentation::{
    ItemHandlers,
    api,
    internal,
};
use tower::ServiceBuilder;
use tower_http::{
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::openapi::ApiDoc;

pub(crate) fn router(handlers: ItemHandlers) -> Router {
    let debug = std::env::var("DEBUG").is_ok_and(|value| value.trim().eq_ignore_ascii_case("true"));
    router_with_debug(handlers, debug)
}

pub(crate) fn router_with_debug(
    handlers: ItemHandlers,
    debug: bool,
) -> Router {
    let api = Router::new()
        .merge(api::routes::health())
        .merge(api::routes::item())
        .with_state(handlers.clone());
    let internal = tonic::service::Routes::new(internal::server(handlers)).into_axum_router();
    let app = Router::new().nest("/api", api).nest("/internal", internal);
    let app = if debug {
        app.merge(SwaggerUi::new("/docs/swagger").url("/docs/openapi.json", ApiDoc::openapi()))
    }
    else {
        app
    };

    app.layer(
        ServiceBuilder::new().layer(TraceLayer::new_for_http()).layer(
            TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, Duration::from_secs(5)),
        ),
    )
}
