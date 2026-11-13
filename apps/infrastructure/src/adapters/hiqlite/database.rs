use anyhow::Context;
use hiqlite::{
    Client,
    NodeConfig,
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "migrations"]
pub(super) struct EmbeddedMigrations;

pub struct Database {
    client: Client,
}

impl Database {
    pub async fn connect(config: NodeConfig) -> anyhow::Result<Self> {
        let client = hiqlite::start_node(config).await.context("Hiqlite failed to start")?;
        client
            .try_migrate::<EmbeddedMigrations>()
            .await
            .context("Hiqlite could not apply migrations")?;

        Ok(Self {
            client,
        })
    }

    pub fn client(&self) -> Client {
        self.client.clone()
    }

    pub async fn shutdown(&self) -> anyhow::Result<()> {
        self.client.shutdown().await.context("HiQLite could not shut down")
    }
}
