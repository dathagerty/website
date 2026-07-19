use std::{
    ffi::{OsStr, OsString},
    fs,
    future::Future,
    io,
    num::ParseIntError,
    path::PathBuf,
    time::Duration,
};

use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::{conn::auto, graceful::GracefulShutdown},
};
use tokio::net::TcpListener;
use topcoat::asset::AssetBundle;
use topcoat::router::{Router, RouterService};

pub mod content;
pub mod error;
pub mod routes;
pub mod views;

pub use routes::build_router;

const CONNECTION_DRAIN_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, thiserror::Error)]
pub enum ApplicationAssetError {
    #[error("failed to discover the Topcoat asset bundle")]
    Load(#[source] io::Error),
    #[error("asset bundle does not contain the required application stylesheet")]
    MissingStylesheet,
    #[error("required application stylesheet at {path} is not readable")]
    UnreadableStylesheet {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("required application stylesheet at {path} is not safely servable: {reason}")]
    UnservableStylesheet { path: PathBuf, reason: &'static str },
}

pub fn load_application_assets() -> Result<AssetBundle, ApplicationAssetError> {
    let bundle = AssetBundle::load().map_err(ApplicationAssetError::Load)?;
    validate_application_assets(&bundle)?;
    Ok(bundle)
}

pub fn validate_application_assets(bundle: &AssetBundle) -> Result<(), ApplicationAssetError> {
    let stylesheet = bundle
        .get(views::STYLESHEET)
        .ok_or(ApplicationAssetError::MissingStylesheet)?;
    let path = stylesheet.path();
    if path.file_name().and_then(OsStr::to_str).is_none() {
        return Err(ApplicationAssetError::UnservableStylesheet {
            path: path.to_path_buf(),
            reason: "filename is missing or is not valid UTF-8",
        });
    }
    if stylesheet.content_type() != "text/css" {
        return Err(ApplicationAssetError::UnservableStylesheet {
            path: path.to_path_buf(),
            reason: "content type is not text/css",
        });
    }
    fs::read(path).map_err(|source| ApplicationAssetError::UnreadableStylesheet {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("PORT is not valid Unicode")]
    InvalidPortUnicode(OsString),
    #[error("PORT must be a valid u16")]
    InvalidPort(#[source] ParseIntError),
    #[error("INCLUDE_DRAFTS is not valid Unicode")]
    InvalidIncludeDraftsUnicode(OsString),
    #[error("INCLUDE_DRAFTS must be one of: true, 1, false, 0; got {0:?}")]
    InvalidIncludeDrafts(String),
}

pub fn server_port() -> Result<u16, ConfigError> {
    let value = std::env::var_os("PORT");
    parse_server_port(value.as_deref())
}

fn parse_server_port(value: Option<&OsStr>) -> Result<u16, ConfigError> {
    match value {
        None => Ok(3000),
        Some(value) => value
            .to_str()
            .ok_or_else(|| ConfigError::InvalidPortUnicode(value.to_owned()))?
            .parse()
            .map_err(ConfigError::InvalidPort),
    }
}

pub fn include_drafts() -> Result<bool, ConfigError> {
    let value = std::env::var_os("INCLUDE_DRAFTS");
    parse_include_drafts(value.as_deref())
}

fn parse_include_drafts(value: Option<&OsStr>) -> Result<bool, ConfigError> {
    match value {
        Some(value) => match value
            .to_str()
            .ok_or_else(|| ConfigError::InvalidIncludeDraftsUnicode(value.to_owned()))?
        {
            "true" | "1" => Ok(true),
            "false" | "0" => Ok(false),
            value => Err(ConfigError::InvalidIncludeDrafts(value.to_owned())),
        },
        None => Ok(false),
    }
}

/// Serves a Topcoat router until `shutdown` completes, then drains open connections.
///
/// Topcoat 0.1.3's `serve` function does not expose graceful shutdown. This uses
/// its public router service with the same Hyper connection builder while tracking
/// connections so shutdown affects the accept loop and waits for in-flight work.
pub async fn serve_with_shutdown(
    listener: TcpListener,
    router: Router,
    shutdown: impl Future<Output = io::Result<()>>,
) -> io::Result<()> {
    let notification = topcoat::dev::notify_ready(listener.local_addr().ok());
    serve_with_shutdown_and_notify(listener, router, shutdown, notification).await
}

async fn serve_with_shutdown_and_notify(
    listener: TcpListener,
    router: Router,
    shutdown: impl Future<Output = io::Result<()>>,
    notification: impl Future<Output = ()>,
) -> io::Result<()> {
    tokio::pin!(shutdown);
    tokio::pin!(notification);

    tokio::select! {
        result = &mut shutdown => return result,
        () = &mut notification => {}
    }

    let service = RouterService::new(router);
    let graceful = GracefulShutdown::new();

    let shutdown_result = loop {
        tokio::select! {
            result = &mut shutdown => break result,
            accepted = listener.accept() => {
                let (stream, _remote) = match accepted {
                    Ok(connection) => connection,
                    Err(error) => break Err(error),
                };
                let io = TokioIo::new(stream);
                let service = service.clone();
                let watcher = graceful.watcher();

                tokio::spawn(async move {
                    let builder = connection_builder();
                    let connection = builder.serve_connection(io, service);
                    if let Err(error) = watcher.watch(connection).await {
                        tracing::trace!(%error, "connection ended with an error");
                    }
                });
            }
        }
    };

    drain_after_stop(
        shutdown_result,
        graceful.shutdown(),
        CONNECTION_DRAIN_TIMEOUT,
    )
    .await
}

// Keep protocol and future upgrade configuration in parity with Topcoat 0.1.3's server here.
fn connection_builder() -> auto::Builder<TokioExecutor> {
    auto::Builder::new(TokioExecutor::new())
}

async fn drain_after_stop(
    stop_result: io::Result<()>,
    drain: impl Future<Output = ()>,
    deadline: Duration,
) -> io::Result<()> {
    if tokio::time::timeout(deadline, drain).await.is_err() {
        tracing::warn!(
            timeout_seconds = deadline.as_secs_f64(),
            "connection drain deadline elapsed"
        );
    }
    stop_result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        ffi::{OsStr, OsString},
        fs,
    };
    use tempfile::TempDir;
    use topcoat::{
        asset::{AssetBundle, MANIFEST_VERSION, Manifest, ManifestEntry},
        router::{Body, Request, StatusCode},
    };

    use crate::{
        content::{ContentRepository, EmbeddedContentRepository},
        views::STYLESHEET,
    };

    fn asset_bundle(file: Option<&str>, write_file: bool) -> (TempDir, AssetBundle) {
        let dir = TempDir::new().unwrap();
        let assets = file
            .map(|file| ManifestEntry {
                id: STYLESHEET,
                file: file.to_owned(),
                hash: "test".to_owned(),
                content_type: "text/css".to_owned(),
            })
            .into_iter()
            .collect();
        Manifest {
            version: MANIFEST_VERSION,
            assets,
        }
        .save(dir.path().join("manifest.toml"))
        .unwrap();
        if write_file {
            fs::write(dir.path().join(file.unwrap()), "body {}\n").unwrap();
        }
        let bundle = AssetBundle::load_dir(dir.path()).unwrap();
        (dir, bundle)
    }

    #[test]
    fn application_assets_reject_a_bundle_without_the_stylesheet_id() {
        let (_dir, bundle) = asset_bundle(None, false);

        let error = validate_application_assets(&bundle).unwrap_err();

        assert!(matches!(error, ApplicationAssetError::MissingStylesheet));
    }

    #[test]
    fn application_assets_reject_a_manifest_entry_with_a_missing_file() {
        let (dir, bundle) = asset_bundle(Some("missing.css"), false);

        let error = validate_application_assets(&bundle).unwrap_err();

        assert!(matches!(
            error,
            ApplicationAssetError::UnreadableStylesheet { path, source }
                if path == dir.path().join("missing.css")
                    && source.kind() == io::ErrorKind::NotFound
        ));
    }

    #[test]
    fn application_assets_accept_a_readable_stylesheet() {
        let (_dir, bundle) = asset_bundle(Some("styles.css"), true);

        validate_application_assets(&bundle).unwrap();
    }

    #[test]
    fn server_port_defaults_to_3000() {
        assert_eq!(parse_server_port(None).unwrap(), 3000);
    }

    #[test]
    fn server_port_uses_port_environment_variable() {
        assert_eq!(parse_server_port(Some(OsStr::new("8080"))).unwrap(), 8080);
    }

    #[test]
    fn server_port_rejects_malformed_port() {
        assert!(matches!(
            parse_server_port(Some(OsStr::new("not-a-port"))),
            Err(ConfigError::InvalidPort(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn server_port_rejects_non_unicode_port() {
        use std::os::unix::ffi::OsStringExt;

        let value = OsString::from_vec(vec![0xff]);

        assert!(matches!(
            parse_server_port(Some(value.as_os_str())),
            Err(ConfigError::InvalidPortUnicode(_))
        ));
    }

    #[test]
    fn include_drafts_defaults_to_false() {
        assert!(!parse_include_drafts(None).unwrap());
    }

    #[test]
    fn include_drafts_accepts_documented_values() {
        for (value, expected) in [("true", true), ("1", true), ("false", false), ("0", false)] {
            assert_eq!(
                parse_include_drafts(Some(OsStr::new(value))).unwrap(),
                expected,
                "value: {value}"
            );
        }
    }

    #[test]
    fn include_drafts_rejects_invalid_values() {
        assert!(matches!(
            parse_include_drafts(Some(OsStr::new("yes"))),
            Err(ConfigError::InvalidIncludeDrafts(value)) if value == "yes"
        ));
    }

    #[cfg(unix)]
    #[test]
    fn include_drafts_rejects_non_unicode_values() {
        use std::os::unix::ffi::OsStringExt;

        let value = OsString::from_vec(vec![b't', b'r', 0xff]);

        assert!(matches!(
            parse_include_drafts(Some(value.as_os_str())),
            Err(ConfigError::InvalidIncludeDraftsUnicode(_))
        ));
    }

    #[tokio::test]
    async fn build_router_serves_health_check() {
        let repository: std::sync::Arc<dyn ContentRepository> =
            std::sync::Arc::new(EmbeddedContentRepository::load(false).unwrap());
        let router = build_router(repository, AssetBundle::empty());

        let response = router
            .handle(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn server_stops_when_shutdown_future_completes() {
        let repository: std::sync::Arc<dyn ContentRepository> =
            std::sync::Arc::new(EmbeddedContentRepository::load(false).unwrap());
        let router = build_router(repository, AssetBundle::empty());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();

        serve_with_shutdown(listener, router, std::future::ready(Ok(())))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn shutdown_interrupts_pending_ready_notification() {
        use std::task::{Context, Poll, Waker};

        let repository: std::sync::Arc<dyn ContentRepository> =
            std::sync::Arc::new(EmbeddedContentRepository::load(false).unwrap());
        let router = build_router(repository, AssetBundle::empty());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut server = Box::pin(serve_with_shutdown_and_notify(
            listener,
            router,
            std::future::ready(Ok(())),
            std::future::pending(),
        ));
        let mut context = Context::from_waker(Waker::noop());

        assert!(matches!(
            server.as_mut().poll(&mut context),
            Poll::Ready(Ok(()))
        ));
    }

    #[tokio::test]
    async fn accept_error_is_returned_after_connections_drain() {
        use std::cell::RefCell;

        let events = RefCell::new(Vec::new());
        let result = drain_after_stop(
            Err(io::Error::other("accept failed")),
            async {
                events.borrow_mut().push("drained");
            },
            Duration::from_secs(1),
        )
        .await;
        events.borrow_mut().push("returned");

        assert_eq!(&*events.borrow(), &["drained", "returned"]);
        assert_eq!(result.unwrap_err().to_string(), "accept failed");
    }

    #[tokio::test]
    async fn pending_connections_stop_draining_at_the_deadline_and_preserve_accept_error() {
        let result = drain_after_stop(
            Err(io::Error::other("accept failed")),
            std::future::pending(),
            Duration::ZERO,
        )
        .await;

        assert_eq!(result.unwrap_err().to_string(), "accept failed");
    }

    #[tokio::test]
    async fn server_returns_shutdown_signal_errors() {
        let repository: std::sync::Arc<dyn ContentRepository> =
            std::sync::Arc::new(EmbeddedContentRepository::load(false).unwrap());
        let router = build_router(repository, AssetBundle::empty());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();

        let error = serve_with_shutdown(
            listener,
            router,
            std::future::ready(Err(std::io::Error::other("signal failed"))),
        )
        .await
        .unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::Other);
        assert_eq!(error.to_string(), "signal failed");
    }
}
