//! The trust-chain fetch: root.json (verified against an out-of-band pinned
//! key) -> targets.json (verified against the key root.json vouches for) ->
//! resumable artifact download -> digest verification. Mirrors the client
//! bootstrap flow real Uptane/TUF devices use — never trusting a
//! server-supplied key for anything it wasn't already vouched for.

use futures_util::StreamExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use vxota_common::metadata::{self, RootMetadata, SignedEnvelope, TargetsMetadata};
use vxota_common::store::sha256_file;

#[derive(Clone)]
pub struct FetchConfig {
    pub server: String,
    pub target: String,
    pub trust_root_pubkey_hex: String,
    pub out: PathBuf,
    pub timeout: Duration,
}

#[derive(Debug)]
pub struct FetchOutcome {
    pub bytes: u64,
    pub duration: Duration,
}

pub async fn fetch_and_verify(
    client: &reqwest::Client,
    cfg: &FetchConfig,
) -> Result<FetchOutcome, String> {
    let start = Instant::now();
    fetch_and_verify_inner(client, cfg)
        .await
        .map(|bytes| FetchOutcome {
            bytes,
            duration: start.elapsed(),
        })
        .map_err(|e| e.to_string())
}

async fn fetch_and_verify_inner(
    client: &reqwest::Client,
    cfg: &FetchConfig,
) -> Result<u64, Box<dyn std::error::Error>> {
    let trusted_root_key = metadata::parse_public_key(&cfg.trust_root_pubkey_hex)?;
    let base = cfg.server.trim_end_matches('/');

    // 1. root.json, verified against the pinned out-of-band key.
    let root_bytes = client
        .get(format!("{base}/root.json"))
        .timeout(cfg.timeout)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    let root_env: SignedEnvelope<RootMetadata> = serde_json::from_slice(&root_bytes)?;
    metadata::verify(&root_env, &trusted_root_key, root_env.signed.expires_unix)?;

    // 2. targets.json, verified against the key the now-trusted root vouches
    //    for — never a key the server merely claims out-of-band.
    let targets_role = root_env
        .signed
        .roles
        .get("targets")
        .ok_or("root.json has no targets role")?;
    let targets_keyid = targets_role
        .keyids
        .first()
        .ok_or("targets role declares no keys")?;
    let targets_key_hex = &root_env
        .signed
        .keys
        .get(targets_keyid)
        .ok_or("targets role keyid not present in root.json's keys")?
        .public;
    let targets_key = metadata::parse_public_key(targets_key_hex)?;

    let targets_bytes = client
        .get(format!("{base}/targets.json"))
        .timeout(cfg.timeout)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    let targets_env: SignedEnvelope<TargetsMetadata> = serde_json::from_slice(&targets_bytes)?;
    metadata::verify(&targets_env, &targets_key, targets_env.signed.expires_unix)?;

    // 3. Resolve the requested target.
    let info = targets_env
        .signed
        .targets
        .get(&cfg.target)
        .ok_or_else(|| format!("target \"{}\" not in targets.json", cfg.target))?;

    // 4. Resumable download.
    let artifact_url = format!("{base}/artifacts/{}", info.sha256);
    download_resumable(client, &artifact_url, &cfg.out, info.length, cfg.timeout).await?;

    // 5. Verify digest — the step that actually makes step 4 trustworthy.
    let got = sha256_file(&cfg.out)?;
    if got != info.sha256 {
        return Err(format!("digest mismatch: expected {}, got {got}", info.sha256).into());
    }
    Ok(info.length)
}

async fn download_resumable(
    client: &reqwest::Client,
    url: &str,
    out: &Path,
    expected_len: u64,
    timeout: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    let existing = tokio::fs::metadata(out).await.map(|m| m.len()).unwrap_or(0);
    if existing >= expected_len {
        return Ok(()); // already complete; the digest check upstream catches any mismatch
    }

    let mut req = client.get(url).timeout(timeout);
    if existing > 0 {
        req = req.header(reqwest::header::RANGE, format!("bytes={existing}-"));
    }
    let resp = req.send().await?.error_for_status()?;
    // Only append if the server actually honored the Range request (206) —
    // a 200 in response to a Range ask means "full content", and appending
    // that to what we already have would corrupt the file.
    let resume = existing > 0 && resp.status() == reqwest::StatusCode::PARTIAL_CONTENT;

    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(resume)
        .truncate(!resume)
        .open(out)
        .await?;
    let mut stream = resp.bytes_stream();
    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = stream.next().await {
        file.write_all(&chunk?).await?;
    }
    // Without an explicit flush, the digest check that immediately follows
    // (a synchronous std::fs::read) can observe a truncated/empty file —
    // tokio::fs::File's writes aren't guaranteed visible to other readers
    // until flushed.
    file.flush().await?;
    Ok(())
}
