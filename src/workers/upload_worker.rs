use crate::workers::{
    DistributedLock, FileUploadJob, RedisQueue, WorkerConfig, WorkerError, WorkerResult, WorkerMetrics
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

/// FileUploadWorker processes file upload jobs from a Redis queue
pub struct FileUploadWorker {
    config: WorkerConfig,
    redis_client: Client,
    shutdown_signal: Arc<AtomicBool>,
    metrics: Arc<WorkerMetrics>,
}

impl FileUploadWorker {
    pub fn new(config: WorkerConfig, shutdown_signal: Arc<AtomicBool>, metrics: Arc<WorkerMetrics>) -> WorkerResult<Self> {
        let redis_client = Client::open(&config.redis_url[..])?;

        Ok(Self {
            config,
            redis_client,
            shutdown_signal,
            metrics,
        })
    }

    /// Start the worker pool with the configured number of threads
    pub async fn start(&self) -> WorkerResult<()> {
        info!(
            "Starting FileUploadWorker with {} threads",
            self.config.background_worker_consumer_thread_count
        );

        let (tx, mut rx) = mpsc::channel(100);

        // Spawn consumer threads
        let mut handles = Vec::new();
        for i in 0..self.config.background_worker_consumer_thread_count {
            let worker_id = format!("worker-{}", i);
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
                    error!("Worker thread exited with error: {}", e);
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
                info!("Worker {} completed graceful shutdown", worker_id);
                completed_count += 1;
            }

            info!(
                "All {} worker threads completed graceful shutdown",
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
        info!("Worker thread started");

        // Create Redis connection
        let conn_manager = ConnectionManager::new(client).await?;

        // Create queue handler
        let mut queue = RedisQueue::new(
            &config.redis_url,
            config.worker_upload_file_queue.clone(),
            config.worker_upload_file_dlq.clone(),
        )
        .await?;

        // Periodically update queue metrics
        let metrics_clone = metrics.clone();
        let mut queue_clone = queue.clone();
        tokio::spawn(async move {
            loop {
                if let (Ok(main_depth), Ok(dlq_depth)) = (queue_clone.get_queue_length().await, queue_clone.get_dlq_length().await) {
                    metrics_clone.update_queue_depth(main_depth, dlq_depth);
                }
                sleep(std::time::Duration::from_secs(60)).await;
            }
        });

        loop {
            // Check if shutdown was requested
            if shutdown_signal.load(Ordering::Relaxed) {
                info!("Shutdown signal received, stopping worker");
                break;
            }

            info!("Worker {} polling for jobs", worker_id);
            let ran = uuid::Uuid::new_v4();
            info!("UUID: {} -> hello bos!!!", ran);
            sleep(std::time::Duration::from_millis(1000)).await;

            // // Dequeue a job with timeout
            // let job_result = queue
            //     .dequeue_job(config.worker_consumer_wait_interval.as_secs())
            //     .await;
            //
            // match job_result {
            //     Ok(Some(job)) => {
            //         // Process the job
            //         let process_result = Self::process_job(&worker_id, &mut queue, conn_manager.clone(), &config, job, metrics.clone()).await;
            //
            //         if let Err(e) = process_result {
            //             error!("Error processing job: {}", e);
            //         }
            //     }
            //     Ok(None) => {
            //         // No job available, continue polling
            //         debug!("No job available, waiting for next job");
            //     }
            //     Err(e) => {
            //         // Error dequeuing job
            //         error!("Error dequeuing job: {}", e);
            //
            //         // Brief delay before retrying to prevent tight loops on persistent errors
            //         sleep(std::time::Duration::from_millis(1000)).await;
            //     }
            // }
        }

        // Signal completion
        if let Err(e) = completion_tx.send(worker_id.clone()).await {
            error!("Failed to signal worker completion: {}", e);
        }

        info!("Worker thread exiting");
        Ok(())
    }

    #[instrument(skip(queue, conn_manager, config, metrics), fields(job_id = %job.id, esign_id = %job.esign_id))]
    async fn process_job(
        worker_id: &str,
        queue: &mut RedisQueue,
        conn_manager: ConnectionManager,
        config: &WorkerConfig,
        mut job: FileUploadJob,
        metrics: Arc<WorkerMetrics>,
    ) -> WorkerResult<()> {
        info!("Processing job: {}", job.id);
        let start_time = Instant::now();
        let _timer = metrics.start_timer();
        metrics.record_job_processed();

        // Try to acquire a distributed lock based on esign_id to prevent concurrent processing
        let lock_key = job.get_lock_key();
        let mut lock = DistributedLock::new(
            conn_manager.clone(),
            lock_key,
            config.lock_timeout,
        );

        // Try to acquire the lock with retries
        let lock_acquired = lock
            .acquire(config.lock_retry_interval, config.lock_timeout)
            .await?;

        if !lock_acquired {
            warn!("Could not acquire lock for job {}, will retry later", job.id);
            return Ok(());
        }

        // We have the lock, process the job
        let result = Self::upload_file(&job).await;

        match result {
            Ok(_) => {
                // Job successful
                info!(
                    "Job {} completed successfully in {:?}",
                    job.id,
                    start_time.elapsed()
                );
                metrics.record_job_succeeded();

                // Lock will be released when it goes out of scope
                return Ok(());
            }
            Err(WorkerError::DocumentUrlExpired) => {
                // Document URL has expired, move to DLQ without retries
                warn!(
                    "Job {} failed: document URL expired, moving to DLQ",
                    job.id
                );

                metrics.record_url_expired_error();
                metrics.record_job_moved_to_dlq();
                queue.move_to_dlq(&job).await?;
            }
            Err(e) => {
                // General error, implement retry logic
                job.increment_retry();
                metrics.record_general_error();

                if job.retry_count < config.worker_consumer_max_retry {
                    // Retry the job
                    warn!(
                        "Job {} failed: {}, retrying ({}/{})",
                        job.id,
                        e,
                        job.retry_count,
                        config.worker_consumer_max_retry
                    );

                    // Re-enqueue the job
                    queue.enqueue_job(&job).await?;
                } else {
                    // Max retries exceeded, move to DLQ
                    error!(
                        "Job {} failed after {} retries, moving to DLQ: {}",
                        job.id, job.retry_count, e
                    );

                    metrics.record_job_moved_to_dlq();
                    queue.move_to_dlq(&job).await?;
                }
            }
        }

        // Lock will be released when it goes out of scope
        Ok(())
    }

    async fn upload_file(job: &FileUploadJob) -> WorkerResult<()> {
        // This is where you would implement the actual document upload logic
        // For this example, we'll simulate the upload process

        // Simulate URL expiry check (in a real system, you'd validate this properly)
        if job.document_url.contains("expired") {
            return Err(WorkerError::DocumentUrlExpired);
        }

        // Simulate random failures for testing retry logic
        if rand::random::<f32>() < 0.1 {
            return Err(WorkerError::UploadFailed("Random upload failure".to_string()));
        }

        // Simulate successful upload (with some processing time)
        sleep(std::time::Duration::from_millis(500)).await;

        // In a real implementation, you would:
        // 1. Download the document from the URL
        // 2. Validate the document
        // 3. Process it as needed
        // 4. Upload to final destination
        // 5. Update any related records in your database

        info!(
            "Successfully uploaded document: {} ({})",
            job.document_name, job.document_type
        );

        Ok(())
    }
}
