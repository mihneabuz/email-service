use reqwest::Client;

use crate::helpers::{spawn_app, TestApp};

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    let TestApp { address, db } = spawn_app().await;
    let client = Client::new();

    let (email, name) = ("ursula_le_guin@gmail.com", "le guin");
    let response = client
        .post(&format!("{}/subscriptions", address))
        .form(&[("email", email), ("name", name)])
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&db)
        .await
        .expect("failed to fetch saved subscription");

    assert_eq!(saved.email, email);
    assert_eq!(saved.name, name);
}

#[tokio::test]
async fn subscribe_returns_409_when_email_already_subscribed() {
    let TestApp { address, db } = spawn_app().await;
    let client = Client::new();

    let (email, name) = ("ursula_le_guin@gmail.com", "le guin");
    let response = client
        .post(&format!("{}/subscriptions", address))
        .form(&[("email", email), ("name", name)])
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&db)
        .await
        .expect("failed to fetch saved subscription");

    assert_eq!(saved.email, email);
    assert_eq!(saved.name, name);

    let response = client
        .post(&format!("{}/subscriptions", address))
        .form(&[("email", email), ("name", name)])
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(409, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_returns_422_when_data_is_missing() {
    let address = spawn_app().await.address;
    let client = Client::new();

    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(&format!("{}/subscriptions", address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");

        assert_eq!(
            422,
            response.status().as_u16(),
            "The API did not fail with 422 when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_400_when_fields_are_invalid() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (body, description) in test_cases {
        let response = client
            .post(&format!("{}/subscriptions", &app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.");

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 when the payload was {}.",
            description
        );
    }
}
