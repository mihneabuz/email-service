use once_cell::sync::Lazy;
use reqwest::Client;
use serde::Serialize;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use ulid::Ulid;
use wiremock::MockServer;

use email_service::{
    configuration::{get_configuration, DatabaseSettings},
    startup::{self, Application},
    telemetry,
};

static TRACING: Lazy<()> = Lazy::new(|| {
    if std::env::var("TEST_LOG").is_ok() {
        telemetry::init_subscriber(std::io::stdout);
    } else {
        telemetry::init_subscriber(std::io::sink);
    }
});

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub db: PgPool,
    pub http_client: Client,
    pub email_server: MockServer,
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;

    // Randomise configuration to ensure test isolation
    let configuration = {
        let mut c = get_configuration().expect("Failed to read configuration.");
        // Use a different database for each test case
        c.database.database_name = Ulid::new().to_string();
        // Use a random OS port
        c.application.port = 0;
        // Use the mock server as email API
        c.email_client.base_url = email_server.uri();
        c
    };

    // Create and migrate the database
    configure_database(&configuration.database).await;

    let server = Application::build(configuration.clone())
        .await
        .expect("Failed to build Application");

    let port = server.port();
    let address = format!("http://127.0.0.1:{}", port);

    tokio::spawn(server.run());

    TestApp {
        address,
        port,
        db: startup::get_connection_pool(&configuration.database).unwrap(),
        http_client: Client::new(),
        email_server,
    }
}

impl TestApp {
    pub async fn post_subscriptions<T: Serialize>(&self, form: &T) -> reqwest::Response {
        self.http_client
            .post(&format!("{}/subscriptions", &self.address))
            .form(form)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_subscriptions_raw(&self, body: &str) -> reqwest::Response {
        self.http_client
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body.to_owned())
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_newsletters(&self, body: serde_json::Value) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/newsletters", &self.address))
            .json(&body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();

            assert_eq!(links.len(), 1);

            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");

            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(&body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(&body["TextBody"].as_str().unwrap());

        ConfirmationLinks { html, plain_text }
    }
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    let mut connection = PgConnection::connect(&config.connection_string_without_db())
        .await
        .expect("failed to connect to database");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("failed to create database");

    let connection_pool = PgPool::connect(&config.connection_string())
        .await
        .expect("failed to connect to Postgres");

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("failed to migrate the database");

    connection_pool
}
