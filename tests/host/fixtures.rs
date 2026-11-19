use std::sync::Arc;

use domain::Item;
use shared::ItemRepository;

use super::repository::TestRepository;

pub(super) fn test_app(debug: bool) -> axum::Router {
    app_with_repository(Arc::new(TestRepository::default()), debug)
}

pub(super) fn app_with_repository(
    repository: Arc<dyn ItemRepository<Entity = Item>>,
    debug: bool,
) -> axum::Router {
    crate::routes::router(crate::di::item_handlers(repository), debug)
}
