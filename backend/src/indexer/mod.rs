pub mod worker;

use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use std::str::FromStr;
use std::sync::Arc;

// ──────────────────────────────────────────────
// Future gRPC integration (kept for production use)
// ──────────────────────────────────────────────
// use my_solana_indexer::adapters::parsers::pump_fun::PumpFunParser;
// use my_solana_indexer::domain::TransactionEvent;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IndexedTransaction {
    pub signature: String,
    pub from: String,
    pub to: String,
    pub lamports: u64,
    pub block_time: i64,
    pub slot: u64,
}

/// Start the RPC-polling indexer that watches a specific wallet address
/// for incoming and outgoing SOL transfers.
///
/// In production, this would be replaced with gRPC streaming via
/// `my-solana-indexer`'s `GrpcSourceAdapter` for real-time updates.
pub async fn start_indexer(
    redis_client: redis::Client,
    rpc_url: String,
    db_pool: crate::db::DbPool,
) {
    let mut con = redis_client
        .get_multiplexed_async_connection()
        .await
        .expect("Failed to connect to Redis for indexer");

    let rpc = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    tracing::info!("Starting RPC-polling Solana indexer...");

    // Track the last signature we've seen per address to avoid re-processing
    let mut last_signatures: std::collections::HashMap<String, Signature> =
        std::collections::HashMap::new();

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        let records = match sqlx::query(
            "SELECT server_pubkey FROM tiplinks WHERE state = 'waiting'",
        )
        .fetch_all(&db_pool)
        .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Failed to fetch watch addresses: {}", e);
                continue;
            }
        };

        let addresses: Vec<String> = records
            .into_iter()
            .filter_map(|r| {
                use sqlx::Row;
                r.try_get("server_pubkey").ok()
            })
            .collect();

        for addr_str in &addresses {
            let pubkey = match Pubkey::from_str(addr_str) {
                Ok(pk) => pk,
                Err(_) => continue,
            };

            // Fetch recent signatures for this address
            let config = solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config {
                before: None,
                until: last_signatures.get(addr_str).copied(),
                limit: Some(20),
                commitment: Some(CommitmentConfig::confirmed()),
            };

            let sigs = match rpc
                .get_signatures_for_address_with_config(&pubkey, config)
                .await
            {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("Failed to fetch signatures for {}: {}", addr_str, e);
                    continue;
                }
            };

            if sigs.is_empty() {
                continue;
            }

            // Update the last seen signature (first in list is the newest)
            if let Ok(sig) = Signature::from_str(&sigs[0].signature) {
                last_signatures.insert(addr_str.to_string(), sig);
            }

            for sig_info in &sigs {
                // Skip failed transactions
                if sig_info.err.is_some() {
                    continue;
                }

                let sig = match Signature::from_str(&sig_info.signature) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                // Fetch full transaction details
                let tx_config = RpcTransactionConfig {
                    encoding: Some(solana_transaction_status::UiTransactionEncoding::Base64),
                    commitment: Some(CommitmentConfig::confirmed()),
                    max_supported_transaction_version: Some(0),
                };

                let tx = match rpc.get_transaction_with_config(&sig, tx_config).await {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::debug!("Could not fetch tx {}: {}", sig, e);
                        continue;
                    }
                };

                let block_time = tx.block_time.unwrap_or(0);
                let slot = tx.slot;

                // Create a simplified indexed transaction
                let indexed = IndexedTransaction {
                    signature: sig_info.signature.clone(),
                    from: addr_str.to_string(),
                    to: String::new(), // Parsed from instruction data in worker
                    lamports: 0,       // Parsed from instruction data in worker
                    block_time,
                    slot,
                };

                if let Ok(json) = serde_json::to_string(&indexed) {
                    let _: redis::RedisResult<()> =
                        con.lpush("tiplink:transactions:queue", json).await;
                    tracing::info!(
                        "Indexed transaction {} for wallet {}",
                        sig_info.signature,
                        addr_str
                    );
                }
            }
        }
    }
}

// ──────────────────────────────────────────────
// gRPC Indexer (for future production use)
// ──────────────────────────────────────────────
// pub async fn start_grpc_indexer(redis_client: redis::Client) {
//     let parser = PumpFunParser::new();
//     // ... connect to Yellowstone gRPC and stream events
//     // ... use parser.parse_protobuf() on raw bytes
//     // ... push TransactionEvent::PumpFunTrade to Redis
// }
