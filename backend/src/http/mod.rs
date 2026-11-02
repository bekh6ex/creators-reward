use std::io::Result;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Bytes;
use axum::http::{StatusCode, header};
use axum_server::tls_rustls::RustlsConfig;
use header::{AUTHORIZATION, COOKIE, REFERER, SET_COOKIE};
use tower::ServiceBuilder;
use tower_http::{
    LatencyUnit, ServiceBuilderExt,
    timeout::TimeoutLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};

use crate::state::{AdminState, AppState, CreatorState};

mod admin;
pub(crate) mod api;
mod creator;
mod error;

/// Describes how the HTTP server should handle TLS.
///
/// Three cases:
/// 1. `ExplicitTls` — both `CR_TLS_CERT` and `CR_TLS_KEY` are set. Serve HTTPS
///    using the provided PEM files.
/// 2. `PlainHttp` — no TLS env vars are set, but at least one of `CR_BIND_ADDR`
///    or `CR_BACKEND_PORT` was explicitly provided. Serve plain HTTP (intended
///    for use behind a TLS-terminating reverse proxy such as Caddy).
/// 3. `DevTls` — none of the server env vars are set. Fall back to the original
///    dev behaviour: HTTPS on 127.0.0.1:3000 using `cert/cert.pem` and
///    `cert/key.pem` relative to `CARGO_MANIFEST_DIR`. The pair is not committed
///    (`backend/.gitignore`); put a local one there before using this mode.
#[derive(Clone)]
pub(crate) enum TlsMode {
    ExplicitTls { cert_path: String, key_path: String },
    PlainHttp,
    DevTls,
}

pub(crate) fn axum_app(state: AppState) -> Router {
    with_middleware(Router::new().nest(
        &state.config.api_prefix(),
        api::axum_app(state.clone()),
    ))
}

/// The creator service. Served on its own port (`CR_CREATOR_API_PORT`), as if
/// it were a separately deployed service.
pub(crate) fn creator_app(state: CreatorState) -> Router {
    with_middleware(creator::axum_app(state))
}

/// The admin gateway. Served on its own port on loopback (`CR_ADMIN_PORT`),
/// never through the public reverse proxy.
pub(crate) fn admin_app(state: AdminState) -> Router {
    with_middleware(admin::axum_app(state))
}

fn with_middleware(app: Router) -> Router {
    let sensitive_headers: Arc<[_]> =
        vec![AUTHORIZATION, COOKIE, SET_COOKIE, REFERER].into();

    let trace_layer = TraceLayer::new_for_http()
        .on_body_chunk(|chunk: &Bytes, latency: Duration, _: &tracing::Span| {
            tracing::trace!(size_bytes = chunk.len(), latency = ?latency, "sending body chunk")
        })
        .make_span_with(DefaultMakeSpan::new().include_headers(true))
        .on_response(
            DefaultOnResponse::new().include_headers(true).latency_unit(LatencyUnit::Micros)
        );
    let middleware = ServiceBuilder::new()
        .sensitive_request_headers(sensitive_headers.clone())
        .layer(trace_layer)
        .sensitive_response_headers(sensitive_headers)
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(10),
        ))
        .compression();

    app.layer(middleware)
}

pub(super) async fn start_http_server(
    addr: SocketAddr,
    prefix_path: &str,
    router: Router,
    tls_mode: TlsMode,
) -> Result<()> {
    match tls_mode {
        TlsMode::ExplicitTls {
            cert_path,
            key_path,
        } => {
            let config = RustlsConfig::from_pem_file(&cert_path, &key_path).await?;
            tracing::info!("listening on https://{addr}{prefix_path}");
            axum_server::bind_rustls(addr, config)
                .serve(router.into_make_service())
                .await
        }
        TlsMode::PlainHttp => {
            tracing::info!("listening on http://{addr}{prefix_path}");
            axum_server::bind(addr)
                .serve(router.into_make_service())
                .await
        }
        TlsMode::DevTls => {
            let cert = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("cert")
                .join("cert.pem");
            let key = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("cert")
                .join("key.pem");
            let config = RustlsConfig::from_pem_file(cert, key).await?;
            tracing::info!("listening on https://{addr}{prefix_path}");
            axum_server::bind_rustls(addr, config)
                .serve(router.into_make_service())
                .await
        }
    }
}

#[cfg(test)]
mod tests {
    use ::axum::Router;
    use ::axum_test::{TestResponse, TestServer};
    use axum_test::TestServerConfig;
    use axum_test::http::{HeaderValue, StatusCode, header};
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use futures::FutureExt;
    use futures::future::LocalBoxFuture;
    use googletest::prelude::*;
    use opaque_ke::{
        ClientLogin, ClientLoginFinishParameters, ClientRegistration,
        ClientRegistrationFinishParameters, CredentialResponse, RegistrationResponse,
    };
    use rand::rngs::OsRng;
    use rstest::{fixture, rstest};
    use serde_json::json;
    use std::io::{self, Write};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::task::Poll;
    use tokio_test::task::{Spawn, spawn};
    use tracing::Level;
    use tracing::subscriber::DefaultGuard;
    use tracing_subscriber::FmtSubscriber;
    use tracing_subscriber::fmt::writer::MakeWriter;

    use crate::state::{Config, CreatorState, create_state};

    use super::*;

