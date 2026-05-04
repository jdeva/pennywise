use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde_json;
use uuid::Uuid;

use crate::models::{ChartOfAccounts, RotationPeriod, Workspace, UserAuth, UserCategories, UserProfile};
use crate::utils::AppError;

#[derive(Clone)]
pub struct FileStore {
    data_path: String,
}

impl FileStore {
    pub fn new(data_path: String) -> Self {
        let users_path = Path::new(&data_path).join("users");
        if let Err(e) = fs::create_dir_all(&users_path) {
            log::warn!("Failed to create users directory: {}", e);
        }
        let accounts_path = Path::new(&data_path).join("accounts");
        if let Err(e) = fs::create_dir_all(&accounts_path) {
            log::warn!("Failed to create accounts directory: {}", e);
        }
        let workspaces_path = Path::new(&data_path).join("workspaces");
        if let Err(e) = fs::create_dir_all(&workspaces_path) {
            log::warn!("Failed to create workspaces directory: {}", e);
        }
        Self { data_path }
    }

    pub fn data_path(&self) -> &str {
        &self.data_path
    }

    fn atomic_write(
        &self,
        target_path: &Path,
        data: &[u8],
        permissions: Option<u32>,
    ) -> Result<(), AppError> {
        let parent = target_path
            .parent()
            .ok_or_else(|| AppError::Internal("Target path has no parent directory".into()))?;

        let tmp_name = format!(
            "{}.tmp.{}",
            target_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file"),
            Uuid::new_v4()
        );
        let tmp_path = parent.join(&tmp_name);

        let mut file = fs::File::create(&tmp_path).map_err(|e| {
            AppError::Internal(format!("Failed to create temp file {:?}: {}", tmp_path, e))
        })?;

        file.write_all(data).map_err(|e| {
            let _ = fs::remove_file(&tmp_path);
            AppError::Internal(format!("Failed to write temp file {:?}: {}", tmp_path, e))
        })?;

        file.flush().map_err(|e| {
            let _ = fs::remove_file(&tmp_path);
            AppError::Internal(format!("Failed to flush temp file {:?}: {}", tmp_path, e))
        })?;
        drop(file);

        #[cfg(unix)]
        if let Some(mode) = permissions {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(mode);
            fs::set_permissions(&tmp_path, perms).map_err(|e| {
                let _ = fs::remove_file(&tmp_path);
                AppError::Internal(format!(
                    "Failed to set permissions on {:?}: {}",
                    tmp_path, e
                ))
            })?;
        }

        fs::rename(&tmp_path, target_path).map_err(|e| {
            let _ = fs::remove_file(&tmp_path);
            AppError::Internal(format!(
                "Failed to rename {:?} to {:?}: {}",
                tmp_path, target_path, e
            ))
        })?;

        Ok(())
    }

    pub fn write_profile(&self, profile: &UserProfile) -> Result<(), AppError> {
        let target = Path::new(&self.data_path)
            .join("users")
            .join(format!("user-{}.json", profile.id));

        let data = serde_json::to_string_pretty(profile)
            .map_err(|e| AppError::Internal(format!("Failed to serialize profile: {}", e)))?;

        self.atomic_write(&target, data.as_bytes(), None)
    }

    pub fn write_auth(&self, auth: &UserAuth) -> Result<(), AppError> {
        let target = Path::new(&self.data_path)
            .join("users")
            .join(format!("user-{}-auth.json", auth.user_id));

        let data = serde_json::to_string_pretty(auth)
            .map_err(|e| AppError::Internal(format!("Failed to serialize auth data: {}", e)))?;

        self.atomic_write(&target, data.as_bytes(), Some(0o600))
    }

