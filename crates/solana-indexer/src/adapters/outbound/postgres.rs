use anyhow::{Ok, Result};
use sqlx::{PgPool, postgres::PgPoolOptions};
use async_trait::async_trait;
use bigdecimal::BigDecimal;

use crate::{application::TransactionRepository, domain::{IndexerState, SolanaTransaction, TransactionEvent}};

pub struct PostgresRepository {
    pool: PgPool
}

impl PostgresRepository {
    pub async fn new(url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
                    .max_connections(5)
                    .connect_url(url)
                    .await?;

        return Ok(Self {pool})
    }
}


