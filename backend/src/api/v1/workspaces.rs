use actix_web::{web, HttpRequest, HttpResponse, Result};

use crate::models::{
    BalanceQuery, CreateWorkspaceRequest, OpeningBalanceRequest, PostTransactionRequest,
    RegisterQuery, ShareWorkspaceRequest, UpdateWorkspaceRequest, WorkspacePublic,
};
use crate::services::{TransactionService, WorkspaceService};
use crate::utils::auth::get_user_id_from_request;
use crate::utils::validation::{
    validate_create_workspace, validate_opening_balance, validate_post_transaction,
    validate_share_workspace, validate_update_workspace,
};
use crate::utils::AppError;

pub fn workspaces_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/workspaces")
            .route("", web::post().to(create_workspace))
            .route("", web::get().to(list_workspaces))
            .route("/{id}", web::get().to(get_workspace))
            .route("/{id}", web::put().to(update_workspace))
            .route("/{id}/deactivate", web::post().to(deactivate_workspace))
            .route("/{id}/share", web::post().to(share_workspace))
            .route("/{id}/share/{user_id}", web::delete().to(unshare_workspace))
            .route("/{id}/transactions", web::post().to(post_transaction))
            .route("/{id}/balance", web::get().to(query_balance))
            .route("/{id}/register", web::get().to(query_register))
            .route("/{id}/initialize", web::post().to(post_opening_balance))
            .configure(crate::api::v1::budgets::budgets_routes),
    );
}

async fn create_workspace(
    req: HttpRequest,
    data: web::Json<CreateWorkspaceRequest>,
    ws_service: web::Data<WorkspaceService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    if let Err(details) = validate_create_workspace(&data) {
        return Err(AppError::Validation(details));
    }

    let workspace = ws_service.create_workspace(&user_id, data.name.trim().to_string(), data.currency.clone())?;
    Ok(HttpResponse::Created().json(ws_service.to_public(workspace)))
}

async fn list_workspaces(
    req: HttpRequest,
    ws_service: web::Data<WorkspaceService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    let workspaces: Vec<WorkspacePublic> = ws_service
        .list_workspaces(&user_id)?
        .into_iter()
        .map(|w| ws_service.to_public(w))
        .collect();

    Ok(HttpResponse::Ok().json(workspaces))
}

async fn get_workspace(
    req: HttpRequest,
    path: web::Path<String>,
    ws_service: web::Data<WorkspaceService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    let workspace_id = uuid::Uuid::parse_str(&path.into_inner())
        .map_err(|_| AppError::BadRequest("Invalid workspace ID".to_string()))?;

    let workspace = ws_service.get_workspace_authorized(&workspace_id, &user_id)?;
    Ok(HttpResponse::Ok().json(ws_service.to_public(workspace)))
}

async fn update_workspace(
    req: HttpRequest,
    path: web::Path<String>,
    data: web::Json<UpdateWorkspaceRequest>,
    ws_service: web::Data<WorkspaceService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    let workspace_id = uuid::Uuid::parse_str(&path.into_inner())
        .map_err(|_| AppError::BadRequest("Invalid workspace ID".to_string()))?;

    if let Err(details) = validate_update_workspace(&data) {
        return Err(AppError::Validation(details));
    }

    let workspace = ws_service.update_workspace(&workspace_id, &user_id, data.name.trim().to_string())?;
    Ok(HttpResponse::Ok().json(ws_service.to_public(workspace)))
}

async fn deactivate_workspace(
    req: HttpRequest,
    path: web::Path<String>,
    ws_service: web::Data<WorkspaceService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    let workspace_id = uuid::Uuid::parse_str(&path.into_inner())
        .map_err(|_| AppError::BadRequest("Invalid workspace ID".to_string()))?;

    ws_service.deactivate_workspace(&workspace_id, &user_id)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Workspace deactivated successfully"
    })))
}

