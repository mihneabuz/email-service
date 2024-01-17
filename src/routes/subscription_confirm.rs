use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use sqlx::{types::Uuid, PgPool};

#[derive(Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

pub async fn confirm(State(pool): State<Arc<PgPool>>, params: Query<Parameters>) -> StatusCode {
    let id = match get_subscriber_id_from_token(&pool, &params.subscription_token).await {
        Ok(id) => id,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };

    match id {
        None => StatusCode::UNAUTHORIZED,
        Some(subscriber_id) => {
            let Ok(()) = confirm_subscriber(&pool, subscriber_id).await else {
                return StatusCode::INTERNAL_SERVER_ERROR;
            };

            StatusCode::OK
        }
    }
}

pub async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1"#,
        subscription_token,
    )
    .fetch_optional(pool)
    .await?;

    Ok(result.map(|r| r.subscriber_id))
}
