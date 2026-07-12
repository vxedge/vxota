use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use clap::Args;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tower_http::services::ServeDir;

/// OTA transport rides the manifest's `ota` slice over WireGuard
/// (docs/ARCHITECTURE.md §7) — plain HTTP here is correct, not an
/// oversight: adding TLS on top would duplicate transport encryption
/// already provided by the tunnel.
pub struct ServeState {
    pub requests_total: AtomicU64,
}

#[derive(Args)]
pub struct ServeArgs {
    #[arg(long, default_value = "0.0.0.0:9900")]
    pub listen: SocketAddr,
    /// Root directory written by `vxota keygen`/`vxota publish`: expects
    /// root.json, targets.json, and artifacts/<sha256> under it.
    #[arg(long)]
    pub artifacts_dir: PathBuf,
}

async fn count_requests(
    State(state): State<Arc<ServeState>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    state.requests_total.fetch_add(1, Ordering::Relaxed);
    next.run(req).await
}

async fn metrics(State(state): State<Arc<ServeState>>) -> impl IntoResponse {
    let mut out = String::new();
    vxota_common::prom::line(
        &mut out,
        "vxota_server_requests_total",
        &[],
        state.requests_total.load(Ordering::Relaxed) as f64,
    );
    (StatusCode::OK, out)
}

pub async fn run(args: ServeArgs) -> Result<(), Box<dyn std::error::Error>> {
    if !args.artifacts_dir.join("root.json").exists() {
        return Err(format!(
            "no root.json in {} — run `vxota keygen` first",
            args.artifacts_dir.display()
        )
        .into());
    }
    let state = Arc::new(ServeState {
        requests_total: AtomicU64::new(0),
    });

    // root.json, targets.json, and artifacts/<sha256> already live at the
    // URL paths clients expect, so the whole artifacts_dir is served
    // directly — ServeDir handles Range/If-Range for resumable downloads.
    let files = ServeDir::new(&args.artifacts_dir);

    let app = Router::new()
        .route("/metrics", get(metrics))
        .with_state(state.clone())
        .layer(middleware::from_fn_with_state(state, count_requests))
        .fallback_service(files);

    let listener = tokio::net::TcpListener::bind(args.listen).await?;
    tracing::info!(listen = %args.listen, artifacts_dir = %args.artifacts_dir.display(), "vxota server listening");
    axum::serve(listener, app).await?;
    Ok(())
}
