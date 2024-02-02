use std::{net::TcpListener, sync::Arc};

use anyhow::Result;
use axum::extract::FromRef;
use axum::http::{HeaderValue, Request};
use axum::{
    routing::{get, post},
    Router,
};
use sqlx::PgPool;
use tower::ServiceBuilder;
use tower_http::{
    request_id::{MakeRequestId, RequestId},
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
    ServiceBuilderExt,
};
use tracing::info;
use ulid::Ulid;

use crate::configuration::{DatabaseSettings, Settings};
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

#[derive(Clone)]
pub struct AppState {
    db: Arc<PgPool>,
    email: Arc<EmailClient>,
    base_url: Arc<str>,
}

impl FromRef<AppState> for Arc<PgPool> {
    fn from_ref(input: &AppState) -> Self {
        Arc::clone(&input.db)
    }
}

impl FromRef<AppState> for Arc<EmailClient> {
    fn from_ref(input: &AppState) -> Self {
        Arc::clone(&input.email)
    }
}

impl FromRef<AppState> for Arc<str> {
    fn from_ref(input: &AppState) -> Self {
        Arc::clone(&input.base_url)
    }
}

pub struct Application {
    app: Router,
    listener: TcpListener,
}

pub fn get_connection_pool(database: &DatabaseSettings) -> Result<PgPool> {
    Ok(PgPool::connect_lazy(&database.connection_string())?)
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self> {
        let connection_pool =
            get_connection_pool(&configuration.database).expect("Failed to connecto to Postgres");

        let sender_email = configuration
            .email_client
            .sender()
            .expect("Invalid sender email address.");

        let email_client = EmailClient::new(
            configuration.email_client.base_url.clone(),
            sender_email,
            configuration.email_client.authorization_token.clone(),
        );

        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        let listener = TcpListener::bind(address)?;
        listener.set_nonblocking(true)?;

        let trace_layer = TraceLayer::new_for_http()
            .on_request(DefaultOnRequest::new().level(tracing::Level::INFO))
            .make_span_with(
                DefaultMakeSpan::new()
                    .include_headers(true)
                    .level(tracing::Level::INFO),
            )
            .on_response(
                DefaultOnResponse::new()
                    .include_headers(true)
                    .level(tracing::Level::INFO),
            );

        let app = Router::new()
            .route("/", get(routes::home))
            .route("/login", get(routes::login_get))
            .route("/login", post(routes::login_post))
            .route("/health_check", get(routes::health_check))
            .route("/subscriptions", post(routes::subscribe))
            .route("/subscriptions/confirm", get(routes::confirm))
            .route("/newsletters", post(routes::publish_newsletter))
            .layer(
                ServiceBuilder::new()
                    .set_x_request_id(MakeUlidRequestId)
                    .layer(trace_layer)
                    .propagate_x_request_id(),
            )
            .with_state(AppState {
                db: Arc::new(connection_pool),
                email: Arc::new(email_client),
                base_url: Arc::from(configuration.application.base_url),
            });

        info!("starting server");

        Ok(Application { app, listener })
    }

    pub fn port(&self) -> u16 {
        self.listener.local_addr().unwrap().port()
    }

    pub async fn run(self) -> Result<()> {
        axum::serve(tokio::net::TcpListener::from_std(self.listener)?, self.app).await?;
        Ok(())
    }
}
