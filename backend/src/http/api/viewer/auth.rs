use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use tower_cookies::cookie::SameSite;
use tower_cookies::cookie::time::Duration;
use tower_cookies::{Cookie, Cookies};

use crate::http::error::HttpError;
use crate::state::AppState;
use auth::public_service::{LoginResult, ViewerKeyMaterial};

pub(super) fn axum_app(state: AppState) -> Router {
    Router::new()
        .route("/register/start", post(register_start))
        .route("/register/finish", post(register_finish))
        .route("/login/start", post(login_start))
        .route("/login/finish", post(login_finish))
        .route("/logout", post(logout))
        .route("/session", get(session))
        .with_state(state.clone())
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegisterStartRequest {
    email: String,
    registration_request: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegisterStartResponse {
    registration_response: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegisterFinishRequest {
    email: String,
    registration_record: String,
    public_key: String,
    encrypted_private_key: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginStartRequest {
    email: String,
    start_login_request: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginStartResponse {
    login_flow_id: String,
    login_response: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginFinishRequest {
    login_flow_id: String,
    finish_login_request: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginFinishResponse {
    encrypted_private_key: String,
}

async fn register_start(
    State(state): State<AppState>,
    Json(p): Json<RegisterStartRequest>,
) -> Result<Json<RegisterStartResponse>, StatusCode> {
    let email = p.email.to_lowercase().trim().to_string();
    let registration_response = state
        .auth
        .register_start(email, p.registration_request)
        .await
        .map_err(|e| e.status())?;
    Ok(Json(RegisterStartResponse {
        registration_response,
    }))
}

#[expect(
    clippy::disallowed_methods,
    reason = "the viewer's own events token and session id are placed into HttpOnly Secure cookies"
)]
async fn register_finish(
    State(state): State<AppState>,
    cookies: Cookies,
    Json(p): Json<RegisterFinishRequest>,
) -> StatusCode {
    let email = p.email.to_lowercase().trim().to_string();
    let result = match state
        .auth
        .register_finish(
            email,
            p.registration_record,
            ViewerKeyMaterial {
                public_key: p.public_key,
                encrypted_private_key: p.encrypted_private_key,
            },
        )
        .await
    {
        Ok(result) => result,
        Err(err) => return err.status(),
    };
    let config = state.auth.config;
    cookies.add(
        Cookie::build((
            config.events_token_cookie_name.clone(),
            result.events_token.expose_secret().to_string(),
        ))
        .path(config.events_path.clone())
        .http_only(true)
        .max_age(Duration::days(365))
        .secure(true)
        .same_site(SameSite::None)
        .build(),
    );
    cookies.add(
        Cookie::build((
            "cr_session",
            result.session_id.expose_secret().to_string(),
        ))
        .path("/")
        .http_only(true)
        .max_age(Duration::hours(24))
        .secure(true)
        .same_site(SameSite::Lax)
        .build(),
    );
    StatusCode::OK
}

async fn login_start(
    State(state): State<AppState>,
    Json(p): Json<LoginStartRequest>,
) -> Result<Json<LoginStartResponse>, StatusCode> {
    let email = p.email.to_lowercase().trim().to_string();
    let (login_flow_id, login_response) = state
        .auth
        .login_start(email, p.start_login_request)
        .await
        .map_err(|e| e.status())?;
    Ok(Json(LoginStartResponse {
        login_flow_id,
        login_response,
    }))
}

#[expect(
    clippy::disallowed_methods,
    reason = "the viewer's own events token and session id are placed into HttpOnly Secure cookies"
)]
async fn login_finish(
    State(state): State<AppState>,
    cookies: Cookies,
    Json(p): Json<LoginFinishRequest>,
) -> Result<Json<LoginFinishResponse>, StatusCode> {
    let LoginResult {
        session: result,
        encrypted_private_key,
    } = state
        .auth
        .login_finish(p.login_flow_id, p.finish_login_request)
        .await
        .map_err(|e| e.status())?;
    let config = state.auth.config;
    cookies.add(
        Cookie::build((
            config.events_token_cookie_name.clone(),
            result.events_token.expose_secret().to_string(),
        ))
        .path(config.events_path.clone())
        .http_only(true)
        .max_age(Duration::days(365))
        .secure(true)
        .same_site(SameSite::None)
        .build(),
    );
    cookies.add(
        Cookie::build((
            "cr_session",
            result.session_id.expose_secret().to_string(),
        ))
        .path("/")
        .http_only(true)
        .max_age(Duration::hours(24))
        .secure(true)
        .same_site(SameSite::Lax)
        .build(),
    );
    Ok(Json(LoginFinishResponse {
        encrypted_private_key,
    }))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionResponse {
    email: String,
}

async fn logout(State(state): State<AppState>, cookies: Cookies) -> StatusCode {
    if let Some(session_cookie) = cookies.get("cr_session") {
        let session_id = session_cookie.value().to_string();
        state.auth.logout(session_id);
    }

    let config = state.auth.config;

    cookies.add(
        Cookie::build((config.events_token_cookie_name.clone(), ""))
            .path(config.events_path.clone())
            .http_only(true)
            .max_age(Duration::seconds(0))
            .secure(true)
            .same_site(SameSite::None)
            .build(),
    );

    cookies.add(
        Cookie::build(("cr_session", ""))
            .path("/")
            .http_only(true)
            .max_age(Duration::seconds(0))
            .secure(true)
            .same_site(SameSite::Lax)
            .build(),
    );

    StatusCode::NO_CONTENT
}

async fn session(
    State(state): State<AppState>,
    cookies: Cookies,
) -> Result<Json<SessionResponse>, StatusCode> {
    let session_id = cookies
        .get("cr_session")
        .map(|c| c.value().to_string())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    match state.auth.session(session_id) {
        Some(email) => Ok(Json(SessionResponse { email })),
        None => Err(StatusCode::UNAUTHORIZED),
    }
}
