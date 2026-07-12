use clap::Args;
use ed25519_dalek::SigningKey;
use std::path::{Path, PathBuf};
use vxota_common::metadata;
use vxota_common::store::ArtifactStore;

#[derive(Args)]
pub struct PublishArgs {
    #[arg(long)]
    pub artifacts_dir: PathBuf,
    /// File to publish.
    #[arg(long)]
    pub file: PathBuf,
    /// Name the target is referenced under in targets.json (what clients
    /// pass to `vxota-probe fetch --target`).
    #[arg(long)]
    pub name: String,
    /// hex-encoded 32-byte ed25519 private key seed (from `vxota keygen`).
    #[arg(long)]
    pub signing_key: PathBuf,
    #[arg(long, default_value_t = 30)]
    pub expires_days: u64,
}

fn load_signing_key(path: &Path) -> Result<SigningKey, Box<dyn std::error::Error>> {
    let hex_str = std::fs::read_to_string(path)?;
    let bytes = hex::decode(hex_str.trim())?;
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| "signing key file must contain a 32-byte hex seed")?;
    Ok(SigningKey::from_bytes(&arr))
}

pub fn run(args: PublishArgs) -> Result<(), Box<dyn std::error::Error>> {
    let store = ArtifactStore::open(&args.artifacts_dir)?;
    let bytes = std::fs::read(&args.file)?;
    let (sha256, length) = store.put(&bytes)?;

    let sk = load_signing_key(&args.signing_key)?;

    let targets_path = args.artifacts_dir.join("targets.json");
    let mut targets = if targets_path.exists() {
        let raw = std::fs::read(&targets_path)?;
        let env: metadata::SignedEnvelope<metadata::TargetsMetadata> =
            serde_json::from_slice(&raw)?;
        env.signed
    } else {
        metadata::TargetsMetadata::default()
    };
    targets.version += 1;
    targets.expires_unix = metadata::now_unix() + args.expires_days * 86400;
    targets.targets.insert(
        args.name.clone(),
        metadata::TargetInfo {
            length,
            sha256: sha256.clone(),
        },
    );

    let env = metadata::sign(targets, &sk)?;
    std::fs::write(&targets_path, serde_json::to_vec_pretty(&env)?)?;

    println!(
        "published {} as target \"{}\" (sha256 {sha256}, {length} bytes); targets.json updated",
        args.file.display(),
        args.name
    );
    Ok(())
}
