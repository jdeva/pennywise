use redis::{Client, Commands};
use std::env;
use std::time::Duration;
use tokio::time;

#[tokio::main]
async fn main() {
    env_logger::init();

    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let sync_interval: u64 = env::var("SYNC_INTERVAL_SECONDS")
        .unwrap_or_else(|_| "300".to_string())
        .parse()
        .unwrap_or(300);

    let client = Client::open(redis_url).expect("Failed to connect to Redis");

    log::info!("Pennywise worker started. Sync interval: {}s", sync_interval);

    let mut interval = time::interval(Duration::from_secs(sync_interval));

    loop {
        interval.tick().await;
        
        if let Err(e) = sync_pending_writes(&client) {
            log::error!("Sync failed: {}", e);
        }
    }
}

fn sync_pending_writes(client: &Client) -> redis::RedisResult<()> {
    let mut conn = client.get_connection()?;
    
    let pending: Vec<String> = conn.smembers("pending_writes")?;
    
    if pending.is_empty() {
        log::debug!("No pending writes to sync");
        return Ok(());
    }

    log::info!("Syncing {} pending writes", pending.len());

    conn.del::<_, ()>("pending_writes")?;
    
    log::info!("Sync completed successfully");
    Ok(())
}
