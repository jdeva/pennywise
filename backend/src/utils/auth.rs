use actix_web::{dev::ServiceRequest, Error, HttpMessage};
use actix_web::error::ErrorUnauthorized;
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::env;
use uuid::Uuid;

use crate::utils::AppError;

#[derive(Clone)]
pub struct JwtConfig {
    pub secret: Vec<u8>,
    pub access_expiry_seconds: i64,
    pub refresh_expiry_seconds: i64,
}

impl JwtConfig {
    pub fn from_env() -> Self {
        let secret = env::var("JWT_SECRET").unwrap_or_else(|_| {
            panic!("JWT_SECRET environment variable is required but not set")
        });

        if secret.len() < 32 {
            panic!(
                "JWT_SECRET must be at least 32 characters, got {}",
                secret.len()
            );
        }

        let access_expiry_seconds = env::var("JWT_ACCESS_EXPIRY_SECONDS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(900);

        let refresh_expiry_seconds = env::var("JWT_REFRESH_EXPIRY_SECONDS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(604_800);

        JwtConfig {
            secret: secret.into_bytes(),
            access_expiry_seconds,
            refresh_expiry_seconds,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: i64,
    pub iat: i64,
}

pub fn create_access_token(config: &JwtConfig, user_id: &Uuid) -> Result<String, AppError> {
    let now = Utc::now().timestamp();
    let claims = Claims {
        sub: user_id.to_string(),
        exp: now + config.access_expiry_seconds,
        iat: now,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(&config.secret),
    )
    .map_err(|e| AppError::Internal(format!("Failed to create access token: {}", e)))
}

pub fn verify_access_token(config: &JwtConfig, token: &str) -> Result<Claims, AppError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(&config.secret),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|e| AppError::Unauthorized(format!("Invalid token: {}", e)))
}

pub fn create_refresh_token() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    hex::encode(bytes)
}

pub fn hash_refresh_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn extract_user_id(req: &ServiceRequest) -> Result<Uuid, Error> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| ErrorUnauthorized("Missing authorization header"))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| ErrorUnauthorized("Invalid authorization format"))?;

    let config = req
        .app_data::<actix_web::web::Data<JwtConfig>>()
        .ok_or_else(|| ErrorUnauthorized("JWT configuration not available"))?;

    let claims = verify_access_token(config, token)
        .map_err(|_| ErrorUnauthorized("Invalid token"))?;

    Uuid::parse_str(&claims.sub)
        .map_err(|_| ErrorUnauthorized("Invalid user ID in token"))
}

pub fn get_user_id_from_request(req: &actix_web::HttpRequest) -> Result<Uuid, Error> {
    req.extensions()
        .get::<Uuid>()
        .copied()
        .ok_or_else(|| ErrorUnauthorized("User not authenticated"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn test_config() -> JwtConfig {
        JwtConfig {
            secret: b"this-is-a-test-secret-that-is-at-least-32-chars-long".to_vec(),
            access_expiry_seconds: 900,
            refresh_expiry_seconds: 604_800,
        }
    }

    #[test]
    fn create_and_verify_access_token() {
        let config = test_config();
        let user_id = Uuid::new_v4();

        let token = create_access_token(&config, &user_id).unwrap();
        let claims = verify_access_token(&config, &token).unwrap();

        assert_eq!(claims.sub, user_id.to_string());
        assert!(claims.exp > claims.iat);
        assert_eq!(claims.exp - claims.iat, 900);
    }

    #[test]
    fn verify_rejects_token_with_wrong_secret() {
        let config = test_config();
        let user_id = Uuid::new_v4();
        let token = create_access_token(&config, &user_id).unwrap();

        let wrong_config = JwtConfig {
            secret: b"a-completely-different-secret-that-is-also-32-chars".to_vec(),
            ..config
        };

        let result = verify_access_token(&wrong_config, &token);
        assert!(result.is_err());
    }

    #[test]
    fn verify_rejects_garbage_token() {
        let config = test_config();
        let result = verify_access_token(&config, "not.a.valid.jwt");
        assert!(result.is_err());
    }

    #[test]
    fn refresh_token_is_64_hex_chars() {
        let token = create_refresh_token();
        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn refresh_tokens_are_unique() {
        let t1 = create_refresh_token();
        let t2 = create_refresh_token();
        assert_ne!(t1, t2);
    }

    #[test]
    fn hash_refresh_token_is_deterministic() {
        let token = "some-refresh-token";
        let h1 = hash_refresh_token(token);
        let h2 = hash_refresh_token(token);
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_refresh_token_differs_for_different_inputs() {
        let h1 = hash_refresh_token("token-a");
        let h2 = hash_refresh_token("token-b");
        assert_ne!(h1, h2);
    }

    #[test]
    fn hash_refresh_token_is_64_hex_chars() {
        let hash = hash_refresh_token("any-token");
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    #[should_panic(expected = "JWT_SECRET environment variable is required")]
    fn from_env_panics_when_secret_missing() {
        env::remove_var("JWT_SECRET");
        JwtConfig::from_env();
    }

    #[test]
    #[should_panic(expected = "JWT_SECRET must be at least 32 characters")]
    fn from_env_panics_when_secret_too_short() {
        env::set_var("JWT_SECRET", "short");
        JwtConfig::from_env();
    }

    #[test]
    fn from_env_uses_defaults_for_expiry() {
        env::set_var("JWT_SECRET", "a-secret-that-is-definitely-at-least-32-characters-long");
        env::remove_var("JWT_ACCESS_EXPIRY_SECONDS");
        env::remove_var("JWT_REFRESH_EXPIRY_SECONDS");

        let config = JwtConfig::from_env();
        assert_eq!(config.access_expiry_seconds, 900);
        assert_eq!(config.refresh_expiry_seconds, 604_800);
    }

    #[test]
    fn from_env_reads_custom_expiry() {
        env::set_var("JWT_SECRET", "a-secret-that-is-definitely-at-least-32-characters-long");
        env::set_var("JWT_ACCESS_EXPIRY_SECONDS", "1800");
        env::set_var("JWT_REFRESH_EXPIRY_SECONDS", "86400");

        let config = JwtConfig::from_env();
        assert_eq!(config.access_expiry_seconds, 1800);
        assert_eq!(config.refresh_expiry_seconds, 86400);

        // Clean up
        env::remove_var("JWT_ACCESS_EXPIRY_SECONDS");
        env::remove_var("JWT_REFRESH_EXPIRY_SECONDS");
    }
}
