//! MPC Engine — wraps 2-party Ed25519 key generation and signing
//!
//! Uses the `my-mpc` crate's PartyId for party identification and
//! the `two_party` module for actual cryptographic operations.

pub mod two_party;

use std::sync::Arc;
use my_mpc::PartyId;
use thiserror::Error;

pub use two_party::KeyShares;

#[derive(Error, Debug)]
pub enum MpcError {
    #[error("Failed to initialize MPC context")]
    InitError,
    #[error("Signing failed: {0}")]
    SigningFailed(String),
    #[error("Invalid share length: expected 32 bytes, got {0}")]
    InvalidShareLength(usize),
}

pub struct MpcEngine {
    /// This server's party identifier in the MPC protocol
    party_id: PartyId,
}

impl MpcEngine {
    pub fn new() -> Self {
        // Server is always Party 1 in the 2-party protocol
        Self {
            party_id: PartyId(1),
        }
    }

    pub fn two_party_keygen(&self, client_pubkey_bytes: &[u8; 32]) -> Result<two_party::KeyShares, MpcError> {
        two_party::server_keygen(client_pubkey_bytes).map_err(|e| MpcError::SigningFailed(e.to_string()))
    }

    pub fn two_party_generate_nonce(&self) -> two_party::ServerNonceContext {
        two_party::server_generate_nonce()
    }

    pub fn two_party_compute_partial_signature(
        &self,
        server_share: &[u8; 32],
        k_server_bytes: &[u8; 32],
        r_client_bytes: &[u8; 32],
        combined_pubkey_bytes: &[u8; 32],
        message: &[u8],
    ) -> Result<[u8; 32], MpcError> {
        two_party::server_compute_partial_signature(
            server_share,
            k_server_bytes,
            r_client_bytes,
            combined_pubkey_bytes,
            message,
        )
        .map_err(|e| MpcError::SigningFailed(e.to_string()))
    }
}

/// Type alias for shared state across Actix handlers
pub type SharedMpcEngine = Arc<MpcEngine>;
