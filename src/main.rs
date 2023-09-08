use std::net::{SocketAddr, TcpListener};

use anyhow::Result;

use email_service::run;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr)?;

    run(listener).await
}
