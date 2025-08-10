use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// WorkerMetrics tracks performance statistics for the worker pools
pub struct WorkerMetrics {
    // Success/failure counters
    pub jobs_processed: AtomicU64,
    pub jobs_succeeded: AtomicU64,
    pub jobs_failed: AtomicU64,
    pub jobs_moved_to_dlq: AtomicU64,
    
    // Error type counters
    pub url_expired_errors: AtomicU64,
    pub general_errors: AtomicU64,
    
    // Timing metrics (stored as milliseconds)
    pub total_processing_time_ms: AtomicU64,
    
    // Queue depth
    pub main_queue_depth: AtomicU64,
    pub dlq_depth: AtomicU64,
}

impl WorkerMetrics {
    pub fn new() -> Self {
        Self {
            jobs_processed: AtomicU64::new(0),
            jobs_succeeded: AtomicU64::new(0),
            jobs_failed: AtomicU64::new(0),
            jobs_moved_to_dlq: AtomicU64::new(0),
            url_expired_errors: AtomicU64::new(0),
            general_errors: AtomicU64::new(0),
            total_processing_time_ms: AtomicU64::new(0),
            main_queue_depth: AtomicU64::new(0),
            dlq_depth: AtomicU64::new(0),
        }
    }
    
    pub fn record_job_processed(&self) {
        self.jobs_processed.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_job_succeeded(&self) {
        self.jobs_succeeded.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_job_failed(&self) {
        self.jobs_failed.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_job_moved_to_dlq(&self) {
        self.jobs_moved_to_dlq.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_url_expired_error(&self) {
        self.url_expired_errors.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_general_error(&self) {
        self.general_errors.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_processing_time(&self, duration: Duration) {
        let ms = duration.as_millis() as u64;
        self.total_processing_time_ms.fetch_add(ms, Ordering::Relaxed);
    }
    
    pub fn update_queue_depth(&self, main_depth: u64, dlq_depth: u64) {
        self.main_queue_depth.store(main_depth, Ordering::Relaxed);
        self.dlq_depth.store(dlq_depth, Ordering::Relaxed);
    }
    
    pub fn log_metrics(&self) {
        let jobs_processed = self.jobs_processed.load(Ordering::Relaxed);
        
        if jobs_processed > 0 {
            let jobs_succeeded = self.jobs_succeeded.load(Ordering::Relaxed);
            let jobs_failed = self.jobs_failed.load(Ordering::Relaxed);
            let jobs_moved_to_dlq = self.jobs_moved_to_dlq.load(Ordering::Relaxed);
            let url_expired_errors = self.url_expired_errors.load(Ordering::Relaxed);
            let general_errors = self.general_errors.load(Ordering::Relaxed);
            let total_time_ms = self.total_processing_time_ms.load(Ordering::Relaxed);
            let avg_time_ms = if jobs_processed > 0 {
                total_time_ms / jobs_processed
            } else {
                0
            };
            let main_depth = self.main_queue_depth.load(Ordering::Relaxed);
            let dlq_depth = self.dlq_depth.load(Ordering::Relaxed);
            
            info!(
                "Worker metrics: processed={}, succeeded={}, failed={}, moved_to_dlq={}, \
                 url_expired_errors={}, general_errors={}, avg_time_ms={}, \
                 main_queue_depth={}, dlq_depth={}",
                jobs_processed,
                jobs_succeeded,
                jobs_failed,
                jobs_moved_to_dlq,
                url_expired_errors,
                general_errors,
                avg_time_ms,
                main_depth,
                dlq_depth
            );
            
            // Alert if DLQ is growing
            if dlq_depth > 10 {
                warn!("DLQ depth is high: {}", dlq_depth);
            }
            
            // Alert if error rate is high
            let error_rate = if jobs_processed > 0 {
                (jobs_failed as f64) / (jobs_processed as f64)
            } else {
                0.0
            };
            
            if error_rate > 0.1 {
                warn!("Worker error rate is high: {:.2}%", error_rate * 100.0);
            }
        }
    }
    
    /// Create a timer that will record processing time when dropped
    pub fn start_timer(&self) -> MetricsTimer {
        MetricsTimer {
            metrics: self,
            start_time: Instant::now(),
        }
    }
}

impl Default for WorkerMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Timer that automatically records the duration when it goes out of scope
pub struct MetricsTimer<'a> {
    metrics: &'a WorkerMetrics,
    start_time: Instant,
}

impl<'a> Drop for MetricsTimer<'a> {
    fn drop(&mut self) {
        let duration = self.start_time.elapsed();
        self.metrics.record_processing_time(duration);
    }
}
