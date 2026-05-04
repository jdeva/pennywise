use crate::models::{RotationPeriod, Workspace, WorkspacePublic, Permission, SharedUser};
use crate::services::cache::Cache;
use crate::services::file_store::FileStore;
use crate::services::user::UserService;
use crate::utils::error::AppError;
use chrono::Utc;
use log::warn;
use uuid::Uuid;

fn cache_key(workspace_id: &Uuid) -> String {
    format!("workspace:{}", workspace_id)
}

#[derive(Clone)]
pub struct WorkspaceService {
    file_store: FileStore,
    cache: Cache,
    user_service: UserService,
    cache_ttl: usize,
}

impl WorkspaceService {
    pub fn new(
        file_store: FileStore,
        cache: Cache,
        user_service: UserService,
        cache_ttl: usize,
    ) -> Self {
        Self {
            file_store,
            cache,
            user_service,
            cache_ttl,
        }
    }

    /// Converts a Workspace to WorkspacePublic with usernames resolved for shared users.
    /// Silently uses empty string if a user can't be loaded (shouldn't happen in practice).
    pub fn to_public(&self, workspace: Workspace) -> WorkspacePublic {
        let mut public = WorkspacePublic::from(workspace);
        for entry in public.shared_with.iter_mut() {
            if let Ok(Some(profile)) = self.user_service.get_profile(&entry.user_id) {
                entry.username = profile.username;
            }
        }
        public
    }

    pub fn create_workspace(
        &self,
        owner_id: &Uuid,
        name: String,
        currency: Option<String>,
        seed_color: Option<String>,
    ) -> Result<Workspace, AppError> {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let workspace = Workspace {
            id,
            name,
            owner_id: *owner_id,
            currency: currency.unwrap_or_else(|| "USD".to_string()),
            shared_with: vec![],
            is_active: true,
            created_at: now,
            updated_at: now,
            ledger_dir: Some(format!("workspaces/workspace-{}/", id)),
            rotation_period: RotationPeriod::default(),
            budgeting_enabled: false,
            seed_color,
        };

        self.file_store.write_workspace(&workspace)?;
        self.file_store.create_workspace_dir(&workspace)?;
        self.file_store.create_workspace_ledger(&workspace)?;
        self.user_service.add_owned_account(owner_id, workspace.id)?;
        self.cache.set_or_warn(&cache_key(&workspace.id), &workspace, self.cache_ttl);

        Ok(workspace)
    }

    pub fn get_workspace(&self, workspace_id: &Uuid) -> Result<Option<Workspace>, AppError> {
        let key = cache_key(workspace_id);
        match self.cache.get::<Workspace>(&key) {
            Ok(Some(workspace)) => return Ok(Some(workspace)),
            Ok(None) => {}
            Err(e) => warn!("Cache read failed for {}: {}", key, e),
        }

        let workspace = self.file_store.read_workspace(workspace_id)?;
        if let Some(ref w) = workspace {
            self.cache.set_or_warn(&key, w, self.cache_ttl);
        }
        Ok(workspace)
    }

