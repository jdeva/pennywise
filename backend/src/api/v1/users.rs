use actix_web::{web, HttpRequest, HttpResponse, Result};
use log::warn;

use crate::models::{
    AuthResponse, ChangePasswordRequest, DeactivateRequest, LoginRequest, RefreshRequest,
    RegisterRequest, UpdateProfileRequest, UserPublic,
};
use crate::services::{Cache, UserService};
use crate::utils::auth::{
    create_access_token, create_refresh_token, hash_refresh_token, get_user_id_from_request,
    JwtConfig,
};
use crate::utils::validation::{validate_change_password, validate_register, validate_update_profile};
use crate::utils::AppError;

pub fn auth_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .route("/register", web::post().to(register))
            .route("/login", web::post().to(login))
            .route("/refresh", web::post().to(refresh)),
    );
}

pub fn users_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/users/me")
            .route("", web::get().to(get_profile))
            .route("", web::put().to(update_profile))
            .route("/password", web::post().to(change_password))
            .route("/deactivate", web::post().to(deactivate)),
    );
}


async fn register(
    data: web::Json<RegisterRequest>,
    user_service: web::Data<UserService>,
    jwt_config: web::Data<JwtConfig>,
    cache: web::Data<Cache>,
) -> Result<HttpResponse, AppError> {
    if let Err(details) = validate_register(&data) {
        return Err(AppError::Validation(details));
    }

    let profile = user_service.create_user(
        data.username.trim().to_string(),
        data.email.trim().to_string(),
        data.password.clone(),
    )?;

    let access_token = create_access_token(&jwt_config, &profile.id)?;
    let raw_refresh = create_refresh_token();
    let token_hash = hash_refresh_token(&raw_refresh);

    if let Err(e) = cache.store_refresh_token(
        &token_hash,
        &profile.id,
        jwt_config.refresh_expiry_seconds as usize,
    ) {
        warn!("Failed to store refresh token: {}", e);
    }

    let response = AuthResponse {
        access_token,
        refresh_token: raw_refresh,
        user: UserPublic::from(profile),
    };

    Ok(HttpResponse::Created().json(response))
}

async fn login(
    data: web::Json<LoginRequest>,
    user_service: web::Data<UserService>,
    jwt_config: web::Data<JwtConfig>,
    cache: web::Data<Cache>,
) -> Result<HttpResponse, AppError> {
    let user_id = user_service
        .get_user_id_by_username(&data.username)?
        .ok_or_else(|| AppError::Unauthorized("Invalid credentials".to_string()))?;

    let auth = user_service
        .get_auth(&user_id)?
        .ok_or_else(|| AppError::Unauthorized("Invalid credentials".to_string()))?;

    let password_valid = bcrypt::verify(&data.password, &auth.password_hash)
        .map_err(|e| AppError::Internal(format!("Password verification failed: {}", e)))?;

    if !password_valid {
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    let profile = user_service
        .get_profile(&user_id)?
        .ok_or_else(|| AppError::Unauthorized("Invalid credentials".to_string()))?;

    if !profile.is_active {
        return Err(AppError::Unauthorized("Account is deactivated".to_string()));
    }

    let access_token = create_access_token(&jwt_config, &profile.id)?;
    let raw_refresh = create_refresh_token();
    let token_hash = hash_refresh_token(&raw_refresh);

    if let Err(e) = cache.store_refresh_token(
        &token_hash,
        &profile.id,
        jwt_config.refresh_expiry_seconds as usize,
    ) {
        warn!("Failed to store refresh token: {}", e);
    }

    let response = AuthResponse {
        access_token,
        refresh_token: raw_refresh,
        user: UserPublic::from(profile),
    };

    Ok(HttpResponse::Ok().json(response))
}

async fn refresh(
    data: web::Json<RefreshRequest>,
    jwt_config: web::Data<JwtConfig>,
    cache: web::Data<Cache>,
    user_service: web::Data<UserService>,
) -> Result<HttpResponse, AppError> {
    let old_hash = hash_refresh_token(&data.refresh_token);

    let user_id = cache
        .get_refresh_token_user(&old_hash)
        .map_err(|e| AppError::Internal(format!("Cache error: {}", e)))?
        .ok_or_else(|| AppError::Unauthorized("Invalid refresh token".to_string()))?;

    cache
        .delete_refresh_token(&old_hash)
        .map_err(|e| AppError::Internal(format!("Failed to delete old refresh token: {}", e)))?;

    let profile = user_service
        .get_profile(&user_id)?
        .ok_or_else(|| AppError::Unauthorized("User not found".to_string()))?;

    if !profile.is_active {
        return Err(AppError::Unauthorized("Account is deactivated".to_string()));
    }

    let access_token = create_access_token(&jwt_config, &user_id)?;
    let raw_refresh = create_refresh_token();
    let new_hash = hash_refresh_token(&raw_refresh);

    if let Err(e) = cache.store_refresh_token(
        &new_hash,
        &user_id,
        jwt_config.refresh_expiry_seconds as usize,
    ) {
        warn!("Failed to store new refresh token: {}", e);
    }

    let response = AuthResponse {
        access_token,
        refresh_token: raw_refresh,
        user: UserPublic::from(profile),
    };

    Ok(HttpResponse::Ok().json(response))
}

async fn get_profile(
    req: HttpRequest,
    user_service: web::Data<UserService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    let profile = user_service
        .get_profile(&user_id)?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    Ok(HttpResponse::Ok().json(UserPublic::from(profile)))
}

async fn update_profile(
    req: HttpRequest,
    data: web::Json<UpdateProfileRequest>,
    user_service: web::Data<UserService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    if let Err(details) = validate_update_profile(&data) {
        return Err(AppError::Validation(details));
    }

    let trimmed_username = data.username.as_ref().map(|u| u.trim().to_string());
    let trimmed_email = data.email.as_ref().map(|e| e.trim().to_string());

    let profile = user_service.update_profile(&user_id, trimmed_username, trimmed_email)?;

    Ok(HttpResponse::Ok().json(UserPublic::from(profile)))
}

async fn change_password(
    req: HttpRequest,
    data: web::Json<ChangePasswordRequest>,
    user_service: web::Data<UserService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    if let Err(details) = validate_change_password(&data) {
        return Err(AppError::Validation(details));
    }

    let auth = user_service
        .get_auth(&user_id)?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let current_valid = bcrypt::verify(&data.current_password, &auth.password_hash)
        .map_err(|e| AppError::Internal(format!("Password verification failed: {}", e)))?;

    if !current_valid {
        return Err(AppError::Unauthorized("Current password is incorrect".to_string()));
    }

    user_service.change_password(&user_id, data.new_password.clone())?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Password changed successfully"
    })))
}

async fn deactivate(
    req: HttpRequest,
    data: web::Json<DeactivateRequest>,
    user_service: web::Data<UserService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".to_string()))?;

    let auth = user_service
        .get_auth(&user_id)?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let password_valid = bcrypt::verify(&data.password, &auth.password_hash)
        .map_err(|e| AppError::Internal(format!("Password verification failed: {}", e)))?;

    if !password_valid {
        return Err(AppError::Unauthorized("Invalid password".to_string()));
    }

    user_service.deactivate_user(&user_id)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Account deactivated successfully"
    })))
}
