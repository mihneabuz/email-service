use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::spawn_app;

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    let app = spawn_app().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

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
async fn subscribe_persists_the_new_subscriber() {
    let app = spawn_app().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let (email, name) = ("ursula_le_guin@gmail.com", "le guin");
    app.post_subscriptions(&[("email", email), ("name", name)])
        .await;

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&app.db)
        .await
        .expect("failed to fetch saved subscription");

    assert_eq!(saved.email, email);
    assert_eq!(saved.name, name);
    assert_eq!(saved.status, "pending_confirmation");
    ()
}

#[tokio::test]
async fn subscribe_returns_409_when_email_already_subscribed() {
    let app = spawn_app().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

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

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    let app = spawn_app().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let (email, name) = ("ursula_le_guin@gmail.com", "le guin");
    let response = app
        .post_subscriptions(&[("email", email), ("name", name)])
        .await;

    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    let app = spawn_app().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    let (email, name) = ("ursula_le_guin@gmail.com", "le guin");
    app.post_subscriptions(&[("email", email), ("name", name)])
        .await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];

    let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

    let get_link = |s: &str| {
        let links: Vec<_> = linkify::LinkFinder::new()
            .links(s)
            .filter(|l| *l.kind() == linkify::LinkKind::Url)
            .collect();
        assert_eq!(links.len(), 1);
        links[0].as_str().to_owned()
    };

    let html_link = get_link(&body["HtmlBody"].as_str().unwrap());
    let text_link = get_link(&body["TextBody"].as_str().unwrap());

    assert_eq!(html_link, text_link);
}