    pub fn read_profile(&self, user_id: &Uuid) -> Result<Option<UserProfile>, AppError> {
        let path = Path::new(&self.data_path)
            .join("users")
            .join(format!("user-{}.json", user_id));

        match fs::read_to_string(&path) {
            Ok(contents) => {
                let profile: UserProfile = serde_json::from_str(&contents).map_err(|e| {
                    AppError::Internal(format!("Malformed profile file {:?}: {}", path, e))
                })?;
                Ok(Some(profile))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(AppError::Internal(format!(
                "Failed to read profile {:?}: {}",
                path, e
            ))),
        }
    }

    pub fn read_auth(&self, user_id: &Uuid) -> Result<Option<UserAuth>, AppError> {
        let path = Path::new(&self.data_path)
            .join("users")
            .join(format!("user-{}-auth.json", user_id));

        match fs::read_to_string(&path) {
            Ok(contents) => {
                let auth: UserAuth = serde_json::from_str(&contents).map_err(|e| {
                    AppError::Internal(format!("Malformed auth file {:?}: {}", path, e))
                })?;
                Ok(Some(auth))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(AppError::Internal(format!(
                "Failed to read auth file {:?}: {}",
                path, e
            ))),
        }
    }

    pub fn create_master_ledger(&self, user: &UserProfile) -> Result<(), AppError> {
        let target = Path::new(&self.data_path)
            .join(&user.master_ledger);

        let mut content = String::new();
        content.push_str(&format!("; Master ledger for user: {}\n", user.username));
        content.push_str(&format!("; User ID: {}\n", user.id));
        content.push_str(&format!("; Created: {}\n\n", Utc::now().format("%Y-%m-%dT%H:%M:%SZ")));

        for workspace_id in &user.owned_accounts {
            content.push_str(&format!(
                "!include ../workspaces/workspace-{0}/workspace-{0}.ledger\n",
                workspace_id
            ));
        }

        for workspace_id in &user.shared_accounts {
            content.push_str(&format!(
                "!include ../workspaces/workspace-{0}/workspace-{0}.ledger  ; shared\n",
                workspace_id
            ));
        }

        self.atomic_write(&target, content.as_bytes(), None)
    }

    pub fn update_master_ledger(&self, user: &UserProfile) -> Result<(), AppError> {
        let target = Path::new(&self.data_path)
            .join(&user.master_ledger);

        let mut content = String::new();
        content.push_str(&format!("; Master ledger for user: {}\n", user.username));
        content.push_str(&format!("; User ID: {}\n", user.id));
        content.push_str(&format!("; Updated: {}\n\n", Utc::now().format("%Y-%m-%dT%H:%M:%SZ")));

        for workspace_id in &user.owned_accounts {
            content.push_str(&format!(
                "!include ../workspaces/workspace-{0}/workspace-{0}.ledger\n",
                workspace_id
            ));
        }

        for workspace_id in &user.shared_accounts {
            content.push_str(&format!(
                "!include ../workspaces/workspace-{0}/workspace-{0}.ledger  ; shared\n",
                workspace_id
            ));
        }

        self.atomic_write(&target, content.as_bytes(), None)
    }

    pub fn find_user_id_by_username(&self, username: &str) -> Result<Option<Uuid>, AppError> {
        self.scan_profiles(|profile| profile.username == username)
    }

    pub fn find_user_id_by_email(&self, email: &str) -> Result<Option<Uuid>, AppError> {
        self.scan_profiles(|profile| profile.email == email)
    }

    pub fn write_account(&self, account: &Workspace) -> Result<(), AppError> {
        let target = Path::new(&self.data_path)
            .join("accounts")
            .join(format!("account-{}.json", account.id));

        let data = serde_json::to_string_pretty(account)
            .map_err(|e| AppError::Internal(format!("Failed to serialize account: {}", e)))?;

        self.atomic_write(&target, data.as_bytes(), None)
    }

    pub fn read_account(&self, account_id: &Uuid) -> Result<Option<Workspace>, AppError> {
        let path = Path::new(&self.data_path)
            .join("accounts")
            .join(format!("account-{}.json", account_id));

        match fs::read_to_string(&path) {
            Ok(contents) => {
                let account: Workspace = serde_json::from_str(&contents).map_err(|e| {
                    AppError::Internal(format!("Malformed account file {:?}: {}", path, e))
                })?;
                Ok(Some(account))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(AppError::Internal(format!(
                "Failed to read account {:?}: {}",
                path, e
            ))),
        }
    }

    /// Write workspace metadata to workspaces/workspace-{uuid}.json
    pub fn write_workspace(&self, workspace: &Workspace) -> Result<(), AppError> {
        let target = Path::new(&self.data_path)
            .join("workspaces")
            .join(format!("workspace-{}.json", workspace.id));

        let data = serde_json::to_string_pretty(workspace)
            .map_err(|e| AppError::Internal(format!("Failed to serialize workspace: {}", e)))?;

        self.atomic_write(&target, data.as_bytes(), None)
    }

    /// Read workspace metadata from workspaces/workspace-{uuid}.json,
    /// falling back to accounts/account-{uuid}.json for legacy data
    pub fn read_workspace(&self, workspace_id: &Uuid) -> Result<Option<Workspace>, AppError> {
        // Try new workspaces/ path first
        let ws_path = Path::new(&self.data_path)
            .join("workspaces")
            .join(format!("workspace-{}.json", workspace_id));

        match fs::read_to_string(&ws_path) {
            Ok(contents) => {
                let workspace: Workspace = serde_json::from_str(&contents).map_err(|e| {
                    AppError::Internal(format!("Malformed workspace file {:?}: {}", ws_path, e))
                })?;
                return Ok(Some(workspace));
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Fall through to legacy path
            }
            Err(e) => {
                return Err(AppError::Internal(format!(
                    "Failed to read workspace {:?}: {}",
                    ws_path, e
                )));
            }
        }

        // Fall back to legacy accounts/ path
        self.read_account(workspace_id)
    }

    /// Create workspace directory at workspaces/workspace-{uuid}/
    pub fn create_workspace_dir(&self, workspace: &Workspace) -> Result<(), AppError> {
        let dir_path = Path::new(&self.data_path)
            .join("workspaces")
            .join(format!("workspace-{}", workspace.id));

        fs::create_dir_all(&dir_path).map_err(|e| {
            AppError::Internal(format!(
                "Failed to create workspace directory {:?}: {}",
                dir_path, e
            ))
        })
    }

    /// Create workspace ledger file with header and initial period include
    pub fn create_workspace_ledger(&self, workspace: &Workspace) -> Result<(), AppError> {
        let target = Path::new(&self.data_path)
            .join("workspaces")
            .join(format!("workspace-{}", workspace.id))
            .join(format!("workspace-{}.ledger", workspace.id));

        let mut content = String::new();
        content.push_str(&format!("; Workspace: {}\n", workspace.name));
        content.push_str(&format!("; Workspace ID: {}\n", workspace.id));
        content.push_str(&format!("; Created: {}\n", workspace.created_at.format("%Y-%m-%dT%H:%M:%SZ")));

        // Add initial period include for the current date
        let now = Utc::now().date_naive();
        let period_label = workspace.rotation_period.period_label(&now);
        let period_filename = workspace.rotation_period.period_filename(&workspace.id, &now);

        // Create the initial period file
        self.create_period_file(workspace, &period_label)?;

        content.push_str(&format!("\n!include {}\n", period_filename));

        self.atomic_write(&target, content.as_bytes(), None)
    }

    /// Create a period file with header comment
    pub fn create_period_file(
        &self,
        workspace: &Workspace,
        period_label: &str,
    ) -> Result<(), AppError> {
        let target = Path::new(&self.data_path)
            .join("workspaces")
            .join(format!("workspace-{}", workspace.id))
            .join(format!("workspace-{}-{}.ledger", workspace.id, period_label));

        let content = format!(
            "; Period: {}\n; Workspace ID: {}\n",
            period_label, workspace.id
        );

        self.atomic_write(&target, content.as_bytes(), None)
    }

    /// Append an include directive to the workspace ledger, maintaining chronological order
    pub fn add_include_to_workspace_ledger(
        &self,
        workspace: &Workspace,
        period_filename: &str,
    ) -> Result<(), AppError> {
        let ledger_path = Path::new(&self.data_path)
            .join("workspaces")
            .join(format!("workspace-{}", workspace.id))
            .join(format!("workspace-{}.ledger", workspace.id));

        let contents = fs::read_to_string(&ledger_path).map_err(|e| {
            AppError::Internal(format!(
                "Failed to read workspace ledger {:?}: {}",
                ledger_path, e
            ))
        })?;

        let new_include = format!("!include {}", period_filename);

        // Check if already present
        if contents.lines().any(|line| line.trim() == new_include) {
            return Ok(());
        }

        // Separate header lines from include lines
        let mut header_lines: Vec<&str> = Vec::new();
        let mut include_lines: Vec<String> = Vec::new();

        for line in contents.lines() {
            if line.starts_with("!include ") {
                include_lines.push(line.to_string());
            } else {
                // Only add to header if we haven't seen any includes yet
                if include_lines.is_empty() {
                    header_lines.push(line);
                }
            }
        }

        // Add the new include and sort chronologically (lexicographic sort works
        // because period labels like 2026-Q1, 2026-Q2, 2026-H1 sort correctly)
        include_lines.push(new_include);
        include_lines.sort();

        // Rebuild the file
        let mut new_content = String::new();
        for line in &header_lines {
            new_content.push_str(line);
            new_content.push('\n');
        }
        for line in &include_lines {
            new_content.push_str(line);
            new_content.push('\n');
        }

        self.atomic_write(&ledger_path, new_content.as_bytes(), None)
    }

    /// Check if the workspace ledger already has an include for the given period filename
    pub fn workspace_ledger_has_include(
        &self,
        workspace: &Workspace,
        period_filename: &str,
    ) -> Result<bool, AppError> {
        let ledger_path = Path::new(&self.data_path)
            .join("workspaces")
            .join(format!("workspace-{}", workspace.id))
            .join(format!("workspace-{}.ledger", workspace.id));

        let contents = fs::read_to_string(&ledger_path).map_err(|e| {
            AppError::Internal(format!(
                "Failed to read workspace ledger {:?}: {}",
                ledger_path, e
            ))
        })?;

        let target = format!("!include {}", period_filename);
        Ok(contents.lines().any(|line| line.trim() == target))
    }

    /// Append a transaction entry to the correct period file, creating it and
    /// updating the workspace ledger include if needed
    pub fn append_to_period_file(
        &self,
        workspace: &Workspace,
        period_label: &str,
        entry: &str,
    ) -> Result<(), AppError> {
        let period_filename = format!("workspace-{}-{}.ledger", workspace.id, period_label);
        let period_path = Path::new(&self.data_path)
            .join("workspaces")
            .join(format!("workspace-{}", workspace.id))
            .join(&period_filename);

        // If the period file doesn't exist, create it and add include to workspace ledger
        if !period_path.exists() {
            self.create_period_file(workspace, period_label)?;
            self.add_include_to_workspace_ledger(workspace, &period_filename)?;
        }

        // Append the entry to the period file
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&period_path)
            .map_err(|e| {
                AppError::Internal(format!(
                    "Failed to open period file {:?}: {}",
                    period_path, e
                ))
            })?;

        file.write_all(format!("\n{}", entry).as_bytes())
            .map_err(|e| {
                AppError::Internal(format!(
                    "Failed to append to period file {:?}: {}",
                    period_path, e
                ))
            })?;

        file.flush().map_err(|e| {
            AppError::Internal(format!(
                "Failed to flush period file {:?}: {}",
                period_path, e
            ))
        })?;

        Ok(())
    }

    pub fn create_account_ledger(&self, account: &Workspace) -> Result<(), AppError> {
        let target = Path::new(&self.data_path)
            .join("accounts")
            .join(format!("account-{}.ledger", account.id));

        let content = format!(
            "; Account: {}\n; Account ID: {}\n; Created: {}\n",
            account.name, account.id, Utc::now()
        );

        self.atomic_write(&target, content.as_bytes(), None)
    }

    pub fn write_categories(&self, categories: &UserCategories) -> Result<(), AppError> {
        let target = Path::new(&self.data_path)
            .join("users")
            .join(format!("user-{}-categories.json", categories.user_id));

        let data = serde_json::to_string_pretty(categories)
            .map_err(|e| AppError::Internal(format!("Failed to serialize categories: {}", e)))?;

        self.atomic_write(&target, data.as_bytes(), None)
    }

    pub fn read_categories(&self, user_id: &Uuid) -> Result<Option<UserCategories>, AppError> {
        let path = Path::new(&self.data_path)
            .join("users")
            .join(format!("user-{}-categories.json", user_id));

        match fs::read_to_string(&path) {
            Ok(contents) => {
                let categories: UserCategories = serde_json::from_str(&contents).map_err(|e| {
                    AppError::Internal(format!("Malformed categories file {:?}: {}", path, e))
                })?;
                Ok(Some(categories))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(AppError::Internal(format!(
                "Failed to read categories {:?}: {}",
                path, e
            ))),
        }
    }

    pub fn write_chart_of_accounts(&self, chart: &ChartOfAccounts) -> Result<(), AppError> {
        let target = Path::new(&self.data_path)
            .join("users")
            .join(format!("user-{}-chart-of-accounts.json", chart.user_id));

        let data = serde_json::to_string_pretty(chart)
            .map_err(|e| AppError::Internal(format!("Failed to serialize chart of accounts: {}", e)))?;

        self.atomic_write(&target, data.as_bytes(), None)
    }

    pub fn read_chart_of_accounts(&self, user_id: &Uuid) -> Result<Option<ChartOfAccounts>, AppError> {
        let path = Path::new(&self.data_path)
            .join("users")
            .join(format!("user-{}-chart-of-accounts.json", user_id));

        match fs::read_to_string(&path) {
            Ok(contents) => {
                let chart: ChartOfAccounts = serde_json::from_str(&contents).map_err(|e| {
                    AppError::Internal(format!("Malformed chart of accounts file {:?}: {}", path, e))
                })?;
                Ok(Some(chart))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(AppError::Internal(format!(
                "Failed to read chart of accounts {:?}: {}",
                path, e
            ))),
        }
    }

    pub fn delete_categories_file(&self, user_id: &Uuid) -> Result<(), AppError> {
        let path = Path::new(&self.data_path)
            .join("users")
            .join(format!("user-{}-categories.json", user_id));

        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(AppError::Internal(format!(
                "Failed to delete categories file {:?}: {}",
                path, e
            ))),
        }
    }

    /// Migrate a legacy workspace from single-file layout to the new directory hierarchy.
    ///
    /// 1. Creates `workspaces/workspace-{uuid}/` directory
    /// 2. Reads existing content from `accounts/account-{uuid}.ledger` (if any)
    /// 3. Computes the initial period label from the workspace's rotation period and current date
    /// 4. Writes the content into an initial period file
    /// 5. Creates workspace ledger with header and include directive for the period file
    /// 6. Updates workspace metadata with `ledger_dir` and persists via `write_workspace`
    pub fn migrate_workspace_ledger(
        &self,
        workspace: &mut Workspace,
    ) -> Result<(), AppError> {
        // 1. Create workspace directory
        self.create_workspace_dir(workspace)?;

        // 2. Read existing legacy ledger content (if any)
        let legacy_path = Path::new(&self.data_path)
            .join("accounts")
            .join(format!("account-{}.ledger", workspace.id));

        let legacy_content = match fs::read_to_string(&legacy_path) {
            Ok(content) => content,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => {
                return Err(AppError::Internal(format!(
                    "Failed to read legacy ledger {:?}: {}",
                    legacy_path, e
                )));
            }
        };

        // 3. Determine the initial period label
        let now = Utc::now().date_naive();
        let period_label = workspace.rotation_period.period_label(&now);
        let period_filename = workspace.rotation_period.period_filename(&workspace.id, &now);

        // 4. Write the existing content into the initial period file
        let period_path = Path::new(&self.data_path)
            .join("workspaces")
            .join(format!("workspace-{}", workspace.id))
            .join(&period_filename);

        let period_content = if legacy_content.trim().is_empty() {
            format!(
                "; Period: {}\n; Workspace ID: {}\n",
                period_label, workspace.id
            )
        } else {
            format!(
                "; Period: {}\n; Workspace ID: {}\n\n{}",
                period_label, workspace.id, legacy_content
            )
        };

        self.atomic_write(&period_path, period_content.as_bytes(), None)?;

        // 5. Create workspace ledger with header and include directive
        let ledger_path = Path::new(&self.data_path)
            .join("workspaces")
            .join(format!("workspace-{}", workspace.id))
            .join(format!("workspace-{}.ledger", workspace.id));

        let mut ledger_content = String::new();
        ledger_content.push_str(&format!("; Workspace: {}\n", workspace.name));
        ledger_content.push_str(&format!("; Workspace ID: {}\n", workspace.id));
        ledger_content.push_str(&format!(
            "; Created: {}\n",
            workspace.created_at.format("%Y-%m-%dT%H:%M:%SZ")
        ));
        ledger_content.push_str(&format!("\n!include {}\n", period_filename));

        self.atomic_write(&ledger_path, ledger_content.as_bytes(), None)?;

        // 6. Update workspace metadata with ledger_dir and persist
        workspace.ledger_dir = Some(format!("workspaces/workspace-{}/", workspace.id));
        self.write_workspace(workspace)?;

        Ok(())
    }


    pub fn append_to_ledger(
        &self,
        account_id: &Uuid,
        entry: &str,
    ) -> Result<(), AppError> {
        let path = Path::new(&self.data_path)
            .join("accounts")
            .join(format!("account-{}.ledger", account_id));

        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)
            .map_err(|e| {
                AppError::Internal(format!("Failed to open ledger file {:?}: {}", path, e))
            })?;

        file.write_all(format!("\n{}", entry).as_bytes())
            .map_err(|e| {
                AppError::Internal(format!("Failed to append to ledger {:?}: {}", path, e))
            })?;

        file.flush().map_err(|e| {
            AppError::Internal(format!("Failed to flush ledger {:?}: {}", path, e))
        })?;

        Ok(())
    }

    pub fn get_ledger_path(&self, account_id: &Uuid) -> PathBuf {
        Path::new(&self.data_path)
            .join("accounts")
            .join(format!("account-{}.ledger", account_id))
    }

    /// List all period file paths for a migrated workspace, excluding the
    /// workspace-level include file. Returns the paths sorted by file name so
    /// iteration order matches the period labels (lexicographic sort works
    /// because labels like `2026-Q1`, `2026-Q2` collate correctly).
    pub fn list_period_files(&self, workspace: &Workspace) -> Result<Vec<PathBuf>, AppError> {
        let mut out = Vec::new();
        if workspace.ledger_dir.is_some() {
            let ws_dir = Path::new(&self.data_path)
                .join("workspaces")
                .join(format!("workspace-{}", workspace.id));
            let root = format!("workspace-{}.ledger", workspace.id);
            match fs::read_dir(&ws_dir) {
                Ok(entries) => {
                    for entry in entries {
                        let entry = entry.map_err(|e| {
                            AppError::Internal(format!(
                                "Failed to read workspace dir {:?}: {}",
                                ws_dir, e
                            ))
                        })?;
                        let name = entry.file_name();
                        let name_str = name.to_string_lossy().to_string();
                        if name_str == root {
                            continue;
                        }
                        if name_str.ends_with(".ledger") {
                            out.push(entry.path());
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => {
                    return Err(AppError::Internal(format!(
                        "Failed to read workspace dir {:?}: {}",
                        ws_dir, e
                    )));
                }
            }
        } else {
            let legacy = self.get_ledger_path(&workspace.id);
            if legacy.exists() {
                out.push(legacy);
            }
        }
        out.sort();
        Ok(out)
    }

    /// Read a ledger file's contents. Returns `Ok(None)` if the file doesn't exist.
    pub fn read_ledger_file(&self, path: &Path) -> Result<Option<String>, AppError> {
        match fs::read_to_string(path) {
            Ok(s) => Ok(Some(s)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(AppError::Internal(format!(
                "Failed to read ledger file {:?}: {}",
                path, e
            ))),
        }
    }

    /// Atomically overwrite a ledger file with new contents.
    pub fn write_ledger_file(&self, path: &Path, contents: &str) -> Result<(), AppError> {
        self.atomic_write(path, contents.as_bytes(), None)
    }

    /// Path to the period file for a given workspace and period label.
    pub fn period_file_path(&self, workspace: &Workspace, period_label: &str) -> PathBuf {
        Path::new(&self.data_path)
            .join("workspaces")
            .join(format!("workspace-{}", workspace.id))
            .join(format!("workspace-{}-{}.ledger", workspace.id, period_label))
    }

    /// Returns the ledger path for a workspace — workspace ledger for migrated, legacy path for unmigrated
    pub fn get_workspace_ledger_path(&self, workspace: &Workspace) -> PathBuf {
        if workspace.ledger_dir.is_some() {
            Path::new(&self.data_path)
                .join("workspaces")
                .join(format!("workspace-{}", workspace.id))
                .join(format!("workspace-{}.ledger", workspace.id))
        } else {
            Path::new(&self.data_path)
                .join("accounts")
                .join(format!("account-{}.ledger", workspace.id))
        }
    }

    /// Returns the path to the budget file for a workspace
    pub fn get_budget_file_path(&self, workspace: &Workspace) -> PathBuf {
        Path::new(&self.data_path)
            .join("workspaces")
            .join(format!("workspace-{}", workspace.id))
            .join(format!("workspace-{}-budget.ledger", workspace.id))
    }

    /// Read the budget file for a workspace, returning None if it doesn't exist
    pub fn read_budget_file(&self, workspace: &Workspace) -> Result<Option<String>, AppError> {
        let path = self.get_budget_file_path(workspace);

        match fs::read_to_string(&path) {
            Ok(contents) => Ok(Some(contents)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(AppError::Internal(format!(
                "Failed to read budget file {:?}: {}",
                path, e
            ))),
        }
    }

    /// Write the budget file for a workspace using atomic write (temp file + rename)
    pub fn write_budget_file(&self, workspace: &Workspace, content: &str) -> Result<(), AppError> {
        let target = self.get_budget_file_path(workspace);
        self.atomic_write(&target, content.as_bytes(), None)
    }

    /// Path to the recurring-transactions file for a workspace.
    pub fn get_recurring_file_path(&self, workspace: &Workspace) -> PathBuf {
        Path::new(&self.data_path)
            .join("workspaces")
            .join(format!("workspace-{}", workspace.id))
            .join(format!("workspace-{}-recurring.ledger", workspace.id))
    }

    pub fn read_recurring_file(&self, workspace: &Workspace) -> Result<Option<String>, AppError> {
        let path = self.get_recurring_file_path(workspace);
        match fs::read_to_string(&path) {
            Ok(contents) => Ok(Some(contents)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(AppError::Internal(format!(
                "Failed to read recurring file {:?}: {}",
                path, e
            ))),
        }
    }

    pub fn write_recurring_file(&self, workspace: &Workspace, content: &str) -> Result<(), AppError> {
        let target = self.get_recurring_file_path(workspace);
        self.atomic_write(&target, content.as_bytes(), None)
    }


    pub fn has_any_users(&self) -> Result<bool, AppError> {
        let users_dir = Path::new(&self.data_path).join("users");

        let entries = match fs::read_dir(&users_dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(e) => {
                return Err(AppError::Internal(format!(
                    "Failed to read users directory: {}",
                    e
                )))
            }
        };

        for entry in entries {
            let entry = entry.map_err(|e| {
                AppError::Internal(format!("Failed to read directory entry: {}", e))
            })?;

            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();

            if name.starts_with("user-") && name.ends_with(".json") && !name.ends_with("-auth.json") {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn read_all_profiles(&self) -> Result<Vec<UserProfile>, AppError> {
        let users_dir = Path::new(&self.data_path).join("users");

        let entries = match fs::read_dir(&users_dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => {
                return Err(AppError::Internal(format!(
                    "Failed to read users directory: {}",
                    e
                )))
            }
        };

        let mut profiles = Vec::new();

        for entry in entries {
            let entry = entry.map_err(|e| {
                AppError::Internal(format!("Failed to read directory entry: {}", e))
            })?;

            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();

            if !name.starts_with("user-") || !name.ends_with(".json") || name.ends_with("-auth.json") {
                continue;
            }

            let contents = fs::read_to_string(entry.path()).map_err(|e| {
                AppError::Internal(format!("Failed to read {:?}: {}", entry.path(), e))
            })?;

            match serde_json::from_str::<UserProfile>(&contents) {
                Ok(profile) => profiles.push(profile),
                Err(e) => {
                    log::warn!("Skipping malformed profile {:?}: {}", entry.path(), e);
                }
            }
        }

        Ok(profiles)
    }

    fn scan_profiles<F>(&self, predicate: F) -> Result<Option<Uuid>, AppError>
    where
        F: Fn(&UserProfile) -> bool,
    {
        let users_dir = Path::new(&self.data_path).join("users");

        let entries = match fs::read_dir(&users_dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => {
                return Err(AppError::Internal(format!(
                    "Failed to read users directory: {}",
                    e
                )))
            }
        };

        for entry in entries {
            let entry = entry.map_err(|e| {
                AppError::Internal(format!("Failed to read directory entry: {}", e))
            })?;

            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();

            if !name.starts_with("user-") || !name.ends_with(".json") || name.ends_with("-auth.json")
            {
                continue;
            }

            let contents = fs::read_to_string(entry.path()).map_err(|e| {
                AppError::Internal(format!("Failed to read {:?}: {}", entry.path(), e))
            })?;

            match serde_json::from_str::<UserProfile>(&contents) {
                Ok(profile) => {
                    if predicate(&profile) {
                        return Ok(Some(profile.id));
                    }
                }
                Err(e) => {
                    log::warn!("Skipping malformed profile {:?}: {}", entry.path(), e);
                }
            }
        }

        Ok(None)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::TempDir;

    fn make_profile(id: Uuid, username: &str, email: &str) -> UserProfile {
        UserProfile {
            id,
            username: username.to_string(),
            email: email.to_string(),
            master_ledger: format!("users/user-{}-master.ledger", id),
            owned_accounts: vec![],
            shared_accounts: vec![],
            is_active: true,
            is_admin: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_auth(user_id: Uuid) -> UserAuth {
        UserAuth {
            user_id,
            password_hash: "$2b$14$somehashvalue".to_string(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn new_creates_users_directory() {
        let tmp = TempDir::new().unwrap();
        let data_path = tmp.path().to_str().unwrap().to_string();
        let _store = FileStore::new(data_path.clone());
        assert!(Path::new(&data_path).join("users").exists());
    }

    #[test]
    fn write_and_read_profile_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let id = Uuid::new_v4();
        let profile = make_profile(id, "alice", "alice@example.com");

        store.write_profile(&profile).unwrap();
        let read_back = store.read_profile(&id).unwrap().expect("profile should exist");
        assert_eq!(profile, read_back);
    }

    #[test]
    fn write_and_read_auth_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let id = Uuid::new_v4();
        let auth = make_auth(id);

        store.write_auth(&auth).unwrap();
        let read_back = store.read_auth(&id).unwrap().expect("auth should exist");
        assert_eq!(auth, read_back);
    }

    #[test]
    fn read_profile_returns_none_when_missing() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let result = store.read_profile(&Uuid::new_v4()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn read_auth_returns_none_when_missing() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let result = store.read_auth(&Uuid::new_v4()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn read_profile_returns_error_on_malformed_json() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let id = Uuid::new_v4();
        let path = tmp.path().join("users").join(format!("user-{}.json", id));
        fs::write(&path, "not valid json").unwrap();

        let result = store.read_profile(&id);
        assert!(result.is_err());
    }

    #[test]
    fn read_auth_returns_error_on_malformed_json() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let id = Uuid::new_v4();
        let path = tmp.path().join("users").join(format!("user-{}-auth.json", id));
        fs::write(&path, "not valid json").unwrap();

        let result = store.read_auth(&id);
        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn write_auth_sets_permissions_600() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let id = Uuid::new_v4();
        let auth = make_auth(id);

        store.write_auth(&auth).unwrap();

        let path = tmp.path().join("users").join(format!("user-{}-auth.json", id));
        let metadata = fs::metadata(&path).unwrap();
        let mode = metadata.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn find_user_id_by_username_finds_existing() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let id = Uuid::new_v4();
        let profile = make_profile(id, "bob", "bob@example.com");
        store.write_profile(&profile).unwrap();

        let found = store.find_user_id_by_username("bob").unwrap();
        assert_eq!(found, Some(id));
    }

    #[test]
    fn find_user_id_by_email_finds_existing() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let id = Uuid::new_v4();
        let profile = make_profile(id, "carol", "carol@example.com");
        store.write_profile(&profile).unwrap();

        let found = store.find_user_id_by_email("carol@example.com").unwrap();
        assert_eq!(found, Some(id));
    }

    #[test]
    fn find_user_id_returns_none_when_not_found() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());

        assert_eq!(store.find_user_id_by_username("nobody").unwrap(), None);
        assert_eq!(store.find_user_id_by_email("nobody@example.com").unwrap(), None);
    }

    #[test]
    fn find_user_skips_auth_files() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let id = Uuid::new_v4();
        let profile = make_profile(id, "dave", "dave@example.com");
        store.write_profile(&profile).unwrap();
        store.write_auth(&make_auth(id)).unwrap();

        let found = store.find_user_id_by_username("dave").unwrap();
        assert_eq!(found, Some(id));
    }

    #[test]
    fn create_master_ledger_writes_file() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let id = Uuid::new_v4();
        let profile = make_profile(id, "eve", "eve@example.com");

        store.create_master_ledger(&profile).unwrap();

        let ledger_path = tmp.path().join(&profile.master_ledger);
        assert!(ledger_path.exists());
        let contents = fs::read_to_string(&ledger_path).unwrap();
        assert!(contents.contains(&format!("User ID: {}", id)));
    }

    #[test]
    fn update_master_ledger_includes_accounts() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let id = Uuid::new_v4();
        let acct = Uuid::new_v4();
        let mut profile = make_profile(id, "frank", "frank@example.com");
        profile.owned_accounts = vec![acct];

        store.create_master_ledger(&profile).unwrap();
        store.update_master_ledger(&profile).unwrap();

        let ledger_path = tmp.path().join(&profile.master_ledger);
        let contents = fs::read_to_string(&ledger_path).unwrap();
        assert!(contents.contains(&format!(
            "!include ../workspaces/workspace-{0}/workspace-{0}.ledger",
            acct
        )));
    }

    #[test]
    fn atomic_write_overwrites_existing_file() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let id = Uuid::new_v4();

        let profile1 = make_profile(id, "original", "original@example.com");
        store.write_profile(&profile1).unwrap();

        let mut profile2 = profile1.clone();
        profile2.username = "updated".to_string();
        store.write_profile(&profile2).unwrap();

        let read_back = store.read_profile(&id).unwrap().unwrap();
        assert_eq!(read_back.username, "updated");
    }

    fn make_account(id: Uuid, name: &str, owner_id: Uuid) -> Workspace {
        Workspace {
            id,
            name: name.to_string(),
            owner_id,
            currency: "USD".to_string(),
            shared_with: vec![],
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ledger_dir: None,
            rotation_period: RotationPeriod::default(),
            budgeting_enabled: false,
            seed_color: None,
        }
    }

    #[test]
    fn write_and_read_account_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let id = Uuid::new_v4();
        let account = make_account(id, "Checking", Uuid::new_v4());

        store.write_account(&account).unwrap();
        let read_back = store.read_account(&id).unwrap().expect("account should exist");
        assert_eq!(account, read_back);
    }

    #[test]
    fn read_account_returns_none_when_missing() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let result = store.read_account(&Uuid::new_v4()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn read_account_returns_error_on_malformed_json() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let id = Uuid::new_v4();
        let path = tmp.path().join("accounts").join(format!("account-{}.json", id));
        fs::write(&path, "not valid json").unwrap();

        let result = store.read_account(&id);
        assert!(result.is_err());
    }

    #[test]
    fn create_account_ledger_writes_file() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let id = Uuid::new_v4();
        let account = make_account(id, "Savings", Uuid::new_v4());

        store.create_account_ledger(&account).unwrap();

        let ledger_path = tmp.path().join("accounts").join(format!("account-{}.ledger", id));
        assert!(ledger_path.exists());
        let contents = fs::read_to_string(&ledger_path).unwrap();
        assert!(contents.contains(&format!("Account ID: {}", id)));
        assert!(contents.contains("Account: Savings"));
    }

    #[test]
    fn new_creates_accounts_directory() {
        let tmp = TempDir::new().unwrap();
        let data_path = tmp.path().to_str().unwrap().to_string();
        let _store = FileStore::new(data_path.clone());
        assert!(Path::new(&data_path).join("accounts").exists());
    }

    #[test]
    fn new_creates_workspaces_directory() {
        let tmp = TempDir::new().unwrap();
        let data_path = tmp.path().to_str().unwrap().to_string();
        let _store = FileStore::new(data_path.clone());
        assert!(Path::new(&data_path).join("workspaces").exists());
    }

    fn make_categories(user_id: Uuid) -> UserCategories {
        UserCategories {
            user_id,
            expense: vec![
                "Expenses:Food:Groceries".to_string(),
                "Expenses:Transport:Gas".to_string(),
            ],
            income: vec!["Income:Salary".to_string()],
        }
    }

    #[test]
    fn write_and_read_categories_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let user_id = Uuid::new_v4();
        let categories = make_categories(user_id);

        store.write_categories(&categories).unwrap();
        let read_back = store
            .read_categories(&user_id)
            .unwrap()
            .expect("categories should exist");
        assert_eq!(categories, read_back);
    }

    #[test]
    fn read_categories_returns_none_when_missing() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let result = store.read_categories(&Uuid::new_v4()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn read_categories_returns_error_on_malformed_json() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let id = Uuid::new_v4();
        let path = tmp
            .path()
            .join("users")
            .join(format!("user-{}-categories.json", id));
        fs::write(&path, "not valid json").unwrap();

        let result = store.read_categories(&id);
        assert!(result.is_err());
    }

    #[test]
    fn append_to_ledger_creates_file_and_appends() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let account_id = Uuid::new_v4();

        store
            .append_to_ledger(&account_id, "2025-07-15 Grocery Store\n    Expenses:Food  $42.50")
            .unwrap();

        let path = tmp
            .path()
            .join("accounts")
            .join(format!("account-{}.ledger", account_id));
        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("2025-07-15 Grocery Store"));
        assert!(contents.contains("Expenses:Food  $42.50"));
    }

    #[test]
    fn append_to_ledger_multiple_appends_accumulate() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let account_id = Uuid::new_v4();

        store
            .append_to_ledger(&account_id, "2025-07-15 First Entry")
            .unwrap();
        store
            .append_to_ledger(&account_id, "2025-07-16 Second Entry")
            .unwrap();

        let path = tmp
            .path()
            .join("accounts")
            .join(format!("account-{}.ledger", account_id));
        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("2025-07-15 First Entry"));
        assert!(contents.contains("2025-07-16 Second Entry"));
    }

    #[test]
    fn get_ledger_path_returns_correct_path() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let account_id = Uuid::new_v4();

        let path = store.get_ledger_path(&account_id);
        let expected = tmp
            .path()
            .join("accounts")
            .join(format!("account-{}.ledger", account_id));
        assert_eq!(path, expected);
    }

