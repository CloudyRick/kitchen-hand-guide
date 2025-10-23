mod auth;
mod db;
mod handlers;
mod middleware;
mod models;
mod utils;

use actix_files as fs;
use actix_web::{middleware as actix_middleware, web, App, HttpServer};
use dotenv::dotenv;
use std::env;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables from .env file
    dotenv().ok();

    // Initialize logger
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // Get configuration from environment
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env file");

    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());

    let upload_dir = env::var("UPLOAD_DIR")
        .unwrap_or_else(|_| "./static/uploads".to_string());

    // Create upload directory if it doesn't exist
    std::fs::create_dir_all(&upload_dir)
        .expect("Failed to create upload directory");

    // Create database connection pool
    println!("Connecting to database...");
    let pool = db::create_pool(&database_url)
        .await
        .expect("Failed to create database pool");

    // Test database connection
    db::test_connection(&pool)
        .await
        .expect("Failed to connect to database");

    println!("Database connection successful!");

    // Initialize S3 client
    println!("Initializing AWS S3 client...");
    let s3_client = utils::init_s3_client().await;
    println!("AWS S3 client initialized!");

    let server_address = format!("{}:{}", host, port);
    println!("Starting server at http://{}", server_address);

    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            // Add logger middleware
            .wrap(actix_middleware::Logger::default())
            // Add database pool to app state
            .app_data(web::Data::new(pool.clone()))
            // Add S3 client to app state
            .app_data(web::Data::new(s3_client.clone()))
            // Public Routes
            .route("/", web::get().to(handlers::index))
            .route("/search", web::get().to(handlers::search))
            .route("/preparations", web::get().to(handlers::preparations_index))
            // Authentication Routes
            .route("/login", web::get().to(handlers::login_form))
            .route("/login", web::post().to(handlers::login))
            .route("/register", web::get().to(handlers::register_form))
            .route("/register", web::post().to(handlers::register))
            .route("/logout", web::get().to(handlers::logout))
            // Error Pages
            .route("/401", web::get().to(handlers::error_401))
            // Serve static files
            .service(fs::Files::new("/static", "./static").show_files_listing())
            // Protected Routes - Require Authentication (specific routes first to avoid conflicts)
            .service(
                web::resource("/product/new")
                    .route(web::get().to(handlers::new_product_form))
                    .wrap(middleware::Authentication)
            )
            .service(
                web::resource("/product")
                    .route(web::post().to(handlers::create_product))
                    .wrap(middleware::Authentication)
            )
            .service(
                web::resource("/product/{id}/edit")
                    .route(web::get().to(handlers::edit_product_form))
                    .wrap(middleware::Authentication)
            )
            .service(
                web::resource("/product/{id}/update")
                    .route(web::post().to(handlers::update_product))
                    .wrap(middleware::Authentication)
            )
            .service(
                web::resource("/preparation/new")
                    .route(web::get().to(handlers::new_preparation_form))
                    .wrap(middleware::Authentication)
            )
            .service(
                web::resource("/preparation")
                    .route(web::post().to(handlers::create_preparation))
                    .wrap(middleware::Authentication)
            )
            .service(
                web::resource("/preparation/{id}/edit")
                    .route(web::get().to(handlers::edit_preparation_form))
                    .wrap(middleware::Authentication)
            )
            .service(
                web::resource("/preparation/{id}/update")
                    .route(web::post().to(handlers::update_preparation))
                    .wrap(middleware::Authentication)
            )
            // Public detail routes (accessible without authentication, MUST come after specific routes)
            .route("/product/{id}", web::get().to(handlers::product_detail))
            .route("/preparation/{preparation_id}", web::get().to(handlers::preparation_detail))
    })
    .bind(&server_address)?
    .run()
    .await
}
