//! vxota core: key generation, signed-artifact publishing, and the HTTP
//! delivery server. `main.rs` is a thin CLI wrapper over this.

pub mod keygen;
pub mod publish;
pub mod serve;
