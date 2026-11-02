//! The creator service's HTTP API, on its own port
//! (`CR_CREATOR_API_PORT`) as if it were a separately deployed service; the
//! creator portal reaches it as `/api/*` on its own host. Its session cookie is
//! its own, so a creator session and a viewer session never stand in for each
//! other.

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use tower_cookies::cookie::SameSite;
use tower_cookies::cookie::time::Duration;
use tower_cookies::{Cookie, CookieManagerLayer, Cookies};

use ::creator::public_service::{Creator, Website};

use crate::http::error::HttpError;
use crate::state::CreatorState;

const SESSION_COOKIE: &str = "cr_creator_session";

const API_PREFIX: &str = "/api";

pub(super) fn axum_app(state: CreatorState) -> Router {
    let api = Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/session", get(session))
        .route("/websites", get(websites).post(add_website))
        .with_state(state)
        .layer(ServiceBuilder::new().layer(CookieManagerLayer::new()));
    Router::new().nest(API_PREFIX, api)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Credentials {
    email: String,
    password: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionResponse {
    email: String,
}

fn session_cookie(value: String, max_age: Duration) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, value))
        .path(API_PREFIX)
        .http_only(true)
        .max_age(max_age)
        .secure(true)
        .same_site(SameSite::Strict)
        .build()
}

async fn register(
    State(state): State<CreatorState>,
    cookies: Cookies,
    Json(credentials): Json<Credentials>,
) -> StatusCode {
    match state
        .creator
        .register(&credentials.email, &credentials.password)
        .await
    {
        Ok(session) => {
            cookies.add(session_cookie(session, Duration::hours(24)));
            StatusCode::CREATED
        }
        Err(err) => err.status(),
    }
}

async fn login(
    State(state): State<CreatorState>,
    cookies: Cookies,
    Json(credentials): Json<Credentials>,
) -> StatusCode {
    match state
        .creator
        .login(&credentials.email, &credentials.password)
        .await
    {
        Ok(session) => {
            cookies.add(session_cookie(session, Duration::hours(24)));
            StatusCode::NO_CONTENT
        }
        Err(err) => err.status(),
    }
}

async fn logout(State(state): State<CreatorState>, cookies: Cookies) -> StatusCode {
    if let Some(cookie) = cookies.get(SESSION_COOKIE) {
        state.creator.logout(cookie.value()).await;
    }
    cookies.add(session_cookie(String::new(), Duration::seconds(0)));
    StatusCode::NO_CONTENT
}

async fn current_creator(state: &CreatorState, cookies: &Cookies) -> Option<Creator> {
    let session = cookies.get(SESSION_COOKIE)?;
    state.creator.session(session.value()).await
}

async fn session(
    State(state): State<CreatorState>,
    cookies: Cookies,
) -> Result<Json<SessionResponse>, StatusCode> {
    let creator = current_creator(&state, &cookies)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;
    Ok(Json(SessionResponse {
        email: creator.email,
    }))
}

async fn websites(
    State(state): State<CreatorState>,
    cookies: Cookies,
) -> Result<Json<Vec<Website>>, StatusCode> {
    let creator = current_creator(&state, &cookies)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;
    Ok(Json(state.creator.websites(creator.id).await))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddWebsiteRequest {
    domain: String,
    #[serde(default)]
    title: String,
}

async fn add_website(
    State(state): State<CreatorState>,
    cookies: Cookies,
    Json(request): Json<AddWebsiteRequest>,
) -> Result<(StatusCode, Json<Website>), StatusCode> {
    let creator = current_creator(&state, &cookies)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;
    match state
        .creator
        .add_website(creator.id, &request.domain, &request.title)
        .await
    {
        Ok(website) => {
            tracing::info!("creator connected {}", website.domain);
            Ok((StatusCode::CREATED, Json(website)))
        }
        Err(err) => Err(err.status()),
    }
}
