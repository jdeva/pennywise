pub mod admin;
pub mod budgets;
pub mod chart_of_accounts;
pub mod workspaces;
pub mod transactions;
pub mod users;

use actix_web::{web, HttpResponse};

async fn health() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "service": "pennywise-backend"
    }))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .route("/health", web::get().to(health))
            .configure(users::auth_config)
            .configure(users::users_config)
            .configure(admin::admin_config)
            .configure(workspaces::workspaces_config)
            .configure(transactions::transactions_config)
            .configure(chart_of_accounts::chart_of_accounts_config)
            .configure(budgets::budgets_config),
    );
}
