use axum::{response::IntoResponse, routing::get, Router};
use tracing::{info, instrument};
use flair::{get_tagline, get_word};
use maud::{html, DOCTYPE};

mod flair;

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

    html!({
        (DOCTYPE)
        a href="/" {
            h1 {
                "david's " (get_word())
            }
        }
        h2 {
            (get_tagline())
        }
        "Holy crap, it's a website"
    })
}
