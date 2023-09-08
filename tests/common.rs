use std::net::TcpListener;

use email_service::configuration::{get_configuration, DatabaseSettings};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use ulid::Ulid;

pub struct TestApp {
    pub address: String,
    pub db: PgPool,
}

pub async fn spawn_app() -> TestApp {
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind random port");

    let port = listener
        .local_addr()
        .expect("failed to get local_addr")
        .port();

    let mut config = get_configuration().expect("failed to read configuration");
    config.database.database_name = Ulid::new().to_string();
    let connection = configure_database(&config.database).await;

    let app = email_service::startup::run(listener, connection.clone());
    let _ = tokio::spawn(app);

    TestApp {
        address: format!("http://127.0.0.1:{}", port),
        db: connection,
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
