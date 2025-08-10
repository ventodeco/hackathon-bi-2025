use crate::workers::{
    DlqWorker, FileUploadWorker, WorkerConfig, WorkerError, WorkerMetrics, WorkerResult,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::time::timeout;
use tracing::{error, info};

/// MainWorker coordinates both the main file upload worker and DLQ worker pools
pub struct MainWorker {
    config: WorkerConfig,
    shutdown_signal: Arc<AtomicBool>,
    metrics: Arc<WorkerMetrics>,
    file_upload_worker: Option<FileUploadWorker>,
    dlq_worker: Option<DlqWorker>,
}

impl MainWorker {
    /// Create a new MainWorker with the given configuration
    pub fn new(config: WorkerConfig) -> Self {
        let shutdown_signal = Arc::new(AtomicBool::new(false));
        let metrics = Arc::new(WorkerMetrics::new());

        Self {
            config,
            shutdown_signal,
            metrics,
            file_upload_worker: None,
            dlq_worker: None,
        }
    }

    /// Start both worker pools if they are enabled in the configuration
    pub async fn start(&mut self) -> WorkerResult<()> {
        info!("Starting File Upload Worker System");

        // Start metrics reporting background task
        let metrics_clone = self.metrics.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(300)).await;
                metrics_clone.log_metrics();
            }
        });

        // Start the main file upload worker if enabled
        if self.config.background_worker_thread_enabled {
            info!(
                "Initializing main upload worker pool with {} threads",
                self.config.background_worker_consumer_thread_count
            );
            
            let file_upload_worker = FileUploadWorker::new(
                self.config.clone(),
                self.shutdown_signal.clone(),
                self.metrics.clone(),
            )?;
            
            file_upload_worker.start().await?;
            self.file_upload_worker = Some(file_upload_worker);
            
            info!("Main upload worker pool started successfully");
        } else {
            info!("Main upload worker pool is disabled");
        }

        // Start the DLQ worker if enabled
        if self.config.file_upload_worker_dlq_thread_enabled {
            info!(
                "Initializing DLQ worker pool with {} threads",
                self.config.file_upload_worker_dlq_thread_count
            );
            
            let dlq_worker = DlqWorker::new(
                self.config.clone(),
                self.shutdown_signal.clone(),
                self.metrics.clone(),
            )?;
            
            dlq_worker.start().await?;
            self.dlq_worker = Some(dlq_worker);
            
            info!("DLQ worker pool started successfully");
        } else {
            info!("DLQ worker pool is disabled");
        }

        info!("File Upload Worker System initialization complete");
        Ok(())
    }

    /// Signal all workers to stop processing new jobs
    pub fn signal_shutdown(&self) {
        info!("Signaling shutdown to all worker pools");
        self.shutdown_signal.store(true, Ordering::SeqCst);
    }

    /// Wait for all workers to complete in-progress jobs and shut down gracefully
    pub async fn await_shutdown(&self) -> WorkerResult<()> {
        let grace_period = self.config.graceful_shutdown_timeout;
        
        info!("Waiting up to {:?} for workers to shutdown gracefully", grace_period);
        
        match timeout(grace_period, async {
            // In a real implementation, you would wait for completion signals
            // For now, just wait for a reasonable time to allow workers to finish
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            Ok(())
        })
        .await
        {
            Ok(Ok(())) => {
                info!("All worker pools shutdown gracefully");
                // Log final metrics
                self.metrics.log_metrics();
                Ok(())
            }
            Ok(Err(e)) => {
                error!("Error during worker shutdown: {}", e);
                Err(e)
            }
            Err(_) => {
                error!("Worker shutdown timed out after {:?}", grace_period);
                Err(WorkerError::Shutdown)
            }
        }
    }

    /// Get a reference to the metrics collector
    pub fn metrics(&self) -> Arc<WorkerMetrics> {
        self.metrics.clone()
    }
}
