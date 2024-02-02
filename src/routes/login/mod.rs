use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{Response, StatusCode},
    response::{Html, IntoResponse, Redirect},
    Form,
};
use secrecy::Secret;
use sqlx::PgPool;

use crate::authentication::{validate_credentials, Credentials};

pub async fn login_get() -> impl IntoResponse {
    Html::from(include_str!("login.html"))
}

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

pub async fn login_post(
    State(pool): State<Arc<PgPool>>,
    Form(form): Form<FormData>,
) -> Response<Body> {
    let credentials = Credentials {
        username: form.username,
        password: form.password,
    };

    match validate_credentials(credentials, &pool).await {
        Some(_) => Redirect::to("/").into_response(),
        None => StatusCode::UNAUTHORIZED.into_response(),
    }
}
