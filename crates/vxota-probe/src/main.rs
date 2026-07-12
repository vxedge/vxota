//! vxota-probe — vxota client probe (thin CLI): fetches + verifies signed
//! artifacts and reports success/failure + delivery latency, the same way
//! vxprobe reports RTT — the H4 (OTA co-location) experiment driver.
//!
//! `vxota-probe fetch --server http://10.101.0.20:9900 --target fw.bin \
//!    --trust-root <hex-pubkey> --out /tmp/fw.bin`                            # one-shot
//! `vxota-probe fetch ... --repeat 30 --interval-s 10 --metrics 0.0.0.0:9901`  # H4 driver

use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use vxota_probe::fetch::{self, FetchConfig};
use vxota_probe::state::OtaState;

#[derive(Parser)]
#[command(
    name = "vxota-probe",
    version,
    about = "vxedge OTA client probe (fetch + verify signed artifacts)"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Fetch and verify a signed artifact, once or repeatedly (H4 driver mode).
    Fetch {
        /// Base URL of the vxota server, e.g. http://10.101.0.20:9900
        #[arg(long)]
        server: String,
        /// Target name as published (`vxota publish --name`).
        #[arg(long)]
        target: String,
        /// hex-encoded 32-byte ed25519 root public key, pinned out-of-band
        /// (printed by `vxota keygen`).
        #[arg(long)]
        trust_root: String,
        #[arg(long)]
        out: PathBuf,
        /// Number of fetch attempts; >1 drives a sustained H4 experiment window.
        #[arg(long, default_value_t = 1)]
        repeat: u64,
        #[arg(long, default_value_t = 10)]
        interval_s: u64,
        #[arg(long, default_value_t = 30)]
        timeout_s: u64,
        /// Serve live Prometheus metrics on this address during the run.
        #[arg(long)]
        metrics: Option<SocketAddr>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr) // stdout is the machine-readable JSON contract
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let Cmd::Fetch {
        server,
        target,
        trust_root,
        out,
        repeat,
        interval_s,
        timeout_s,
        metrics,
    } = Cli::parse().cmd;

    let state = OtaState::new();
    if let Some(addr) = metrics {
        let s = state.clone();
        tokio::spawn(vxota_common::exporter::serve(
            addr,
            Arc::new(move || s.render()),
        ));
    }

    let client = reqwest::Client::new();
    let cfg = FetchConfig {
        server,
        target,
        trust_root_pubkey_hex: trust_root,
        out,
        timeout: Duration::from_secs(timeout_s),
    };

    for i in 0..repeat.max(1) {
        if i > 0 {
            tokio::time::sleep(Duration::from_secs(interval_s)).await;
            // Each repeat iteration is meant to be an independent delivery
            // (H4 driver mode: simulate a device checking for/downloading an
            // update repeatedly over an experiment window) — without this,
            // download_resumable's "already complete" short-circuit would
            // make every iteration after the first a no-op digest check
            // instead of a real transfer, silently understating the OTA
            // workload's actual network/CPU footprint.
            let _ = std::fs::remove_file(&cfg.out);
        }
        match fetch::fetch_and_verify(&client, &cfg).await {
            Ok(outcome) => {
                tracing::info!(
                    attempt = i + 1,
                    bytes = outcome.bytes,
                    duration_ms = outcome.duration.as_millis() as u64,
                    "fetch ok"
                );
                state.record_success(outcome.duration);
            }
            Err(e) => {
                tracing::warn!(attempt = i + 1, error = %e, "fetch failed");
                state.record_failure();
            }
        }
    }

    let summary = state.summary();
    println!("{}", serde_json::to_string_pretty(&summary)?);
    if summary.succeeded == 0 {
        // No attempt ever succeeded — a failed measurement, mirroring
        // vxprobe's zero-received convention.
        std::process::exit(1);
    }
    Ok(())
}
