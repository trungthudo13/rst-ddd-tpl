mod bootstrap;
mod di;
mod openapi;
mod routes;
mod server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    bootstrap::run().await
}

#[cfg(test)]
mod tests;
