use axum::Router;

use crate::state::AppState;

mod viewer;

pub(super) fn axum_app(state: AppState) -> Router {
    viewer::axum_app(state)
}
