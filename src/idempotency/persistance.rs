use std::str::FromStr;

use super::IdempotencyKey;

use axum::{
    body::Body,
    http::{HeaderMap, HeaderName, HeaderValue, Response, StatusCode},
    response::IntoResponse,
};
use futures::StreamExt;
use sqlx::{postgres::PgHasArrayType, PgPool};
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

    let status_code = StatusCode::from_u16(record.response_status_code.try_into().ok()?).ok()?;

    let header_map = record.response_headers.into_iter().fold(
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
    pool: &PgPool,
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
        INSERT INTO idempotency (
        user_id,
        idempotency_key,
        response_status_code,
        response_headers,
        response_body,
        created_at
        )
        VALUES ($1, $2, $3, $4, $5, now())
        "#,
        user_id,
        idempotency_key.as_ref(),
        status_code,
        headers,
        bytes
    )
    .execute(pool)
    .await?;

    Ok((parts, Body::from(bytes)).into_response())
}
