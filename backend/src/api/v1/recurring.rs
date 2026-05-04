use actix_web::{web, HttpRequest, HttpResponse, Result};

use crate::models::{
    CreateRecurringRequest, RecurringForecastQuery, UpdateRecurringRequest,
};
use crate::services::RecurringService;
use crate::utils::auth::get_user_id_from_request;
use crate::utils::validation::{validate_create_recurring, validate_update_recurring};
use crate::utils::AppError;

pub async fn create(
    req: HttpRequest,
    path: web::Path<String>,
    data: web::Json<CreateRecurringRequest>,
    svc: web::Data<RecurringService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;
    let workspace_id = uuid::Uuid::parse_str(&path.into_inner())
        .map_err(|_| AppError::BadRequest("Invalid workspace ID".to_string()))?;

    if let Err(details) = validate_create_recurring(&data) {
        return Err(AppError::Validation(details));
    }
    let amount: f64 = data
        .amount
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid amount".to_string()))?;
    let currency = data.currency.as_deref().unwrap_or("$");
    let response = svc.create(
        &workspace_id,
        &user_id,
        data.period.trim(),
        data.account.trim(),
        data.counter_account.trim(),
        amount,
        currency,
        data.payee.as_deref(),
    )?;
    Ok(HttpResponse::Created().json(response))
}

pub async fn list(
    req: HttpRequest,
    path: web::Path<String>,
    svc: web::Data<RecurringService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;
    let workspace_id = uuid::Uuid::parse_str(&path.into_inner())
        .map_err(|_| AppError::BadRequest("Invalid workspace ID".to_string()))?;
    let items = svc.list(&workspace_id, &user_id)?;
    Ok(HttpResponse::Ok().json(items))
}

pub async fn update(
    req: HttpRequest,
    path: web::Path<(String, usize)>,
    data: web::Json<UpdateRecurringRequest>,
    svc: web::Data<RecurringService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;
    let (workspace_id_str, index) = path.into_inner();
    let workspace_id = uuid::Uuid::parse_str(&workspace_id_str)
        .map_err(|_| AppError::BadRequest("Invalid workspace ID".to_string()))?;

    if let Err(details) = validate_update_recurring(&data) {
        return Err(AppError::Validation(details));
    }
    let amount: f64 = data
        .amount
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid amount".to_string()))?;
    let currency = data.currency.as_deref().unwrap_or("$");
    let response = svc.update(
        &workspace_id,
        &user_id,
        index,
        data.period.trim(),
        data.account.trim(),
        data.counter_account.trim(),
        amount,
        currency,
        data.payee.as_deref(),
    )?;
    Ok(HttpResponse::Ok().json(response))
}

pub async fn delete(
    req: HttpRequest,
    path: web::Path<(String, usize)>,
    svc: web::Data<RecurringService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;
    let (workspace_id_str, index) = path.into_inner();
    let workspace_id = uuid::Uuid::parse_str(&workspace_id_str)
        .map_err(|_| AppError::BadRequest("Invalid workspace ID".to_string()))?;
    svc.delete(&workspace_id, &user_id, index)?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn forecast(
    req: HttpRequest,
    path: web::Path<String>,
    query: web::Query<RecurringForecastQuery>,
    svc: web::Data<RecurringService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;
    let workspace_id = uuid::Uuid::parse_str(&path.into_inner())
        .map_err(|_| AppError::BadRequest("Invalid workspace ID".to_string()))?;
    let response = svc.forecast(&workspace_id, &user_id, query.end_date.as_deref())?;
    Ok(HttpResponse::Ok().json(response))
}
