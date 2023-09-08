mod common;

use reqwest::Client;

use common::spawn_app;

#[tokio::test]
async fn health_check_works() {
    let address = spawn_app().await.address;
    let client = Client::new();

    let response = client
        .get(format!("{}/health_check", address))
        .send()
        .await
        .expect("failed to execute request");

    assert!(response.status().is_success());
}
