mod grpc_tests;
mod repository;
mod router_tests;

use std::sync::Arc;

use domain::Item;
use repository::TestRepository;
use shared::ItemRepository;

fn test_app(debug: bool) -> axum::Router {
    app_with_repository(Arc::new(TestRepository::default()), debug)
}

fn app_with_repository(
    repository: Arc<dyn ItemRepository<Entity = Item>>,
    debug: bool,
) -> axum::Router {
    crate::routes::router_with_debug(crate::di::item_handlers(repository), debug)
}
