use argon2::{Argon2, PasswordHash, PasswordVerifier};
use secrecy::{ExposeSecret, Secret};
use sqlx::{types::Uuid, PgPool};

pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}

pub async fn validate_credentials(credentials: Credentials, pool: &PgPool) -> Option<Uuid> {
    let user = sqlx::query!(
        r#"
        SELECT user_id, password_hash
        FROM users
        WHERE username = $1
        "#,
        credentials.username,
    )
    .fetch_optional(pool)
    .await
    .ok()??;

    tokio::task::spawn_blocking(move || {
        verify_password_hash(Secret::new(user.password_hash), credentials.password)
    })
    .await
    .ok()??;

    Some(user.user_id)
}

pub fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Option<()> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret()).ok()?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .ok()?;

    Some(())
}
