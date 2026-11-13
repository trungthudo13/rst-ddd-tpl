use std::time::Duration;

use axum::{
    Router,
    http::StatusCode,
    routing::{
        get,
        post,
    },
};
use presentation::{
    ItemHandlers,
    api::handlers,
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
        .route("/health", get(handlers::health::health))
        .route(
            "/items",
            post(handlers::item::create_item).get(handlers::item::list_items),
        )
        .route("/items/bulk-delete", post(handlers::item::delete_items))
        .route(
            "/items/{id}",
            get(handlers::item::get_item)
                .patch(handlers::item::update_item)
                .delete(handlers::item::delete_item),
        );
    let api = if debug {
        api.merge(SwaggerUi::new("/docs/swagger").url("/docs/openapi.json", ApiDoc::openapi()))
    }
    else {
        api
    };

    tonic::service::Routes::from(api.with_state(handlers.clone()))
        .add_service(internal::server(handlers))
        .into_axum_router()
        .layer(
            ServiceBuilder::new().layer(TraceLayer::new_for_http()).layer(
                TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, Duration::from_secs(5)),
            ),
        )
}
