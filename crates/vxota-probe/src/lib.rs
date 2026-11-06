//! vxota-probe core: the trust-chain fetch/verify logic and live/aggregate
//! state tracking. `main.rs` is a thin CLI wrapper over this.

pub mod fetch;
pub mod state;

/// The HTTP client the probe fetches with. reqwest 0.13 leaves the TLS crypto provider to the
/// program; this installs rustls' `ring` provider, the one reqwest 0.12 used, instead of the
/// aws-lc default.
pub fn http_client() -> reqwest::Client {
    // An error only means a provider is already installed in this process, which is fine.
    let _ = rustls::crypto::ring::default_provider().install_default();
    reqwest::Client::new()
}
