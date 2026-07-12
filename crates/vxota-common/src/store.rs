//! Content-addressed artifact store: published bytes are written to
//! `<root>/artifacts/<sha256>`, addressed by their own digest (vxota/TASKS.md
//! phase 1). Writes are atomic (write to a per-process tmp file, then
//! rename) so a server reading concurrently never sees a partial artifact.

use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::StoreError;

pub struct ArtifactStore {
    root: PathBuf,
}

impl ArtifactStore {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, StoreError> {
        let root = root.into();
        std::fs::create_dir_all(root.join("artifacts"))?;
        Ok(Self { root })
    }

    pub fn artifacts_dir(&self) -> PathBuf {
        self.root.join("artifacts")
    }

    pub fn root_path(&self) -> &Path {
        &self.root
    }

    /// Content-address `bytes` into the store; returns (sha256_hex, length).
    /// Idempotent: re-publishing identical bytes is a no-op write.
    pub fn put(&self, bytes: &[u8]) -> Result<(String, u64), StoreError> {
        let digest = hex::encode(Sha256::digest(bytes));
        let dest = self.artifacts_dir().join(&digest);
        if !dest.exists() {
            let tmp = self
                .artifacts_dir()
                .join(format!("{digest}.tmp-{}", std::process::id()));
            {
                let mut f = std::fs::File::create(&tmp)?;
                f.write_all(bytes)?;
            }
            std::fs::rename(&tmp, &dest)?; // atomic within the same filesystem
        }
        Ok((digest, bytes.len() as u64))
    }

    pub fn path_for(&self, sha256_hex: &str) -> PathBuf {
        self.artifacts_dir().join(sha256_hex)
    }
}

/// SHA256 of a file's full contents, hex-encoded. v1 reads the whole file
/// into memory — fine at this testbed's artifact sizes; streaming hashing
/// is a follow-up if multi-GB images are ever exercised.
pub fn sha256_file(path: &Path) -> Result<String, StoreError> {
    let bytes = std::fs::read(path)?;
    Ok(hex::encode(Sha256::digest(&bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_is_content_addressed_and_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ArtifactStore::open(tmp.path()).unwrap();
        let (d1, len1) = store.put(b"hello ota").unwrap();
        let (d2, len2) = store.put(b"hello ota").unwrap();
        assert_eq!(d1, d2);
        assert_eq!(len1, len2);
        assert!(store.path_for(&d1).exists());
        assert_eq!(std::fs::read(store.path_for(&d1)).unwrap(), b"hello ota");
    }

    #[test]
    fn sha256_file_matches_put_digest() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ArtifactStore::open(tmp.path()).unwrap();
        let (digest, _) = store.put(b"verify me").unwrap();
        assert_eq!(sha256_file(&store.path_for(&digest)).unwrap(), digest);
    }
}
