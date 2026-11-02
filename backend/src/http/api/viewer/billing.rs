use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use billing::period::PeriodId;

use crate::http::error::HttpError;
use crate::state::AppState;

pub(crate) fn axum_app(state: AppState) -> Router {
    Router::new()
        .route("/period-key", get(period_key))
        .with_state(state)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PeriodKeyResponse {
    period: PeriodId,
    public_key: String,
}

/// The current period's public key, which the events iframe seals billing
/// records to. Needs no cookie and the iframe sends none: the key is the
/// same for every reader, and the server cannot hand out per-reader keys if
/// it does not know who asks.
async fn period_key(
    State(state): State<AppState>,
) -> Result<Json<PeriodKeyResponse>, StatusCode> {
    let (period, key) = state
        .billing
        .current_period_key()
        .await
        .map_err(|e| e.status())?;
    Ok(Json(PeriodKeyResponse {
        period,
        public_key: key.to_base64url(),
    }))
}
