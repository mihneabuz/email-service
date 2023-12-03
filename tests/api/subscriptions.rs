use crate::helpers::spawn_app;

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    let app = spawn_app().await;

    let (email, name) = ("ursula_le_guin@gmail.com", "le guin");
    let response = app
        .post_subscriptions(&[("email", email), ("name", name)])
        .await;

    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&app.db)
        .await
        .expect("failed to fetch saved subscription");

    assert_eq!(saved.email, email);
    assert_eq!(saved.name, name);
}

#[tokio::test]
async fn subscribe_returns_409_when_email_already_subscribed() {
    let app = spawn_app().await;

    let (email, name) = ("ursula_le_guin@gmail.com", "le guin");
    let response = app
        .post_subscriptions(&[("email", email), ("name", name)])
        .await;

    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&app.db)
        .await
        .expect("failed to fetch saved subscription");

    assert_eq!(saved.email, email);
    assert_eq!(saved.name, name);

    let response = app
        .post_subscriptions(&[("email", email), ("name", name)])
        .await;

    assert_eq!(409, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_returns_422_when_data_is_missing() {
    let app = spawn_app().await;

    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app.post_subscriptions_raw(&invalid_body).await;

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

    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app.post_subscriptions_raw(&invalid_body).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 when the payload was {}.",
            error_message
        );
    }
}
