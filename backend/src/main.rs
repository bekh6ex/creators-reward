use std::env;
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::process::ExitCode;
use std::str::FromStr;
use std::sync::Arc;

use billing::payments::StubPaymentProvider;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

use crate::http::{TlsMode, start_http_server};
use crate::state::{Config, CreatorState, create_states};

mod http;
mod queue;
mod state;

struct ServerConfig {
    bind_addr: IpAddr,
    port: u16,
    /// The admin gateway always binds to loopback.
    admin_port: u16,
    /// The creator service, on `bind_addr` like the viewer gateway.
    creator_api_port: u16,
    tls_mode: TlsMode,
}

impl ServerConfig {
    fn from_env() -> Result<Self, String> {
        let bind_addr = env::var("CR_BIND_ADDR")
            .ok()
            .map(|s| IpAddr::from_str(&s))
            .transpose()
            .map_err(|e| format!("CR_BIND_ADDR: {e}"))?
            .unwrap_or(IpAddr::from([127, 0, 0, 1]));

        let port = env::var("CR_BACKEND_PORT")
            .ok()
            .map(|s| u16::from_str(&s))
            .transpose()
            .map_err(|e| format!("CR_BACKEND_PORT: {e}"))?
            .unwrap_or(3000);

        let admin_port = env::var("CR_ADMIN_PORT")
            .ok()
            .map(|s| u16::from_str(&s))
            .transpose()
            .map_err(|e| format!("CR_ADMIN_PORT: {e}"))?
            .or_else(|| port.checked_add(1))
            .ok_or("CR_ADMIN_PORT: set it, CR_BACKEND_PORT + 1 overflows")?;

        let creator_api_port = env::var("CR_CREATOR_API_PORT")
            .ok()
            .map(|s| u16::from_str(&s))
            .transpose()
            .map_err(|e| format!("CR_CREATOR_API_PORT: {e}"))?
            .or_else(|| port.checked_add(2))
            .ok_or("CR_CREATOR_API_PORT: set it, CR_BACKEND_PORT + 2 overflows")?;

        let cert = env::var("CR_TLS_CERT").ok();
        let key = env::var("CR_TLS_KEY").ok();

        let tls_mode = match (cert, key) {
            (Some(cert_path), Some(key_path)) => TlsMode::ExplicitTls {
                cert_path,
                key_path,
            },
            (None, None) => {
                let addr_explicit = env::var_os("CR_BIND_ADDR").is_some();
                let port_explicit = env::var_os("CR_BACKEND_PORT").is_some();
                if addr_explicit || port_explicit {
                    TlsMode::PlainHttp
                } else {
                    TlsMode::DevTls
                }
            }
            _ => {
                return Err(
                    "Both CR_TLS_CERT and CR_TLS_KEY must be set, or neither."
                        .to_string(),
                );
            }
        };

        Ok(ServerConfig {
            bind_addr,
            port,
            admin_port,
            creator_api_port,
            tls_mode,
        })
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(e) = setup_tracing() {
        eprintln!("Error: {e}");
        return ExitCode::FAILURE;
    }

    let server_config = match ServerConfig::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {e}");
            return ExitCode::FAILURE;
        }
    };

    let web_viewer_events_addr =
        SocketAddr::from((server_config.bind_addr, server_config.port));
    let mut app_config = Config::default();
    if let Err(e) = billing_config_from_env(&mut app_config.billing) {
        eprintln!("Error: {e}");
        return ExitCode::FAILURE;
    }
    let (state, admin_state, listeners) = create_states(
        app_config.clone(),
        Arc::new(StubPaymentProvider::default()),
    );
    let router = http::axum_app(state);
    let prefix_path = format!(
        "{}{}/web",
        app_config.api_prefix(),
        app_config.viewer_events_path_part()
    );
    let server_fut = start_http_server(
        web_viewer_events_addr,
        &prefix_path,
        router,
        server_config.tls_mode.clone(),
    );
    let admin_addr = SocketAddr::from(([127, 0, 0, 1], server_config.admin_port));
    let admin_fut = start_http_server(
        admin_addr,
        "/admin",
        http::admin_app(admin_state),
        server_config.tls_mode.clone(),
    );
    // Shares nothing with the services above: stands in for a separately
    // deployed creator service.
    let creator_addr =
        SocketAddr::from((server_config.bind_addr, server_config.creator_api_port));
    let creator_fut = start_http_server(
        creator_addr,
        "/api",
        http::creator_app(CreatorState::default()),
        server_config.tls_mode,
    );

    let b = Box::pin(async {
        if let Err(e) = server_fut.await {
            tracing::error!("HTTP server failed: {e}");
            std::process::exit(1);
        }
    });
    let unordered: FuturesUnordered<Pin<Box<dyn Future<Output = ()>>>> =
        FuturesUnordered::new();
    unordered.push(b);
    unordered.push(Box::pin(async {
        if let Err(e) = admin_fut.await {
            tracing::error!("admin HTTP server failed: {e}");
            std::process::exit(1);
        }
    }));
    unordered.push(Box::pin(async {
        if let Err(e) = creator_fut.await {
            tracing::error!("creator HTTP server failed: {e}");
            std::process::exit(1);
        }
    }));
    unordered.push(Box::pin(listeners));
    unordered.for_each(|_| async {}).await;
    ExitCode::SUCCESS
}

/// `CR_BILLING_PERIOD` (`month` or e.g. `120s`), `CR_BILLING_UNIT_PRICE_CENTS`,
/// `CR_BILLING_DEFAULT_LIMIT_CENTS`.
fn billing_config_from_env(config: &mut billing::BillingConfig) -> Result<(), String> {
    if let Ok(period) = env::var("CR_BILLING_PERIOD") {
        config.period_length = period
            .parse()
            .map_err(|e| format!("CR_BILLING_PERIOD: {e}"))?;
    }
    for (name, target) in [
        ("CR_BILLING_UNIT_PRICE_CENTS", &mut config.unit_price),
        (
            "CR_BILLING_DEFAULT_LIMIT_CENTS",
            &mut config.default_limit,
        ),
    ] {
        if let Ok(value) = env::var(name) {
            target.0 = value
                .parse()
                .map_err(|e| format!("{name}: {e}"))?;
        }
    }
    Ok(())
}

fn setup_tracing() -> Result<(), String> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|_| "setting default tracing subscriber failed".to_string())
}
