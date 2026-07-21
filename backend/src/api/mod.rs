use actix_web::{web, HttpResponse, Responder, ResponseError};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Signature,
    system_instruction,
    transaction::Transaction,
};
use std::str::FromStr;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;
use validator::Validate;

use crate::db::{tiplink, DbPool};
use crate::mpc::SharedMpcEngine;

/// Shared Solana RPC client
pub type SharedRpcClient = Arc<RpcClient>;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Database error: {0}")]
    DbError(#[from] sqlx::Error),
    #[error("Validation error: {0}")]
    ValidationError(#[from] validator::ValidationErrors),
    #[error("Invalid transaction format")]
    InvalidTransactionFormat,
    #[error("MPC error: {0}")]
    MpcError(String),
    #[error("Solana RPC error: {0}")]
    SolanaError(String),
    #[error("Not found")]
    NotFound,
    #[error("Invalid address: {0}")]
    InvalidAddress(String),
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ApiError::DbError(_) => HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Internal server error"})),
            ApiError::ValidationError(e) => {
                HttpResponse::BadRequest().json(serde_json::json!({"error": e.to_string()}))
            }
            ApiError::InvalidTransactionFormat => {
                HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid hex transaction data"}))
            }
            ApiError::MpcError(e) => HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": format!("MPC failed: {}", e)})),
            ApiError::SolanaError(e) => HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": format!("Solana RPC failed: {}", e)})),
            ApiError::NotFound => {
                HttpResponse::NotFound().json(serde_json::json!({"error": "Resource not found"}))
            }
            ApiError::InvalidAddress(e) => {
                HttpResponse::BadRequest().json(serde_json::json!({"error": format!("Invalid address: {}", e)}))
            }
        }
    }
}

// ──────────────────────────────────────────────
// Init Wallet — 2-party DKG
// ──────────────────────────────────────────────

#[derive(Deserialize, Validate)]
pub struct InitWalletReq {
    #[validate(length(min = 32, max = 44, message = "Invalid client pubkey length"))]
    pub client_pubkey: String,
}

#[derive(Serialize)]
pub struct InitWalletResp {
    pub tiplink_id: Uuid,
    pub server_pubkey: String,
    pub combined_pubkey: String,
    pub message: String,
}

pub async fn init_wallet(
    req: web::Json<InitWalletReq>,
    db: web::Data<DbPool>,
    mpc: web::Data<SharedMpcEngine>,
) -> Result<impl Responder, ApiError> {
    req.validate()?;

    let client_pubkey_bytes = bs58::decode(&req.client_pubkey)
        .into_vec()
        .map_err(|e| ApiError::InvalidAddress(e.to_string()))?;
        
    let client_pubkey_array: [u8; 32] = client_pubkey_bytes
        .try_into()
        .map_err(|_| ApiError::InvalidAddress("Invalid client pubkey length".into()))?;

    // Perform interactive DKG
    let shares = mpc.two_party_keygen(&client_pubkey_array)
        .map_err(|e| ApiError::MpcError(e.to_string()))?;

    let tiplink_id = Uuid::new_v4();
    let combined_pubkey_str = bs58::encode(shares.combined_pubkey).into_string();

    tiplink::create_tiplink(db.get_ref(), tiplink_id, &combined_pubkey_str, &shares.server_share)
        .await
        .map_err(|_| ApiError::DbError(sqlx::Error::RowNotFound))?; // using DbError for now since DatabaseError was not found
    
    Ok(web::Json(InitWalletResp {
        message: "TipLink initialized with interactive MPC".into(),
        tiplink_id,
        server_pubkey: bs58::encode(shares.server_share).into_string(),
        combined_pubkey: combined_pubkey_str,
    }))
}

// -----------------------------------------------------------------------------
// Interactive 2-Party Transfer
// -----------------------------------------------------------------------------

