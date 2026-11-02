use askama::Template;
use axum::extract::{Path, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_SECURITY_POLICY, CONTENT_TYPE};
use axum::{
    Json, Router,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use tower_cookies::Cookies;

use auth::RawViewerEventsToken;
use auth::ValidatedViewerEventsToken;
use billing::ceremony_format::SEALED_USAGE_LEN;
use common::ViewerEventsId;
use common::billing_usage::UsageRecorded;
use web_viewer_events::enabled_donations::{EnabledDonationId, SealedEnabledDonation};

use crate::http::error::HttpError;
use crate::state::AppState;

pub(crate) fn axum_app(state: AppState) -> Router {
    let middleware = ServiceBuilder::new();

    let router: Router<()> = Router::new()
        .route("/usage", post(post_usage))
        .route("/public-key", get(public_key))
        .route(
            "/enabled-donations",
            post(enable_donations).get(enabled_donations),
        )
        .route(
            "/enabled-donations/{id}",
            get(donations_enabled).delete(disable_donations),
        )
        .route("/iframe.html", get(iframe))
        .route("/hpke.js", get(hpke_js))
        .with_state(state)
        .layer(middleware);
    router
}

#[derive(Template)]
#[template(path = "events/web/iframe.html")]
struct IframeTemplate;

/// Event-collecting iframe, embedded by creator sites. Served under the events
/// path so the events token cookie attaches to this document request.
async fn iframe() -> Response {
    match IframeTemplate.render() {
        Ok(html) => (
            [
                // Embeddable by any site; rendered per viewer, so never cached.
                (CONTENT_SECURITY_POLICY, "frame-ancestors *"),
                (CACHE_CONTROL, "no-store"),
            ],
            Html(html),
        )
            .into_response(),
        Err(err) => {
            tracing::error!("iframe template rendering failed: {err}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[derive(Template)]
#[template(path = "events/web/embed.js", escape = "none")]
struct EmbedScriptTemplate;

/// Script a creator site includes to add the events iframe to its pages.
///
/// Served at `/api/embed.js`, deliberately outside the events cookie path:
/// the request comes from the creator's page, so it carries that page as
/// `Referer`, and it must not carry the viewer's events token next to it.
pub(crate) async fn embed_js() -> Response {
    match EmbedScriptTemplate.render() {
        Ok(js) => (
            [
                (CONTENT_TYPE, "text/javascript; charset=utf-8"),
                (CACHE_CONTROL, "no-cache"),
            ],
            js,
        )
            .into_response(),
        Err(err) => {
            tracing::error!("embed.js template rendering failed: {err}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// HPKE sender used by the iframe to seal records in the browser. A plain
/// file, not a template: nothing in it varies per request.
async fn hpke_js() -> Response {
    (
        [
            (CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (CACHE_CONTROL, "no-cache"),
        ],
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/events/web/hpke.js"
        )),
    )
        .into_response()
}

/// One unit of reading for billing, sealed in the iframe to the period key,
/// plus the enabled-donation record that allows it, like a session.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageRequest {
    enabled_donation_id: EnabledDonationId,
    period: u64,
    sealed: String,
}

async fn post_usage(
    State(state): State<AppState>,
    cookies: Cookies,
    Json(request): Json<UsageRequest>,
) -> StatusCode {
    let Some(viewer_id) = events_viewer(&state, &cookies).await else {
        return StatusCode::UNAUTHORIZED;
    };
    let Some(sealed) = URL_SAFE_NO_PAD
        .decode(request.sealed)
        .ok()
        .filter(|bytes| bytes.len() == SEALED_USAGE_LEN)
    else {
        return StatusCode::UNPROCESSABLE_ENTITY;
    };
    if !state
        .web_events
        .donations_enabled(viewer_id, request.enabled_donation_id)
        .await
    {
        tracing::debug!("usage discarded: donations not enabled");
        return StatusCode::ACCEPTED;
    }
    // Sealed to another period's key: the iframe should fetch the key again.
    let (period, _) = match state.billing.current_period_key().await {
        Ok(key) => key,
        Err(billing::service::CurrentPeriodKeyError) => {
            return StatusCode::CONFLICT;
        }
    };
    if period.get() != request.period {
        return StatusCode::CONFLICT;
    }
    let usage = UsageRecorded {
        viewer_id,
        period: request.period,
        sealed,
    };
    match state.web_events.record_usage(usage).await {
        Ok(()) => StatusCode::ACCEPTED,
        Err(err) => {
            tracing::error!("usage queue failed: {err:?}");
            err.status()
        }
    }
}

/// The viewer behind the events token cookie, if the token is valid.
async fn events_viewer(state: &AppState, cookies: &Cookies) -> Option<ViewerEventsId> {
    let cookie_name = state
        .config
        .auth_service_config()
        .events_token_cookie_name;
    let raw_token: String = cookies.get(&cookie_name)?.value().into();
    let pub_key = state
        .internal_auth
        .viewer_event_token_pub_key()
        .await;
    match ValidatedViewerEventsToken::try_from(
        RawViewerEventsToken::from(raw_token),
        pub_key,
    ) {
        Ok(token) => Some(token.viewer_id()),
        Err(err) => {
            tracing::info!("{:?}", err);
            None
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicKeyResponse {
    public_key: String,
}

/// The key the iframe seals enabled donations to (raw X25519, base64url).
async fn public_key(
    State(state): State<AppState>,
    cookies: Cookies,
) -> Result<Json<PublicKeyResponse>, StatusCode> {
    let viewer_id = events_viewer(&state, &cookies)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let public_key = state
        .web_events
        .public_key(viewer_id)
        .await
        .map_err(|e| e.status())?;
    Ok(Json(PublicKeyResponse {
        public_key: URL_SAFE_NO_PAD.encode(public_key.as_bytes()),
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EnableDonationsRequest {
    /// Sealed in the browser; the site never reaches the server.
    sealed: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EnableDonationsResponse {
    id: EnabledDonationId,
}

/// Sent by the events iframe's "Enable donations" button.
async fn enable_donations(
    State(state): State<AppState>,
    cookies: Cookies,
    Json(request): Json<EnableDonationsRequest>,
) -> Result<(StatusCode, Json<EnableDonationsResponse>), StatusCode> {
    let viewer_id = events_viewer(&state, &cookies)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let sealed = URL_SAFE_NO_PAD
        .decode(request.sealed)
        .ok()
        .and_then(|bytes| SealedEnabledDonation::try_from(bytes).ok())
        .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
    let id = state
        .web_events
        .enable_donations(viewer_id, sealed)
        .await;
    Ok((
        StatusCode::CREATED,
        Json(EnableDonationsResponse { id }),
    ))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SealedEnabledDonationItem {
    id: EnabledDonationId,
    sealed: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EnabledDonationsResponse {
    /// The recipient key the records were sealed to; HPKE binds it into the
    /// key schedule, and the browser cannot derive it from its private key.
    public_key: String,
    items: Vec<SealedEnabledDonationItem>,
}

/// Lists the viewer's sealed enabled donations. Only the viewer's browser can
/// open them.
async fn enabled_donations(
    State(state): State<AppState>,
    cookies: Cookies,
) -> Result<Json<EnabledDonationsResponse>, StatusCode> {
    let viewer_id = events_viewer(&state, &cookies)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let public_key = state
        .web_events
        .public_key(viewer_id)
        .await
        .map_err(|e| e.status())?;
    let items = state
        .web_events
        .enabled_donations(viewer_id)
        .await
        .into_iter()
        .map(|(id, sealed)| SealedEnabledDonationItem {
            id,
            sealed: URL_SAFE_NO_PAD.encode(sealed.as_bytes()),
        })
        .collect();
    Ok(Json(EnabledDonationsResponse {
        public_key: URL_SAFE_NO_PAD.encode(public_key.as_bytes()),
        items,
    }))
}

/// Lets the iframe check that the record it remembers for this site still
/// exists. The id is random, so the check says nothing about the site.
async fn donations_enabled(
    State(state): State<AppState>,
    cookies: Cookies,
    Path(id): Path<EnabledDonationId>,
) -> StatusCode {
    let Some(viewer_id) = events_viewer(&state, &cookies).await else {
        return StatusCode::UNAUTHORIZED;
    };
    if state
        .web_events
        .donations_enabled(viewer_id, id)
        .await
    {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

/// Always 204: removed, or nothing to remove. Idempotent, and it does not
/// tell a caller whether the id ever existed.
async fn disable_donations(
    State(state): State<AppState>,
    cookies: Cookies,
    Path(id): Path<EnabledDonationId>,
) -> StatusCode {
    let Some(viewer_id) = events_viewer(&state, &cookies).await else {
        return StatusCode::UNAUTHORIZED;
    };
    state
        .web_events
        .disable_donations(viewer_id, id)
        .await;
    StatusCode::NO_CONTENT
}
