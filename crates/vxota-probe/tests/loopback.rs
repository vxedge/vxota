//! End-to-end loopback test: keygen -> publish -> serve -> probe fetch +
//! verify, over 127.0.0.1. Runs unprivileged, so it executes in plain
//! `cargo test` and CI (mirrors vxagents/vxprobe's tests/loopback.rs).

use std::net::TcpListener as StdTcpListener;
use std::time::Duration;
use vxota::{keygen, publish, serve};
use vxota_probe::fetch::{FetchConfig, fetch_and_verify};

async fn publish_and_serve() -> (tempfile::TempDir, std::net::SocketAddr, String) {
    let tmp = tempfile::tempdir().unwrap();
    let artifacts_dir = tmp.path().join("artifacts_root");
    let priv_key_path = tmp.path().join("root.key");

    keygen::run(keygen::KeygenArgs {
        artifacts_dir: artifacts_dir.clone(),
        out_priv: priv_key_path.clone(),
        root_expires_days: 1,
    })
    .expect("keygen succeeds");

    // Recover the trusted pubkey the way an operator would: derive it from
    // the private key (out-of-band distribution in real deployments).
    let priv_hex = std::fs::read_to_string(&priv_key_path).unwrap();
    let priv_bytes: [u8; 32] = hex::decode(priv_hex.trim()).unwrap().try_into().unwrap();
    let sk = ed25519_dalek::SigningKey::from_bytes(&priv_bytes);
    let trust_root_hex = hex::encode(sk.verifying_key().as_bytes());

    let artifact_path = tmp.path().join("firmware.bin");
    std::fs::write(&artifact_path, b"pretend firmware bytes, not actually tiny").unwrap();
    publish::run(publish::PublishArgs {
        artifacts_dir: artifacts_dir.clone(),
        file: artifact_path,
        name: "zone-fw".to_string(),
        signing_key: priv_key_path,
        expires_days: 1,
    })
    .expect("publish succeeds");

    // Grab an ephemeral port, release it, and serve there.
    let addr = {
        let l = StdTcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap()
    };
    tokio::spawn(async move {
        serve::run(serve::ServeArgs {
            listen: addr,
            artifacts_dir,
        })
        .await
        .expect("server runs");
    });
    tokio::time::sleep(Duration::from_millis(100)).await;

    (tmp, addr, trust_root_hex)
}

#[tokio::test(flavor = "multi_thread")]
async fn probe_fetches_and_verifies_over_loopback() {
    let (tmp, addr, trust_root_hex) = publish_and_serve().await;
    let artifact_path = {
        // Re-derive: publish_and_serve wrote it before publishing.
        let mut p = tmp.path().to_path_buf();
        p.push("firmware.bin");
        p
    };

    let out_path = tmp.path().join("downloaded.bin");
    let client = reqwest::Client::new();
    let cfg = FetchConfig {
        server: format!("http://{addr}"),
        target: "zone-fw".to_string(),
        trust_root_pubkey_hex: trust_root_hex,
        out: out_path.clone(),
        timeout: Duration::from_secs(5),
    };

    let outcome = fetch_and_verify(&client, &cfg)
        .await
        .expect("fetch+verify succeeds");
    let expected_bytes = std::fs::read(&artifact_path).unwrap();
    assert_eq!(outcome.bytes, expected_bytes.len() as u64);
    assert_eq!(std::fs::read(&out_path).unwrap(), expected_bytes);

    // Re-fetching into the same output path (already complete) is a no-op
    // digest-verified short-circuit, not a re-download.
    let outcome2 = fetch_and_verify(&client, &cfg)
        .await
        .expect("re-fetch succeeds");
    assert_eq!(outcome2.bytes, expected_bytes.len() as u64);
}

#[tokio::test(flavor = "multi_thread")]
async fn probe_rejects_wrong_trust_root() {
    let (tmp, addr, _correct_root_hex) = publish_and_serve().await;
    let wrong_sk = ed25519_dalek::SigningKey::from_bytes(&[7u8; 32]);
    let wrong_root_hex = hex::encode(wrong_sk.verifying_key().as_bytes());

    let client = reqwest::Client::new();
    let cfg = FetchConfig {
        server: format!("http://{addr}"),
        target: "zone-fw".to_string(),
        trust_root_pubkey_hex: wrong_root_hex,
        out: tmp.path().join("should-not-exist.bin"),
        timeout: Duration::from_secs(5),
    };

    let err = fetch_and_verify(&client, &cfg)
        .await
        .expect_err("a root.json signed by a different key must not verify");
    assert!(
        err.contains("signature") || err.contains("verification"),
        "expected a signature-verification failure, got: {err}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn probe_rejects_unknown_target() {
    let (tmp, addr, trust_root_hex) = publish_and_serve().await;
    let client = reqwest::Client::new();
    let cfg = FetchConfig {
        server: format!("http://{addr}"),
        target: "does-not-exist".to_string(),
        trust_root_pubkey_hex: trust_root_hex,
        out: tmp.path().join("should-not-exist.bin"),
        timeout: Duration::from_secs(5),
    };

    let err = fetch_and_verify(&client, &cfg)
        .await
        .expect_err("an unpublished target must not resolve");
    assert!(err.contains("not in targets.json"), "got: {err}");
}
