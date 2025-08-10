use redis::{AsyncCommands, Client, Connection};
use redis::aio::ConnectionManager;
use crate::workers::{FileUploadJob, WorkerError, WorkerResult};
use tracing::{info, warn, error};

#[derive(Clone)]
pub struct RedisQueue {
    connection_manager: ConnectionManager,
    queue_name: String,
    dlq_name: String,
}

impl RedisQueue {
    pub async fn new(redis_url: &str, queue_name: String, dlq_name: String) -> WorkerResult<Self> {
        let client = Client::open(redis_url)?;
        let connection_manager = ConnectionManager::new(client).await?;

        Ok(Self {
            connection_manager,
            queue_name,
            dlq_name,
        })
    }

    pub async fn enqueue_job(&mut self, job: &FileUploadJob) -> WorkerResult<()> {
        let job_json = job.to_json()?;
        self.connection_manager
            .lpush::<_, _, ()>(&self.queue_name, job_json)
            .await?;

        info!("Job {} enqueued to {}", job.id, self.queue_name);
        Ok(())
    }

    pub async fn dequeue_job(&mut self, timeout_seconds: u64) -> WorkerResult<Option<FileUploadJob>> {
        let result: Option<(String, String)> = self.connection_manager
            .brpop(&self.queue_name, timeout_seconds as f64)
            .await?;

        match result {
            Some((_, job_json)) => {
                match FileUploadJob::from_json(&job_json) {
                    Ok(job) => {
                        info!("Job {} dequeued from {}", job.id, self.queue_name);
                        Ok(Some(job))
                    }
                    Err(e) => {
                        error!("Failed to deserialize job from {}: {}", self.queue_name, e);
                        Err(WorkerError::Json(e))
                    }
                }
            }
            None => Ok(None), // Timeout reached
        }
    }

    pub async fn move_to_dlq(&mut self, job: &FileUploadJob) -> WorkerResult<()> {
        let job_json = job.to_json()?;
        self.connection_manager
            .lpush::<_, _, ()>(&self.dlq_name, job_json)
            .await?;

        warn!("Job {} moved to DLQ: {}", job.id, self.dlq_name);
        Ok(())
    }

    pub async fn dequeue_dlq_job(&mut self, timeout_seconds: u64) -> WorkerResult<Option<FileUploadJob>> {
        let result: Option<(String, String)> = self.connection_manager
            .brpop(&self.dlq_name, timeout_seconds as f64)
            .await?;

        match result {
            Some((_, job_json)) => {
                match FileUploadJob::from_json(&job_json) {
                    Ok(job) => {
                        info!("Job {} dequeued from DLQ: {}", job.id, self.dlq_name);
                        Ok(Some(job))
                    }
                    Err(e) => {
                        error!("Failed to deserialize job from DLQ {}: {}", self.dlq_name, e);
                        Err(WorkerError::Json(e))
                    }
                }
            }
            None => Ok(None), // Timeout reached
        }
    }

    pub async fn get_queue_length(&mut self) -> WorkerResult<u64> {
        let length: u64 = self.connection_manager
            .llen(&self.queue_name)
            .await?;
        Ok(length)
    }

    pub async fn get_dlq_length(&mut self) -> WorkerResult<u64> {
        let length: u64 = self.connection_manager
            .llen(&self.dlq_name)
            .await?;
        Ok(length)
    }
}
