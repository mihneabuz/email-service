mod dashboard;
mod logout;
mod password;

pub use dashboard::*;
pub use logout::*;
pub use password::*;

use sqlx::PgPool;
use uuid::Uuid;

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
