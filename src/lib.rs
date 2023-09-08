use std::net::TcpListener;

use anyhow::Result;
use axum::{routing::get, Router, Server};

pub async fn run(listener: TcpListener) -> Result<()> {
    let app = Router::new().route("/health_check", get(health_check));

    Server::from_tcp(listener)?
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn health_check() -> &'static str {
    "ok"
}
