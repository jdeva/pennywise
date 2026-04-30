use actix_web::{dev::Payload, web, FromRequest, HttpRequest, HttpMessage};
use futures_util::future::LocalBoxFuture;
use uuid::Uuid;

use crate::services::UserService;
use crate::utils::error::AppError;

pub struct AdminGuard(pub Uuid);

impl FromRequest for AdminGuard {
    type Error = AppError;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        Box::pin(async move {
            let user_id = req
                .extensions()
                .get::<Uuid>()
                .copied()
                .ok_or_else(|| AppError::Unauthorized("Not authenticated".to_string()))?;

            let user_service = req
                .app_data::<web::Data<UserService>>()
                .ok_or_else(|| AppError::Internal("UserService not configured".to_string()))?;

            let profile = user_service
                .get_profile(&user_id)?
                .ok_or_else(|| AppError::Unauthorized("User not found".to_string()))?;

            if !profile.is_admin {
                return Err(AppError::Forbidden(
                    "Admin access required".to_string(),
                ));
            }

            Ok(AdminGuard(user_id))
        })
    }
}
