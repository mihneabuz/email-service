use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use secrecy::Secret;
use sqlx::PgPool;
use time::Duration;

use crate::{
    authentication::{validate_credentials, Credentials},
    session_state::TypedSession,
};

pub async fn login_get(cookies: CookieJar) -> impl IntoResponse {
    let error_html = match cookies.get("_flash") {
        None => "".into(),
        Some(cookie) => {
            format!(
                "<p><i>{}</i></p>",
                htmlescape::encode_minimal(cookie.value())
            )
        }
    };

    let html = Html::from(format!(
        r#"
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta http-equiv="content-type" content="text/html; charset=utf-8">
            <title>Login</title>
        </head>
        <body>
            {error_html}
            <form action="/login" method="post">
                <label>Username
                <input type="text" placeholder="Enter Username" name="username">
                </label>

                <label>Password
                <input type="password" placeholder="Enter Password" name="password">
                </label>

                <button type="submit">Login</button>
            </form>
        </body>
        </html>
    "#
    ));

    let cookie = Cookie::build(("_flash", "")).max_age(Duration::ZERO);
    (CookieJar::new().add(cookie), html).into_response()
}

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

pub async fn login_post(
    session: TypedSession,
    State(pool): State<Arc<PgPool>>,
    Form(form): Form<FormData>,
) -> Response<Body> {
    let credentials = Credentials {
        username: form.username,
        password: form.password,
    };

    match validate_credentials(credentials, &pool).await {
        Some(user_id) => {
            session.renew().await.unwrap();
            session.insert_user_id(user_id).await.unwrap();

            Redirect::to("/admin/dashboard").into_response()
        }

        None => {
            let cookie = Cookie::new("_flash", "Authentication failed");
            (CookieJar::new().add(cookie), Redirect::to("/login")).into_response()
        }
    }
}
