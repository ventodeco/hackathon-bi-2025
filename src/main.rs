mod config;
mod controllers;
mod models;
mod repositories;
mod services;
mod utils;

use actix_web::{web, App, HttpServer};
use dotenv::dotenv;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use crate::services::metrics_service::MetricsService;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    
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

    HttpServer::new(move || {
        App::new()
            .app_data(pool.clone())
            .app_data(metrics_service.clone())
            .service(
                web::scope("/v1")
                    .service(controllers::auth::register)
                    .service(controllers::auth::login),
            )
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await
}
