use anyhow::Result;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tracing::{debug, info};

const POLL_TIMEOUT_SECS: usize = 30;

#[derive(Clone)]
pub struct QueueClient {
    conn: ConnectionManager,
}

impl QueueClient {
    pub async fn new(redis_url: &str) -> Result<Self> {
        info!(url = %redis_url, "connecting to Redis");
        let client = redis::Client::open(redis_url)?;
        let conn = ConnectionManager::new(client).await?;
        info!("connected to Redis");
        Ok(Self { conn })
    }

    pub async fn push<T: Serialize>(&self, queue: &str, payload: &T) -> Result<()> {
        let mut conn = self.conn.clone();
        let json = serde_json::to_string(payload)?;
        conn.lpush::<&str, String, i64>(queue, json).await?;
        Ok(())
    }

    pub async fn push_raw(&self, queue: &str, payload: &str) -> Result<()> {
        let mut conn = self.conn.clone();
        conn.lpush::<&str, &str, i64>(queue, payload).await?;
        Ok(())
    }

    pub async fn pop<T: DeserializeOwned>(&self, queue: &str) -> Result<Option<T>> {
        let mut conn = self.conn.clone();
        let result: Option<(String, String)> = conn.brpop(queue, POLL_TIMEOUT_SECS as f64).await?;
        match result {
            Some((_, json)) => {
                let preview: String = json.chars().take(120).collect();
                info!(queue = %queue, preview = %preview, "BRPOP received message");
                Ok(Some(serde_json::from_str(&json)?))
            }
            None => {
                debug!(queue = %queue, "BRPOP timed out (no message)");
                Ok(None)
            }
        }
    }

    pub async fn queue_depth(&self, queue: &str) -> Result<i64> {
        let mut conn = self.conn.clone();
        Ok(conn.llen(queue).await?)
    }

    pub async fn increment_counter(&self, key: &str) -> Result<i64> {
        let mut conn = self.conn.clone();
        Ok(conn.incr(key, 1).await?)
    }

    pub async fn sleep(&self, secs: u64) {
        tokio::time::sleep(tokio::time::Duration::from_secs(secs)).await;
    }
}
