use hiqlite::{
    Client,
    NodeConfig,
};
use rust_embed::RustEmbed;
use shared::Error;

use super::mappers::errors;

#[derive(RustEmbed)]
#[folder = "migrations"]
pub(super) struct EmbeddedMigrations;

pub struct Database {
    client: Client,
}

impl Database {
    pub async fn connect(config: NodeConfig) -> Result<Self, Error> {
        let client = hiqlite::start_node(config).await.map_err(errors::startup)?;
        if let Err(error) = client.try_migrate::<EmbeddedMigrations>().await {
            let shutdown = client.shutdown().await.err();
            return Err(errors::migration(error, shutdown));
        }
        let database = Self {
            client,
        };
        Ok(database)
    }

    pub fn client(&self) -> Client {
        self.client.clone()
    }

    pub async fn shutdown(&self) -> Result<(), Error> {
        self.client.shutdown().await.map_err(errors::shutdown)
    }
}
