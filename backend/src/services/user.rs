use crate::models::{UserAuth, UserProfile};
use crate::services::cache::Cache;
use crate::services::file_store::FileStore;
use crate::utils::error::AppError;
use chrono::Utc;
use log::warn;
use uuid::Uuid;

const BCRYPT_COST: u32 = 14;

fn user_key(user_id: &Uuid) -> String {
    format!("user:{}", user_id)
}

fn username_key(username: &str) -> String {
    format!("username:{}", username)
}

fn email_key(email: &str) -> String {
    format!("email:{}", email)
}

#[derive(Clone)]
pub struct UserService {
    file_store: FileStore,
    cache: Cache,
    cache_ttl: usize,
}

impl UserService {
    pub fn new(file_store: FileStore, cache: Cache, cache_ttl: usize) -> Self {
        Self {
            file_store,
            cache,
            cache_ttl,
        }
    }

    pub fn create_user(
        &self,
        username: String,
        email: String,
        password: String,
    ) -> Result<UserProfile, AppError> {
        if self.get_user_id_by_username(&username)?.is_some() {
            return Err(AppError::Conflict(format!(
                "Username '{}' is already taken",
                username
            )));
        }
        if self.get_user_id_by_email(&email)?.is_some() {
            return Err(AppError::Conflict(format!(
                "Email '{}' is already registered",
                email
            )));
        }

        let password_hash = bcrypt::hash(&password, BCRYPT_COST)
            .map_err(|e| AppError::Internal(format!("Failed to hash password: {}", e)))?;

        let user_id = Uuid::new_v4();
        let now = Utc::now();

        let auth = UserAuth {
            user_id,
            password_hash,
            updated_at: now,
        };
        self.file_store.write_auth(&auth)?;

        let is_first_user = !self.file_store.has_any_users()?;

        let profile = UserProfile {
            id: user_id,
            username: username.clone(),
            email: email.clone(),
            master_ledger: format!("users/user-{}-master.ledger", user_id),
            owned_accounts: vec![],
            shared_accounts: vec![],
            is_active: true,
            is_admin: is_first_user,
            created_at: now,
            updated_at: now,
        };
        self.file_store.write_profile(&profile)?;
        self.file_store.create_master_ledger(&profile)?;

        self.cache.set_or_warn(&user_key(&user_id), &profile, self.cache_ttl);
        self.cache.set_or_warn(&username_key(&username), &user_id.to_string(), self.cache_ttl);
        self.cache.set_or_warn(&email_key(&email), &user_id.to_string(), self.cache_ttl);

        Ok(profile)
    }

    pub fn get_profile(&self, user_id: &Uuid) -> Result<Option<UserProfile>, AppError> {
        let key = user_key(user_id);
        match self.cache.get::<UserProfile>(&key) {
            Ok(Some(profile)) => return Ok(Some(profile)),
            Ok(None) => {}
            Err(e) => warn!("Cache read failed for {}: {}", key, e),
        }

        let profile = self.file_store.read_profile(user_id)?;
        if let Some(ref p) = profile {
            self.cache.set_or_warn(&key, p, self.cache_ttl);
        }
        Ok(profile)
    }

    pub fn get_auth(&self, user_id: &Uuid) -> Result<Option<UserAuth>, AppError> {
        self.file_store.read_auth(user_id)
    }

    pub fn get_user_id_by_username(&self, username: &str) -> Result<Option<Uuid>, AppError> {
        let key = username_key(username);
        match self.cache.get::<String>(&key) {
            Ok(Some(id_str)) => {
                if let Ok(id) = Uuid::parse_str(&id_str) {
                    return Ok(Some(id));
                }
            }
            Ok(None) => {}
            Err(e) => warn!("Cache read failed for {}: {}", key, e),
        }

        let user_id = self.file_store.find_user_id_by_username(username)?;
        if let Some(ref id) = user_id {
            self.cache.set_or_warn(&key, &id.to_string(), self.cache_ttl);
        }
        Ok(user_id)
    }

    pub fn get_user_id_by_email(&self, email: &str) -> Result<Option<Uuid>, AppError> {
        let key = email_key(email);
        match self.cache.get::<String>(&key) {
            Ok(Some(id_str)) => {
                if let Ok(id) = Uuid::parse_str(&id_str) {
                    return Ok(Some(id));
                }
            }
            Ok(None) => {}
            Err(e) => warn!("Cache read failed for {}: {}", key, e),
        }

        let user_id = self.file_store.find_user_id_by_email(email)?;
        if let Some(ref id) = user_id {
            self.cache.set_or_warn(&key, &id.to_string(), self.cache_ttl);
        }
        Ok(user_id)
    }

