use actix_web::{web, HttpRequest, HttpResponse, Result};

use crate::models::{AccountType, AddCategoryRequest, CategoryType, DeleteCategoryRequest, ListCategoriesQuery};
use crate::services::TransactionService;
use crate::utils::auth::get_user_id_from_request;
use crate::utils::validation::{validate_add_category, validate_category_name};
use crate::utils::AppError;

pub fn transactions_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/categories")
            .route("", web::get().to(list_categories))
            .route("", web::post().to(add_category))
            .route("", web::delete().to(delete_category)),
    );
}

/// Map legacy CategoryType to the new AccountType
fn map_category_to_account_type(category_type: &CategoryType) -> AccountType {
    match category_type {
        CategoryType::Expense => AccountType::Expenses,
        CategoryType::Income => AccountType::Income,
    }
}

async fn list_categories(
    req: HttpRequest,
    query: web::Query<ListCategoriesQuery>,
    tx_service: web::Data<TransactionService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    let account_type = map_category_to_account_type(&query.r#type);
    let accounts = tx_service.list_accounts(&user_id, &account_type)?;
    Ok(HttpResponse::Ok().json(accounts))
}

async fn add_category(
    req: HttpRequest,
    data: web::Json<AddCategoryRequest>,
    tx_service: web::Data<TransactionService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    if let Err(details) = validate_add_category(&data) {
        return Err(AppError::Validation(details));
    }

    let account_type = map_category_to_account_type(&data.category_type);
    tx_service.add_account(&user_id, data.name.trim().to_string(), &account_type)?;
    Ok(HttpResponse::Created().json(serde_json::json!({
        "message": "Category added successfully"
    })))
}

async fn delete_category(
    req: HttpRequest,
    data: web::Json<DeleteCategoryRequest>,
    tx_service: web::Data<TransactionService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    if let Err(details) = validate_category_name(&data.name) {
        return Err(AppError::Validation(details));
    }

    let account_type = map_category_to_account_type(&data.category_type);
    tx_service.delete_account(&user_id, data.name.trim(), &account_type)?;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Category deleted successfully"
    })))
}
