use std::{net::TcpListener, sync::Arc};

use anyhow::Result;
use axum::http::{HeaderValue, Request};
use axum::{
    routing::{get, post},
    Router, Server,
};
use sqlx::PgPool;
use tower::ServiceBuilder;
use tower_http::trace::DefaultMakeSpan;
use tower_http::{
    request_id::{MakeRequestId, RequestId},
    trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer},
    ServiceBuilderExt,
};
use tracing::info;
use ulid::Ulid;

use crate::email_client::EmailClient;
use crate::routes;

#[derive(Clone)]
struct MakeUlidRequestId;

impl MakeRequestId for MakeUlidRequestId {
    fn make_request_id<B>(&mut self, _: &Request<B>) -> Option<RequestId> {
        Some(RequestId::new(
            HeaderValue::from_str(Ulid::new().to_string().as_str()).unwrap(),
        ))
    }
}

pub async fn run(listener: TcpListener, connection: PgPool, client: EmailClient) -> Result<()> {
    let trace_layer = TraceLayer::new_for_http()
        .on_request(DefaultOnRequest::new().level(tracing::Level::INFO))
        .make_span_with(DefaultMakeSpan::new().include_headers(true).level(tracing::Level::INFO))
        .on_response(
            DefaultOnResponse::new()
                .include_headers(true)
                .level(tracing::Level::INFO),
        );

    let app = Router::new()
        .route("/health_check", get(routes::health_check))
        .route("/subscriptions", post(routes::subscribe))
        .layer(
            ServiceBuilder::new()
                .set_x_request_id(MakeUlidRequestId)
                .layer(trace_layer)
                .propagate_x_request_id(),
        )
        .with_state(Arc::new(connection))
        .with_state(Arc::new(client));

    info!("starting server");

    Server::from_tcp(listener)?.serve(app.into_make_service()).await?;

    Ok(())
}
