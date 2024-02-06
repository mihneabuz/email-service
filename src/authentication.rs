use argon2::{
    password_hash::SaltString, Algorithm, Argon2, Params, PasswordHash, PasswordHasher,
    PasswordVerifier, Version,
};
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

fn compute_password_hash(password: Secret<String>) -> Option<Secret<String>> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(15000, 2, 1, None).unwrap(),
    )
    .hash_password(password.expose_secret().as_bytes(), &salt)
    .ok()?
    .to_string();

    Some(Secret::new(password_hash))
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

pub async fn change_password(
    user_id: uuid::Uuid,
    password: Secret<String>,
    pool: &PgPool,
) -> Option<()> {
    let password_hash = tokio::task::spawn_blocking(move || compute_password_hash(password))
        .await
        .ok()??;

    sqlx::query!(
        r#"
        UPDATE users
        SET password_hash = $1
        WHERE user_id = $2
        "#,
        password_hash.expose_secret(),
        user_id
    )
    .execute(pool)
    .await
    .ok()?;

    Some(())
}