#[derive(Deserialize, Validate)]
pub struct TransferSignReq {
    pub tiplink_id: Uuid,
    #[validate(length(min = 32, max = 44, message = "Invalid recipient address"))]
    pub to_address: String,
    pub lamports: u64,
    pub r_client: String,
}

#[derive(Serialize)]
pub struct TransferSignResp {
    pub r_server: String,
    pub s_server: String,
    pub message_data: String,
    pub recent_blockhash: String,
}

pub async fn sign_transfer(
    req: web::Json<TransferSignReq>,
    db: web::Data<DbPool>,
    mpc: web::Data<SharedMpcEngine>,
    rpc: web::Data<SharedRpcClient>,
) -> Result<impl Responder, ApiError> {
    req.validate()?;

    let to_pubkey = Pubkey::from_str(&req.to_address)
        .map_err(|e| ApiError::InvalidAddress(e.to_string()))?;

    let shares = tiplink::get_tiplink_shares(db.get_ref(), req.tiplink_id)
        .await
        .map_err(|_| ApiError::NotFound)?;

    let from_pubkey = Pubkey::from_str(&shares.combined_pubkey)
        .map_err(|e| ApiError::InvalidAddress(e.to_string()))?;

    let r_client_bytes = bs58::decode(&req.r_client)
        .into_vec()
        .map_err(|e| ApiError::InvalidAddress(e.to_string()))?;
        
    let r_client_array: [u8; 32] = r_client_bytes
        .try_into()
        .map_err(|_| ApiError::InvalidAddress("Invalid r_client length".into()))?;

    let transfer_ix = system_instruction::transfer(&from_pubkey, &to_pubkey, req.lamports);

    let recent_blockhash = rpc
        .get_latest_blockhash()
        .await
        .map_err(|e| ApiError::SolanaError(e.to_string()))?;

    let message = solana_sdk::message::Message::new_with_blockhash(
        &[transfer_ix],
        Some(&from_pubkey),
        &recent_blockhash,
    );
    let mut tx = Transaction::new_unsigned(message);
    let message_data = tx.message_data();

    // Server round 1 & 2: Generate nonce, compute partial signature
    let server_nonce = mpc.two_party_generate_nonce();
    
    let server_share_array: [u8; 32] = shares.server_share
        .try_into()
        .map_err(|_| ApiError::MpcError("Invalid server share length".into()))?;
    
    let combined_pubkey_array: [u8; 32] = bs58::decode(&shares.combined_pubkey)
        .into_vec()
        .map_err(|_| ApiError::InvalidAddress("Invalid combined pubkey format".into()))?
        .try_into()
        .map_err(|_| ApiError::InvalidAddress("Invalid combined pubkey length".into()))?;

    let s_server_bytes = mpc
        .two_party_compute_partial_signature(
            &server_share_array,
            &server_nonce.k_server,
            &r_client_array,
            &combined_pubkey_array,
            &message_data,
        )
        .map_err(|e| ApiError::MpcError(e.to_string()))?;

    Ok(web::Json(TransferSignResp {
        r_server: bs58::encode(server_nonce.r_server).into_string(),
        s_server: bs58::encode(s_server_bytes).into_string(),
        message_data: bs58::encode(message_data).into_string(),
        recent_blockhash: recent_blockhash.to_string(),
    }))
}

#[derive(Deserialize, Validate)]
pub struct TransferSubmitReq {
    pub tiplink_id: Uuid,
    pub to_address: String,
    pub lamports: u64,
    pub recent_blockhash: String,
    pub signature: String, // 64-byte signature base58
}

#[derive(Serialize)]
pub struct TransferResp {
    pub message: String,
    pub signature: String,
    pub status: String,
}

