use axum::{response::IntoResponse, routing::get, Router};
use tracing::{info, instrument};

mod flair;
mod templates;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let site = Router::new()
        .route("/", get(root));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, site).await.unwrap();
}

#[instrument]
async fn root() -> impl IntoResponse {
    info!("serving");
    templates::index()
}
