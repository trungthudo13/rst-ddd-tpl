use std::{
    future::Future,
    pin::pin,
    time::Duration,
};

use axum::{
    Router,
    body::Body,
};
use hyper::{
    Request,
    body::Incoming,
};
use hyper_util::{
    rt::{
        TokioExecutor,
        TokioIo,
    },
    server::{
        conn::auto::Builder,
        graceful::GracefulShutdown,
    },
    service::TowerToHyperService,
};
use tokio::net::TcpListener;
use tower::{
    ServiceExt,
    service_fn,
};

pub(crate) async fn serve(app: Router) -> anyhow::Result<()> {
    let address = std::env::var("APP_ADDR").unwrap_or_else(|_| "0.0.0.0:13000".to_owned());
    let listener = TcpListener::bind(&address).await?;
    tracing::info!(%address, "listening");
    serve_with_shutdown(listener, app, shutdown_signal()).await
}

pub(crate) async fn serve_with_shutdown(
    listener: TcpListener,
    app: Router,
    shutdown: impl Future<Output = ()>,
) -> anyhow::Result<()> {
    let mut shutdown = pin!(shutdown);
    let server = Builder::new(TokioExecutor::new());
    let graceful = GracefulShutdown::new();

    loop {
        tokio::select! {
            connection = listener.accept() => {
                let (stream, peer) = match connection {
                    Ok(connection) => connection,
                    Err(error) => {
                        tracing::warn!(%error, "accept failed");
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                };
                let app = app.clone();
                let connection = server.serve_connection_with_upgrades(
                    TokioIo::new(stream),
                    TowerToHyperService::new(service_fn(move |request: Request<Incoming>| {
                        let app = app.clone();
                        async move { app.oneshot(request.map(Body::new)).await }
                    })),
                );
                let connection = graceful.watch(connection.into_owned());
                tokio::spawn(async move {
                    if let Err(error) = connection.await {
                        tracing::debug!(%peer, %error, "connection closed with error");
                    }
                });
            }
            () = shutdown.as_mut() => break,
        }
    }

    tokio::select! {
        () = graceful.shutdown() => {}
        () = tokio::time::sleep(Duration::from_secs(10)) => {
            tracing::warn!("graceful shutdown timed out");
        }
    }
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut signal) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            signal.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
    tracing::info!("shutdown signal received");
}
