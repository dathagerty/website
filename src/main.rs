use axum::{Router, extract::Path, response::IntoResponse, routing::get};
use build_info::build_info;
use tracing::{info, instrument};

mod config;
mod flair;
mod templates;

build_info!(fn version);

const COMMIT_SHORT: &str = build_info::format!("{}", $.version_control?.git()?.commit_short_id);

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Load configuration
    let server_config = config::ServerConfig::new().unwrap_or_default();

    info!(
        commit = COMMIT_SHORT,
        port = server_config.port,
        "starting website"
    );

    let site = Router::new()
        .route("/", get(root))
        .route("/about", get(page))
        .route("/blag", get(page))
        .route("/blag/{slug}", get(post));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", server_config.port))
        .await
        .unwrap();
    axum::serve(listener, site).await.unwrap();
}

#[instrument]
async fn root() -> impl IntoResponse {
    info!("serving");
    templates::root()
}

#[instrument]
async fn page() -> impl IntoResponse {
    info!("serving page");
    templates::page()
}

#[instrument]
async fn post(Path(slug): Path<String>) -> impl IntoResponse {
    info!(slug, "serving post");
    templates::post(slug)
}
