use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Form};
use chrono::Utc;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::Deserialize;
use sqlx::types::Uuid;
use sqlx::PgPool;
use tracing::{error, info, warn};
use ulid::Ulid;

use crate::{domain::NewSubscriber, email_client::EmailClient};

#[derive(Deserialize)]
pub struct SubscribeData {
    pub name: String,
    pub email: String,
}

pub async fn subscribe(
    State(pool): State<Arc<PgPool>>,
    State(email): State<Arc<EmailClient>>,
    State(base_url): State<Arc<str>>,
    Form(form): Form<SubscribeData>,
) -> StatusCode {
    info!("new subscriber {} <{}>", form.name, form.email);

    let Ok(new_subscriber) = NewSubscriber::try_from(form) else {
        return StatusCode::BAD_REQUEST;
    };

    let id = match insert_subscriber(&pool, &new_subscriber).await {
        Ok(id) => id,

        Err(sqlx::Error::Database(e)) => {
            warn!("database error: {:?}", e);
            return match e.kind() {
                sqlx::error::ErrorKind::UniqueViolation => StatusCode::CONFLICT,
                _ => StatusCode::BAD_REQUEST,
            };
        }

        Err(other) => {
            error!("failed to execute query: {:?}", other);
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };

    let token = generate_subscriptions_token();
    let Ok(()) = store_token(&pool, id, &token).await else {
        return StatusCode::INTERNAL_SERVER_ERROR;
    };

    match send_email(&email, &new_subscriber, &base_url, &token).await {
        Ok(()) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn generate_subscriptions_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

async fn insert_subscriber(pool: &PgPool, sub: &NewSubscriber) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::from_bytes(Ulid::new().to_bytes());

    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        id,
        sub.email.as_ref(),
        sub.name.as_ref(),
        Utc::now()
    )
    .execute(pool)
    .await?;

    Ok(id)
}

pub async fn store_token(
    pool: &PgPool,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)
        "#,
        subscription_token,
        subscriber_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn send_email(
    client: &EmailClient,
    subscriber: &NewSubscriber,
    base_url: &str,
    token: &str,
) -> anyhow::Result<()> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, token
    );

    client
        .send_email(
            subscriber.email.clone(),
            "Welcome!",
            &format!(
                "Welcome to our newsletter!\nClink {} to confirm.",
                confirmation_link
            ),
            &format!(
                "Welcome to our newsletter!\nClink {} to confirm.",
                confirmation_link
            ),
        )
        .await
}
