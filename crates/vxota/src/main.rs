//! vxota — vxedge OTA server (thin CLI): key generation, signed-artifact
//! publishing, and HTTP delivery with resumable downloads.
//!
//! `vxota keygen --artifacts-dir <dir> --out-priv <key>`             # root key + root.json
//! `vxota publish --artifacts-dir <dir> --file <f> --name <n> \
//!    --signing-key <key>`                                           # sign + publish a target
//! `vxota serve --artifacts-dir <dir> --listen 0.0.0.0:9900`         # HTTP delivery + /metrics

use clap::{Parser, Subcommand};
use vxota::{keygen, publish, serve};

#[derive(Parser)]
#[command(
    name = "vxota",
    version,
    about = "vxedge OTA server: signed-artifact publishing + delivery"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Generate the root signing key + a self-signed root.json.
    Keygen(keygen::KeygenArgs),
    /// Content-address + sign an artifact into targets.json.
    Publish(publish::PublishArgs),
    /// Serve root.json/targets.json/artifacts over HTTP (resumable downloads).
    Serve(serve::ServeArgs),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    match Cli::parse().cmd {
        Cmd::Keygen(args) => keygen::run(args),
        Cmd::Publish(args) => publish::run(args),
        Cmd::Serve(args) => serve::run(args).await,
    }
}
