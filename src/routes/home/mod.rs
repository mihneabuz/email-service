use axum::response::{Html, IntoResponse};

pub async fn home() -> impl IntoResponse {
    Html::from(include_str!("home.html"))
}