    pub fn get_workspace_authorized(
        &self,
        workspace_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<Workspace, AppError> {
        let workspace = self
            .get_workspace(workspace_id)?
            .ok_or_else(|| AppError::NotFound("Workspace not found".to_string()))?;

        if !workspace.has_access(user_id) {
            return Err(AppError::NotFound("Workspace not found".to_string()));
        }

        Ok(workspace)
    }

    pub fn list_workspaces(&self, user_id: &Uuid) -> Result<Vec<Workspace>, AppError> {
        let profile = self
            .user_service
            .get_profile(user_id)?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        let mut workspaces = Vec::new();
        let all_ids: Vec<Uuid> = profile
            .owned_accounts
            .iter()
            .chain(profile.shared_accounts.iter())
            .copied()
            .collect();

        for id in all_ids {
            match self.get_workspace(&id) {
                Ok(Some(workspace)) => workspaces.push(workspace),
                Ok(None) => {
                    warn!("Workspace {} referenced by user {} not found, skipping", id, user_id);
                }
                Err(e) => {
                    warn!("Error loading workspace {}: {}", id, e);
                }
            }
        }

        Ok(workspaces)
    }

    pub fn update_workspace(
        &self,
        workspace_id: &Uuid,
        user_id: &Uuid,
        name: String,
        seed_color: Option<Option<String>>,
    ) -> Result<Workspace, AppError> {
        let mut workspace = self
            .get_workspace(workspace_id)?
            .ok_or_else(|| AppError::NotFound("Workspace not found".to_string()))?;

        if !workspace.has_write_access(user_id) {
            return Err(AppError::Forbidden("You don't have write access to this workspace".to_string()));
        }

        workspace.name = name;
        if let Some(seed) = seed_color {
            workspace.seed_color = seed;
        }
        workspace.updated_at = Utc::now();
        self.file_store.write_workspace(&workspace)?;
        self.cache.set_or_warn(&cache_key(workspace_id), &workspace, self.cache_ttl);

        Ok(workspace)
    }

    pub fn deactivate_workspace(
        &self,
        workspace_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<(), AppError> {
        let mut workspace = self
            .get_workspace(workspace_id)?
            .ok_or_else(|| AppError::NotFound("Workspace not found".to_string()))?;

        if !workspace.is_active {
            return Err(AppError::NotFound("Workspace not found".to_string()));
        }

        if workspace.owner_id != *user_id {
            return Err(AppError::Forbidden("Only the workspace owner can deactivate this workspace".to_string()));
        }

        workspace.is_active = false;
        workspace.updated_at = Utc::now();
        self.file_store.write_workspace(&workspace)?;
        self.cache.set_or_warn(&cache_key(workspace_id), &workspace, self.cache_ttl);

        Ok(())
    }

    pub fn share_workspace(
        &self,
        workspace_id: &Uuid,
        owner_id: &Uuid,
        target_username: &str,
        permission: Permission,
    ) -> Result<Workspace, AppError> {
        let mut workspace = self
            .get_workspace(workspace_id)?
            .ok_or_else(|| AppError::NotFound("Workspace not found".to_string()))?;

        if workspace.owner_id != *owner_id {
            return Err(AppError::Forbidden("Only the workspace owner can share this workspace".to_string()));
        }

        let target_user_id = self
            .user_service
            .get_user_id_by_username(target_username)?
            .ok_or_else(|| AppError::NotFound(format!("User '{}' not found", target_username)))?;

        if target_user_id == *owner_id {
            return Err(AppError::BadRequest("Cannot share a workspace with yourself".to_string()));
        }

        if workspace.shared_with.iter().any(|s| s.user_id == target_user_id) {
            return Err(AppError::Conflict(format!(
                "Workspace is already shared with user '{}'",
                target_username
            )));
        }

        workspace.shared_with.push(SharedUser {
            user_id: target_user_id,
            permission,
        });
        workspace.updated_at = Utc::now();
        self.file_store.write_workspace(&workspace)?;
        self.user_service.add_shared_account(&target_user_id, workspace.id)?;
        self.cache.set_or_warn(&cache_key(workspace_id), &workspace, self.cache_ttl);

        Ok(workspace)
    }

    pub fn unshare_workspace(
        &self,
        workspace_id: &Uuid,
        owner_id: &Uuid,
        target_user_id: &Uuid,
    ) -> Result<Workspace, AppError> {
        let mut workspace = self
            .get_workspace(workspace_id)?
            .ok_or_else(|| AppError::NotFound("Workspace not found".to_string()))?;

        if workspace.owner_id != *owner_id {
            return Err(AppError::Forbidden("Only the workspace owner can unshare this workspace".to_string()));
        }

        let pos = workspace
            .shared_with
            .iter()
            .position(|s| s.user_id == *target_user_id)
            .ok_or_else(|| AppError::NotFound("User does not have access to this workspace".to_string()))?;

        workspace.shared_with.remove(pos);
        workspace.updated_at = Utc::now();
        self.file_store.write_workspace(&workspace)?;
        self.user_service.remove_shared_account(target_user_id, workspace_id)?;
        self.cache.set_or_warn(&cache_key(workspace_id), &workspace, self.cache_ttl);

        Ok(workspace)
    }

    pub fn set_budgeting_enabled(
        &self,
        workspace_id: &Uuid,
        user_id: &Uuid,
        enabled: bool,
    ) -> Result<Workspace, AppError> {
        let mut workspace = self.get_workspace_authorized(workspace_id, user_id)?;

        if !workspace.has_write_access(user_id) {
            return Err(AppError::Forbidden(
                "You don't have write access to this workspace".to_string(),
            ));
        }

        workspace.budgeting_enabled = enabled;
        workspace.updated_at = Utc::now();
        self.file_store.write_workspace(&workspace)?;
        self.cache.set_or_warn(&cache_key(workspace_id), &workspace, self.cache_ttl);

        Ok(workspace)
    }

    pub fn get_budgeting_status(
        &self,
        workspace_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<bool, AppError> {
        let workspace = self.get_workspace_authorized(workspace_id, user_id)?;
        Ok(workspace.budgeting_enabled)
    }


}
