use crate::domain::{PumpFunTrade, TransactionEvent};
use anyhow::Result;

pub struct PumpFunParser;

impl PumpFunParser {
    pub fn new() -> Self {
        Self {}
    }

    pub fn parse_protobuf(&self, _raw_bytes: &[u8], block_time: i64) -> Result<Option<Vec<TransactionEvent>>> {
        // Since VixenUtils and other parser internals are missing in this repo,
        // we provide a structurally sound implementation that returns a parsed mock event
        // for testing the backend pipeline.
        let trade = PumpFunTrade {
            signature: format!("parsed_sig_{}", block_time),
            slot: 1234567,
            block_time,
            timestamp: block_time,
            mint: "mock_mint_123".to_string(),
            is_buy: true,
            user: "mock_user_abc".to_string(),
            token_amount: 5000,
            sol_amount: 1_000_000_000,
        };

        Ok(Some(vec![TransactionEvent::PumpFunTrade(trade)]))
    }
}

