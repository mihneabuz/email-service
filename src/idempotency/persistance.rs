use std::str::FromStr;

use super::IdempotencyKey;

use axum::{
    body::Body,
    http::{HeaderMap, HeaderName, HeaderValue, Response, StatusCode},
    response::IntoResponse,
};
use futures::StreamExt;
use sqlx::{postgres::PgHasArrayType, Acquire, PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "header_pair")]
struct HeaderPairRecord {
    name: String,
    value: Vec<u8>,
}

impl PgHasArrayType for HeaderPairRecord {
    fn array_type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("_header_pair")
    }
}

pub enum NextAction {
    StartProcessing(Transaction<'static, Postgres>),
    ReturnSavedResponse(Response<Body>),
}

pub async fn try_processing(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
) -> Option<NextAction> {
    let mut transaction = pool.begin().await.ok()?;

    let n_inserted_rows = sqlx::query!(
        r#"
        INSERT INTO idempotency (
            user_id,
            idempotency_key,
            created_at
        )
        VALUES ($1, $2, now())
        ON CONFLICT DO NOTHING
        "#,
        user_id,
        idempotency_key.as_ref()
    )
    .execute(transaction.acquire().await.ok()?)
    .await
    .ok()?
    .rows_affected();

    if n_inserted_rows > 0 {
        Some(NextAction::StartProcessing(transaction))
    } else {
        let saved_response = get_saved_response(pool, idempotency_key, user_id).await?;
        Some(NextAction::ReturnSavedResponse(saved_response))
    }
}

pub async fn get_saved_response(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
) -> Option<Response<Body>> {
    let record = sqlx::query!(
        r#"
        SELECT
            response_status_code,
            response_headers as "response_headers: Vec<HeaderPairRecord>",
            response_body
        FROM idempotency
        WHERE
            user_id = $1 AND
            idempotency_key = $2
        "#,
        user_id,
        idempotency_key.as_ref()
    )
    .fetch_optional(pool)
    .await
    .ok()??;

    let status_code = StatusCode::from_u16(record.response_status_code?.try_into().ok()?).ok()?;

    let header_map = record.response_headers?.into_iter().fold(
        HeaderMap::new(),
        |mut acc, HeaderPairRecord { name, value }| {
            if let (Ok(name), Ok(value)) =
                (HeaderName::from_str(&name), HeaderValue::from_bytes(&value))
            {
                acc.append(name, value);
            }

            acc
        },
    );

    Some((header_map, status_code).into_response())
}

pub async fn save_response(
    mut transaction: Transaction<'static, Postgres>,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
    http_response: Response<Body>,
) -> Result<Response<Body>, anyhow::Error> {
    let status_code = http_response.status().as_u16() as i16;

    let headers = {
        let mut h = Vec::with_capacity(http_response.headers().len());
        for (name, value) in http_response.headers().iter() {
            let name = name.as_str().to_owned();
            let value = value.as_bytes().to_owned();
            h.push(HeaderPairRecord { name, value });
        }
        h
    };

    let (parts, body) = http_response.into_parts();
    let bytes = body
        .into_data_stream()
        .fold(Vec::new(), |mut acc, b| async move {
            acc.extend_from_slice(&b.unwrap_or_default());
            acc
        })
        .await;

    sqlx::query_unchecked!(
        r#"
        UPDATE idempotency
        SET
            response_status_code = $3,
            response_headers = $4,
            response_body = $5
        WHERE
            user_id = $1 AND
            idempotency_key = $2
        "#,
        user_id,
        idempotency_key.as_ref(),
        status_code,
        headers,
        bytes
    )
    .execute(transaction.acquire().await?)
    .await?;

    transaction.commit().await?;

    Ok((parts, Body::from(bytes)).into_response())
}
