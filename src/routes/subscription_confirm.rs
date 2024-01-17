use axum::{http::StatusCode, extract::Query};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

pub async fn confirm(params: Query<Parameters>) -> StatusCode {
    StatusCode::OK
}
