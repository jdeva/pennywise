use actix_web::{web, HttpRequest, HttpResponse, Result};

use crate::models::{
    BudgetReportQuery, CreateBudgetRequest, ForecastQuery, SetBudgetingRequest,
    UpdateBudgetRequest, WorkspacePublic,
};
use crate::services::{BudgetService, WorkspaceService};
use crate::utils::auth::get_user_id_from_request;
use crate::utils::validation::{validate_create_budget, validate_update_budget};
use crate::utils::AppError;

pub fn budgets_config(cfg: &mut web::ServiceConfig) {
    // This registers as a standalone scope — kept for backward compat but
    // the routes are now also available via budgets_routes inside /workspaces.
}

/// Registers budget routes inside an existing /workspaces scope.
pub fn budgets_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/{id}/budgeting", web::put().to(set_budgeting))
        .route("/{id}/budgeting", web::get().to(get_budgeting))
        .route("/{id}/budgets", web::post().to(create_budget))
        .route("/{id}/budgets", web::get().to(list_budgets))
        .route("/{id}/budgets/report", web::get().to(budget_report))
        .route("/{id}/budgets/unbudgeted", web::get().to(unbudgeted_report))
        .route("/{id}/budgets/forecast", web::get().to(forecast_report))
        .route("/{id}/budgets/{index}", web::put().to(update_budget))
        .route("/{id}/budgets/{index}", web::delete().to(delete_budget));
}

async fn set_budgeting(
    req: HttpRequest,
    path: web::Path<String>,
    data: web::Json<SetBudgetingRequest>,
    ws_service: web::Data<WorkspaceService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    let workspace_id = uuid::Uuid::parse_str(&path.into_inner())
        .map_err(|_| AppError::BadRequest("Invalid workspace ID".to_string()))?;

    let workspace = ws_service.set_budgeting_enabled(&workspace_id, &user_id, data.enabled)?;
    Ok(HttpResponse::Ok().json(WorkspacePublic::from(workspace)))
}

async fn get_budgeting(
    req: HttpRequest,
    path: web::Path<String>,
    ws_service: web::Data<WorkspaceService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    let workspace_id = uuid::Uuid::parse_str(&path.into_inner())
        .map_err(|_| AppError::BadRequest("Invalid workspace ID".to_string()))?;

    let budgeting_enabled = ws_service.get_budgeting_status(&workspace_id, &user_id)?;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "budgeting_enabled": budgeting_enabled
    })))
}

async fn create_budget(
    req: HttpRequest,
    path: web::Path<String>,
    data: web::Json<CreateBudgetRequest>,
    budget_service: web::Data<BudgetService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    let workspace_id = uuid::Uuid::parse_str(&path.into_inner())
        .map_err(|_| AppError::BadRequest("Invalid workspace ID".to_string()))?;

    if let Err(details) = validate_create_budget(&data) {
        return Err(AppError::Validation(details));
    }

    let amount: f64 = data
        .amount
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid amount".to_string()))?;
    let currency = data.currency.as_deref().unwrap_or("$");

    let response = budget_service.create_budget_definition(
        &workspace_id,
        &user_id,
        &data.period,
        data.account.trim(),
        amount,
        currency,
    )?;

    Ok(HttpResponse::Created().json(response))
}

async fn list_budgets(
    req: HttpRequest,
    path: web::Path<String>,
    budget_service: web::Data<BudgetService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    let workspace_id = uuid::Uuid::parse_str(&path.into_inner())
        .map_err(|_| AppError::BadRequest("Invalid workspace ID".to_string()))?;

    let definitions = budget_service.list_budget_definitions(&workspace_id, &user_id)?;
    Ok(HttpResponse::Ok().json(definitions))
}

async fn update_budget(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    data: web::Json<UpdateBudgetRequest>,
    budget_service: web::Data<BudgetService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    let (id, index_str) = path.into_inner();
    let workspace_id = uuid::Uuid::parse_str(&id)
        .map_err(|_| AppError::BadRequest("Invalid workspace ID".to_string()))?;
    let index: usize = index_str
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid budget index".to_string()))?;

    if let Err(details) = validate_update_budget(&data) {
        return Err(AppError::Validation(details));
    }

    let amount: f64 = data
        .amount
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid amount".to_string()))?;
    let currency = data.currency.as_deref().unwrap_or("$");

    let response = budget_service.update_budget_definition(
        &workspace_id,
        &user_id,
        index,
        &data.period,
        data.account.trim(),
        amount,
        currency,
    )?;

    Ok(HttpResponse::Ok().json(response))
}

async fn delete_budget(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    budget_service: web::Data<BudgetService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    let (id, index_str) = path.into_inner();
    let workspace_id = uuid::Uuid::parse_str(&id)
        .map_err(|_| AppError::BadRequest("Invalid workspace ID".to_string()))?;
    let index: usize = index_str
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid budget index".to_string()))?;

    budget_service.delete_budget_definition(&workspace_id, &user_id, index)?;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Budget definition deleted successfully"
    })))
}

async fn budget_report(
    req: HttpRequest,
    path: web::Path<String>,
    query: web::Query<BudgetReportQuery>,
    budget_service: web::Data<BudgetService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    let workspace_id = uuid::Uuid::parse_str(&path.into_inner())
        .map_err(|_| AppError::BadRequest("Invalid workspace ID".to_string()))?;

    let response = budget_service.budget_report(
        &workspace_id,
        &user_id,
        query.begin.as_deref(),
        query.end.as_deref(),
    )?;

    Ok(HttpResponse::Ok().json(response))
}

async fn unbudgeted_report(
    req: HttpRequest,
    path: web::Path<String>,
    query: web::Query<BudgetReportQuery>,
    budget_service: web::Data<BudgetService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    let workspace_id = uuid::Uuid::parse_str(&path.into_inner())
        .map_err(|_| AppError::BadRequest("Invalid workspace ID".to_string()))?;

    let response = budget_service.unbudgeted_report(
        &workspace_id,
        &user_id,
        query.begin.as_deref(),
        query.end.as_deref(),
    )?;

    Ok(HttpResponse::Ok().json(response))
}

async fn forecast_report(
    req: HttpRequest,
    path: web::Path<String>,
    query: web::Query<ForecastQuery>,
    budget_service: web::Data<BudgetService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    let workspace_id = uuid::Uuid::parse_str(&path.into_inner())
        .map_err(|_| AppError::BadRequest("Invalid workspace ID".to_string()))?;

    let response = budget_service.forecast_report(
        &workspace_id,
        &user_id,
        query.end_date.as_deref(),
    )?;

    Ok(HttpResponse::Ok().json(response))
}