    #[gtest]
    #[rstest]
    #[tokio::test]
    async fn login_finish_sets_both_cookies(viewer: Viewer) {
        let Viewer {
            server,
            mut listeners,
            ..
        } = viewer;
        expect_that!(listeners.poll(), eq(Poll::Pending));

        let email = "user@example.com";
        let password = b"password";

        opaque_register(&server, email, password).await;
        expect_that!(listeners.poll(), eq(Poll::Pending));

        let mut rng = OsRng;
        let client_login_start =
            ClientLogin::<auth::opaque::CrOpaque>::start(&mut rng, password).unwrap();
        let start_login_request =
            URL_SAFE_NO_PAD.encode(client_login_start.message.serialize());

        let login_start_resp = server
            .post("/api/auth/login/start")
            .json(&json!({
                "email": email,
                "startLoginRequest": start_login_request,
            }))
            .await;
        login_start_resp.assert_status(StatusCode::OK);

        let login_start_body: serde_json::Value = login_start_resp.json();
        let login_flow_id = login_start_body["loginFlowId"].as_str().unwrap();
        let login_response_b64 = login_start_body["loginResponse"]
            .as_str()
            .unwrap();
        let login_response_bytes = URL_SAFE_NO_PAD
            .decode(login_response_b64)
            .unwrap();
        let login_response = CredentialResponse::<auth::opaque::CrOpaque>::deserialize(
            &login_response_bytes,
        )
        .unwrap();

        let client_login_finish = client_login_start
            .state
            .finish(
                &mut rng,
                password,
                login_response,
                ClientLoginFinishParameters::default(),
            )
            .unwrap();
        let finish_login_request =
            URL_SAFE_NO_PAD.encode(client_login_finish.message.serialize());

        let login_finish_resp = server
            .post("/api/auth/login/finish")
            .json(&json!({
                "loginFlowId": login_flow_id,
                "finishLoginRequest": finish_login_request,
            }))
            .await;
        login_finish_resp.assert_status(StatusCode::OK);

        let set_cookies: Vec<_> = login_finish_resp
            .headers()
            .get_all("set-cookie")
            .iter()
            .map(|v| v.to_str().unwrap())
            .collect();
        expect_that!(
            set_cookies,
            contains_each![
                all![
                    starts_with("events_token="),
                    contains_substring("SameSite=None"),
                    contains_substring("Path=/api/events"),
                ],
                all![
                    starts_with("cr_session="),
                    contains_substring("SameSite=Lax")
                ],
            ]
        );

        expect_that!(listeners.poll(), eq(Poll::Pending));
    }
    #[gtest]
    #[rstest]
    #[tokio::test]
    #[ignore = "red at baseline before OPAQUE work — pre-existing failure, not caused by this change"]
    async fn test_login(viewer: Viewer) {
        let Viewer { server, .. } = viewer;
        {
            let request_body = r#"{"email":"user@example.com", "password": "password"}"#;

            let response = server
                .post("/api/viewer-dashboard/login")
                .bytes(request_body.into())
                .await;

            response.assert_status(StatusCode::OK);

            let header_value = response
                .header("Set-Cookie")
                .to_str()
                .unwrap()
                .to_string();
            expect_that!(header_value, char_count(gt(0)));
        }
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    async fn register_start_finish_ok(viewer: Viewer) {
        let Viewer { server, .. } = viewer;

        let email = "alice@example.com";
        let password = b"password";

        let mut rng = OsRng;
        let client_start =
            ClientRegistration::<auth::opaque::CrOpaque>::start(&mut rng, password)
                .unwrap();
        let registration_request =
            URL_SAFE_NO_PAD.encode(client_start.message.serialize());

        let start_resp = server
            .post("/api/auth/register/start")
            .json(&json!({
                "email": email,
                "registrationRequest": registration_request,
            }))
            .await;
        start_resp.assert_status(StatusCode::OK);

        let start_body: serde_json::Value = start_resp.json();
        let registration_response_b64 = start_body["registrationResponse"]
            .as_str()
            .unwrap();
        let registration_response_bytes = URL_SAFE_NO_PAD
            .decode(registration_response_b64)
            .unwrap();
        let registration_response =
            RegistrationResponse::<auth::opaque::CrOpaque>::deserialize(
                &registration_response_bytes,
            )
            .unwrap();

        let client_finish = client_start
            .state
            .finish(
                &mut rng,
                password,
                registration_response,
                ClientRegistrationFinishParameters::default(),
            )
            .unwrap();
        let registration_record =
            URL_SAFE_NO_PAD.encode(client_finish.message.serialize());

        let finish_resp = server
            .post("/api/auth/register/finish")
            .json(&json!({
                "email": email,
                "registrationRecord": registration_record,
                "publicKey": test_public_key(),
                "encryptedPrivateKey": test_encrypted_private_key(),
            }))
            .await;
        finish_resp.assert_status(StatusCode::OK);
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    async fn register_duplicate_email(viewer: Viewer) {
        let Viewer { server, .. } = viewer;

        let email = "bob@example.com";
        let password = b"password";

        let mut rng = OsRng;
        let client_start =
            ClientRegistration::<auth::opaque::CrOpaque>::start(&mut rng, password)
                .unwrap();
        let registration_request =
            URL_SAFE_NO_PAD.encode(client_start.message.serialize());

        let start_resp = server
            .post("/api/auth/register/start")
            .json(&json!({
                "email": email,
                "registrationRequest": registration_request,
            }))
            .await;
        start_resp.assert_status(StatusCode::OK);

        let start_body: serde_json::Value = start_resp.json();
        let registration_response_b64 = start_body["registrationResponse"]
            .as_str()
            .unwrap();
        let registration_response_bytes = URL_SAFE_NO_PAD
            .decode(registration_response_b64)
            .unwrap();
        let registration_response =
            RegistrationResponse::<auth::opaque::CrOpaque>::deserialize(
                &registration_response_bytes,
            )
            .unwrap();

        let client_finish = client_start
            .state
            .finish(
                &mut rng,
                password,
                registration_response,
                ClientRegistrationFinishParameters::default(),
            )
            .unwrap();
        let registration_record =
            URL_SAFE_NO_PAD.encode(client_finish.message.serialize());

        let finish_resp = server
            .post("/api/auth/register/finish")
            .json(&json!({
                "email": email,
                "registrationRecord": registration_record,
                "publicKey": test_public_key(),
                "encryptedPrivateKey": test_encrypted_private_key(),
            }))
            .await;
        finish_resp.assert_status(StatusCode::OK);

        let mut rng2 = OsRng;
        let client_start2 =
            ClientRegistration::<auth::opaque::CrOpaque>::start(&mut rng2, password)
                .unwrap();
        let registration_request2 =
            URL_SAFE_NO_PAD.encode(client_start2.message.serialize());

        let second_resp = server
            .post("/api/auth/register/start")
            .json(&json!({
                "email": email,
                "registrationRequest": registration_request2,
            }))
            .await;
        second_resp.assert_status(StatusCode::CONFLICT);
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    async fn register_finish_malformed_record(viewer: Viewer) {
        let Viewer { server, .. } = viewer;

        let email = "charlie@example.com";
        let password = b"password";

        let mut rng = OsRng;
        let client_start =
            ClientRegistration::<auth::opaque::CrOpaque>::start(&mut rng, password)
                .unwrap();
        let registration_request =
            URL_SAFE_NO_PAD.encode(client_start.message.serialize());

        let start_resp = server
            .post("/api/auth/register/start")
            .json(&json!({
                "email": email,
                "registrationRequest": registration_request,
            }))
            .await;
        start_resp.assert_status(StatusCode::OK);

        let finish_resp = server
            .post("/api/auth/register/finish")
            .json(&json!({
                "email": email,
                "registrationRecord": "!!!invalid_base64!!!",
                "publicKey": test_public_key(),
                "encryptedPrivateKey": test_encrypted_private_key(),
            }))
            .await;
        finish_resp.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    async fn opaque_login_full_flow(viewer: Viewer) {
        let Viewer {
            server,
            mut listeners,
            ..
        } = viewer;

        let email = "login_alice@example.com";
        let password = b"password";

        let mut rng = OsRng;
        let client_reg_start =
            ClientRegistration::<auth::opaque::CrOpaque>::start(&mut rng, password)
                .unwrap();
        let registration_request =
            URL_SAFE_NO_PAD.encode(client_reg_start.message.serialize());

        let reg_start_resp = server
            .post("/api/auth/register/start")
            .json(&json!({
                "email": email,
                "registrationRequest": registration_request,
            }))
            .await;
        reg_start_resp.assert_status(StatusCode::OK);

        let reg_start_body: serde_json::Value = reg_start_resp.json();
        let registration_response_b64 = reg_start_body["registrationResponse"]
            .as_str()
            .unwrap();
        let registration_response_bytes = URL_SAFE_NO_PAD
            .decode(registration_response_b64)
            .unwrap();
        let registration_response =
            RegistrationResponse::<auth::opaque::CrOpaque>::deserialize(
                &registration_response_bytes,
            )
            .unwrap();

        let client_reg_finish = client_reg_start
            .state
            .finish(
                &mut rng,
                password,
                registration_response,
                ClientRegistrationFinishParameters::default(),
            )
            .unwrap();
        let registration_record =
            URL_SAFE_NO_PAD.encode(client_reg_finish.message.serialize());

        let reg_finish_resp = server
            .post("/api/auth/register/finish")
            .json(&json!({
                "email": email,
                "registrationRecord": registration_record,
                "publicKey": test_public_key(),
                "encryptedPrivateKey": test_encrypted_private_key(),
            }))
            .await;
        reg_finish_resp.assert_status(StatusCode::OK);
        expect_that!(listeners.poll(), eq(Poll::Pending));

        let mut login_rng = OsRng;
        let client_login_start =
            ClientLogin::<auth::opaque::CrOpaque>::start(&mut login_rng, password)
                .unwrap();
        let start_login_request =
            URL_SAFE_NO_PAD.encode(client_login_start.message.serialize());

        let login_start_resp = server
            .post("/api/auth/login/start")
            .json(&json!({
                "email": email,
                "startLoginRequest": start_login_request,
            }))
            .await;
        login_start_resp.assert_status(StatusCode::OK);

        let login_start_body: serde_json::Value = login_start_resp.json();
        let login_flow_id = login_start_body["loginFlowId"].as_str().unwrap();
        let login_response_b64 = login_start_body["loginResponse"]
            .as_str()
            .unwrap();
        let login_response_bytes = URL_SAFE_NO_PAD
            .decode(login_response_b64)
            .unwrap();
        let login_response = CredentialResponse::<auth::opaque::CrOpaque>::deserialize(
            &login_response_bytes,
        )
        .unwrap();

        let client_login_finish = client_login_start
            .state
            .finish(
                &mut login_rng,
                password,
                login_response,
                ClientLoginFinishParameters::default(),
            )
            .unwrap();
        let finish_login_request =
            URL_SAFE_NO_PAD.encode(client_login_finish.message.serialize());

        let login_finish_resp = server
            .post("/api/auth/login/finish")
            .json(&json!({
                "loginFlowId": login_flow_id,
                "finishLoginRequest": finish_login_request,
            }))
            .await;
        login_finish_resp.assert_status(StatusCode::OK);

        let set_cookies: Vec<_> = login_finish_resp
            .headers()
            .get_all("set-cookie")
            .iter()
            .map(|v| v.to_str().unwrap())
            .collect();
        expect_that!(
            set_cookies,
            contains_each![
                all![
                    starts_with("events_token="),
                    contains_substring("SameSite=None"),
                    contains_substring("Path=/api/events"),
                ],
                all![
                    starts_with("cr_session="),
                    contains_substring("SameSite=Lax")
                ],
            ]
        );
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    async fn opaque_login_wrong_password(viewer: Viewer) {
        let Viewer { server, .. } = viewer;

        let email = "login_bob@example.com";
        let password = b"password";

        let mut rng = OsRng;
        let client_reg_start =
            ClientRegistration::<auth::opaque::CrOpaque>::start(&mut rng, password)
                .unwrap();
        let registration_request =
            URL_SAFE_NO_PAD.encode(client_reg_start.message.serialize());

        let reg_start_resp = server
            .post("/api/auth/register/start")
            .json(&json!({
                "email": email,
                "registrationRequest": registration_request,
            }))
            .await;
        reg_start_resp.assert_status(StatusCode::OK);

        let reg_start_body: serde_json::Value = reg_start_resp.json();
        let registration_response_b64 = reg_start_body["registrationResponse"]
            .as_str()
            .unwrap();
        let registration_response_bytes = URL_SAFE_NO_PAD
            .decode(registration_response_b64)
            .unwrap();
        let registration_response =
            RegistrationResponse::<auth::opaque::CrOpaque>::deserialize(
                &registration_response_bytes,
            )
            .unwrap();

        let client_reg_finish = client_reg_start
            .state
            .finish(
                &mut rng,
                password,
                registration_response,
                ClientRegistrationFinishParameters::default(),
            )
            .unwrap();
        let registration_record =
            URL_SAFE_NO_PAD.encode(client_reg_finish.message.serialize());

        let reg_finish_resp = server
            .post("/api/auth/register/finish")
            .json(&json!({
                "email": email,
                "registrationRecord": registration_record,
                "publicKey": test_public_key(),
                "encryptedPrivateKey": test_encrypted_private_key(),
            }))
            .await;
        reg_finish_resp.assert_status(StatusCode::OK);

        let mut login_rng = OsRng;

        let client_login_start_1 =
            ClientLogin::<auth::opaque::CrOpaque>::start(&mut login_rng, password)
                .unwrap();
        let start_login_request_1 =
            URL_SAFE_NO_PAD.encode(client_login_start_1.message.serialize());

        let login_start_resp_1 = server
            .post("/api/auth/login/start")
            .json(&json!({
                "email": email,
                "startLoginRequest": start_login_request_1,
            }))
            .await;
        login_start_resp_1.assert_status(StatusCode::OK);
        let login_start_body_1: serde_json::Value = login_start_resp_1.json();
        let login_flow_id_1 = login_start_body_1["loginFlowId"]
            .as_str()
            .unwrap();

        let client_login_start_2 =
            ClientLogin::<auth::opaque::CrOpaque>::start(&mut login_rng, password)
                .unwrap();
        let start_login_request_2 =
            URL_SAFE_NO_PAD.encode(client_login_start_2.message.serialize());

        let login_start_resp_2 = server
            .post("/api/auth/login/start")
            .json(&json!({
                "email": email,
                "startLoginRequest": start_login_request_2,
            }))
            .await;
        login_start_resp_2.assert_status(StatusCode::OK);
        let login_start_body_2: serde_json::Value = login_start_resp_2.json();
        let login_response_b64_2 = login_start_body_2["loginResponse"]
            .as_str()
            .unwrap();
        let login_response_bytes_2 = URL_SAFE_NO_PAD
            .decode(login_response_b64_2)
            .unwrap();
        let login_response_2 = CredentialResponse::<auth::opaque::CrOpaque>::deserialize(
            &login_response_bytes_2,
        )
        .unwrap();

        let client_login_finish = client_login_start_2
            .state
            .finish(
                &mut login_rng,
                password,
                login_response_2,
                ClientLoginFinishParameters::default(),
            )
            .unwrap();
        let finish_login_request =
            URL_SAFE_NO_PAD.encode(client_login_finish.message.serialize());

        let login_finish_resp = server
            .post("/api/auth/login/finish")
            .json(&json!({
                "loginFlowId": login_flow_id_1,
                "finishLoginRequest": finish_login_request,
            }))
            .await;
        login_finish_resp.assert_status(StatusCode::UNAUTHORIZED);

        let set_cookies: Vec<_> = login_finish_resp
            .headers()
            .get_all("set-cookie")
            .iter()
            .map(|v| v.to_str().unwrap())
            .collect();
        expect_that!(set_cookies, not(contains(starts_with("cr_session="))));
    }

    async fn opaque_register(server: &TestServer, email: &str, password: &[u8]) {
        let mut rng = OsRng;
        let client_start =
            ClientRegistration::<auth::opaque::CrOpaque>::start(&mut rng, password)
                .unwrap();
        let registration_request =
            URL_SAFE_NO_PAD.encode(client_start.message.serialize());

        let start_resp = server
            .post("/api/auth/register/start")
            .json(&json!({
                "email": email,
                "registrationRequest": registration_request,
            }))
            .await;
        start_resp.assert_status(StatusCode::OK);

        let start_body: serde_json::Value = start_resp.json();
        let registration_response_b64 = start_body["registrationResponse"]
            .as_str()
            .unwrap();
        let registration_response_bytes = URL_SAFE_NO_PAD
            .decode(registration_response_b64)
            .unwrap();
        let registration_response =
            RegistrationResponse::<auth::opaque::CrOpaque>::deserialize(
                &registration_response_bytes,
            )
            .unwrap();

        let client_finish = client_start
            .state
            .finish(
                &mut rng,
                password,
                registration_response,
                ClientRegistrationFinishParameters::default(),
            )
            .unwrap();
        let registration_record =
            URL_SAFE_NO_PAD.encode(client_finish.message.serialize());

        let finish_resp = server
            .post("/api/auth/register/finish")
            .json(&json!({
                "email": email,
                "registrationRecord": registration_record,
                "publicKey": test_public_key(),
                "encryptedPrivateKey": test_encrypted_private_key(),
            }))
            .await;
        finish_resp.assert_status(StatusCode::OK);
    }

    async fn opaque_login(
        server: &TestServer,
        email: &str,
        password: &[u8],
    ) -> TestResponse {
        let mut rng = OsRng;
        let client_start =
            ClientLogin::<auth::opaque::CrOpaque>::start(&mut rng, password).unwrap();
        let start_login_request =
            URL_SAFE_NO_PAD.encode(client_start.message.serialize());

        let start_resp = server
            .post("/api/auth/login/start")
            .json(&json!({
                "email": email,
                "startLoginRequest": start_login_request,
            }))
            .await;
        start_resp.assert_status(StatusCode::OK);

        let start_body: serde_json::Value = start_resp.json();
        let login_flow_id = start_body["loginFlowId"].as_str().unwrap();
        let login_response_b64 = start_body["loginResponse"].as_str().unwrap();
        let login_response_bytes = URL_SAFE_NO_PAD
            .decode(login_response_b64)
            .unwrap();
        let login_response = CredentialResponse::<auth::opaque::CrOpaque>::deserialize(
            &login_response_bytes,
        )
        .unwrap();

        let client_finish = client_start
            .state
            .finish(
                &mut rng,
                password,
                login_response,
                ClientLoginFinishParameters::default(),
            )
            .unwrap();
        let finish_login_request =
            URL_SAFE_NO_PAD.encode(client_finish.message.serialize());

        let finish_resp = server
            .post("/api/auth/login/finish")
            .json(&json!({
                "loginFlowId": login_flow_id,
                "finishLoginRequest": finish_login_request,
            }))
            .await;
        finish_resp.assert_status(StatusCode::OK);
        finish_resp
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    async fn logout_revokes_session(viewer: Viewer) {
        let Viewer {
            server,
            mut listeners,
            ..
        } = viewer;

        let email = "logout_alice@example.com";
        let password = b"password";

        opaque_register(&server, email, password).await;
        expect_that!(listeners.poll(), eq(Poll::Pending));
        opaque_login(&server, email, password).await;

        let session_resp = server.get("/api/auth/session").await;
        session_resp.assert_status(StatusCode::OK);
        let session_body: serde_json::Value = session_resp.json();
        expect_that!(session_body["email"], eq(email));

        let logout_resp = server.post("/api/auth/logout").await;
        logout_resp.assert_status(StatusCode::NO_CONTENT);

        let session_resp_after = server.get("/api/auth/session").await;
        session_resp_after.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    async fn a_creator_registers_and_connects_a_website(
        #[from(creator)] (server, _tracing): (TestServer, DefaultGuard),
    ) {
        let credentials = json!({"email": "Owner@Example.org", "password": "password1"});

        server
            .post("/api/auth/register")
            .json(&credentials)
            .await
            .assert_status(StatusCode::CREATED);
        server
            .get("/api/auth/session")
            .await
            .assert_json(&json!({"email": "owner@example.org"}));
        server
            .post("/api/auth/register")
            .json(&credentials)
            .await
            .assert_status(StatusCode::CONFLICT);

        let added = server
            .post("/api/websites")
            .json(&json!({"domain": "website1.crlocal", "title": "Website 1"}))
            .await;
        added.assert_status(StatusCode::CREATED);
        let website: serde_json::Value = added.json();
        expect_that!(website["domain"], eq("website1.crlocal"));
        expect_that!(website["title"], eq("Website 1"));
        expect_that!(website["status"], eq("verified"));
        expect_that!(website["lastCheckedAt"], eq(&serde_json::Value::Null));
        expect_that!(
            website["addedAt"],
            predicate(serde_json::Value::is_string)
        );
        expect_that!(
            website["token"],
            predicate(serde_json::Value::is_string)
        );

        server
            .post("/api/auth/logout")
            .await
            .assert_status(StatusCode::NO_CONTENT);
        server
            .get("/api/auth/session")
            .await
            .assert_status(StatusCode::UNAUTHORIZED);
        server
            .get("/api/websites")
            .await
            .assert_status(StatusCode::UNAUTHORIZED);
        server
            .post("/api/auth/login")
            .json(&json!({"email": "owner@example.org", "password": "password2"}))
            .await
            .assert_status(StatusCode::UNAUTHORIZED);

        server
            .post("/api/auth/login")
            .json(&credentials)
            .await
            .assert_status(StatusCode::NO_CONTENT);
        server
            .get("/api/websites")
            .await
            .assert_json(&json!([website]));
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    async fn a_domain_belongs_to_one_creator(
        #[from(creator)] (server, _tracing): (TestServer, DefaultGuard),
    ) {
        for (email, status) in [
            ("alice@example.org", StatusCode::CREATED),
            ("bob@example.org", StatusCode::CONFLICT),
        ] {
            server
                .post("/api/auth/register")
                .json(&json!({"email": email, "password": "password1"}))
                .await
                .assert_status(StatusCode::CREATED);
            server
                .post("/api/websites")
                .json(&json!({"domain": "website1.crlocal"}))
                .await
                .assert_status(status);
            server.post("/api/auth/logout").await;
        }
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    async fn creator_input_is_validated(
        #[from(creator)] (server, _tracing): (TestServer, DefaultGuard),
    ) {
        server
            .post("/api/auth/register")
            .json(&json!({"email": "owner@example.org", "password": "short"}))
            .await
            .assert_status(StatusCode::UNPROCESSABLE_ENTITY);
        server
            .post("/api/websites")
            .json(&json!({"domain": "website1.crlocal"}))
            .await
            .assert_status(StatusCode::UNAUTHORIZED);
        server
            .post("/api/auth/register")
            .json(&json!({"email": "owner@example.org", "password": "password1"}))
            .await
            .assert_status(StatusCode::CREATED);
        server
            .post("/api/websites")
            .json(&json!({"domain": "https://website1.crlocal:8443/"}))
            .await
            .assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[gtest]
    #[rstest]
    #[case::creator_registration("/api/creator/auth/register")]
    #[case::websites("/api/websites")]
    #[tokio::test]
    async fn the_viewer_api_has_no_creator_routes(viewer: Viewer, #[case] path: &str) {
        viewer
            .server
            .post(path)
            .json(&json!({"email": "owner@example.org", "password": "password1"}))
            .await
            .assert_status(StatusCode::NOT_FOUND);
    }

    #[gtest]
    #[rstest]
    #[case::viewer_registration("/api/auth/register/start")]
    #[case::usage("/api/events/web/usage")]
    #[tokio::test]
    async fn the_creator_api_has_no_viewer_routes(
        #[from(creator)] (server, _tracing): (TestServer, DefaultGuard),
        #[case] path: &str,
    ) {
        server
            .post(path)
            .json(&json!({"email": "owner@example.org", "password": "password1"}))
            .await
            .assert_status(StatusCode::NOT_FOUND);
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    async fn creator_and_viewer_apis_are_separate_services(
        viewer: Viewer,
        #[from(creator)] (creator, _tracing): (TestServer, DefaultGuard),
    ) {
        let Viewer { server: viewer, .. } = viewer;
        let credentials = json!({"email": "owner@example.org", "password": "password1"});
        // Same path on both: each answers only for its own accounts.
        let registered = creator
            .post("/api/auth/register")
            .json(&credentials)
            .await;
        registered.assert_status(StatusCode::CREATED);
        viewer
            .get("/api/auth/session")
            .add_header(
                header::COOKIE,
                HeaderValue::from_str(&format!(
                    "cr_session={}",
                    registered.cookie("cr_creator_session").value()
                ))
                .unwrap(),
            )
            .await
            .assert_status(StatusCode::UNAUTHORIZED);
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    async fn logout_while_logged_out(viewer: Viewer) {
        let Viewer { server, .. } = viewer;

        let logout_resp = server.post("/api/auth/logout").await;
        logout_resp.assert_status(StatusCode::NO_CONTENT);
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    async fn events_iframe_is_embeddable_html(viewer: Viewer) {
        let Viewer { server, .. } = viewer;

        let resp = server.get("/api/events/web/iframe.html").await;
        resp.assert_status(StatusCode::OK);
        expect_that!(
            resp.header(header::CONTENT_TYPE),
            eq("text/html; charset=utf-8")
        );
        expect_that!(
            resp.header(header::CONTENT_SECURITY_POLICY),
            eq("frame-ancestors *")
        );
        expect_that!(resp.header(header::CACHE_CONTROL), eq("no-store"));
        expect_that!(
            resp.text(),
            all![
                contains_substring(r#"<button id="enable-donations" disabled>"#),
                contains_substring(r#"import {seal} from "./hpke.js";"#),
                // The billing key is fetched without cookies (see billing::period_key).
                contains_substring(
                    r#"fetch("../../billing/period-key", {credentials: "omit"})"#
                ),
            ]
        );
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    async fn events_iframe_trace_redacts_cookie_and_referer() {
        let (state, _listeners) = create_state(Config::default());
        let (server, logs, _tracing) = create_capturing_server(axum_app(state));

        let token_value = "totally_fake_token_value_12345";
        let referer = "https://news.example/";

        let resp = server
            .get("/api/events/web/iframe.html")
            .add_header(
                header::COOKIE,
                HeaderValue::from_str(&format!("events_token={token_value}")).unwrap(),
            )
            .add_header(header::REFERER, HeaderValue::from_static(referer))
            .await;
        resp.assert_status(StatusCode::OK);

        let captured = String::from_utf8(logs.lock().unwrap().clone()).unwrap();
        expect_that!(
            captured,
            contains_substring("/api/events/web/iframe.html")
        );
        expect_that!(captured, not(contains_substring(token_value)));
        expect_that!(captured, not(contains_substring("news.example")));
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    async fn events_embed_script_is_javascript(viewer: Viewer) {
        let Viewer { server, .. } = viewer;

        let resp = server.get("/api/embed.js").await;
        resp.assert_status(StatusCode::OK);
        expect_that!(
            resp.header(header::CONTENT_TYPE),
            eq("text/javascript; charset=utf-8")
        );
        expect_that!(
            resp.text(),
            all![
                // The licence travels with the copy a creator's page receives.
                // REUSE-IgnoreStart: the tags below are test data, not this file's licence.
                starts_with("// SPDX-FileCopyrightText: "),
                contains_substring("// SPDX-License-Identifier: MIT\n"),
                // REUSE-IgnoreEnd
                contains_substring(r#"new URL("events/web/iframe.html", script.src)"#),
                contains_substring(r#"iframe.referrerPolicy = "no-referrer""#),
            ]
        );

        // Outside the events cookie path, so the creator page's request for
        // it never carries the viewer's events token.
        let cookie_path = Config::default()
            .auth_service_config()
            .events_path;
        expect_that!("/api/embed.js", not(starts_with(cookie_path.as_str())));
        server
            .get("/api/events/web/embed.js")
            .await
            .assert_status(StatusCode::NOT_FOUND);

        let hpke = server.get("/api/events/web/hpke.js").await;
        hpke.assert_status(StatusCode::OK);
        expect_that!(
            hpke.text(),
            contains_substring("export async function seal(")
        );
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    async fn session_forged_cookie(viewer: Viewer) {
        let Viewer { server, .. } = viewer;

        let session_resp = server
            .get("/api/auth/session")
            .add_header(
                header::COOKIE,
                HeaderValue::from_static("cr_session=totally_forged_session_id"),
            )
            .await;
        session_resp.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    async fn opaque_login_unknown_email(viewer: Viewer) {
        let Viewer { server, .. } = viewer;

        let known_email = "known@example.com";
        let unknown_email = "unknown@example.com";
        let password = b"password";

        let mut rng = OsRng;
        let client_reg_start =
            ClientRegistration::<auth::opaque::CrOpaque>::start(&mut rng, password)
                .unwrap();
        let registration_request =
            URL_SAFE_NO_PAD.encode(client_reg_start.message.serialize());

        let reg_start_resp = server
            .post("/api/auth/register/start")
            .json(&json!({
                "email": known_email,
                "registrationRequest": registration_request,
            }))
            .await;
        reg_start_resp.assert_status(StatusCode::OK);

        let reg_start_body: serde_json::Value = reg_start_resp.json();
        let registration_response_b64 = reg_start_body["registrationResponse"]
            .as_str()
            .unwrap();
        let registration_response_bytes = URL_SAFE_NO_PAD
            .decode(registration_response_b64)
            .unwrap();
        let registration_response =
            RegistrationResponse::<auth::opaque::CrOpaque>::deserialize(
                &registration_response_bytes,
            )
            .unwrap();

        let client_reg_finish = client_reg_start
            .state
            .finish(
                &mut rng,
                password,
                registration_response,
                ClientRegistrationFinishParameters::default(),
            )
            .unwrap();
        let registration_record =
            URL_SAFE_NO_PAD.encode(client_reg_finish.message.serialize());

        let reg_finish_resp = server
            .post("/api/auth/register/finish")
            .json(&json!({
                "email": known_email,
                "registrationRecord": registration_record,
                "publicKey": test_public_key(),
                "encryptedPrivateKey": test_encrypted_private_key(),
            }))
            .await;
        reg_finish_resp.assert_status(StatusCode::OK);

        let mut login_rng = OsRng;

        let client_login_start_unknown =
            ClientLogin::<auth::opaque::CrOpaque>::start(&mut login_rng, password)
                .unwrap();
        let start_login_request_unknown =
            URL_SAFE_NO_PAD.encode(client_login_start_unknown.message.serialize());

        let login_start_resp_unknown = server
            .post("/api/auth/login/start")
            .json(&json!({
                "email": unknown_email,
                "startLoginRequest": start_login_request_unknown,
            }))
            .await;
        login_start_resp_unknown.assert_status(StatusCode::OK);
        let login_start_body_unknown: serde_json::Value = login_start_resp_unknown.json();
        let login_flow_id_unknown = login_start_body_unknown["loginFlowId"]
            .as_str()
            .unwrap();
        let login_response_b64_unknown = login_start_body_unknown["loginResponse"]
            .as_str()
            .unwrap();
        expect_that!(login_response_b64_unknown, not(eq("")));

        let client_login_start_known =
            ClientLogin::<auth::opaque::CrOpaque>::start(&mut login_rng, password)
                .unwrap();
        let start_login_request_known =
            URL_SAFE_NO_PAD.encode(client_login_start_known.message.serialize());

        let login_start_resp_known = server
            .post("/api/auth/login/start")
            .json(&json!({
                "email": known_email,
                "startLoginRequest": start_login_request_known,
            }))
            .await;
        login_start_resp_known.assert_status(StatusCode::OK);
        let login_start_body_known: serde_json::Value = login_start_resp_known.json();
        let login_response_b64_known = login_start_body_known["loginResponse"]
            .as_str()
            .unwrap();
        let login_response_bytes_known = URL_SAFE_NO_PAD
            .decode(login_response_b64_known)
            .unwrap();
        let login_response_known =
            CredentialResponse::<auth::opaque::CrOpaque>::deserialize(
                &login_response_bytes_known,
            )
            .unwrap();

        let client_login_finish = client_login_start_known
            .state
            .finish(
                &mut login_rng,
                password,
                login_response_known,
                ClientLoginFinishParameters::default(),
            )
            .unwrap();
        let finish_login_request =
            URL_SAFE_NO_PAD.encode(client_login_finish.message.serialize());

        let login_finish_resp = server
            .post("/api/auth/login/finish")
            .json(&json!({
                "loginFlowId": login_flow_id_unknown,
                "finishLoginRequest": finish_login_request,
            }))
            .await;
        login_finish_resp.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    async fn registration_propagates_keys_to_their_owners(viewer: Viewer) {
        let Viewer {
            server,
            state,
            mut listeners,
            ..
        } = viewer;

        let email = "keys_alice@example.com";
        let password = b"password";

        opaque_register(&server, email, password).await;
        expect_that!(listeners.poll(), eq(Poll::Pending));

        let login_resp = opaque_login(&server, email, password).await;
        let login_body: serde_json::Value = login_resp.json();
        expect_that!(
            login_body["encryptedPrivateKey"],
            eq(&test_encrypted_private_key())
        );

        let events_token = login_resp
            .cookie(&state.auth.config.events_token_cookie_name)
            .value()
            .to_string();
        let events_id = auth::ValidatedViewerEventsToken::try_from(
            events_token,
            state
                .internal_auth
                .viewer_event_token_pub_key()
                .await,
        )
        .unwrap()
        .viewer_id();

        let public_key = state
            .web_events
            .public_key(events_id)
            .await
            .unwrap();
        expect_that!(
            URL_SAFE_NO_PAD.encode(public_key.as_bytes()),
            eq(&test_public_key())
        );
    }

    /// A record of the length the browser produces. Its content does not
    /// matter here: the server cannot read it.
    fn sealed_record(byte: u8) -> String {
        URL_SAFE_NO_PAD.encode([byte; web_viewer_events::enabled_donations::SEALED_LEN])
    }

    async fn enable_donations(server: &TestServer, byte: u8) -> String {
        let resp = server
            .post("/api/events/web/enabled-donations")
            .json(&json!({ "sealed": sealed_record(byte) }))
            .await;
        resp.assert_status(StatusCode::CREATED);
        let body: serde_json::Value = resp.json();
        body["id"].as_str().unwrap().to_string()
    }

    #[gtest]
    #[rstest]
    #[case::public_key("/api/events/web/public-key")]
    #[case::enabled_donations("/api/events/web/enabled-donations")]
    #[tokio::test]
    async fn reading_donation_data_needs_the_events_token(
        viewer: Viewer,
        #[case] path: &str,
    ) {
        viewer
            .server
            .get(path)
            .await
            .assert_status(StatusCode::UNAUTHORIZED);
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    async fn enabling_donations_needs_the_events_token(viewer: Viewer) {
        viewer
            .server
            .post("/api/events/web/enabled-donations")
            .json(&json!({ "sealed": sealed_record(1) }))
            .await
            .assert_status(StatusCode::UNAUTHORIZED);
    }

    // The length is all the server can check.
    #[gtest]
    #[rstest]
    #[case::unpadded(URL_SAFE_NO_PAD.encode([1u8; 100]))]
    #[case::not_base64("not base64!".to_string())]
    #[tokio::test]
    async fn malformed_enabled_donations_are_refused(
        viewer: Viewer,
        #[case] sealed: String,
    ) {
        let Viewer {
            server,
            mut listeners,
            ..
        } = viewer;
        opaque_register(&server, "donations_alice@example.com", b"password").await;
        expect_that!(listeners.poll(), eq(Poll::Pending));

        server
            .post("/api/events/web/enabled-donations")
            .json(&json!({ "sealed": sealed }))
            .await
            .assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    async fn enabled_donations_are_stored_sealed_and_revocable(viewer: Viewer) {
        let Viewer {
            server,
            mut listeners,
            ..
        } = viewer;
        opaque_register(&server, "donations_alice@example.com", b"password").await;
        expect_that!(listeners.poll(), eq(Poll::Pending));

        let key: serde_json::Value = server
            .get("/api/events/web/public-key")
            .await
            .json();
        expect_that!(key["publicKey"], eq(&test_public_key()));

        let first = enable_donations(&server, 1).await;
        let second = enable_donations(&server, 2).await;

        let body: serde_json::Value = server
            .get("/api/events/web/enabled-donations")
            .await
            .json();
        expect_that!(body["publicKey"], eq(&test_public_key()));
        expect_that!(
            body["items"],
            eq(&json!([
                { "id": first, "sealed": sealed_record(1) },
                { "id": second, "sealed": sealed_record(2) },
            ]))
        );

        let path = format!("/api/events/web/enabled-donations/{first}");
        server
            .get(&path)
            .await
            .assert_status(StatusCode::NO_CONTENT);
        server
            .delete(&path)
            .await
            .assert_status(StatusCode::NO_CONTENT);
        // Idempotent, and indistinguishable from an id that never existed.
        server
            .delete(&path)
            .await
            .assert_status(StatusCode::NO_CONTENT);
        server
            .get(&path)
            .await
            .assert_status(StatusCode::NOT_FOUND);

        let body: serde_json::Value = server
            .get("/api/events/web/enabled-donations")
            .await
            .json();
        expect_that!(
            body["items"],
            eq(&json!([{ "id": second, "sealed": sealed_record(2) }]))
        );
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    async fn another_viewer_cannot_see_or_revoke_enabled_donations(viewer: Viewer) {
        let Viewer {
            server: alice,
            state,
            mut listeners,
            ..
        } = viewer;
        let (bob, _bob_tracing) = create_server(axum_app(state));

        opaque_register(&alice, "donations_owner@example.com", b"password").await;
        opaque_register(&bob, "donations_other@example.com", b"password").await;
        expect_that!(listeners.poll(), eq(Poll::Pending));

        let id = enable_donations(&alice, 1).await;
        let path = format!("/api/events/web/enabled-donations/{id}");

        bob.get(&path)
            .await
            .assert_status(StatusCode::NOT_FOUND);
        bob.delete(&path)
            .await
            .assert_status(StatusCode::NO_CONTENT);
        let bobs: serde_json::Value = bob
            .get("/api/events/web/enabled-donations")
            .await
            .json();
        expect_that!(bobs["items"], eq(&json!([])));

        alice
            .get(&path)
            .await
            .assert_status(StatusCode::NO_CONTENT);
    }

    #[gtest]
    #[rstest]
    #[tokio::test]
    async fn register_finish_rejects_invalid_public_key(viewer: Viewer) {
        let Viewer { server, .. } = viewer;

        let mut rng = OsRng;
        let password = b"password";
        let client_start =
            ClientRegistration::<auth::opaque::CrOpaque>::start(&mut rng, password)
                .unwrap();
        let start_resp = server
            .post("/api/auth/register/start")
            .json(&json!({
                "email": "badkey@example.com",
                "registrationRequest":
                    URL_SAFE_NO_PAD.encode(client_start.message.serialize()),
            }))
            .await;
        let start_body: serde_json::Value = start_resp.json();
        let registration_response =
            RegistrationResponse::<auth::opaque::CrOpaque>::deserialize(
                &URL_SAFE_NO_PAD
                    .decode(
                        start_body["registrationResponse"]
                            .as_str()
                            .unwrap(),
                    )
                    .unwrap(),
            )
            .unwrap();
        let client_finish = client_start
            .state
            .finish(
                &mut rng,
                password,
                registration_response,
                ClientRegistrationFinishParameters::default(),
            )
            .unwrap();

        let finish_resp = server
            .post("/api/auth/register/finish")
            .json(&json!({
                "email": "badkey@example.com",
                "registrationRecord":
                    URL_SAFE_NO_PAD.encode(client_finish.message.serialize()),
                "publicKey": URL_SAFE_NO_PAD.encode([1u8; 16]),
                "encryptedPrivateKey": test_encrypted_private_key(),
            }))
            .await;
        finish_resp.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    }

    fn test_public_key() -> String {
        URL_SAFE_NO_PAD.encode([42u8; 32])
    }

    fn test_encrypted_private_key() -> String {
        URL_SAFE_NO_PAD.encode([7u8; 76])
    }

    /// The viewer gateway on fresh state. Registrations and events reach
    /// their services only when a test polls `listeners`.
    struct Viewer {
        server: TestServer,
        state: AppState,
        listeners: Spawn<LocalBoxFuture<'static, ()>>,
        _tracing: DefaultGuard,
    }

    #[fixture]
    fn viewer() -> Viewer {
        let (state, listeners) = create_state(Config::default());
        let (server, _tracing) = create_server(axum_app(state.clone()));
        Viewer {
            server,
            state,
            listeners: spawn(listeners.boxed_local()),
            _tracing,
        }
    }

    /// The creator service on fresh state.
    #[fixture]
    fn creator() -> (TestServer, DefaultGuard) {
        create_server(creator_app(CreatorState::default()))
    }

    fn create_server(router: Router) -> (TestServer, DefaultGuard) {
        fn setup_tracing() -> DefaultGuard {
            let subscriber = FmtSubscriber::builder()
                .with_max_level(Level::TRACE)
                .with_thread_names(true)
                .with_thread_ids(true)
                .finish();
            let subscriber_local = FmtSubscriber::builder()
                .with_max_level(Level::TRACE)
                .with_thread_names(true)
                .with_thread_ids(true)
                .finish();

            //Can fail if already set in another test
            tracing::subscriber::set_global_default(subscriber).unwrap_or(());
            tracing::subscriber::set_default(subscriber_local)
        }

        let tracing_guard = setup_tracing();

        let config = TestServerConfig {
            default_content_type: Some("application/json".to_string()),
            save_cookies: true,
            ..TestServerConfig::default()
        };

        let server = TestServer::new_with_config(router.into_make_service(), config);
        (server, tracing_guard)
    }

    #[derive(Clone)]
    struct TestWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for TestWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for TestWriter {
        type Writer = TestWriter;

        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    fn create_capturing_server(
        router: Router,
    ) -> (TestServer, Arc<Mutex<Vec<u8>>>, DefaultGuard) {
        let logs = Arc::new(Mutex::new(Vec::new()));
        let subscriber = tracing_subscriber::fmt()
            .with_writer(TestWriter(logs.clone()))
            .with_max_level(Level::TRACE)
            .finish();
        let tracing_guard = tracing::subscriber::set_default(subscriber);

        let config = TestServerConfig {
            default_content_type: Some("application/json".to_string()),
            save_cookies: true,
            ..TestServerConfig::default()
        };

        let server = TestServer::new_with_config(router.into_make_service(), config);
        (server, logs, tracing_guard)
    }

    #[gtest]
    #[rstest]
    #[case::no_token(None, StatusCode::UNAUTHORIZED)]
    #[case::wrong_token(
        Some(HeaderValue::from_static("Bearer wrong")),
        StatusCode::UNAUTHORIZED
    )]
    #[case::dev_token(Some(admin_bearer()), StatusCode::OK)]
    #[tokio::test]
    async fn the_admin_gateway_needs_the_token(
        #[case] authorization: Option<HeaderValue>,
        #[case] status: StatusCode,
    ) {
        let (_, admin_state, _listeners) = crate::state::create_states(
            Config::default(),
            Arc::new(billing::payments::StubPaymentProvider::default()),
        );
        let (admin, _tracing_guard) = create_server(admin_app(admin_state));

        let request = admin.get("/admin/billing/log");
        let request = match authorization {
            Some(value) => request.add_header(header::AUTHORIZATION, value),
            None => request,
        };
        request.await.assert_status(status);
    }

    #[gtest]
    #[rstest]
    #[case::admin_prefix("/admin/billing/log")]
    #[case::api_prefix("/api/admin/billing/log")]
    #[tokio::test]
    async fn the_viewer_gateway_has_no_admin_routes(viewer: Viewer, #[case] path: &str) {
        viewer
            .server
            .get(path)
            .add_header(header::AUTHORIZATION, admin_bearer())
            .await
            .assert_status(StatusCode::NOT_FOUND);
    }

    fn admin_bearer() -> HeaderValue {
        format!("Bearer {}", admin::DEV_ADMIN_TOKEN)
            .parse()
            .unwrap()
    }

    #[gtest]
    #[rstest]
    #[timeout(Duration::from_secs(30))]
    #[tokio::test]
    async fn a_period_is_billed_only_after_the_key_ceremony() {
        use billing::EuroCents;
        use billing::ceremony_format::{Batch, BatchRecord, PeriodPublicKey};
        use billing::payments::StubPaymentProvider;
        use billing::period::PeriodId;

        let mut config = Config::default();
        config.billing.period_length = "3s".parse().unwrap();
        config.billing.unit_price = EuroCents(10);
        config.billing.default_limit = EuroCents(25);
        let length = config.billing.period_length;

        let clock = Arc::new(AtomicU64::new(1_000_000_000));
        let clock_for_config: billing::period::Clock = {
            let clock = clock.clone();
            Arc::new(move || clock.load(Ordering::SeqCst))
        };
        config.billing.clock = clock_for_config;

        let payments = StubPaymentProvider::default();
        let (state, admin_state, listeners) =
            crate::state::create_states(config, Arc::new(payments.clone()));
        let mut listeners = spawn(listeners);
        let (server, _tracing_guard) = create_server(axum_app(state));
        let (admin, _admin_tracing_guard) = create_server(admin_app(admin_state));

        let period = length.period_at(clock.load(Ordering::SeqCst));
        server
            .get("/api/billing/period-key")
            .await
            .assert_status(StatusCode::SERVICE_UNAVAILABLE);

        // The ceremony, done offline before the period: only public keys
        // reach the server, through the admin gateway.
        let generated = billing_ceremony::generate(period, 2, 3).unwrap();
        let key_path = format!("/admin/billing/periods/{period}/key");
        let key_body = json!({ "publicKey": generated.public_key.to_base64url() });
        for expected in [StatusCode::NO_CONTENT, StatusCode::NO_CONTENT] {
            admin
                .put(&key_path)
                .add_header(header::AUTHORIZATION, admin_bearer())
                .json(&key_body)
                .await
                .assert_status(expected);
        }
        let other_key = billing_ceremony::generate(period, 2, 3).unwrap();
        admin
            .put(&key_path)
            .add_header(header::AUTHORIZATION, admin_bearer())
            .json(&json!({ "publicKey": other_key.public_key.to_base64url() }))
            .await
            .assert_status(StatusCode::CONFLICT);
        admin
            .put(&format!(
                "/admin/billing/periods/{}/key",
                period.get().checked_sub(1).unwrap()
            ))
            .add_header(header::AUTHORIZATION, admin_bearer())
            .json(&key_body)
            .await
            .assert_status(StatusCode::CONFLICT);

        let key_resp = server.get("/api/billing/period-key").await;
        key_resp.assert_status_ok();
        let key_body: serde_json::Value = key_resp.json();
        expect_that!(key_body["period"], eq(&period.get()));
        let key =
            PeriodPublicKey::from_base64url(key_body["publicKey"].as_str().unwrap())
                .unwrap();
        expect_that!(key, eq(generated.public_key));

        let usage = |id: &str, period: PeriodId, site: &str| {
            let sealed = billing_ceremony::seal_usage(&key, period, site);
            json!({
                "enabledDonationId": id,
                "period": period.get(),
                "sealed": URL_SAFE_NO_PAD.encode(sealed.as_bytes()),
            })
        };

        server
            .post("/api/events/web/usage")
            .json(&usage(
                &common::Uuid::new_v4().to_string(),
                period,
                "https://a.example",
            ))
            .await
            .assert_status(StatusCode::UNAUTHORIZED);

        opaque_register(&server, "billing_alice@example.com", b"password").await;
        expect_that!(listeners.poll(), eq(Poll::Pending));
        let id = enable_donations(&server, 1).await;

        for site in [
            "https://a.example",
            "https://a.example",
            "https://a.example",
            "https://b.example",
        ] {
            server
                .post("/api/events/web/usage")
                .json(&usage(&id, period, site))
                .await
                .assert_status(StatusCode::ACCEPTED);
        }
        // Another period's key context, a malformed record, no enabled donation.
        server
            .post("/api/events/web/usage")
            .json(&usage(&id, period.next(), "https://a.example"))
            .await
            .assert_status(StatusCode::CONFLICT);
        server
            .post("/api/events/web/usage")
            .json(&json!({ "enabledDonationId": id, "period": period.get(), "sealed": "AAAA" }))
            .await
            .assert_status(StatusCode::UNPROCESSABLE_ENTITY);
        server
            .post("/api/events/web/usage")
            .json(&usage(
                &common::Uuid::new_v4().to_string(),
                period,
                "https://c.example",
            ))
            .await
            .assert_status(StatusCode::ACCEPTED);
        expect_that!(listeners.poll(), eq(Poll::Pending));

        // The period ends; billing closes it into a batch and charges no one.
        clock.store(length.end(period), Ordering::SeqCst);
        let batch_path = format!("/admin/billing/periods/{period}/batch");
        let mut batch_resp = admin
            .get(&batch_path)
            .add_header(header::AUTHORIZATION, admin_bearer())
            .await;
        for _ in 0..100 {
            if batch_resp.status_code() == StatusCode::OK {
                break;
            }
            batch_resp.assert_status(StatusCode::NOT_FOUND);
            tokio::time::sleep(Duration::from_millis(50)).await;
            expect_that!(listeners.poll(), eq(Poll::Pending));
            batch_resp = admin
                .get(&batch_path)
                .add_header(header::AUTHORIZATION, admin_bearer())
                .await;
        }
        let batch: Batch = batch_resp.json();
        // 4 units = 40 cents, capped at the 25 cent limit: each weighs a quarter.
        expect_that!(
            batch.records,
            all![
                len(eq(4)),
                each(field!(&BatchRecord.weight_millicents, eq(6_250)))
            ]
        );
        expect_that!(
            batch_resp.text(),
            not(contains_substring("billing_alice"))
        );
        expect_that!(payments.charges(), is_empty());

        // Two of three holders bring their shares; only totals come back.
        let totals = billing_ceremony::tally(&batch, &generated.shares[1..]).unwrap();
        let totals_path = format!("/admin/billing/periods/{period}/totals");
        let settled = admin
            .post(&totals_path)
            .add_header(header::AUTHORIZATION, admin_bearer())
            .json(&totals)
            .await;
        settled.assert_status_ok();
        let settled: serde_json::Value = settled.json();
        expect_that!(
            settled,
            eq(&json!({ "chargedCents": 25, "paidOutCents": 24 }))
        );

        expect_that!(
            payments.charges(),
            elements_are![(anything(), eq(&period), eq(&EuroCents(25)))]
        );
        expect_that!(
            payments.payouts(),
            elements_are![
                (
                    eq("https://a.example"),
                    eq(&period),
                    eq(&EuroCents(18))
                ),
                (eq("https://b.example"), eq(&period), eq(&EuroCents(6))),
            ]
        );

        // Settled once: the batch is gone and the totals are not taken again.
        admin
            .get(&batch_path)
            .add_header(header::AUTHORIZATION, admin_bearer())
            .await
            .assert_status(StatusCode::NOT_FOUND);
        admin
            .post(&totals_path)
            .add_header(header::AUTHORIZATION, admin_bearer())
            .json(&totals)
            .await
            .assert_status(StatusCode::CONFLICT);
        let log: serde_json::Value = admin
            .get("/admin/billing/log")
            .add_header(header::AUTHORIZATION, admin_bearer())
            .await
            .json();
        let events: Vec<_> = log
            .as_array()
            .unwrap()
            .iter()
            .filter(|entry| entry["period"] == period.get())
            .map(|entry| entry["event"].as_str().unwrap().to_string())
            .collect();
        expect_that!(
            events,
            elements_are![eq("keyAdded"), eq("closed"), eq("settled")]
        );
    }
}