    pub fn update_profile(
        &self,
        user_id: &Uuid,
        username: Option<String>,
        email: Option<String>,
    ) -> Result<UserProfile, AppError> {
        let mut profile = self
            .get_profile(user_id)?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        let old_username = profile.username.clone();
        let old_email = profile.email.clone();

        if let Some(ref new_username) = username {
            if *new_username != old_username {
                if let Some(existing_id) = self.get_user_id_by_username(new_username)? {
                    if existing_id != *user_id {
                        return Err(AppError::Conflict(format!(
                            "Username '{}' is already taken",
                            new_username
                        )));
                    }
                }
                profile.username = new_username.clone();
            }
        }

        if let Some(ref new_email) = email {
            if *new_email != old_email {
                if let Some(existing_id) = self.get_user_id_by_email(new_email)? {
                    if existing_id != *user_id {
                        return Err(AppError::Conflict(format!(
                            "Email '{}' is already registered",
                            new_email
                        )));
                    }
                }
                profile.email = new_email.clone();
            }
        }

        profile.updated_at = Utc::now();
        self.file_store.write_profile(&profile)?;

        self.cache.set_or_warn(&user_key(user_id), &profile, self.cache_ttl);

        if profile.username != old_username {
            self.cache.delete_or_warn(&username_key(&old_username));
            self.cache.set_or_warn(&username_key(&profile.username), &user_id.to_string(), self.cache_ttl);
        }
        if profile.email != old_email {
            self.cache.delete_or_warn(&email_key(&old_email));
            self.cache.set_or_warn(&email_key(&profile.email), &user_id.to_string(), self.cache_ttl);
        }

        Ok(profile)
    }

    pub fn change_password(
        &self,
        user_id: &Uuid,
        new_password: String,
    ) -> Result<(), AppError> {
        let password_hash = bcrypt::hash(&new_password, BCRYPT_COST)
            .map_err(|e| AppError::Internal(format!("Failed to hash password: {}", e)))?;

        let auth = UserAuth {
            user_id: *user_id,
            password_hash,
            updated_at: Utc::now(),
        };
        self.file_store.write_auth(&auth)
    }

    pub fn deactivate_user(&self, user_id: &Uuid) -> Result<(), AppError> {
        let mut profile = self
            .get_profile(user_id)?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        profile.is_active = false;
        profile.updated_at = Utc::now();
        self.file_store.write_profile(&profile)?;

        self.cache.delete_or_warn(&user_key(user_id));
        self.cache.delete_or_warn(&username_key(&profile.username));
        self.cache.delete_or_warn(&email_key(&profile.email));

        Ok(())
    }

    pub fn add_owned_account(
        &self,
        user_id: &Uuid,
        workspace_id: Uuid,
    ) -> Result<(), AppError> {
        let mut profile = self
            .get_profile(user_id)?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        if !profile.owned_accounts.contains(&workspace_id) {
            profile.owned_accounts.push(workspace_id);
            profile.updated_at = Utc::now();
            self.file_store.write_profile(&profile)?;
            self.file_store.update_master_ledger(&profile)?;
            self.cache.set_or_warn(&user_key(user_id), &profile, self.cache_ttl);
        }

        Ok(())
    }

    pub fn add_shared_account(
        &self,
        user_id: &Uuid,
        workspace_id: Uuid,
    ) -> Result<(), AppError> {
        let mut profile = self
            .get_profile(user_id)?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        if !profile.shared_accounts.contains(&workspace_id) {
            profile.shared_accounts.push(workspace_id);
            profile.updated_at = Utc::now();
            self.file_store.write_profile(&profile)?;
            self.file_store.update_master_ledger(&profile)?;
            self.cache.set_or_warn(&user_key(user_id), &profile, self.cache_ttl);
        }

        Ok(())
    }

    pub fn remove_shared_account(
        &self,
        user_id: &Uuid,
        workspace_id: &Uuid,
    ) -> Result<(), AppError> {
        let mut profile = self
            .get_profile(user_id)?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        if let Some(pos) = profile.shared_accounts.iter().position(|id| id == workspace_id) {
            profile.shared_accounts.remove(pos);
            profile.updated_at = Utc::now();
            self.file_store.write_profile(&profile)?;
            self.file_store.update_master_ledger(&profile)?;
            self.cache.set_or_warn(&user_key(user_id), &profile, self.cache_ttl);
        }

        Ok(())
    }

    pub fn list_all_users(&self) -> Result<Vec<UserProfile>, AppError> {
        self.file_store.read_all_profiles()
    }

    pub fn set_user_active(&self, user_id: &Uuid, active: bool) -> Result<UserProfile, AppError> {
        let mut profile = self
            .get_profile(user_id)?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        profile.is_active = active;
        profile.updated_at = Utc::now();
        self.file_store.write_profile(&profile)?;

        self.cache.delete_or_warn(&user_key(user_id));

        Ok(profile)
    }

    pub fn set_user_role(&self, user_id: &Uuid, is_admin: bool) -> Result<UserProfile, AppError> {
        let mut profile = self
            .get_profile(user_id)?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        profile.is_admin = is_admin;
        profile.updated_at = Utc::now();
        self.file_store.write_profile(&profile)?;

        self.cache.delete_or_warn(&user_key(user_id));

        Ok(profile)
    }
}
