use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use sqlx::PgPool;

use crate::session_state::TypedSession;

use super::get_username;

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
            <p>Available actions:</p>

            <ol>
                <li><a href="/admin/password">Change password</a></li>
                <li>
                    <form name="logoutForm" action="/logout" method="post">
                        <input type="submit" value="Logout">
                    </form>
                </li>
            </ol>
        </body>
        </html>
        "#
    ))
    .into_response()
}
