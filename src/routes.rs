use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    ItemHandlers,
    api,
    internal,
    middlewares,
    openapi::ApiDoc,
};

pub(crate) fn router(
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
    middlewares::request::apply(app)
}
