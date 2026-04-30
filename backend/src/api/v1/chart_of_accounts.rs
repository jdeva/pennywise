use actix_web::{web, HttpRequest, HttpResponse, Result};

use crate::models::{AddAccountRequest, DeleteAccountRequest, ListAccountsQuery};
use crate::services::TransactionService;
use crate::utils::auth::get_user_id_from_request;
use crate::utils::validation::{validate_add_chart_account, validate_delete_chart_account};
use crate::utils::AppError;

pub fn chart_of_accounts_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/chart-of-accounts")
            .route("", web::get().to(list_accounts))
            .route("", web::post().to(add_account))
            .route("", web::delete().to(delete_account)),
    );
}

async fn list_accounts(
    req: HttpRequest,
    query: web::Query<ListAccountsQuery>,
    tx_service: web::Data<TransactionService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    let accounts = tx_service.list_accounts(&user_id, &query.r#type)?;
    Ok(HttpResponse::Ok().json(accounts))
}

async fn add_account(
    req: HttpRequest,
    data: web::Json<AddAccountRequest>,
    tx_service: web::Data<TransactionService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    if let Err(details) = validate_add_chart_account(&data) {
        return Err(AppError::Validation(details));
    }

    tx_service.add_account(&user_id, data.name.trim().to_string(), &data.account_type)?;
    Ok(HttpResponse::Created().json(serde_json::json!({
        "message": "Account added successfully"
    })))
}

async fn delete_account(
    req: HttpRequest,
    data: web::Json<DeleteAccountRequest>,
    tx_service: web::Data<TransactionService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    if let Err(details) = validate_delete_chart_account(&data) {
        return Err(AppError::Validation(details));
    }

    tx_service.delete_account(&user_id, data.name.trim(), &data.account_type)?;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Account deleted successfully"
    })))
}
