use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};

pub type DbPool = Pool<Postgres>;

pub async fn establish_connection(database_url: &str) -> Result<DbPool, sqlx::Error> {
    let mut retries = 5;
    let mut delay = std::time::Duration::from_secs(1);

    let pool = loop {
        match PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
        {
            Ok(p) => break p,
            Err(e) => {
                if retries == 0 {
                    return Err(e);
                }
                tracing::warn!(
                    "Failed to connect to DB, retrying in {:?}... ({} retries left)",
                    delay,
                    retries
                );
                tokio::time::sleep(delay).await;
                retries -= 1;
                delay *= 2;
            }
        }
    };

    // Run migrations automatically on startup
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

pub mod tiplink {
    use super::DbPool;
    use serde::{Deserialize, Serialize};
    use sqlx::Row;
    use uuid::Uuid;

    #[derive(Debug, Serialize, Deserialize)]
    pub struct TipLink {
        pub id: Uuid,
        pub public_key: String,
        pub server_pubkey: Option<String>,
        pub state: String,
    }

    /// Stores for retrieving MPC shares from the database
    #[derive(Debug)]
    pub struct TipLinkShares {
        pub id: Uuid,
        pub combined_pubkey: String,
        pub server_share: Vec<u8>,
    }

    /// Create a new TipLink with MPC key shares
    pub async fn create_tiplink(
        pool: &DbPool,
        id: Uuid,
        combined_pubkey: &str,
        server_share: &[u8],
    ) -> Result<Uuid, sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO tiplinks (id, public_key, server_share, state)
               VALUES ($1, $2, $3, 'waiting')"#,
        )
        .bind(id)
        .bind(combined_pubkey)
        .bind(server_share)
        .execute(pool)
        .await?;

        Ok(id)
    }

    pub async fn get_tiplink(pool: &DbPool, id: Uuid) -> Result<TipLink, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, public_key, server_pubkey, state FROM tiplinks WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await?;

        Ok(TipLink {
            id: row.try_get("id")?,
            public_key: row.try_get("public_key")?,
            server_pubkey: row.try_get("server_pubkey")?,
            state: row.try_get("state")?,
        })
    }

    /// Retrieve the MPC shares for signing
    pub async fn get_tiplink_shares(
        pool: &DbPool,
        id: Uuid,
    ) -> Result<TipLinkShares, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, public_key, server_share FROM tiplinks WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await?;

        Ok(TipLinkShares {
            id: row.try_get("id")?,
            combined_pubkey: row.try_get("public_key")?,
            server_share: row.try_get("server_share")?,
        })
    }

    /// Update tiplink state (e.g., waiting -> funded -> claimed)
    pub async fn update_state(
        pool: &DbPool,
        id: Uuid,
        new_state: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE tiplinks SET state = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
        )
        .bind(new_state)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Find a tiplink by its server-side wallet pubkey
    pub async fn find_by_server_pubkey(
        pool: &DbPool,
        pubkey: &str,
    ) -> Result<Option<TipLink>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, public_key, server_pubkey, state FROM tiplinks WHERE server_pubkey = $1",
        )
        .bind(pubkey)
        .fetch_optional(pool)
        .await?;

        match row {
            Some(r) => Ok(Some(TipLink {
                id: r.try_get("id")?,
                public_key: r.try_get("public_key")?,
                server_pubkey: r.try_get("server_pubkey")?,
                state: r.try_get("state")?,
            })),
            None => Ok(None),
        }
    }
}

pub mod transactions {
    use super::DbPool;
    use uuid::Uuid;

    pub async fn update_trace_status(
        pool: &DbPool,
        trace_id: Uuid,
        status: &str,
        signature: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE transactions_trace SET status = $1, signature = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $3",
        )
        .bind(status)
        .bind(signature)
        .bind(trace_id)
        .execute(pool)
        .await?;
        Ok(())
    }
}
