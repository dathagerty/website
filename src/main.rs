use axum::{extract::Path, response::IntoResponse, routing::get, Router};
use tracing::{info, instrument};
use build_info::build_info;

mod flair;
mod templates;

build_info!(fn version);

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // I know this is a git repository, so let's panic if things don't work for some reason.
    let vc = version().version_control.as_ref().expect("how is this not a git repository");

    info!(commit = vc.git().unwrap().commit_short_id, "starting website");

    let site = Router::new()
        .route("/", get(root))
        .route("/about", get(page))
        .route("/blag", get(page))
        .route("/blag/{slug}", get(post));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
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
async fn post(
    Path(slug): Path<String>
) -> impl IntoResponse {
    info!(slug, "serving post");
    templates::post(slug)
}
