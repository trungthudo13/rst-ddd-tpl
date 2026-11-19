use std::sync::Arc;

use domain::Item;
use hiqlite::NodeConfig;
use infrastructure::{
    Database,
    HiqliteItemRepository,
};
use shared::ItemRepository;
use tracing_subscriber::EnvFilter;

use crate::{
    config::AppConfig,
    di,
    routes,
    server,
};

pub(crate) async fn run() -> Result<(), shared::Error> {
    let config = NodeConfig::from_env();
    let app_config = AppConfig::from_env();
    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let database = Database::connect(config).await?;
    let repository: Arc<dyn ItemRepository<Entity = Item>> =
        Arc::new(HiqliteItemRepository::new(database.client()));
    let handlers = di::item_handlers(repository);
    let result = server::serve(
        routes::router(handlers, app_config.debug),
        &app_config.address,
    )
    .await;
    database.shutdown().await?;
    result
}
