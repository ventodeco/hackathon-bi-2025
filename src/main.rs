mod config;
mod controllers;
mod models;
mod repositories;
mod services;
mod utils;

use actix_web::{web, App, HttpServer};
use dotenv::dotenv;
use sqlx::postgres::PgPoolOptions;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let host = std::env::var("HOST").expect("HOST must be set");
    let port = std::env::var("PORT").expect("PORT must be set");

    let database_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        std::env::var("DB_USERNAME").expect("DB_USERNAME must be set"),
        std::env::var("DB_PASSWORD").expect("DB_PASSWORD must be set"),
        std::env::var("DB_HOST").expect("DB_HOST must be set"),
        std::env::var("DB_PORT").expect("DB_PORT must be set"),
        std::env::var("DB_NAME").expect("DB_NAME must be set")
    );
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create pool");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
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