pub async fn submit_transfer(
    req: web::Json<TransferSubmitReq>,
    db: web::Data<DbPool>,
    rpc: web::Data<SharedRpcClient>,
) -> Result<impl Responder, ApiError> {
    req.validate()?;

    let to_pubkey = Pubkey::from_str(&req.to_address)
        .map_err(|e| ApiError::InvalidAddress(e.to_string()))?;
        
    let blockhash = solana_sdk::hash::Hash::from_str(&req.recent_blockhash)
        .map_err(|_| ApiError::InvalidAddress("Invalid blockhash".into()))?;

    let shares = tiplink::get_tiplink_shares(db.get_ref(), req.tiplink_id)
        .await
        .map_err(|_| ApiError::NotFound)?;

    let from_pubkey = Pubkey::from_str(&shares.combined_pubkey)
        .map_err(|e| ApiError::InvalidAddress(e.to_string()))?;

    let signature_bytes = bs58::decode(&req.signature)
        .into_vec()
        .map_err(|e| ApiError::InvalidAddress(e.to_string()))?;
        
    if signature_bytes.len() != 64 {
        return Err(ApiError::InvalidAddress("Invalid signature length".into()));
    }

    let transfer_ix = system_instruction::transfer(&from_pubkey, &to_pubkey, req.lamports);

    let message = solana_sdk::message::Message::new_with_blockhash(
        &[transfer_ix],
        Some(&from_pubkey),
        &blockhash,
    );
    let mut tx = Transaction::new_unsigned(message);
    
    tx.signatures[0] = Signature::try_from(signature_bytes.as_slice()).unwrap();

    let tx_sig = rpc
        .send_and_confirm_transaction(&tx)
        .await
        .map_err(|e| ApiError::SolanaError(e.to_string()))?;

    let sig_str = tx_sig.to_string();

    let trace_id = Uuid::new_v4();
    let mut db_tx = db.begin().await?;

    sqlx::query(
        "INSERT INTO transactions_trace (id, tiplink_id, status, signature) VALUES ($1, $2, 'success', $3)",
    )
    .bind(trace_id)
    .bind(req.tiplink_id)
    .bind(&sig_str)
    .execute(&mut *db_tx)
    .await?;

    db_tx.commit().await?;

    Ok(web::Json(TransferResp {
        message: "SOL transfer signed with interactive MPC and confirmed on-chain".into(),
        signature: sig_str,
        status: "success".into(),
    }))
}

// ──────────────────────────────────────────────
// Query endpoints
// ──────────────────────────────────────────────

pub async fn get_tiplink(
    path: web::Path<Uuid>,
    db: web::Data<DbPool>,
) -> Result<impl Responder, ApiError> {
    let id = path.into_inner();
    let t = tiplink::get_tiplink(db.get_ref(), id).await
        .map_err(|_| ApiError::NotFound)?;
    Ok(HttpResponse::Ok().json(t))
}

pub async fn transaction_status(
    path: web::Path<String>,
    db: web::Data<DbPool>,
) -> Result<impl Responder, ApiError> {
    let sig = path.into_inner();
    let row: (String, Option<String>) = sqlx::query_as(
        "SELECT status, error_message FROM transactions_trace WHERE signature = $1",
    )
    .bind(&sig)
    .fetch_optional(db.get_ref())
    .await?
    .ok_or(ApiError::NotFound)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": row.0,
        "error": row.1,
    })))
}

pub async fn health() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

// ──────────────────────────────────────────────
// Route configuration
// ──────────────────────────────────────────────

pub async fn root_handler() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "name": "TipLink MPC API",
        "version": "1.0",
        "status": "active"
    }))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    // Root handler
    cfg.route("/", web::get().to(root_handler));
    
    // API scope
    cfg.service(
        web::scope("/api")
            .route("/health", web::get().to(health))
            .route("/wallet/init", web::post().to(init_wallet))
            .route("/transfer/sign", web::post().to(sign_transfer))
            .route("/transfer/submit", web::post().to(submit_transfer))
            .route("/tiplink/{id}", web::get().to(get_tiplink))
            .route(
                "/transactions/status/{signature}",
                web::get().to(transaction_status),
            ),
    );
}
