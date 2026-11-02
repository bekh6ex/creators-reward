use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use billing::EuroCents;
use billing::admin_service::{LogEntry, SettleError};
use billing::ceremony_format::{Batch, PeriodPublicKey, Totals};
use billing::period::PeriodId;

use crate::http::error::HttpError;
use crate::state::AdminState;

pub(super) fn axum_app(state: AdminState) -> Router {
    Router::new()
        .route("/periods/{period}/key", put(put_key))
        .route("/periods/{period}/batch", get(get_batch))
        .route("/periods/{period}/totals", post(post_totals))
        .route("/log", get(get_log))
        .with_state(state)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PutKeyRequest {
    public_key: String,
}

/// A public key from `billing-ceremony keygen`. Uploading the same key again
/// is a no-op; another key for the same period is refused, since records may
/// already be sealed to the first one.
async fn put_key(
    State(state): State<AdminState>,
    Path(period): Path<u64>,
    Json(request): Json<PutKeyRequest>,
) -> StatusCode {
    let Ok(key) = PeriodPublicKey::from_base64url(&request.public_key) else {
        return StatusCode::UNPROCESSABLE_ENTITY;
    };
    match state
        .billing
        .add_period_key(PeriodId::new(period), key)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT,
        Err(err) => err.status(),
    }
}

/// The ceremony's input: shuffled records with weights, no reader ids.
async fn get_batch(
    State(state): State<AdminState>,
    Path(period): Path<u64>,
) -> Result<Json<Batch>, StatusCode> {
    state
        .billing
        .batch(PeriodId::new(period))
        .await
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SettlementResponse {
    charged_cents: EuroCents,
    paid_out_cents: EuroCents,
}

/// The ceremony's output. Charges readers and pays creators.
async fn post_totals(
    State(state): State<AdminState>,
    Path(period): Path<u64>,
    Json(totals): Json<Totals>,
) -> Result<Json<SettlementResponse>, StatusCode> {
    if totals.period != PeriodId::new(period) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let summary = state
        .billing
        .settle(&totals)
        .await
        .map_err(|err| {
            if matches!(err, SettleError::MoreThanCharged { .. }) {
                tracing::error!("billing totals refused: {err}");
            }
            err.status()
        })?;
    Ok(Json(SettlementResponse {
        charged_cents: summary.charged,
        paid_out_cents: summary.paid_out,
    }))
}

async fn get_log(State(state): State<AdminState>) -> Json<Vec<LogEntry>> {
    Json(state.billing.log().await)
}
