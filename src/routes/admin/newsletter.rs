use std::sync::Arc;

use anyhow::{anyhow, Result};
use axum::{
    body::Body,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Json,
};
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    domain::SubscriberEmail,
    email_client::EmailClient,
    idempotency::{get_saved_response, save_response, IdempotencyKey},
    session_state::TypedSession,
};

pub async fn newsletter_form(session: TypedSession) -> Response<Body> {
    if session.get_user_id().await.unwrap().is_none() {
        return Redirect::to("/login").into_response();
    }

    Html::from(
        r#"
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta http-equiv="content-type" content="text/html; charset=utf-8">
            <title>Send a newsletter issue</title>
        </head>
        <body>
            <form action="/admin/newsletters" method="post">
                <label>Title
                <input type="text" placeholder="Title" name="title">
                </label>
                <br>

                <label>Content
                <input type="text" placeholder="Text content" name="text_content">
                </label>
                <br>

                <label>Html content
                <input type="text" placeholder="Html content" name="html_content">
                </label>
                <br>

                <button type="submit">Send</button>
            </form>
            <p><a href="/admin/dashboard">&lt;- Back</a></p>
        </body>
        </html>
    "#,
    )
    .into_response()
}

#[derive(Deserialize)]
pub struct BodyData {
    title: String,
    text_content: String,
    html_content: String,
    idempotency_key: String,
}

pub async fn publish_newsletter(
    session: TypedSession,
    State(pool): State<Arc<PgPool>>,
    State(client): State<Arc<EmailClient>>,
    Json(body): Json<BodyData>,
) -> Response<Body> {
    let Some(user_id) = session.get_user_id().await.unwrap() else {
        return Redirect::to("/login").into_response();
    };

    let idempotency_key = IdempotencyKey::try_from(body.idempotency_key).unwrap();
    if let Some(saved_response) = get_saved_response(&pool, &idempotency_key, user_id).await {
        return saved_response;
    }

    let Ok(subscribers) = get_confirmed_subscribers(&pool).await else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    for subscriber in subscribers.into_iter().filter_map(|res| res.ok()) {
        if client
            .send_email(
                &subscriber.email,
                &body.title,
                &body.html_content,
                &body.text_content,
            )
            .await
            .is_err()
        {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    let response = StatusCode::OK.into_response();
    save_response(&pool, &idempotency_key, user_id, response)
        .await
        .unwrap()
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

async fn get_confirmed_subscribers(pool: &PgPool) -> Result<Vec<Result<ConfirmedSubscriber>>> {
    let confirmed_subscribers = sqlx::query!(
        r#"
        SELECT email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| match SubscriberEmail::parse(row.email) {
        Ok(email) => Ok(ConfirmedSubscriber { email }),
        Err(error) => Err(anyhow!(error)),
    })
    .collect();

    Ok(confirmed_subscribers)
}
