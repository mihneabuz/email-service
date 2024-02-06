use async_trait::async_trait;
use axum::{extract::FromRequestParts, http::request::Parts};
use tower_sessions::{session::Error, Session};
use uuid::Uuid;

pub struct TypedSession(Session);

impl TypedSession {
    const USER_ID_KEY: &'static str = "user_id";

    pub async fn renew(&self) -> Result<(), Error> {
        self.0.cycle_id().await
    }

    pub async fn insert_user_id(&self, user_id: Uuid) -> Result<(), Error> {
        self.0.insert(Self::USER_ID_KEY, user_id).await
    }

    pub async fn get_user_id(&self) -> Result<Option<Uuid>, Error> {
        self.0.get(Self::USER_ID_KEY).await
    }

    pub async fn log_out(self) -> Result<(), Error> {
        self.0.delete().await
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for TypedSession
where
    S: Send + Sync,
{
    type Rejection = <Session as FromRequestParts<S>>::Rejection;

    async fn from_request_parts(req: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(req, state).await?;
        Ok(Self(session))
    }
}
