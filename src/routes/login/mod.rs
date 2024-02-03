use std::sync::Arc;

use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Redirect},
    Form,
};
use hmac::{Hmac, Mac};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use sqlx::PgPool;

use crate::authentication::{validate_credentials, Credentials};

#[derive(Deserialize)]
pub struct LoginParameters {
    error: String,
    tag: String,
}

impl LoginParameters {
    fn error(&self, secret: &Secret<String>) -> Option<String> {
        let tag = hex::decode(&self.tag).ok()?;

        let secret = secret.expose_secret().as_bytes();
        let mut mac = Hmac::<sha2::Sha256>::new_from_slice(secret).unwrap();

        mac.update(self.error.as_bytes());
        mac.verify_slice(&tag).ok()?;

        Some(self.error.clone())
    }
}

pub async fn login_get(
    params: Option<Query<LoginParameters>>,
    State(secret): State<Arc<Secret<String>>>,
) -> impl IntoResponse {
    let error_html = match params.and_then(|params| params.error(&secret)) {
        None => "".into(),
        Some(error) => {
            format!("<p><i>{}</i></p>", htmlescape::encode_minimal(&error))
        }
    };

    Html::from(format!(
        r#"<!DOCTYPE html>
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
    </html>"#
    ))
}

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

pub async fn login_post(
    State(pool): State<Arc<PgPool>>,
    State(secret): State<Arc<Secret<String>>>,
    Form(form): Form<FormData>,
) -> impl IntoResponse {
    let credentials = Credentials {
        username: form.username,
        password: form.password,
    };

    match validate_credentials(credentials, &pool).await {
        Some(_) => Redirect::to("/"),
        None => {
            let error = urlencoding::Encoded::new("Invalid credentials");

            let hmac_tag = {
                let secret = secret.expose_secret().as_bytes();
                let mut mac = Hmac::<sha2::Sha256>::new_from_slice(secret).unwrap();
                mac.update(error.0.as_bytes());
                mac.finalize().into_bytes()
            };

            Redirect::to(&format!("/login?error={error}&tag={hmac_tag:x}"))
        }
    }
}
