//! The admin gateway: what key holders and operators call, on its own port.
//! Every route needs `Authorization: Bearer <token>`.

use axum::Router;
use axum::body::Body;
use axum::extract::Request;
use axum::http::{Response, StatusCode};
use tower_http::validate_request::{ValidateRequest, ValidateRequestHeaderLayer};

use crate::state::AdminState;

mod billing;

/// Hardcoded and published with the source: anyone who can reach the admin
/// port is an admin. Acceptable only because the port listens on loopback;
/// must be replaced before anything is deployed.
pub(crate) const DEV_ADMIN_TOKEN: &str = "dev-admin-token";

#[derive(Clone)]
struct AdminBearerValidator;

impl ValidateRequest<Body> for AdminBearerValidator {
    type ResponseBody = Body;

    fn validate(
        &mut self,
        request: &mut Request,
    ) -> Result<(), Response<Self::ResponseBody>> {
        let expected = format!("Bearer {DEV_ADMIN_TOKEN}");
        let header = request.headers().get("Authorization");
        if header.map(|value| value.as_bytes()) == Some(expected.as_bytes()) {
            Ok(())
        } else {
            let mut response = Response::new(Body::empty());
            *response.status_mut() = StatusCode::UNAUTHORIZED;
            Err(response)
        }
    }
}

pub(super) fn axum_app(state: AdminState) -> Router {
    Router::new()
        .nest("/admin/billing", billing::axum_app(state))
        .layer(ValidateRequestHeaderLayer::custom(
            AdminBearerValidator,
        ))
}
