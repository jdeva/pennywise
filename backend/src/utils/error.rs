use actix_web::{error::ResponseError, http::StatusCode, HttpResponse, Error as ActixError};
use log::error;
use std::fmt;

use crate::models::ValidationDetail;

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    Conflict(String),
    Validation(Vec<ValidationDetail>),
    Internal(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AppError::BadRequest(msg) => write!(f, "{}", msg),
            AppError::Unauthorized(msg) => write!(f, "{}", msg),
            AppError::Forbidden(msg) => write!(f, "{}", msg),
            AppError::NotFound(msg) => write!(f, "{}", msg),
            AppError::Conflict(msg) => write!(f, "{}", msg),
            AppError::Validation(_) => write!(f, "Validation failed"),
            AppError::Internal(msg) => write!(f, "{}", msg),
        }
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        match self {
            AppError::Internal(msg) => {
                error!("Internal server error: {}", msg);
                HttpResponse::build(self.status_code()).json(serde_json::json!({
                    "error": "Internal server error"
                }))
            }
            AppError::Validation(details) => {
                HttpResponse::build(self.status_code()).json(serde_json::json!({
                    "error": "Validation failed",
                    "details": details
                }))
            }
            _ => {
                HttpResponse::build(self.status_code()).json(serde_json::json!({
                    "error": self.to_string()
                }))
            }
        }
    }
}

impl From<ActixError> for AppError {
    fn from(err: ActixError) -> Self {
        AppError::Internal(err.to_string())
    }
}
