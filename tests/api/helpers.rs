use once_cell::sync::Lazy;
use reqwest::Client;
use serde::Serialize;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use ulid::Ulid;

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

pub struct TestApp {
    pub address: String,
    pub db: PgPool,
    pub client: Client,
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    // Randomise configuration to ensure test isolation
    let configuration = {
        let mut c = get_configuration().expect("Failed to read configuration.");
        // Use a different database for each test case
        c.database.database_name = Ulid::new().to_string();
        // Use a random OS port
        c.application.port = 0;
        c
    };

    // Create and migrate the database
    configure_database(&configuration.database).await;

    let server = Application::build(configuration.clone())
        .await
        .expect("Failed to build Application");

    let address = format!("http://127.0.0.1:{}", server.port());

    tokio::spawn(server.run());

    TestApp {
        address,
        db: startup::get_connection_pool(&configuration.database).unwrap(),
        client: Client::new(),
    }
}

impl TestApp {
    pub async fn post_subscriptions<T>(&self, form: &T) -> reqwest::Response
    where
        T: Serialize,
    {
        self.client
            .post(&format!("{}/subscriptions", &self.address))
            .form(form)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_subscriptions_raw(&self, body: &str) -> reqwest::Response {
        self.client
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body.to_owned())
            .send()
            .await
            .expect("Failed to execute request.")
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
