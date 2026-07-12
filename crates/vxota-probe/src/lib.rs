//! vxota-probe core: the trust-chain fetch/verify logic and live/aggregate
//! state tracking. `main.rs` is a thin CLI wrapper over this.

pub mod fetch;
pub mod state;
