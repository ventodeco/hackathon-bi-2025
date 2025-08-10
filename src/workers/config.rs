use std::env;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct WorkerConfig {
    // Main worker pool configuration
    pub background_worker_thread_enabled: bool,
    pub background_worker_consumer_thread_count: usize,
    pub worker_consumer_wait_interval: Duration,
    pub worker_consumer_max_retry: u32,

    // DLQ worker pool configuration
    pub file_upload_worker_dlq_thread_enabled: bool,
    pub file_upload_worker_dlq_thread_count: usize,
    pub file_upload_worker_dlq_wait_interval: Duration,

    // Redis configuration
    pub redis_url: String,
    pub worker_upload_file_queue: String,
    pub worker_upload_file_dlq: String,

    // Lock configuration
    pub lock_timeout: Duration,
    pub lock_retry_interval: Duration,

    // Shutdown configuration
    pub graceful_shutdown_timeout: Duration,
}

impl WorkerConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            background_worker_thread_enabled: env::var("BACKGROUND_WORKER_THREAD_ENABLED")
                .unwrap_or_else(|_| "false".to_string())
                .parse()?,

            background_worker_consumer_thread_count: env::var("BACKGROUND_WORKER_CONSUMER_THREAD_COUNT")
                .unwrap_or_else(|_| "1".to_string())
                .parse()?,

            worker_consumer_wait_interval: Duration::from_millis(
                env::var("WORKER_CONSUMER_WAIT_INTERVAL_IN_MILLISECONDS")
                    .unwrap_or_else(|_| "5000".to_string())
                    .parse()?
            ),

            worker_consumer_max_retry: env::var("WORKER_CONSUMER_MAX_RETRY")
                .unwrap_or_else(|_| "3".to_string())
                .parse()?,

            file_upload_worker_dlq_thread_enabled: env::var("FILE_UPLOAD_WORKER_DLQ_THREAD_ENABLED")
                .unwrap_or_else(|_| "false".to_string())
                .parse()?,

            file_upload_worker_dlq_thread_count: env::var("FILE_UPLOAD_WORKER_DLQ_THREAD_COUNT")
                .unwrap_or_else(|_| "1".to_string())
                .parse()?,

            file_upload_worker_dlq_wait_interval: Duration::from_millis(
                env::var("FILE_UPLOAD_WORKER_DLQ_WAIT_INTERVAL_IN_MILLISECONDS")
                    .unwrap_or_else(|_| "10000".to_string())
                    .parse()?
            ),

            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),

            worker_upload_file_queue: env::var("WORKER_UPLOAD_FILE_QUEUE")
                .unwrap_or_else(|_| "upload_file_queue".to_string()),

            worker_upload_file_dlq: env::var("WORKER_UPLOAD_FILE_DLQ")
                .unwrap_or_else(|_| "upload_file_dlq".to_string()),

            lock_timeout: Duration::from_secs(
                env::var("WORKER_LOCK_TIMEOUT_SECONDS")
                    .unwrap_or_else(|_| "300".to_string())
                    .parse()?
            ),

            lock_retry_interval: Duration::from_millis(
                env::var("WORKER_LOCK_RETRY_INTERVAL_MILLISECONDS")
                    .unwrap_or_else(|_| "100".to_string())
                    .parse()?
            ),

            graceful_shutdown_timeout: Duration::from_secs(
                env::var("WORKER_GRACEFUL_SHUTDOWN_TIMEOUT_SECONDS")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()?
            ),
        })
    }
}
