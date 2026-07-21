mod domain;
mod application;
mod adapters;
mod infrastructre;

// For making the datastructure to be sharable between threads Arc is used, 
// for locking functionality between threads for accessing Mutex is used
use std::sync::{Arc, Mutex};
// Rpc CLinet is basically a rust wrapper on top of JSON-RPC for 
// querying solana node
use solana_client::{rpc_client::RpcClient};

use crate::{adapters::{FileSourceAdaptor, GrpcSourceAdapter, JupiterVixenParser, PostgresRepository, PumpFunParser, RaydiumAmmParser, SplTokenTransfer, TelegramAdapter, run_backfill_producer}, application::{EventBuffer, IngestionPipeline, NotificationService, TransactionParser, TransactionRepository, TransactionSource}, domain::{ChainEvent, IndexerState}, infrastructure::MemoryBuffer};

// It defines the type of the source from which from which the data is going to
// get extracted
#[derive(Debug, PartialEq)]
enum SourceType {
    File, 
    Grpc
}

// Creating helper function from_env() for matching the right url according
// to the source specified by the user
impl SourceType {
    fn from_env()-> Result<Self, String> {
        let raw = std::env::var("SOURCE_TYPE").map_err(|e| "Source Type is not provided".to_string())?;

        match raw.to_lowercase().as_str() {
            "file"=> Ok(SourceType::File),
            "grpc"=> Ok(SourceType::Grpc),
            _ => Err(format!("Invalid SOURCE_TYPE: {}",raw))
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>{
    // Install Crypto Provider for TLS (required for rustls)
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install crypto provider!!");

    // loading the env constants
    dotenv::dotenv().ok();

    // used for starting the trace of the process
    tracing_subscriber::fmt::init();

    // Extracting the various contants provided by the env file by using the 
    // std::env::var() function db_ur, rpc_url, telegram_token, telegram_chat_id
    let db_url = std::env::var("DATABASE_URL").expect("Database URL is not provided!");
    let rpc_url = std::env::var("RPC_URL").expect("RPC URL is not provided");
    let telegram_token = std::env::var("TELEGRAM_BOT_TOKEN").ok();
    let telegram_chat_id = std::env::var("TELEGRAM_CHAT_ID").ok();

    // The notifier service is created to intialize the call to Telegram adapter and send alert on Telegram if
    // the amount of transaction is above 1 sol
    let notifier_service = if let(Some(token), Some(chat_id)) = (telegram_token, telegram_chat_id) {
        tracing::info!("Telegram notifications enabled!!");

        let adapter = Arc::new(TelegramAdapter::new(token, chat_id));

        // Alert input Amount> 1 SOL
        // You should ideally check USD value, but raw amount is fine for now
        Some(Arc::new(NotificationService::new(adapter, 1_000_000_000)))
    } else {
        tracing::warn!("Telegram credentials missing. Notifications disabled");
        None
    };     
    
    // Initialize connection to the postgres
    let postgres_db = PostgresRepository::new(&db_url).expect("Failed to connect the database!!");

    // Wrap the connection to pg inside the Arc
    let repo = Arc::new(postgres_db);

    // Use of adapter on the basis of the source used and wrppaing the source adapter inside the Arc and Mutex
    let source: Arc<Mutex<dyn TransactionSource>> = if source_type == SourceType::File {
        Arc::new(Mutex::new(FileSourceAdaptor::new(50_000)))
    } else {
        let grpc_url = std::env::var("GRPC_URL").unwrap_or("http://127.0.0.1:10000".to_string());
        let grpc_token = std::env::var("GRPC_TOKEN").ok();
        tracing::info!("Connecting to the gRPC at source: {}", grpc_url);
        let source_adaptor = GrpcSourceAdapter::connect(grpc_url, grpc_token).await.expect("Failed to connect the grpc endpoint.");

        Arc::new(Mutex::new(source_adaptor))
    };

    // Create a buffer and store using the MemoryBuffer Struct and then wrap the buffer in Arc
    let (buffer, rx) = MemoryBuffer::new(50_000);
    let buffer = Arc::new(buffer);

    // Extract the last slot from the pg with default value 0
    let last_slot = repo.get_last_slot().await.unwrap_or(0);

    // Now get the current slot to see the difference
    let current_network_slot = RpcClient::new(&rpc_url).get_slope().unwrap();
    tracing::info!("Resuming from Slot: {}", last_slot);

    // Create a vector of all the boxed parsers
    let parser : Vec<Box<dyn TransactionParser>> = vec![
        Box::new(SplTokenTransfer::new()),
        Box::new(RaydiumAmmParser::new()),
        Box::new(JupiterVixenParser::new()),
        Box::new(PumpFunParser::new())
    ];
    
    // Backfill Strategy - diff < 2000 RPC
    // if it is greater than 2000, then I am assuming we are probably starting the indexer for the first time in the mainnet
    // if current_network_slot - last_slot < 2000 {
    // tracing::info!("Huge Gap Detected, Starting Backfilling Mode....")
    // 
    //      let (buffer, rx) = MemoryBuffer::new(50_000);
    //      let buffer = Arc::new(buffer);
    //
    //      let rpc_url_clone = rpc_url.clone();
    //      tokio::spawn(async move {
    //      run_backfill_producer(
    //          rpc_url_clone,
    //          buffer,
    //          last_slot,
    //          current_network_slot
    //      ).await;
    //      });
    //      
    //      let backfill_parsers: Vec<Box<dyn TransactionParser>>  = vec![
    //          Box::new(SplTokenParser::new()),
    //          Box::new(RaydiumAmmParser::new()),
    //          Box::new(JupiterVixenParser::new()),
    //          Box::new(PumpFunParser::new())
    //      ];
    //
    //      let mut pipeline = IngestionPipeline::new(rx, repo.clone(), backfill_parsers, notifier_service.clone());
    //      
    //      pipeline.run().await;
    //      tracing::info!("Backfill Complete!!!");
    // }

    // clone the source and buffer
    let source_clone = source.clone();
    let buffer_clone = buffer.clone();

    // Spawning Fetcher task
    let _  = tokio::spawn(async move{
        tracing::info!("Starting Fetcher Task....");

        loop {
            // Fetch State
            let event = source_clone.lock().await.next_event().await;
            match event {
                Ok(Some(txn)) => {
                    // "Buffer" State
                    if let Err(_) = buffer_clone.produce(txn).await {
                        tracing::error!("Buffer closed, stopped fetcher");
                        break;
                    }
                },
                Ok(None) => {
                    tracing::info!("Source Stream Finished");
                    break;
                },
                Err(e) => {
                    tracing::error!("Error reading source: {:?}", e);
                    // Optional: Add a retry delay here 
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    });

    // Ingestion Pipeline, it does not know it is from what 
    let mut pipeline = IngestionPipeline::new(rx, repo, parsers, notifier_service);

    tracing::info!("Starting Ingestion Pipeline...");
    pipeline.run().await;
    Ok(())

}
