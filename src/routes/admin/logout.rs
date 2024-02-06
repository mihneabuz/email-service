use axum::{
    body::Body,
    response::{IntoResponse, Redirect, Response},
};
use tower_sessions::cookie::{Cookie, CookieJar};

use crate::session_state::TypedSession;

pub async fn log_out(session: TypedSession) -> Response<Body> {
    if session.get_user_id().await.unwrap().is_none() {
        Redirect::to("/login").into_response()
    } else {
        session.log_out().await.unwrap();

        let cookie = Cookie::new("_flash", "You have successfully logged out.");
        (CookieJar::new().add(cookie), Redirect::to("/login")).into_response()
    }
}
