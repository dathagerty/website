use std::{error::Error, io, net::SocketAddr, sync::Arc};

use dathagerty::{
    ApplicationAssetError, ConfigError, build_router,
    content::{ContentError, ContentRepository, EmbeddedContentRepository},
    include_drafts, load_application_assets, serve_with_shutdown, server_port,
};
use thiserror::Error;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Error)]
enum StartupError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error("failed to initialize structured tracing")]
    Tracing(#[source] Box<dyn Error + Send + Sync>),
    #[error("failed to load and validate embedded content")]
    Content(#[source] ContentError),
    #[error("failed to load the Topcoat asset bundle")]
    Assets(#[source] ApplicationAssetError),
    #[error("failed to bind server to {address}")]
    Bind {
        address: SocketAddr,
        #[source]
        source: io::Error,
    },
    #[error("Topcoat server failed")]
    Serve(#[source] io::Error),
}

#[tokio::main]
async fn main() -> Result<(), StartupError> {
    init_tracing().map_err(StartupError::Tracing)?;

    let port = server_port()?;
    let include_drafts = include_drafts()?;
    let repository: Arc<dyn ContentRepository> =
        Arc::new(EmbeddedContentRepository::load(include_drafts).map_err(StartupError::Content)?);
    let asset_bundle = load_application_assets().map_err(StartupError::Assets)?;
    let router = build_router(repository, asset_bundle);

    let address = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(address)
        .await
        .map_err(|source| StartupError::Bind { address, source })?;
    info!(%address, include_drafts, "server listening");

    serve_with_shutdown(listener, router, shutdown_signal())
        .await
        .map_err(StartupError::Serve)
}

fn init_tracing() -> Result<(), Box<dyn Error + Send + Sync>> {
    let filter = match std::env::var("RUST_LOG") {
        Ok(directives) => EnvFilter::try_new(directives)?,
        Err(std::env::VarError::NotPresent) => EnvFilter::new("info"),
        Err(error) => return Err(Box::new(error)),
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().json())
        .try_init()?;
    Ok(())
}

async fn shutdown_signal() -> io::Result<()> {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        tokio::select! {
            result = tokio::signal::ctrl_c() => result,
            _ = terminate.recv() => Ok(()),
        }
    }

    #[cfg(not(unix))]
    tokio::signal::ctrl_c().await
}
