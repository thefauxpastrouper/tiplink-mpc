use super::IndexedTransaction;
use crate::db::{tiplink, DbPool};
use redis::AsyncCommands;

/// Worker that consumes indexed transactions from Redis and persists them
/// to PostgreSQL. Matches transactions against tracked TipLink wallets
/// and updates their state accordingly.
pub async fn start_worker(redis_client: redis::Client, db_pool: DbPool) {
    let mut con = redis_client
        .get_multiplexed_async_connection()
        .await
        .expect("Failed to connect to Redis for worker");

    tracing::info!("Starting Redis → Postgres worker...");

    loop {
        // Pop from the transaction queue non-blockingly
        let result: redis::RedisResult<Option<String>> =
            con.rpop("tiplink:transactions:queue", None).await;

        match result {
            Ok(Some(json)) => {
                let tx = match serde_json::from_str::<IndexedTransaction>(&json) {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::warn!("Failed to parse transaction JSON: {}", e);
                        continue;
                    }
                };

                tracing::info!("Worker processing tx: {}", tx.signature);

                // Insert the transaction into Postgres
                let res = sqlx::query(
                    r#"
                    INSERT INTO transactions (signature, sender, receiver, amount, block_time)
                    VALUES ($1, $2, $3, $4, $5)
                    ON CONFLICT DO NOTHING
                    "#,
                )
                .bind(&tx.signature)
                .bind(&tx.from)
                .bind(&tx.to)
                .bind(tx.lamports as i64)
                .bind(tx.block_time)
                .execute(&db_pool)
                .await;

                if let Err(e) = res {
                    tracing::error!("Failed to insert transaction: {:?}", e);
                    continue;
                }

                // Check if this transaction involves a tracked TipLink wallet
                // For our MVP, the indexer places the tracked TipLink address in `tx.from`.
                // Any transaction on a 'waiting' TipLink means it received its initial funding.
                if let Ok(Some(tiplink_record)) =
                    tiplink::find_by_server_pubkey(&db_pool, &tx.from).await
                {
                    if tiplink_record.state == "waiting" {
                        tracing::info!(
                            "Deposit detected for TipLink {}! Updating state to 'funded'",
                            tiplink_record.id
                        );
                        if let Err(e) =
                            tiplink::update_state(&db_pool, tiplink_record.id, "funded").await
                        {
                            tracing::error!("Failed to update TipLink state: {:?}", e);
                        }
                    }
                }
            }
            Ok(None) => {
                // Queue is empty, sleep for a bit
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
            Err(e) => {
                tracing::error!("Redis RPOP error: {:?}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    }
}
