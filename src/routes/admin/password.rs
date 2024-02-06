use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::extract::CookieJar;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use tower_sessions::cookie::Cookie;

use crate::{
    authentication::{validate_credentials, Credentials},
    routes::admin::get_username,
    session_state::TypedSession,
};

pub async fn change_password_form(cookies: CookieJar, session: TypedSession) -> Response<Body> {
    if session.get_user_id().await.unwrap().is_none() {
        return Redirect::to("/login").into_response();
    }

    let error_html = match cookies.get("_flash") {
        None => "".into(),
        Some(cookie) => {
            format!(
                "<p><i>{}</i></p>",
                htmlescape::encode_minimal(cookie.value())
            )
        }
    };

    Html::from(format!(
        r#"
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta http-equiv="content-type" content="text/html; charset=utf-8">
            <title>Change Password</title>
        </head>
        <body>
            {error_html}
            <form action="/admin/password" method="post">
                <label>Current password
                <input type="password" placeholder="Enter current password" name="current_password">
                </label>
                <br>

                <label>New password
                <input type="password" placeholder="Enter new password" name="new_password">
                </label>
                <br>

                <label>Confirm new password
                <input type="password" placeholder="Type the new password again" name="new_password_check">
                </label>
                <br>

                <button type="submit">Change password</button>
            </form>
            <p><a href="/admin/dashboard">&lt;- Back</a></p>
        </body>
        </html>
        "#,
    )).into_response()
}

#[derive(serde::Deserialize)]
pub struct ChangePassword {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

pub async fn change_password(
    State(pool): State<Arc<PgPool>>,
    session: TypedSession,
    Form(form): Form<ChangePassword>,
) -> Response<Body> {
    let Some(user_id) = session.get_user_id().await.unwrap() else {
        return Redirect::to("/login").into_response();
    };

    let username = get_username(user_id, &pool).await.unwrap();

    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        let cookie = Cookie::new(
            "_flash",
            "You entered two different new passwords - the field values must match.",
        );

        return (
            CookieJar::new().add(cookie),
            Redirect::to("/admin/password"),
        )
            .into_response();
    }

    if form.new_password.expose_secret().len() < 10 {
        let cookie = Cookie::new("_flash", "The new password is too short.");

        return (
            CookieJar::new().add(cookie),
            Redirect::to("/admin/password"),
        )
            .into_response();
    }

    let credentials = Credentials {
        username,
        password: form.current_password,
    };

    if validate_credentials(credentials, &pool).await.is_none() {
        let cookie = Cookie::new("_flash", "The current password is incorrect.");

        return (
            CookieJar::new().add(cookie),
            Redirect::to("/admin/password"),
        )
            .into_response();
    }

    todo!()
}
