pub mod api;
mod bootstrap;
mod config;
mod di;
pub mod internal;
mod middlewares;
mod openapi;
mod routes;
mod server;
mod state;

pub use state::ItemHandlers;

#[tokio::main]
async fn main() -> Result<(), shared::Error> {
    bootstrap::run().await
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod integration_tests;
