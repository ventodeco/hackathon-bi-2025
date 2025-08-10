use thiserror::Error;

#[derive(Error, Debug)]
pub enum WorkerError {
    #[error("Redis connection error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Lock acquisition failed: {0}")]
    LockAcquisition(String),

    #[error("Document URL expired")]
    DocumentUrlExpired,

    #[error("Upload failed: {0}")]
    UploadFailed(String),

    #[error("Worker shutdown")]
    Shutdown,

    #[error("Configuration error: {0}")]
    Config(#[from] anyhow::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),
}

pub type WorkerResult<T> = Result<T, WorkerError>;
