use axum::response::{Html, IntoResponse};

pub async fn login() -> impl IntoResponse {
    Html::from(include_str!("login.html"))
}
