use std::net::{SocketAddr, TcpListener};

use anyhow::Result;

use email_service::{configuration::get_configuration, startup::run};
use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<()> {
    let config = get_configuration().expect("failed to read configuration");

    let address = SocketAddr::from(([127, 0, 0, 1], config.port));
    let listener = TcpListener::bind(address)?;

    let connection = PgPool::connect(&config.database.connection_string()).await?;

    run(listener, connection).await
}
