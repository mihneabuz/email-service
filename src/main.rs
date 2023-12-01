use email_service::{configuration::get_configuration, startup::Application, telemetry};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    telemetry::init_subscriber(std::io::stdout);

    let config = get_configuration().expect("failed to read configuration");

    let app = Application::build(config).await?;

    app.run().await?;

    Ok(())
}
