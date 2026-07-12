//! Uptane-flavored signed metadata: vocabulary/format only, not a dependency
//! on aktualizr or full TUF (docs/ARCHITECTURE.md §8). v1 models the
//! image-repo half of Uptane — signed root + targets metadata with
//! out-of-band root-key trust bootstrap; director-repo / campaign semantics
//! are phase-3 scope (vxota/TASKS.md).
//!
//! Trust chain: the client pins the root public key out-of-band (how a
//! device is provisioned in real Uptane), verifies `root.json` against it,
//! reads the `targets` role's key(s) *from the now-verified root*, and uses
//! those to verify `targets.json` — never trusting a server-supplied key for
//! anything it wasn't already vouched for by a previously-verified document.
//!
//! Canonicalization note: signing re-serializes `signed` with `serde_json`
//! (struct fields in declaration order, `BTreeMap`s in key order) — that's
//! deterministic within this codebase's own serde usage, not full RFC 8785
//! JSON Canonicalization; sufficient because only vxota itself ever produces
//! the bytes that get signed.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::MetadataError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedEnvelope<T> {
    pub signed: T,
    pub signatures: Vec<SignatureEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureEntry {
    pub keyid: String,
    /// hex-encoded ed25519 signature (64 bytes).
    pub sig: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Key {
    pub keytype: String, // "ed25519"
    /// hex-encoded 32-byte public key.
    pub public: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub keyids: Vec<String>,
    pub threshold: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootMetadata {
    pub version: u64,
    pub expires_unix: u64,
    /// keyid -> key
    pub keys: BTreeMap<String, Key>,
    /// role name ("root" | "targets") -> role
    pub roles: BTreeMap<String, Role>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetInfo {
    pub length: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TargetsMetadata {
    pub version: u64,
    pub expires_unix: u64,
    /// target path (as served under `/artifacts/<sha256>`, keyed here by a
    /// human-meaningful name like "zone-fw-2026.1.bin") -> info.
    pub targets: BTreeMap<String, TargetInfo>,
}

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_secs()
}

/// keyid = hex(sha256(raw 32-byte public key)) — deterministic, no external
/// canonical-JSON key-object hashing needed for a single-key-type v1.
pub fn keyid_for(vk: &VerifyingKey) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(vk.as_bytes()))
}

pub fn key_of(vk: &VerifyingKey) -> Key {
    Key {
        keytype: "ed25519".to_string(),
        public: hex::encode(vk.as_bytes()),
    }
}

pub fn parse_public_key(hex_str: &str) -> Result<VerifyingKey, MetadataError> {
    let bytes = hex::decode(hex_str).map_err(|_| MetadataError::BadKeyEncoding)?;
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| MetadataError::BadKeyEncoding)?;
    VerifyingKey::from_bytes(&arr).map_err(|_| MetadataError::BadKeyEncoding)
}

/// Sign `signed` with `signing_key`, producing a single-signature envelope.
pub fn sign<T: Serialize>(
    signed: T,
    signing_key: &SigningKey,
) -> Result<SignedEnvelope<T>, MetadataError> {
    let canonical = serde_json::to_vec(&signed)?;
    let sig: Signature = signing_key.sign(&canonical);
    let keyid = keyid_for(&signing_key.verifying_key());
    Ok(SignedEnvelope {
        signed,
        signatures: vec![SignatureEntry {
            keyid,
            sig: hex::encode(sig.to_bytes()),
        }],
    })
}

/// Verify `env` carries a valid signature from `trusted`, and that `signed`
/// (re-serialized the same way `sign` did) hasn't expired.
pub fn verify<T: Serialize>(
    env: &SignedEnvelope<T>,
    trusted: &VerifyingKey,
    expires_unix: u64,
) -> Result<(), MetadataError> {
    if now_unix() > expires_unix {
        return Err(MetadataError::Expired);
    }
    let canonical = serde_json::to_vec(&env.signed)?;
    let want_keyid = keyid_for(trusted);
    let entry = env
        .signatures
        .iter()
        .find(|s| s.keyid == want_keyid)
        .ok_or(MetadataError::NoMatchingSignature)?;
    let sig_bytes = hex::decode(&entry.sig).map_err(|_| MetadataError::BadSignatureEncoding)?;
    let sig_arr: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| MetadataError::BadSignatureEncoding)?;
    let sig = Signature::from_bytes(&sig_arr);
    trusted
        .verify(&canonical, &sig)
        .map_err(|_| MetadataError::SignatureInvalid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;

    #[test]
    fn sign_then_verify_roundtrips() {
        let sk = SigningKey::generate(&mut OsRng);
        let targets = TargetsMetadata {
            version: 1,
            expires_unix: now_unix() + 3600,
            targets: BTreeMap::new(),
        };
        let env = sign(targets, &sk).unwrap();
        verify(&env, &sk.verifying_key(), env.signed.expires_unix).unwrap();
    }

    #[test]
    fn verify_rejects_wrong_key() {
        let sk = SigningKey::generate(&mut OsRng);
        let other = SigningKey::generate(&mut OsRng);
        let targets = TargetsMetadata {
            version: 1,
            expires_unix: now_unix() + 3600,
            targets: BTreeMap::new(),
        };
        let env = sign(targets, &sk).unwrap();
        let err = verify(&env, &other.verifying_key(), env.signed.expires_unix).unwrap_err();
        assert!(matches!(err, MetadataError::NoMatchingSignature));
    }

    #[test]
    fn verify_rejects_expired() {
        let sk = SigningKey::generate(&mut OsRng);
        let targets = TargetsMetadata {
            version: 1,
            expires_unix: now_unix().saturating_sub(1),
            targets: BTreeMap::new(),
        };
        let env = sign(targets, &sk).unwrap();
        let err = verify(&env, &sk.verifying_key(), env.signed.expires_unix).unwrap_err();
        assert!(matches!(err, MetadataError::Expired));
    }

    #[test]
    fn verify_rejects_tampered_payload() {
        let sk = SigningKey::generate(&mut OsRng);
        let targets = TargetsMetadata {
            version: 1,
            expires_unix: now_unix() + 3600,
            targets: BTreeMap::new(),
        };
        let mut env = sign(targets, &sk).unwrap();
        env.signed.version = 2; // tamper after signing
        let err = verify(&env, &sk.verifying_key(), env.signed.expires_unix).unwrap_err();
        assert!(matches!(err, MetadataError::SignatureInvalid));
    }
}
