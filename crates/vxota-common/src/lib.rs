//! Shared plumbing for vxota: content-addressed artifact store,
//! Uptane-flavored signed metadata (root/targets, image-repo only — see
//! docs/ARCHITECTURE.md §8), and minimal Prometheus exposition helpers.

pub mod error;
pub mod exporter;
pub mod metadata;
pub mod prom;
pub mod store;

pub use error::{MetadataError, StoreError};
