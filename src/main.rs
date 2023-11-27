use std::net::TcpListener;

use sqlx::PgPool;

use email_service::{configuration::get_configuration, email_client::EmailClient, startup::run, telemetry};

#[tokio::main]
async fn main() {
    telemetry::init_subscriber(std::io::stdout);

    let config = get_configuration().expect("failed to read configuration");

    let address = format!("{}:{}", config.application.host, config.application.port);
    let listener = TcpListener::bind(address).expect("failed to bind listener");

    let connection = PgPool::connect_lazy(&config.database.connection_string()).expect("failed to connect to postgres");

    let sender_email = config.email_client.sender().expect("invalid sender email address");
    let email_client = EmailClient::new(
        config.email_client.base_url,
        sender_email,
        config.email_client.authorization_token,
    );

    run(listener, connection, email_client).await.expect("server failed");
}
