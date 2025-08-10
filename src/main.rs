use actix_web::{web, App, HttpServer};
use std::env;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use crate::services::{metrics_service::MetricsService, face_match_service::FaceMatchService};
use crate::workers::{WorkerConfig};
use tracing::{info, warn};
use std::sync::Arc;
use tokio::signal;
use crate::workers::main_worker::MainWorker;

mod commons;
mod controllers;
mod models;
mod repositories;
mod services;
mod utils;
mod submissions;
mod workers;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    
    // Initialize tracing with JSON format
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    // Determine the application mode from environment variable
    let app_mode = env::var("APP_MODE").unwrap_or_else(|_| "api".to_string());
    info!("Starting application in {} mode", app_mode);

    // Initialize worker configuration regardless of mode
    // This is needed for both API mode (if workers are enabled) and worker mode
    let worker_config = match WorkerConfig::from_env() {
        Ok(config) => {
            info!("Worker configuration loaded successfully");
            config
        },
        Err(e) => {
            warn!("Failed to load worker configuration: {}", e);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to load worker configuration"));
        }
    };

    // In worker mode, force worker threads to be enabled regardless of config
    let mut worker_config_final = worker_config.clone();
    if app_mode == "worker" {
        info!("Running in worker mode - forcing worker threads to be enabled");
        worker_config_final.background_worker_thread_enabled = true;
        // Optionally enable DLQ processing in worker mode
        worker_config_final.file_upload_worker_dlq_thread_enabled = true;
    }

    // Initialize the worker
    let mut main_worker = MainWorker::new(worker_config_final);
    
    // Always start the worker in worker mode
    // In API mode, only start if enabled in config
    if app_mode == "worker" || worker_config.background_worker_thread_enabled {
        match main_worker.start().await {
            Ok(_) => info!("File Upload Worker System started successfully"),
            Err(e) => {
                warn!("Failed to start File Upload Worker System: {}", e);
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to start worker"));
            }
        }
    }

    // In worker mode, we only need to set up shutdown handling for the worker
    if app_mode == "worker" {
        info!("Running in worker mode - API server will not be started");
        
        // Set up graceful shutdown for worker only
        let main_worker_ref = Arc::new(main_worker);
        tokio::spawn(async move {
            match signal::ctrl_c().await {
                Ok(()) => {
                    info!("Shutdown signal received, starting graceful worker shutdown");
                    main_worker_ref.signal_shutdown();
                    
                    if let Err(e) = main_worker_ref.await_shutdown().await {
                        warn!("Error during worker shutdown: {}", e);
                    }
                    info!("Worker graceful shutdown completed");
                },
                Err(e) => warn!("Error waiting for interrupt signal: {}", e),
            }
        });

        // Keep the application running until Ctrl+C is received
        match signal::ctrl_c().await {
            Ok(()) => info!("Shutdown signal received, application will exit"),
            Err(e) => warn!("Error waiting for Ctrl+C: {}", e),
        }

        return Ok(());
    }

    // Continue with API server setup only in API mode
    info!("Setting up API server");

    let host = std::env::var("HOST").expect("HOST must be set");
    let port = std::env::var("PORT").expect("PORT must be set");

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create pool");

    let pool = web::Data::new(pool);

    let metrics_service = web::Data::new(MetricsService::new(
        &std::env::var("STATSD_HOST").expect("STATSD_HOST must be set"),
        std::env::var("STATSD_PORT").expect("STATSD_PORT must be set").parse::<u16>().unwrap(),
        &std::env::var("STATSD_PREFIX").expect("STATSD_PREFIX must be set")
    ));

    let face_match_service = web::Data::new(FaceMatchService::new(
        std::env::var("FACE_MATCH_HOST").expect("FACE_MATCH_HOST must be set"),
        std::env::var("FACE_MATCH_THRESHOLD").expect("FACE_MATCH_THRESHOLD must be set").parse::<f64>().unwrap(),
        std::env::var("FACE_MATCH_TIMEOUT_MILLIS").expect("FACE_MATCH_TIMEOUT_MILLIS must be set").parse::<u64>().unwrap(),
        metrics_service.as_ref().clone(),
    ));

    let minio_service = commons::minio_service::MinioService::new(
        &env::var("MINIO_ENDPOINT").expect("MINIO_ENDPOINT must be set"),
        &env::var("MINIO_ACCESS_KEY").expect("MINIO_ACCESS_KEY must be set"),
        &env::var("MINIO_SECRET_KEY").expect("MINIO_SECRET_KEY must be set"),
        &env::var("MINIO_BUCKET_NAME").expect("MINIO_BUCKET_NAME must be set"),
    ).await.expect("Failed to initialize MinIO service");

    let server = HttpServer::new(move || {
        App::new()
            .app_data(pool.clone())
            .app_data(metrics_service.clone())
            .app_data(face_match_service.clone())
            .app_data(web::Data::new(minio_service.clone()))
            .service(
                web::scope("/v1")
                    .service(controllers::auth::register)
                    .service(controllers::auth::login)
                    .service(submissions::submission_controller::presigned_urls)
                    .service(submissions::submission_controller::face_match)
                    .service(submissions::submission_controller::process_submission)
                    .service(submissions::submission_controller::get_submission_status)
            )
    })
    .bind(format!("{}:{}", host, port))?
    .run();

    // Set up graceful shutdown for both the server and worker (if enabled)
    let server_handle = server.handle();
    let main_worker_ref = Arc::new(main_worker);
    
    // Clone for the shutdown task
    let main_worker_shutdown = Arc::clone(&main_worker_ref);
    
    // Handle graceful shutdown
    tokio::spawn(async move {
        // Wait for interrupt signal
        match signal::ctrl_c().await {
            Ok(()) => {
                info!("Shutdown signal received, starting graceful shutdown");
                
                // Signal the worker to stop (if it's running)
                if worker_config.background_worker_thread_enabled {
                    info!("Shutting down worker");
                    main_worker_shutdown.signal_shutdown();
                    
                    // Wait for worker to finish processing in-progress jobs
                    if let Err(e) = main_worker_shutdown.await_shutdown().await {
                        warn!("Error during worker shutdown: {}", e);
                    }
                }
                
                // Stop the HTTP server gracefully
                info!("Shutting down HTTP server");
                server_handle.stop(true).await;
                info!("Graceful shutdown completed");
            }
            Err(e) => warn!("Error waiting for interrupt signal: {}", e),
        }
    });

    // Start the server and wait for it to finish
    info!("API server starting at {}:{}", host, port);
    server.await?;
    
    Ok(())
}
