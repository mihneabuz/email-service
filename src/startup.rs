use std::{net::TcpListener, sync::Arc};

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router, Server,
};
use sqlx::PgPool;

use crate::routes;

pub async fn run(listener: TcpListener, connection: PgPool) -> Result<()> {
    let app = Router::new()
        .route("/health_check", get(routes::health_check))
        .route("/subscriptions", post(routes::subscribe))
        .with_state(Arc::new(connection));

    Server::from_tcp(listener)?
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
