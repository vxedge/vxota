use clap::Args;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use std::collections::BTreeMap;
use std::path::PathBuf;
use vxota_common::metadata::{self, Role};

#[derive(Args)]
pub struct KeygenArgs {
    /// Where root.json gets written (the store's root directory, e.g. what
    /// `--artifacts-dir` points `vxota serve`/`vxota publish` at).
    #[arg(long)]
    pub artifacts_dir: PathBuf,
    /// Write the private signing key (hex-encoded 32-byte seed) here — keep
    /// this offline/secret; it signs both root.json and targets.json in v1's
    /// single-key model (see docs/ARCHITECTURE.md §8).
    #[arg(long)]
    pub out_priv: PathBuf,
    #[arg(long, default_value_t = 3650)]
    pub root_expires_days: u64,
}

pub fn run(args: KeygenArgs) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(&args.artifacts_dir)?;
    let sk = SigningKey::generate(&mut OsRng);
    std::fs::write(&args.out_priv, hex::encode(sk.to_bytes()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&args.out_priv, std::fs::Permissions::from_mode(0o600))?;
    }

    let keyid = metadata::keyid_for(&sk.verifying_key());
    let mut keys = BTreeMap::new();
    keys.insert(keyid.clone(), metadata::key_of(&sk.verifying_key()));
    let mut roles = BTreeMap::new();
    roles.insert(
        "root".to_string(),
        Role {
            keyids: vec![keyid.clone()],
            threshold: 1,
        },
    );
    roles.insert(
        "targets".to_string(),
        Role {
            keyids: vec![keyid.clone()],
            threshold: 1,
        },
    );

    let root = metadata::RootMetadata {
        version: 1,
        expires_unix: metadata::now_unix() + args.root_expires_days * 86400,
        keys,
        roles,
    };
    let env = metadata::sign(root, &sk)?;
    let root_path = args.artifacts_dir.join("root.json");
    std::fs::write(&root_path, serde_json::to_vec_pretty(&env)?)?;

    let pubkey_hex = hex::encode(sk.verifying_key().as_bytes());
    println!("root key generated (keyid {keyid})");
    println!(
        "private key written to {} — keep this offline/secret",
        args.out_priv.display()
    );
    println!("root.json written to {}", root_path.display());
    println!(
        "trusted root public key (give to clients out-of-band, e.g. vxota-probe --trust-root):"
    );
    println!("{pubkey_hex}");
    Ok(())
}