    // --- New workspace methods tests ---

    #[test]
    fn write_and_read_workspace_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let id = Uuid::new_v4();
        let workspace = make_account(id, "Home Expenses", Uuid::new_v4());

        store.write_workspace(&workspace).unwrap();
        let read_back = store.read_workspace(&id).unwrap().expect("workspace should exist");
        assert_eq!(workspace, read_back);

        // Verify it's in workspaces/ directory
        let ws_path = tmp.path().join("workspaces").join(format!("workspace-{}.json", id));
        assert!(ws_path.exists());
    }

    #[test]
    fn read_workspace_falls_back_to_legacy_accounts() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let id = Uuid::new_v4();
        let workspace = make_account(id, "Legacy", Uuid::new_v4());

        // Write to legacy accounts/ path
        store.write_account(&workspace).unwrap();

        // read_workspace should find it via fallback
        let read_back = store.read_workspace(&id).unwrap().expect("should fall back to accounts/");
        assert_eq!(workspace, read_back);
    }

    #[test]
    fn read_workspace_returns_none_when_missing() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let result = store.read_workspace(&Uuid::new_v4()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn create_workspace_dir_creates_directory() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let id = Uuid::new_v4();
        let workspace = make_account(id, "Test", Uuid::new_v4());

        store.create_workspace_dir(&workspace).unwrap();

        let dir_path = tmp.path().join("workspaces").join(format!("workspace-{}", id));
        assert!(dir_path.exists());
        assert!(dir_path.is_dir());
    }

    #[test]
    fn create_workspace_ledger_writes_header_and_initial_include() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let id = Uuid::new_v4();
        let workspace = make_account(id, "Home", Uuid::new_v4());

        store.create_workspace_dir(&workspace).unwrap();
        store.create_workspace_ledger(&workspace).unwrap();

        let ledger_path = tmp.path()
            .join("workspaces")
            .join(format!("workspace-{}", id))
            .join(format!("workspace-{}.ledger", id));
        assert!(ledger_path.exists());

        let contents = fs::read_to_string(&ledger_path).unwrap();
        assert!(contents.contains(&format!("; Workspace: Home")));
        assert!(contents.contains(&format!("; Workspace ID: {}", id)));
        assert!(contents.contains("!include workspace-"));
    }

    #[test]
    fn create_period_file_writes_header() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let id = Uuid::new_v4();
        let workspace = make_account(id, "Test", Uuid::new_v4());

        store.create_workspace_dir(&workspace).unwrap();
        store.create_period_file(&workspace, "2026-Q1").unwrap();

        let period_path = tmp.path()
            .join("workspaces")
            .join(format!("workspace-{}", id))
            .join(format!("workspace-{}-2026-Q1.ledger", id));
        assert!(period_path.exists());

        let contents = fs::read_to_string(&period_path).unwrap();
        assert!(contents.contains("; Period: 2026-Q1"));
        assert!(contents.contains(&format!("; Workspace ID: {}", id)));
    }

    #[test]
    fn add_include_to_workspace_ledger_maintains_order() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let id = Uuid::new_v4();
        let workspace = make_account(id, "Test", Uuid::new_v4());

        store.create_workspace_dir(&workspace).unwrap();
        store.create_workspace_ledger(&workspace).unwrap();

        // Add a second period include using a far-past period so it can never
        // collide with the initial include created for "today" (which would
        // otherwise be deduplicated away).
        let extra_period = "1999-Q1";
        let extra_filename = format!("workspace-{}-{}.ledger", id, extra_period);
        store.create_period_file(&workspace, extra_period).unwrap();
        store.add_include_to_workspace_ledger(&workspace, &extra_filename).unwrap();

        let ledger_path = tmp.path()
            .join("workspaces")
            .join(format!("workspace-{}", id))
            .join(format!("workspace-{}.ledger", id));
        let contents = fs::read_to_string(&ledger_path).unwrap();

        let includes: Vec<&str> = contents.lines()
            .filter(|l| l.starts_with("!include "))
            .collect();
        assert!(includes.len() >= 2);
        // Verify chronological order
        for i in 1..includes.len() {
            assert!(includes[i - 1] <= includes[i], "Includes should be in chronological order");
        }
    }

    #[test]
    fn workspace_ledger_has_include_detects_existing() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let id = Uuid::new_v4();
        let workspace = make_account(id, "Test", Uuid::new_v4());

        store.create_workspace_dir(&workspace).unwrap();
        store.create_workspace_ledger(&workspace).unwrap();

        // The initial period file include should exist
        let now = Utc::now().date_naive();
        let period_filename = workspace.rotation_period.period_filename(&id, &now);
        assert!(store.workspace_ledger_has_include(&workspace, &period_filename).unwrap());

        // A non-existent include should return false
        assert!(!store.workspace_ledger_has_include(&workspace, "nonexistent.ledger").unwrap());
    }

    #[test]
    fn append_to_period_file_creates_and_appends() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let id = Uuid::new_v4();
        let workspace = make_account(id, "Test", Uuid::new_v4());

        store.create_workspace_dir(&workspace).unwrap();
        store.create_workspace_ledger(&workspace).unwrap();

        let entry = "2026-01-15 Grocery Store\n    Expenses:Food  $42.50\n    Assets:Checking  -$42.50";
        store.append_to_period_file(&workspace, "2026-Q1", entry).unwrap();

        let period_path = tmp.path()
            .join("workspaces")
            .join(format!("workspace-{}", id))
            .join(format!("workspace-{}-2026-Q1.ledger", id));
        let contents = fs::read_to_string(&period_path).unwrap();
        assert!(contents.contains("2026-01-15 Grocery Store"));
        assert!(contents.contains("Expenses:Food  $42.50"));
    }

    #[test]
    fn append_to_period_file_creates_new_period_and_updates_ledger() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let id = Uuid::new_v4();
        let workspace = make_account(id, "Test", Uuid::new_v4());

        store.create_workspace_dir(&workspace).unwrap();
        store.create_workspace_ledger(&workspace).unwrap();

        // Append to a new period that doesn't exist yet
        let entry = "2026-04-01 New Quarter\n    Expenses:Misc  $10.00\n    Assets:Checking  -$10.00";
        store.append_to_period_file(&workspace, "2026-Q2", entry).unwrap();

        // Verify the period file was created
        let period_path = tmp.path()
            .join("workspaces")
            .join(format!("workspace-{}", id))
            .join(format!("workspace-{}-2026-Q2.ledger", id));
        assert!(period_path.exists());

        // Verify the include was added to workspace ledger
        let q2_filename = format!("workspace-{}-2026-Q2.ledger", id);
        assert!(store.workspace_ledger_has_include(&workspace, &q2_filename).unwrap());
    }

    #[test]
    fn master_ledger_shared_annotation() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let user_id = Uuid::new_v4();
        let owned_ws = Uuid::new_v4();
        let shared_ws = Uuid::new_v4();

        let mut profile = make_profile(user_id, "alice", "alice@example.com");
        profile.owned_accounts = vec![owned_ws];
        profile.shared_accounts = vec![shared_ws];

        store.create_master_ledger(&profile).unwrap();

        let ledger_path = tmp.path().join(&profile.master_ledger);
        let contents = fs::read_to_string(&ledger_path).unwrap();

        // Owned workspace should NOT have ; shared
        let owned_line = format!(
            "!include ../workspaces/workspace-{0}/workspace-{0}.ledger",
            owned_ws
        );
        assert!(contents.contains(&owned_line));

        // Shared workspace should have ; shared
        let shared_line = format!(
            "!include ../workspaces/workspace-{0}/workspace-{0}.ledger  ; shared",
            shared_ws
        );
        assert!(contents.contains(&shared_line));
    }

    // --- Chart of Accounts FileStore tests ---

    fn make_chart(user_id: Uuid) -> ChartOfAccounts {
        ChartOfAccounts {
            user_id,
            assets: vec!["Assets:Checking".to_string(), "Assets:Savings".to_string()],
            expenses: vec!["Expenses:Food:Groceries".to_string()],
            income: vec!["Income:Salary".to_string()],
            liabilities: vec!["Liabilities:CreditCard".to_string()],
            equity: vec!["Equity:Opening Balances".to_string()],
        }
    }

    #[test]
    fn write_and_read_chart_of_accounts_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let user_id = Uuid::new_v4();
        let chart = make_chart(user_id);

        store.write_chart_of_accounts(&chart).unwrap();
        let read_back = store
            .read_chart_of_accounts(&user_id)
            .unwrap()
            .expect("chart should exist");
        assert_eq!(chart, read_back);
    }

    #[test]
    fn read_chart_of_accounts_returns_none_when_missing() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let result = store.read_chart_of_accounts(&Uuid::new_v4()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn read_chart_of_accounts_returns_error_on_malformed_json() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let id = Uuid::new_v4();
        let path = tmp
            .path()
            .join("users")
            .join(format!("user-{}-chart-of-accounts.json", id));
        fs::write(&path, "not valid json").unwrap();

        let result = store.read_chart_of_accounts(&id);
        assert!(result.is_err());
    }

    #[test]
    fn write_chart_of_accounts_persists_to_correct_path() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let user_id = Uuid::new_v4();
        let chart = ChartOfAccounts::empty(user_id);

        store.write_chart_of_accounts(&chart).unwrap();

        let expected_path = tmp
            .path()
            .join("users")
            .join(format!("user-{}-chart-of-accounts.json", user_id));
        assert!(expected_path.exists());
    }

    #[test]
    fn delete_categories_file_removes_existing() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let user_id = Uuid::new_v4();
        let categories = make_categories(user_id);

        store.write_categories(&categories).unwrap();
        let cat_path = tmp
            .path()
            .join("users")
            .join(format!("user-{}-categories.json", user_id));
        assert!(cat_path.exists());

        store.delete_categories_file(&user_id).unwrap();
        assert!(!cat_path.exists());
    }

    #[test]
    fn delete_categories_file_ignores_missing() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        // Should not error when file doesn't exist
        let result = store.delete_categories_file(&Uuid::new_v4());
        assert!(result.is_ok());
    }

    #[test]
    fn write_chart_of_accounts_atomic_overwrites() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let user_id = Uuid::new_v4();

        let mut chart = ChartOfAccounts::empty(user_id);
        store.write_chart_of_accounts(&chart).unwrap();

        chart.expenses.push("Expenses:New".to_string());
        store.write_chart_of_accounts(&chart).unwrap();

        let read_back = store.read_chart_of_accounts(&user_id).unwrap().unwrap();
        assert_eq!(read_back.expenses, vec!["Expenses:New".to_string()]);
    }

    #[test]
    fn migrate_workspace_ledger_with_existing_content() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let ws_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let mut workspace = make_account(ws_id, "Legacy WS", owner_id);

        // Create legacy ledger with some content
        let legacy_dir = tmp.path().join("accounts");
        fs::create_dir_all(&legacy_dir).unwrap();
        let legacy_content = "2026-01-15 Grocery Store\n    Expenses:Food  $42.50\n    Assets:Checking  -$42.50\n";
        fs::write(
            legacy_dir.join(format!("account-{}.ledger", ws_id)),
            legacy_content,
        )
        .unwrap();

        store.migrate_workspace_ledger(&mut workspace).unwrap();

        // Verify workspace directory was created
        let ws_dir = tmp.path().join("workspaces").join(format!("workspace-{}", ws_id));
        assert!(ws_dir.exists());

        // Verify workspace ledger was created with header and include
        let ws_ledger = ws_dir.join(format!("workspace-{}.ledger", ws_id));
        assert!(ws_ledger.exists());
        let ws_ledger_content = fs::read_to_string(&ws_ledger).unwrap();
        assert!(ws_ledger_content.contains(&format!("; Workspace: Legacy WS")));
        assert!(ws_ledger_content.contains(&format!("; Workspace ID: {}", ws_id)));
        assert!(ws_ledger_content.contains("!include workspace-"));

        // Verify period file was created and contains the legacy content
        let now = Utc::now().date_naive();
        let period_label = workspace.rotation_period.period_label(&now);
        let period_filename = format!("workspace-{}-{}.ledger", ws_id, period_label);
        let period_path = ws_dir.join(&period_filename);
        assert!(period_path.exists());
        let period_content = fs::read_to_string(&period_path).unwrap();
        assert!(period_content.contains(&format!("; Period: {}", period_label)));
        assert!(period_content.contains(legacy_content));

        // Verify workspace metadata was updated
        assert_eq!(
            workspace.ledger_dir,
            Some(format!("workspaces/workspace-{}/", ws_id))
        );

        // Verify workspace JSON was persisted
        let ws_json = store.read_workspace(&ws_id).unwrap().unwrap();
        assert_eq!(ws_json.ledger_dir, Some(format!("workspaces/workspace-{}/", ws_id)));
    }

    #[test]
    fn migrate_workspace_ledger_without_legacy_file() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let ws_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let mut workspace = make_account(ws_id, "New WS", owner_id);

        // No legacy file exists — migration should still succeed
        store.migrate_workspace_ledger(&mut workspace).unwrap();

        // Verify workspace directory was created
        let ws_dir = tmp.path().join("workspaces").join(format!("workspace-{}", ws_id));
        assert!(ws_dir.exists());

        // Verify period file was created with header only (no legacy content)
        let now = Utc::now().date_naive();
        let period_label = workspace.rotation_period.period_label(&now);
        let period_filename = format!("workspace-{}-{}.ledger", ws_id, period_label);
        let period_path = ws_dir.join(&period_filename);
        assert!(period_path.exists());
        let period_content = fs::read_to_string(&period_path).unwrap();
        assert!(period_content.contains(&format!("; Period: {}", period_label)));
        assert!(period_content.contains(&format!("; Workspace ID: {}", ws_id)));
        // Should only have the header, no transaction content
        assert_eq!(period_content.lines().count(), 2);

        // Verify workspace ledger exists with include
        let ws_ledger = ws_dir.join(format!("workspace-{}.ledger", ws_id));
        let ws_ledger_content = fs::read_to_string(&ws_ledger).unwrap();
        assert!(ws_ledger_content.contains(&format!("!include {}", period_filename)));

        // Verify ledger_dir was set
        assert_eq!(
            workspace.ledger_dir,
            Some(format!("workspaces/workspace-{}/", ws_id))
        );
    }

    #[test]
    fn get_budget_file_path_returns_correct_path() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let ws_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let workspace = make_account(ws_id, "Budget WS", owner_id);

        let path = store.get_budget_file_path(&workspace);
        let expected = tmp
            .path()
            .join("workspaces")
            .join(format!("workspace-{}", ws_id))
            .join(format!("workspace-{}-budget.ledger", ws_id));
        assert_eq!(path, expected);
    }

    #[test]
    fn write_and_read_budget_file_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let ws_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let workspace = make_account(ws_id, "Budget WS", owner_id);

        // Create workspace directory so the budget file can be written
        store.create_workspace_dir(&workspace).unwrap();

        let content = "; Budget definitions\n\n~ Monthly\n    Expenses:Food  $500.00\n    Assets:Checking\n";
        store.write_budget_file(&workspace, content).unwrap();

        let read_back = store.read_budget_file(&workspace).unwrap();
        assert_eq!(read_back, Some(content.to_string()));
    }

    #[test]
    fn read_budget_file_returns_none_when_missing() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let ws_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let workspace = make_account(ws_id, "Budget WS", owner_id);

        let result = store.read_budget_file(&workspace).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn write_budget_file_overwrites_existing() {
        let tmp = TempDir::new().unwrap();
        let store = FileStore::new(tmp.path().to_str().unwrap().to_string());
        let ws_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let workspace = make_account(ws_id, "Budget WS", owner_id);

        store.create_workspace_dir(&workspace).unwrap();

        store.write_budget_file(&workspace, "original content").unwrap();
        store.write_budget_file(&workspace, "updated content").unwrap();

        let read_back = store.read_budget_file(&workspace).unwrap();
        assert_eq!(read_back, Some("updated content".to_string()));
    }

}
