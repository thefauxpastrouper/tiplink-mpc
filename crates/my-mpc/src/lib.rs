//! A framework for implementing MPC (Multiparty Computation) protocols in RUST

pub mod party;
pub mod message;
pub mod mpc;
pub mod delivery;
pub mod error;
pub mod protocol;
pub mod crypto;

// Re-export the core types
pub use party::{PartyId, PartyCount};
pub use message::Message;
pub use mpc::Mpc;
pub use delivery::Delivery;
pub use error::{Error, Result};