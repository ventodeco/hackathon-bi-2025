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

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
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
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
