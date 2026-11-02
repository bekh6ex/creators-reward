use axum::Router;
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;

use crate::state::AppState;

pub(super) mod web;

pub(crate) fn axum_app(state: AppState) -> Router {
    let middleware = ServiceBuilder::new().layer(CookieManagerLayer::new());

    Router::new()
        .nest("/web", web::axum_app(state))
        .layer(middleware)
}
