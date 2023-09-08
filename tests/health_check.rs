use std::net::TcpListener;

use reqwest::Client;

async fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind random port");

    let port = listener
        .local_addr()
        .expect("failed to get local_addr")
        .port();

    let app = email_service::run(listener);
    let _ = tokio::spawn(app);

    format!("http://127.0.0.1:{}", port)
}

#[tokio::test]
async fn health_check_works() {
    let address = spawn_app().await;
    let client = Client::new();

    let response = client
        .get(format!("{}/health_check", address))
        .send()
        .await
        .expect("failed to execute request");

    assert!(response.status().is_success());
}
