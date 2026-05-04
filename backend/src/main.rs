mod api;
mod middleware;
mod models;
mod services;
mod utils;

use actix_cors::Cors;
use actix_web::{middleware as actix_middleware, web, App, HttpServer};
use std::env;

use services::{
    BudgetService, Cache, FileStore, RecurringService, TransactionService, UserService,
    WorkspaceService,
};
use utils::auth::JwtConfig;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let ledger_path =
        env::var("LEDGER_DATA_PATH").unwrap_or_else(|_| "./data".to_string());

    let jwt_config = JwtConfig::from_env();
    let redis_client = redis::Client::open(redis_url).expect("Failed to connect to Redis");
    let cache = Cache::new(redis_client);
    let file_store = FileStore::new(ledger_path.clone());
    let user_service = UserService::new(file_store.clone(), cache.clone(), 86400);
    let workspace_service = WorkspaceService::new(file_store.clone(), cache.clone(), user_service.clone(), 86400);
    let transaction_service = TransactionService::new(
        file_store.clone(),
        cache.clone(),
        workspace_service.clone(),
        user_service.clone(),
        86400,
    );
    let budget_service = BudgetService::new(
        file_store.clone(),
        cache.clone(),
        workspace_service.clone(),
        86400,
    );
    let recurring_service = RecurringService::new(
        file_store.clone(),
        cache.clone(),
        workspace_service.clone(),
        86400,
    );

    let port: u16 = env::var("PORT").unwrap_or_else(|_| "8080".to_string()).parse().expect("Invalid PORT");

    log::info!("Starting Pennywise backend on 0.0.0.0:{}", port);
    log::info!("Ledger data path: {}", ledger_path);

    let jwt_data = web::Data::new(jwt_config);
    let cache_data = web::Data::new(cache);
    let file_store_data = web::Data::new(file_store);
    let user_service_data = web::Data::new(user_service);
    let workspace_service_data = web::Data::new(workspace_service);
    let transaction_service_data = web::Data::new(transaction_service);
    let budget_service_data = web::Data::new(budget_service);
    let recurring_service_data = web::Data::new(recurring_service);

    HttpServer::new(move || {
        let cors = Cors::permissive();

        App::new()
            .wrap(actix_middleware::Logger::default())
            .wrap(cors)
            .wrap(middleware::AuthMiddleware)
            .app_data(jwt_data.clone())
            .app_data(cache_data.clone())
            .app_data(file_store_data.clone())
            .app_data(user_service_data.clone())
            .app_data(workspace_service_data.clone())
            .app_data(transaction_service_data.clone())
            .app_data(budget_service_data.clone())
            .app_data(recurring_service_data.clone())
            .configure(api::config)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
