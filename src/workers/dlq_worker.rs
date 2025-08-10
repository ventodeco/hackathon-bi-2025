use crate::workers::{
    FileUploadJob, RedisQueue, WorkerConfig, WorkerError, WorkerResult, WorkerMetrics
};
use redis::aio::ConnectionManager;
use redis::Client;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{debug, error, info, instrument, warn};

/// DlqWorker processes failed jobs from the Dead Letter Queue
pub struct DlqWorker {
    config: WorkerConfig,
    redis_client: Client,
    shutdown_signal: Arc<AtomicBool>,
    metrics: Arc<WorkerMetrics>,
}

impl DlqWorker {
    pub fn new(config: WorkerConfig, shutdown_signal: Arc<AtomicBool>, metrics: Arc<WorkerMetrics>) -> WorkerResult<Self> {
        let redis_client = Client::open(&config.redis_url[..])?;

        Ok(Self {
            config,
            redis_client,
            shutdown_signal,
            metrics,
        })
    }

    /// Start the DLQ worker pool with the configured number of threads
    pub async fn start(&self) -> WorkerResult<()> {
        info!(
            "Starting DlqWorker with {} threads",
            self.config.file_upload_worker_dlq_thread_count
        );

        let (tx, mut rx) = mpsc::channel(100);

        // Spawn consumer threads
        let mut handles = Vec::new();
        for i in 0..self.config.file_upload_worker_dlq_thread_count {
            let worker_id = format!("dlq-worker-{}", i);
            let thread_config = self.config.clone();
            let thread_client = self.redis_client.clone();
            let thread_shutdown = self.shutdown_signal.clone();
            let thread_tx = tx.clone();
            let thread_metrics = self.metrics.clone();

            let handle = tokio::spawn(async move {
                let result = Self::run_consumer(
                    worker_id,
                    thread_config,
                    thread_client,
                    thread_shutdown,
                    thread_tx,
                    thread_metrics,
                )
                .await;

                if let Err(e) = result {
                    error!("DLQ worker thread exited with error: {}", e);
                }
            });

            handles.push(handle);
        }

        // Drop the original sender so the channel can close when all senders are done
        drop(tx);

        // Wait for shutdown signal
        tokio::spawn(async move {
            // Wait for all threads to report completion
            let mut completed_count = 0;
            while let Some(worker_id) = rx.recv().await {
                info!("DLQ Worker {} completed graceful shutdown", worker_id);
                completed_count += 1;
            }

            info!(
                "All {} DLQ worker threads completed graceful shutdown",
                completed_count
            );
        });

        Ok(())
    }

    #[instrument(skip(config, client, shutdown_signal, completion_tx, metrics), fields(worker_id = %worker_id))]
    async fn run_consumer(
        worker_id: String,
        config: WorkerConfig,
        client: Client,
        shutdown_signal: Arc<AtomicBool>,
        completion_tx: mpsc::Sender<String>,
        metrics: Arc<WorkerMetrics>,
    ) -> WorkerResult<()> {
        info!("DLQ worker thread started");
        
        // Create Redis connection
        let conn_manager = ConnectionManager::new(client).await?;
        
        // Create queue handler
        let mut queue = RedisQueue::new(
            &config.redis_url,
            config.worker_upload_file_queue.clone(),
            config.worker_upload_file_dlq.clone(),
        )
        .await?;

        loop {
            // Check if shutdown was requested
            if shutdown_signal.load(Ordering::Relaxed) {
                info!("Shutdown signal received, stopping DLQ worker");
                break;
            }

            // Dequeue a job from DLQ with timeout
            let job_result = queue
                .dequeue_dlq_job(config.file_upload_worker_dlq_wait_interval.as_secs())
                .await;

            match job_result {
                Ok(Some(job)) => {
                    // Process the DLQ job
                    let process_result = Self::process_dlq_job(&worker_id, &mut queue, conn_manager.clone(), &config, job, metrics.clone()).await;
                    
                    if let Err(e) = process_result {
                        error!("Error processing DLQ job: {}", e);
                    }
                }
                Ok(None) => {
                    // No job available, continue polling
                    debug!("No DLQ job available, waiting for next job");
                }
                Err(e) => {
                    // Error dequeuing job
                    error!("Error dequeuing DLQ job: {}", e);
                    
                    // Brief delay before retrying to prevent tight loops on persistent errors
                    sleep(std::time::Duration::from_millis(1000)).await;
                }
            }
        }

        // Signal completion
        if let Err(e) = completion_tx.send(worker_id.clone()).await {
            error!("Failed to signal DLQ worker completion: {}", e);
        }

        info!("DLQ worker thread exiting");
        Ok(())
    }

