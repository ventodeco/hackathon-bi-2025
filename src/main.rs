use actix_web::{web, App, HttpServer};
use std::env;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use crate::services::{metrics_service::MetricsService, face_match_service::FaceMatchService};

mod commons;
mod controllers;
mod models;
mod repositories;
mod services;
mod utils;
mod submissions;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    
    // Initialize tracing with JSON format
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().json())
        .init();

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

    HttpServer::new(move || {
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
                    .service(submissions::submission_controller::process_submission),
            )
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await
}
