#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum MetadataError {
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("key encoding invalid")]
    BadKeyEncoding,
    #[error("no signature found for the trusted key")]
    NoMatchingSignature,
    #[error("signature is not valid hex/length")]
    BadSignatureEncoding,
    #[error("signature verification failed")]
    SignatureInvalid,
    #[error("key not found in root metadata: {0}")]
    KeyNotFound(String),
    #[error("metadata expired")]
    Expired,
    #[error("target not found: {0}")]
    TargetNotFound(String),
    #[error("digest mismatch: expected {expected}, got {got}")]
    DigestMismatch { expected: String, got: String },
    #[error("length mismatch: expected {expected}, got {got}")]
    LengthMismatch { expected: u64, got: u64 },
}