    #[instrument(skip(_worker_id, _queue, _conn_manager, _config, metrics), fields(job_id = %job.id, esign_id = %job.esign_id))]
    async fn process_dlq_job(
        _worker_id: &str,
        _queue: &mut RedisQueue,
        _conn_manager: ConnectionManager,
        _config: &WorkerConfig,
        job: FileUploadJob,
        metrics: Arc<WorkerMetrics>,
    ) -> WorkerResult<()> {
        info!("Processing DLQ job: {}", job.id);
        let start_time = Instant::now();
        metrics.record_job_processed();

        // Analyze job failures
        if Self::is_recoverable_error(&job) {
            info!("DLQ job {} appears to be recoverable, attempting special handling", job.id);
            
            // Try special handling for different error types
            match Self::handle_dlq_job(&job).await {
                Ok(_) => {
                    info!(
                        "DLQ job {} successfully recovered in {:?}",
                        job.id,
                        start_time.elapsed()
                    );
                    metrics.record_job_succeeded();
                    return Ok(());
                }
                Err(e) => {
                    warn!(
                        "DLQ job {} special handling failed: {}",
                        job.id, e
                    );
                    metrics.record_general_error();
                    
                    // Log for manual intervention
                    error!(
                        "DLQ job {} requires manual intervention: {:?}",
                        job.id,
                        job
                    );
                }
            }
        } else {
            // Non-recoverable error, log for manual intervention
            warn!(
                "DLQ job {} has non-recoverable error, flagging for manual intervention: {:?}",
                job.id,
                job
            );
            metrics.record_general_error();
        }

        // Here you might want to:
        // 1. Store the job in a database for manual review
        // 2. Send alerts or notifications for manual intervention
        // 3. Implement more sophisticated recovery mechanisms

        info!(
            "DLQ job {} processing completed in {:?}",
            job.id,
            start_time.elapsed()
        );

        Ok(())
    }

    fn is_recoverable_error(job: &FileUploadJob) -> bool {
        // Implement logic to determine if an error is recoverable
        // For example, URL expiration might be recoverable if we can refresh the URL
        
        // Document URL expired errors might be recoverable by requesting a new URL
        if job.document_url.contains("expired") {
            return true;
        }
        
        // Jobs with specific error types in metadata might be recoverable
        if let Some(error_type) = job.metadata.get("error_type") {
            match error_type.as_str() {
                Some("temporary_network_error") => return true,
                Some("rate_limited") => return true,
                Some("service_unavailable") => return true,
                _ => {}
            }
        }
        
        // Default to non-recoverable
        false
    }

    async fn handle_dlq_job(job: &FileUploadJob) -> WorkerResult<()> {
        // Implement special handling for different error types
        
        // For URL expiration, we might:
        // 1. Request a new URL from the source system
        // 2. Update the job with the new URL
        // 3. Re-process the job
        if job.document_url.contains("expired") {
            // Simulate obtaining a new URL
            info!("Attempting to refresh expired URL for document {}", job.document_name);
            
            // In a real implementation, you would call an API to refresh the URL
            sleep(std::time::Duration::from_millis(500)).await;
            
            // Simulate success or failure
            if rand::random::<bool>() {
                info!("Successfully refreshed URL for document {}", job.document_name);
                return Ok(());
            } else {
                return Err(WorkerError::UploadFailed(
                    "Could not refresh expired URL".to_string()
                ));
            }
        }
        
        // For other error types, implement appropriate recovery strategies
        
        // If we reach here, we don't have a specific handling strategy
        Err(WorkerError::UploadFailed(
            "No recovery strategy available for this error".to_string()
        ))
    }
}
