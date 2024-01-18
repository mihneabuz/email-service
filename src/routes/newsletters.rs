use std::sync::Arc;

use anyhow::{anyhow, Result};
use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;
use sqlx::PgPool;

use crate::{domain::SubscriberEmail, email_client::EmailClient};

#[derive(Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}
#[derive(Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

pub async fn publish_newsletter(
    State(pool): State<Arc<PgPool>>,
    State(client): State<Arc<EmailClient>>,
    Json(body): Json<BodyData>,
) -> StatusCode {
    let Ok(subscribers) = get_confirmed_subscribers(&pool).await else {
        return StatusCode::INTERNAL_SERVER_ERROR;
    };

    for subscriber in subscribers.into_iter().filter_map(|res| res.ok()) {
        if client
            .send_email(
                &subscriber.email,
                &body.title,
                &body.content.html,
                &body.content.text,
            )
            .await
            .is_err()
        {
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    }

    StatusCode::OK
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
