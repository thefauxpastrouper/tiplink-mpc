use curve25519_dalek::constants::ED25519_BASEPOINT_TABLE;
use curve25519_dalek::edwards::CompressedEdwardsY;
use curve25519_dalek::scalar::Scalar;
use rand::rngs::OsRng;
use sha2::{Digest, Sha512};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct ClientMpcContext {
    share: [u8; 32],
}

#[wasm_bindgen]
impl ClientMpcContext {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let x_client = Scalar::random(&mut OsRng);
        Self {
            share: x_client.to_bytes(),
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<ClientMpcContext, JsValue> {
        if bytes.len() != 32 {
            return Err(JsValue::from_str("Invalid share length"));
        }
        let mut share = [0u8; 32];
        share.copy_from_slice(bytes);
        Ok(Self { share })
    }

    pub fn get_share(&self) -> Vec<u8> {
        self.share.to_vec()
    }

    pub fn get_public_point(&self) -> Vec<u8> {
        let x_client = Scalar::from_bytes_mod_order(self.share);
        let pub_client = &x_client * ED25519_BASEPOINT_TABLE;
        pub_client.compress().to_bytes().to_vec()
    }

    pub fn compute_nonce_commitment(&self) -> NonceContext {
        let k_client = Scalar::random(&mut OsRng);
        let r_client = &k_client * ED25519_BASEPOINT_TABLE;
        NonceContext {
            k: k_client.to_bytes(),
            r: r_client.compress().to_bytes(),
        }
    }

    pub fn compute_partial_signature(
        &self,
        k_client_bytes: &[u8],
        r_server_bytes: &[u8],
        r_client_bytes: &[u8],
        combined_pubkey_bytes: &[u8],
        message: &[u8],
    ) -> Result<Vec<u8>, JsValue> {
        if k_client_bytes.len() != 32 || r_server_bytes.len() != 32 || r_client_bytes.len() != 32 || combined_pubkey_bytes.len() != 32 {
            return Err(JsValue::from_str("Invalid key/nonce lengths"));
        }

        let k_client = Scalar::from_bytes_mod_order(
            k_client_bytes.try_into().unwrap()
        );
        let x_client = Scalar::from_bytes_mod_order(self.share);

        let r_server_compressed = CompressedEdwardsY::from_slice(r_server_bytes).unwrap();
        let r_server_point = r_server_compressed.decompress().ok_or_else(|| JsValue::from_str("Invalid server R point"))?;

        let r_client_compressed = CompressedEdwardsY::from_slice(r_client_bytes).unwrap();
        let r_client_point = r_client_compressed.decompress().ok_or_else(|| JsValue::from_str("Invalid client R point"))?;

        // R = R_server + R_client
        let combined_r = r_server_point + r_client_point;
        let combined_r_bytes = combined_r.compress().to_bytes();

        // e = Hash(R || A || M)
        let mut hasher = Sha512::new();
        hasher.update(combined_r_bytes);
        hasher.update(combined_pubkey_bytes);
        hasher.update(message);
        let hash_output = hasher.finalize();
        
        let e = Scalar::from_bytes_mod_order_wide(
            hash_output.as_slice().try_into().unwrap()
        );

        // s_client = k_client + e * x_client
        let s_client = k_client + e * x_client;

        Ok(s_client.to_bytes().to_vec())
    }

    pub fn combine_signatures(
        &self,
        s_client_bytes: &[u8],
        s_server_bytes: &[u8],
        r_client_bytes: &[u8],
        r_server_bytes: &[u8],
    ) -> Result<Vec<u8>, JsValue> {
        if s_client_bytes.len() != 32 || s_server_bytes.len() != 32 || r_client_bytes.len() != 32 || r_server_bytes.len() != 32 {
            return Err(JsValue::from_str("Invalid signature lengths"));
        }

        let s_client = Scalar::from_bytes_mod_order(s_client_bytes.try_into().unwrap());
        let s_server = Scalar::from_bytes_mod_order(s_server_bytes.try_into().unwrap());
        let s_combined = s_client + s_server;

        let r_server_compressed = CompressedEdwardsY::from_slice(r_server_bytes).unwrap();
        let r_server_point = r_server_compressed.decompress().ok_or_else(|| JsValue::from_str("Invalid server R point"))?;

        let r_client_compressed = CompressedEdwardsY::from_slice(r_client_bytes).unwrap();
        let r_client_point = r_client_compressed.decompress().ok_or_else(|| JsValue::from_str("Invalid client R point"))?;

        let combined_r = r_server_point + r_client_point;
        let combined_r_bytes = combined_r.compress().to_bytes();

        let mut signature = vec![0u8; 64];
        signature[..32].copy_from_slice(&combined_r_bytes);
        signature[32..].copy_from_slice(&s_combined.to_bytes());

        Ok(signature)
    }
}

#[wasm_bindgen]
pub struct NonceContext {
    k: [u8; 32],
    r: [u8; 32],
}

#[wasm_bindgen]
impl NonceContext {
    pub fn get_k(&self) -> Vec<u8> {
        self.k.to_vec()
    }
    
    pub fn get_r(&self) -> Vec<u8> {
        self.r.to_vec()
    }
}
