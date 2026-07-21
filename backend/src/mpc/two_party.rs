//! 2-Party Ed25519 Key Generation and Signing Protocol
//!
//! Implements additive secret sharing on the Ed25519 curve:
//! - Key Generation: Two parties each generate a random scalar share.
//!   The combined public key is (x1 + x2) * G.
//! - Signing: Each party generates a nonce share, computes a partial
//!   signature, and the partials are combined into a valid Ed25519 signature.
//!
//! The resulting signatures verify correctly with standard Ed25519 verification
//! because: s*B = (k1+k2)*B + H(R||A||M)*(x1+x2)*B = R + e*A

use curve25519_dalek::constants::ED25519_BASEPOINT_TABLE;
use curve25519_dalek::edwards::CompressedEdwardsY;
use curve25519_dalek::scalar::Scalar;
use rand::rngs::OsRng;
use sha2::{Digest, Sha512};

/// Result of 2-party distributed key generation
#[derive(Debug, Clone)]
pub struct KeyShares {
    /// Server's secret scalar share (32 bytes)
    pub server_share: [u8; 32],
    /// Client's secret scalar share (32 bytes)
    pub client_share: [u8; 32],
    /// Combined Ed25519 public key = (x_server + x_client) * G
    pub combined_pubkey: [u8; 32],
}

/// Interactive 2-Party Ed25519 Distributed Key Generation (Server Side)
pub fn server_keygen(client_pubkey_bytes: &[u8; 32]) -> Result<KeyShares, &'static str> {
    let x_server = Scalar::random(&mut OsRng);
    let pub_server = &x_server * ED25519_BASEPOINT_TABLE;
    
    let client_pub_compressed = CompressedEdwardsY::from_slice(client_pubkey_bytes)
        .map_err(|_| "Invalid client pubkey length")?;
    let pub_client = client_pub_compressed.decompress()
        .ok_or("Invalid client pubkey point")?;
        
    let combined = pub_server + pub_client;

    tracing::info!(
        "Interactive 2-Party DKG complete. Combined pubkey: {}",
        bs58::encode(combined.compress().as_bytes()).into_string()
    );

    Ok(KeyShares {
        server_share: x_server.to_bytes(),
        client_share: [0u8; 32], // Client share is kept by client
        combined_pubkey: combined.compress().to_bytes(),
    })
}

pub struct ServerNonceContext {
    pub k_server: [u8; 32],
    pub r_server: [u8; 32],
}

/// Server generates nonce for signing round
pub fn server_generate_nonce() -> ServerNonceContext {
    let k_server = Scalar::random(&mut OsRng);
    let r_server = &k_server * ED25519_BASEPOINT_TABLE;
    ServerNonceContext {
        k_server: k_server.to_bytes(),
        r_server: r_server.compress().to_bytes(),
    }
}

/// Server computes its partial signature
pub fn server_compute_partial_signature(
    x_server_bytes: &[u8; 32],
    k_server_bytes: &[u8; 32],
    r_client_bytes: &[u8; 32],
    combined_pubkey_bytes: &[u8; 32],
    message: &[u8],
) -> Result<[u8; 32], &'static str> {
    let x_server = Scalar::from_bytes_mod_order(*x_server_bytes);
    let k_server = Scalar::from_bytes_mod_order(*k_server_bytes);

    let r_server = &k_server * ED25519_BASEPOINT_TABLE;
    
    let r_client_compressed = CompressedEdwardsY::from_slice(r_client_bytes)
        .map_err(|_| "Invalid client R length")?;
    let r_client = r_client_compressed.decompress()
        .ok_or("Invalid client R point")?;

    let combined_r = r_server + r_client;
    let combined_r_bytes = combined_r.compress().to_bytes();

    let mut hasher = Sha512::new();
    hasher.update(combined_r_bytes);
    hasher.update(combined_pubkey_bytes);
    hasher.update(message);
    let hash_output = hasher.finalize();
    
    let e = Scalar::from_bytes_mod_order_wide(
        hash_output.as_slice().try_into().expect("SHA-512 produces 64 bytes"),
    );

    let s_server = k_server + e * x_server;
    Ok(s_server.to_bytes())
}

/// Verify a signature produced by the 2-party protocol
/// Uses standard Ed25519 verification
pub fn verify_signature(pubkey: &[u8; 32], message: &[u8], signature: &[u8; 64]) -> bool {
    use ed25519_dalek::{Signature, VerifyingKey};

    let verifying_key = match VerifyingKey::from_bytes(pubkey) {
        Ok(k) => k,
        Err(_) => return false,
    };

    let sig = Signature::from_bytes(signature);

    use ed25519_dalek::Verifier;
    verifying_key.verify(message, &sig).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keygen_and_sign_interactive() {
        // Client side DKG
        let x_client = Scalar::random(&mut OsRng);
        let pub_client = &x_client * ED25519_BASEPOINT_TABLE;
        
        // Server side DKG
        let shares = server_keygen(&pub_client.compress().to_bytes()).unwrap();
        
        let message = b"hello solana";

        // Round 1: Client nonce
        let k_client = Scalar::random(&mut OsRng);
        let r_client = &k_client * ED25519_BASEPOINT_TABLE;
        let r_client_bytes = r_client.compress().to_bytes();
        
        // Round 1: Server nonce
        let server_nonce = server_generate_nonce();
        
        // Server computes partial sig
        let s_server_bytes = server_compute_partial_signature(
            &shares.server_share,
            &server_nonce.k_server,
            &r_client_bytes,
            &shares.combined_pubkey,
            message
        ).unwrap();
        
        // Client computes partial sig
        let r_server_compressed = CompressedEdwardsY::from_slice(&server_nonce.r_server).unwrap();
        let r_server = r_server_compressed.decompress().unwrap();
        
        let combined_r = r_server + r_client;
        let combined_r_bytes = combined_r.compress().to_bytes();
        
        let mut hasher = Sha512::new();
        hasher.update(combined_r_bytes);
        hasher.update(shares.combined_pubkey);
        hasher.update(message);
        let e = Scalar::from_bytes_mod_order_wide(hasher.finalize().as_slice().try_into().unwrap());
        
        let s_client = k_client + e * x_client;
        let s_server = Scalar::from_bytes_mod_order(s_server_bytes);
        let s_combined = s_client + s_server;
        
        let mut signature = [0u8; 64];
        signature[..32].copy_from_slice(&combined_r_bytes);
        signature[32..].copy_from_slice(&s_combined.to_bytes());

        assert!(
            verify_signature(&shares.combined_pubkey, message, &signature),
            "Interactive 2-party signature must verify"
        );
    }
}
