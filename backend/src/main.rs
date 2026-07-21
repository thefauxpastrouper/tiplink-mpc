use actix_web::{web, App, HttpServer};
use dotenvy::dotenv;
use solana_client::nonblocking::rpc_client::RpcClient;
use std::env;
use std::sync::Arc;

mod api;
mod db;
mod indexer;
mod mpc;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load env vars
    dotenv().ok();

    // Init structured logging
    tracing_subscriber::fmt::init();

    let db_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/tiplink".to_string());
    let redis_url =
        env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let rpc_url = env::var("RPC_URL")
        .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());

    // Connect to DB with retry logic and run migrations
    let db_pool = db::establish_connection(&db_url)
        .await
        .expect("Failed to connect to DB");

    // Connect to Redis
    let redis_client = redis::Client::open(redis_url).expect("Invalid Redis URL");

    // Initialize MPC Engine (2-party Ed25519)
    let mpc_engine = Arc::new(mpc::MpcEngine::new());

    // Solana RPC client for transaction submission
    let rpc_client: api::SharedRpcClient = Arc::new(RpcClient::new_with_commitment(
        rpc_url.clone(),
        solana_sdk::commitment_config::CommitmentConfig::confirmed(),
    ));

    // Spawn RPC-polling indexer background task
    let indexer_redis = redis_client.clone();
    let indexer_rpc_url = rpc_url.clone();
    let indexer_db = db_pool.clone();
    tokio::spawn(async move {
        indexer::start_indexer(indexer_redis, indexer_rpc_url, indexer_db).await;
    });

    // Spawn Redis → Postgres worker task
    let worker_redis = redis_client.clone();
    let worker_db = db_pool.clone();
    tokio::spawn(async move {
        indexer::worker::start_worker(worker_redis, worker_db).await;
    });

    tracing::info!("Starting TipLink backend on 0.0.0.0:8080");
    tracing::info!("Solana RPC: {}", rpc_url);

    // Start Actix-Web server
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(mpc_engine.clone()))
            .app_data(web::Data::new(rpc_client.clone()))
            .configure(api::configure)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
