use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Form};
use chrono::Utc;
use serde::Deserialize;
use sqlx::types::Uuid;
use sqlx::PgPool;
use tracing::{error, info, warn};
use ulid::Ulid;

use crate::domain::NewSubscriber;

#[derive(Deserialize)]
pub struct SubscribeData {
    pub name: String,
    pub email: String,
}

pub async fn subscribe(
    State(pool): State<Arc<PgPool>>,
    Form(form): Form<SubscribeData>,
) -> StatusCode {
    info!("new subscriber {} <{}>", form.name, form.email);

    let Ok(new_subscriber) = NewSubscriber::try_from(form) else {
        return StatusCode::BAD_REQUEST;
    };

    match insert_subscriber(&pool, &new_subscriber).await {
        Ok(()) => {
            info!("new subscriber saved");
            StatusCode::OK
        }

        Err(sqlx::Error::Database(e)) => {
            warn!("database error: {:?}", e);
            match e.kind() {
                sqlx::error::ErrorKind::UniqueViolation => StatusCode::CONFLICT,
                _ => StatusCode::BAD_REQUEST,
            }
        }

        Err(other) => {
            error!("failed to execute query: {:?}", other);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

pub async fn insert_subscriber(pool: &PgPool, sub: &NewSubscriber) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'confirmed')
        "#,
        Uuid::from_bytes(Ulid::new().to_bytes()),
        sub.email.as_ref(),
        sub.name.as_ref(),
        Utc::now()
    )
    .execute(pool)
    .await?;

    Ok(())
}
