use redis::aio::ConnectionManager;
use redis::{AsyncCommands, SetOptions, SetExpiry};
use std::time::{Duration, Instant};
use tracing::{debug, warn};
use crate::workers::WorkerResult;

pub struct DistributedLock {
    connection_manager: ConnectionManager,
    lock_key: String,
    lock_value: String,
    lock_timeout: Duration,
}

impl DistributedLock {
    pub fn new(
        connection_manager: ConnectionManager,
        lock_key: String,
        lock_timeout: Duration,
    ) -> Self {
        // Use a random value as the lock identifier to ensure only the owner can release it
        let lock_value = uuid::Uuid::new_v4().to_string();

        Self {
            connection_manager,
            lock_key,
            lock_value,
            lock_timeout,
        }
    }

    pub async fn acquire(&mut self, retry_interval: Duration, max_wait: Duration) -> WorkerResult<bool> {
        let start_time = Instant::now();

        loop {
            // Try to acquire the lock using SET NX EX (only set if key doesn't exist with expiration)
            let options = SetOptions::default()
                // .conditional_set(SetCondition::NX)
                .with_expiration(SetExpiry::EX(self.lock_timeout.as_secs() as usize));

            let acquired: bool = self.connection_manager
                .set_options(&self.lock_key, &self.lock_value, options)
                .await?;

            if acquired {
                debug!("Lock acquired: {}", self.lock_key);
                return Ok(true);
            }

            // Check if we've exceeded the maximum wait time
            if start_time.elapsed() >= max_wait {
                warn!("Failed to acquire lock after {:?}: {}", max_wait, self.lock_key);
                return Ok(false);
            }

            // Wait before retrying
            tokio::time::sleep(retry_interval).await;
        }
    }

    pub async fn release(&mut self) -> WorkerResult<bool> {
        // Use a Lua script to ensure we only delete the key if it contains our lock value
        // This prevents accidentally releasing someone else's lock if our lock expired
        let script = r#"
            if redis.call('GET', KEYS[1]) == ARGV[1] then
                return redis.call('DEL', KEYS[1])
            else
                return 0
            end
        "#;

        let result: i32 = redis::Script::new(script)
            .key(&self.lock_key)
            .arg(&self.lock_value)
            .invoke_async(&mut self.connection_manager)
            .await?;

        let released = result == 1;
        if released {
            debug!("Lock released: {}", self.lock_key);
        } else {
            warn!("Failed to release lock (possibly expired): {}", self.lock_key);
        }

        Ok(released)
    }

    pub async fn refresh(&mut self) -> WorkerResult<bool> {
        // Only refresh if we still own the lock
        let script = r#"
            if redis.call('GET', KEYS[1]) == ARGV[1] then
                return redis.call('EXPIRE', KEYS[1], ARGV[2])
            else
                return 0
            end
        "#;

        let result: i32 = redis::Script::new(script)
            .key(&self.lock_key)
            .arg(&self.lock_value)
            .arg(self.lock_timeout.as_secs())
            .invoke_async(&mut self.connection_manager)
            .await?;

        let refreshed = result == 1;
        if refreshed {
            debug!("Lock refreshed: {}", self.lock_key);
        } else {
            warn!("Failed to refresh lock (possibly expired): {}", self.lock_key);
        }

        Ok(refreshed)
    }
}

impl Drop for DistributedLock {
    fn drop(&mut self) {
        // Create a new runtime for the blocking operation in drop
        let rt = tokio::runtime::Runtime::new().unwrap();

        // Try to release the lock when the instance is dropped
        // This is a best effort and might fail if the process is killed abruptly
        rt.block_on(async {
            if let Err(e) = self.release().await {
                warn!("Failed to release lock during drop: {}: {}", self.lock_key, e);
            }
        });
    }
}
