use axum::{Router, routing::get};
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;

use crate::state::AppState;

mod auth;
mod billing;
mod events;

pub(super) fn axum_app(state: AppState) -> Router {
    let middleware = ServiceBuilder::new().layer(CookieManagerLayer::new());
    Router::new()
        // Outside /events on purpose: see `embed_js`.
        .route("/embed.js", get(events::web::embed_js))
        .nest("/events", events::axum_app(state.clone()))
        .nest("/auth", auth::axum_app(state.clone()))
        .nest("/billing", billing::axum_app(state.clone()))
        .layer(middleware)
}
