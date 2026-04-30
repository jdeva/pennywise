use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Profile data — stored in user-{uuid}.json
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserProfile {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub master_ledger: String,
    pub owned_accounts: Vec<Uuid>,
    pub shared_accounts: Vec<Uuid>,
    pub is_active: bool,
    #[serde(default)]
    pub is_admin: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Auth data — stored in user-{uuid}-auth.json (chmod 600)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserAuth {
    pub user_id: Uuid,
    pub password_hash: String,
    pub updated_at: DateTime<Utc>,
}

// Public-facing user (API responses, no sensitive fields)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPublic {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub master_ledger: String,
    pub owned_accounts: Vec<Uuid>,
    pub shared_accounts: Vec<Uuid>,
    pub is_active: bool,
    pub is_admin: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<UserProfile> for UserPublic {
    fn from(profile: UserProfile) -> Self {
        UserPublic {
            id: profile.id,
            username: profile.username,
            email: profile.email,
            master_ledger: profile.master_ledger,
            owned_accounts: profile.owned_accounts,
            shared_accounts: profile.shared_accounts,
            is_active: profile.is_active,
            is_admin: profile.is_admin,
            created_at: profile.created_at,
            updated_at: profile.updated_at,
        }
    }
}

// Request DTOs
#[derive(Debug, Clone, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProfileRequest {
    pub username: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeactivateRequest {
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

// Admin request DTOs
#[derive(Debug, Clone, Deserialize)]
pub struct SetActiveRequest {
    pub is_active: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetRoleRequest {
    pub is_admin: bool,
}

// Response DTOs
#[derive(Debug, Clone, Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub user: UserPublic,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationErrorResponse {
    pub error: String,
    pub details: Vec<ValidationDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationDetail {
    pub field: String,
    pub message: String,
}
