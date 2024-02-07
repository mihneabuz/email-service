use std::{net::TcpListener, sync::Arc};

use anyhow::Result;
use axum::extract::FromRef;
use axum::http::{HeaderValue, Request};
use axum::{
    routing::{get, post},
    Router,
};
use secrecy::Secret;
use sqlx::PgPool;
use time::Duration;
use tower::ServiceBuilder;
use tower_http::{
    request_id::{MakeRequestId, RequestId},
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
    ServiceBuilderExt,
};
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_redis_store::{
    fred::{clients::RedisPool, interfaces::ClientLike, types::RedisConfig},
    RedisStore,
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
    secret: Arc<Secret<String>>,
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

impl FromRef<AppState> for Arc<Secret<String>> {
    fn from_ref(input: &AppState) -> Self {
        Arc::clone(&input.secret)
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

        let uuid_layer = ServiceBuilder::new()
            .set_x_request_id(MakeUlidRequestId)
            .layer(trace_layer)
            .propagate_x_request_id();

        let redis_pool = RedisPool::new(RedisConfig::default(), None, None, None, 6)?;
        redis_pool.connect();
        redis_pool.wait_for_connect().await?;

        let session_store = RedisStore::new(redis_pool);
        let session_layer = SessionManagerLayer::new(session_store)
            .with_expiry(Expiry::OnInactivity(Duration::MINUTE));

        let app = Router::new()
            .route("/", get(routes::home))
            .route("/login", get(routes::login_get))
            .route("/login", post(routes::login_post))
            .route("/health_check", get(routes::health_check))
            .route("/subscriptions", post(routes::subscribe))
            .route("/subscriptions/confirm", get(routes::confirm))
            .route("/admin/dashboard", get(routes::admin_dashboard))
            .route("/admin/password", get(routes::change_password_form))
            .route("/admin/password", post(routes::change_password))
            .route("/admin/newsletters", get(routes::newsletter_form))
            .route("/admin/newsletters", post(routes::publish_newsletter))
            .route("/logout", post(routes::log_out))
            .layer(session_layer)
            .layer(uuid_layer)
            .with_state(AppState {
                db: Arc::new(connection_pool),
                email: Arc::new(email_client),
                base_url: Arc::from(configuration.application.base_url),
                secret: Arc::new(configuration.application.secret),
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
