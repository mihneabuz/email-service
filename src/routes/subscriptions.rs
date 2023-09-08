use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Form};
use chrono::Utc;
use serde::Deserialize;
use sqlx::types::Uuid;
use sqlx::PgPool;
use ulid::Ulid;

#[derive(Deserialize)]
pub struct SubscribeData {
    name: String,
    email: String,
}

pub async fn subscribe(
    State(pool): State<Arc<PgPool>>,
    Form(form): Form<SubscribeData>,
) -> StatusCode {
    let result = sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::from_bytes(Ulid::new().to_bytes()),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(pool.as_ref())
    .await;

    match result {
        Ok(_) => StatusCode::OK,

        Err(sqlx::Error::Database(e)) => {
            println!("failed to execute query: {}", e);
            match e.kind() {
                sqlx::error::ErrorKind::UniqueViolation => StatusCode::CONFLICT,
                _ => StatusCode::BAD_REQUEST,
            }
        }

        Err(other) => {
            println!("failed to execute query: {}", other);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
