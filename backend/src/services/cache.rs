use log::warn;
use redis::{Client, Commands, RedisResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone)]
pub struct Cache {
    client: Client,
}

impl Cache {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> RedisResult<Option<T>> {
        let mut conn = self.client.get_connection()?;
        let value: Option<String> = conn.get(key)?;

        match value {
            Some(v) => Ok(serde_json::from_str(&v).ok()),
            None => Ok(None),
        }
    }

    pub fn set<T: Serialize>(&self, key: &str, value: &T, ttl_seconds: usize) -> RedisResult<()> {
        let mut conn = self.client.get_connection()?;
        let serialized = serde_json::to_string(value)
            .map_err(|e| redis::RedisError::from((redis::ErrorKind::TypeError, "serialization failed", e.to_string())))?;
        conn.set_ex(key, serialized, ttl_seconds as u64)
    }

    /// Sets a value and logs (but swallows) errors — use from write paths
    /// where Redis failures should not block the primary file-backed write.
    pub fn set_or_warn<T: Serialize>(&self, key: &str, value: &T, ttl_seconds: usize) {
        if let Err(e) = self.set(key, value, ttl_seconds) {
            warn!("Cache write failed for {}: {}", key, e);
        }
    }

    pub fn delete(&self, key: &str) -> RedisResult<()> {
        let mut conn = self.client.get_connection()?;
        conn.del(key)
    }

    pub fn delete_or_warn(&self, key: &str) {
        if let Err(e) = self.delete(key) {
            warn!("Cache delete failed for {}: {}", key, e);
        }
    }

    pub fn store_refresh_token(&self, token_hash: &str, user_id: &Uuid, ttl: usize) -> RedisResult<()> {
        let mut conn = self.client.get_connection()?;
        let key = format!("refresh:{}", token_hash);
        conn.set_ex(key, user_id.to_string(), ttl as u64)
    }

    pub fn get_refresh_token_user(&self, token_hash: &str) -> RedisResult<Option<Uuid>> {
        let mut conn = self.client.get_connection()?;
        let key = format!("refresh:{}", token_hash);
        let value: Option<String> = conn.get(key)?;

        match value {
            Some(v) => Ok(Uuid::parse_str(&v).ok()),
            None => Ok(None),
        }
    }

    pub fn delete_refresh_token(&self, token_hash: &str) -> RedisResult<()> {
        let mut conn = self.client.get_connection()?;
        let key = format!("refresh:{}", token_hash);
        conn.del(key)
    }
}
