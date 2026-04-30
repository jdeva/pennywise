use actix_web::{web, HttpResponse, Result};
use uuid::Uuid;

use crate::models::{SetActiveRequest, SetRoleRequest, UserPublic};
use crate::services::UserService;
use crate::utils::{AdminGuard, AppError};

pub fn admin_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/users")
            .route("", web::get().to(list_users))
            .route("/{id}/active", web::put().to(set_user_active))
            .route("/{id}/role", web::put().to(set_user_role)),
    );
}

async fn list_users(
    _guard: AdminGuard,
    user_service: web::Data<UserService>,
) -> Result<HttpResponse, AppError> {
    let profiles = user_service.list_all_users()?;
    let users: Vec<UserPublic> = profiles.into_iter().map(UserPublic::from).collect();
    Ok(HttpResponse::Ok().json(users))
}

async fn set_user_active(
    guard: AdminGuard,
    path: web::Path<Uuid>,
    body: web::Json<SetActiveRequest>,
    user_service: web::Data<UserService>,
) -> Result<HttpResponse, AppError> {
    let target_id = path.into_inner();

    if target_id == guard.0 {
        return Err(AppError::BadRequest(
            "Cannot change your own active status via admin endpoint".to_string(),
        ));
    }

    let profile = user_service.set_user_active(&target_id, body.is_active)?;
    Ok(HttpResponse::Ok().json(UserPublic::from(profile)))
}

async fn set_user_role(
    guard: AdminGuard,
    path: web::Path<Uuid>,
    body: web::Json<SetRoleRequest>,
    user_service: web::Data<UserService>,
) -> Result<HttpResponse, AppError> {
    let target_id = path.into_inner();

    if target_id == guard.0 {
        return Err(AppError::BadRequest(
            "Cannot change your own admin role via admin endpoint".to_string(),
        ));
    }

    let profile = user_service.set_user_role(&target_id, body.is_admin)?;
    Ok(HttpResponse::Ok().json(UserPublic::from(profile)))
}
