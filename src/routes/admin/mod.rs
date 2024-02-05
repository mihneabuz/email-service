use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use sqlx::{types::Uuid, PgPool};

use crate::session_state::TypedSession;

pub async fn admin_dashboard(
    State(pool): State<Arc<PgPool>>,
    session: TypedSession,
) -> Response<Body> {
    let Ok(user_id) = session.get_user_id().await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let username = if let Some(user_id) = user_id {
        get_username(user_id, &pool).await.unwrap()
    } else {
        return Redirect::to("/login").into_response();
    };

    Html::from(format!(
        r#"
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta http-equiv="content-type" content="text/html; charset=utf-8">
            <title>Admin dashboard</title>
        </head>
        <body>
            <p>Welcome {username}!</p>
        </body>
        </html>
        "#
    ))
    .into_response()
}

async fn get_username(user_id: Uuid, pool: &PgPool) -> Option<String> {
    let row = sqlx::query!(
        r#"
        SELECT username
        FROM users
        WHERE user_id = $1
        "#,
        user_id,
    )
    .fetch_one(pool)
    .await
    .ok()?;

    Some(row.username)
}