async fn share_workspace(
    req: HttpRequest,
    path: web::Path<String>,
    data: web::Json<ShareWorkspaceRequest>,
    ws_service: web::Data<WorkspaceService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    let workspace_id = uuid::Uuid::parse_str(&path.into_inner())
        .map_err(|_| AppError::BadRequest("Invalid workspace ID".to_string()))?;

    if let Err(details) = validate_share_workspace(&data) {
        return Err(AppError::Validation(details));
    }

    let workspace = ws_service.share_workspace(&workspace_id, &user_id, data.username.trim(), data.permission.clone())?;
    Ok(HttpResponse::Ok().json(ws_service.to_public(workspace)))
}

async fn unshare_workspace(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    ws_service: web::Data<WorkspaceService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    let (workspace_id_str, target_user_id_str) = path.into_inner();

    let workspace_id = uuid::Uuid::parse_str(&workspace_id_str)
        .map_err(|_| AppError::BadRequest("Invalid workspace ID".to_string()))?;

    let target_user_id = uuid::Uuid::parse_str(&target_user_id_str)
        .map_err(|_| AppError::BadRequest("Invalid user ID".to_string()))?;

    let workspace = ws_service.unshare_workspace(&workspace_id, &user_id, &target_user_id)?;
    Ok(HttpResponse::Ok().json(ws_service.to_public(workspace)))
}

async fn post_transaction(
    req: HttpRequest,
    path: web::Path<String>,
    data: web::Json<PostTransactionRequest>,
    tx_service: web::Data<TransactionService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    let workspace_id = uuid::Uuid::parse_str(&path.into_inner())
        .map_err(|_| AppError::BadRequest("Invalid workspace ID".to_string()))?;

    if let Err(details) = validate_post_transaction(&data) {
        return Err(AppError::Validation(details));
    }

    let response = tx_service.post_transaction(&workspace_id, &user_id, &data)?;
    Ok(HttpResponse::Created().json(response))
}

async fn query_balance(
    req: HttpRequest,
    path: web::Path<String>,
    query: web::Query<BalanceQuery>,
    tx_service: web::Data<TransactionService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    let workspace_id = uuid::Uuid::parse_str(&path.into_inner())
        .map_err(|_| AppError::BadRequest("Invalid workspace ID".to_string()))?;

    let pivot_user = query.pivot_user.unwrap_or(false);
    let filter_user = query.user.as_deref().filter(|s| !s.is_empty());
    let response = tx_service.query_balance(&workspace_id, &user_id, pivot_user, filter_user)?;
    Ok(HttpResponse::Ok().json(response))
}

async fn query_register(
    req: HttpRequest,
    path: web::Path<String>,
    query: web::Query<RegisterQuery>,
    tx_service: web::Data<TransactionService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    let workspace_id = uuid::Uuid::parse_str(&path.into_inner())
        .map_err(|_| AppError::BadRequest("Invalid workspace ID".to_string()))?;

    let user_filter = query.user.as_deref().filter(|s| !s.is_empty());
    let payee_filter = query.payee.as_deref().filter(|s| !s.is_empty());
    let begin = query.begin.as_deref().filter(|s| !s.is_empty());
    let end = query.end.as_deref().filter(|s| !s.is_empty());
    let response = tx_service.query_register(
        &workspace_id,
        &user_id,
        user_filter,
        payee_filter,
        begin,
        end,
    )?;
    Ok(HttpResponse::Ok().json(response))
}

async fn post_opening_balance(
    req: HttpRequest,
    path: web::Path<String>,
    data: web::Json<OpeningBalanceRequest>,
    tx_service: web::Data<TransactionService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    let workspace_id = uuid::Uuid::parse_str(&path.into_inner())
        .map_err(|_| AppError::BadRequest("Invalid workspace ID".to_string()))?;

    if let Err(details) = validate_opening_balance(&data) {
        return Err(AppError::Validation(details));
    }

    let response = tx_service.post_opening_balance(&workspace_id, &user_id, &data)?;
    Ok(HttpResponse::Created().json(response))
}
