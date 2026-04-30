use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Rotation period for ledger file splitting
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RotationPeriod {
    Quarterly,
    SemiAnnual,
    Yearly,
}

impl Default for RotationPeriod {
    fn default() -> Self {
        RotationPeriod::Quarterly
    }
}

impl RotationPeriod {
    /// Compute the period label for a given date
    pub fn period_label(&self, date: &NaiveDate) -> String {
        let year = date.year();
        match self {
            RotationPeriod::Quarterly => {
                let quarter = (date.month0() / 3) + 1;
                format!("{}-Q{}", year, quarter)
            }
            RotationPeriod::SemiAnnual => {
                let half = if date.month() <= 6 { 1 } else { 2 };
                format!("{}-H{}", year, half)
            }
            RotationPeriod::Yearly => format!("{}", year),
        }
    }

    /// Compute the period file name for a workspace and date
    pub fn period_filename(&self, workspace_id: &Uuid, date: &NaiveDate) -> String {
        format!(
            "workspace-{}-{}.ledger",
            workspace_id,
            self.period_label(date)
        )
    }
}

/// Workspace metadata — persisted as workspace-{uuid}.json
/// A workspace is a shared expense collection (e.g., "Home", "Vacation")
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Workspace {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub currency: String,
    pub shared_with: Vec<SharedUser>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Relative path to workspace ledger directory (None for legacy workspaces)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ledger_dir: Option<String>,
    /// Period rotation setting
    #[serde(default)]
    pub rotation_period: RotationPeriod,
    /// Whether budgeting features are enabled for this workspace
    #[serde(default)]
    pub budgeting_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SharedUser {
    pub user_id: Uuid,
    pub permission: Permission,
}

/// Shared-user entry in API responses — like SharedUser but with username resolved.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SharedUserPublic {
    pub user_id: Uuid,
    pub username: String,
    pub permission: Permission,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Permission {
    #[serde(alias = "ro")]
    Read,
    #[serde(alias = "rw")]
    Write,
}

impl Workspace {
    pub fn has_access(&self, user_id: &Uuid) -> bool {
        self.owner_id == *user_id || self.shared_with.iter().any(|s| s.user_id == *user_id)
    }

    pub fn has_write_access(&self, user_id: &Uuid) -> bool {
        self.owner_id == *user_id
            || self.shared_with.iter().any(|s| s.user_id == *user_id && s.permission == Permission::Write)
    }
}

/// Public-facing workspace for API responses
/// Represents a shared expense collection visible to owner and shared users
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspacePublic {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub currency: String,
    pub shared_with: Vec<SharedUserPublic>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ledger_dir: Option<String>,
    #[serde(default)]
    pub rotation_period: RotationPeriod,
    /// Whether budgeting features are enabled for this workspace
    #[serde(default)]
    pub budgeting_enabled: bool,
}

impl From<Workspace> for WorkspacePublic {
    fn from(a: Workspace) -> Self {
        let shared_with = a
            .shared_with
            .into_iter()
            .map(|s| SharedUserPublic {
                user_id: s.user_id,
                username: String::new(),
                permission: s.permission,
            })
            .collect();
        WorkspacePublic {
            id: a.id,
            name: a.name,
            owner_id: a.owner_id,
            currency: a.currency,
            shared_with,
            is_active: a.is_active,
            created_at: a.created_at,
            updated_at: a.updated_at,
            ledger_dir: a.ledger_dir,
            rotation_period: a.rotation_period,
            budgeting_enabled: a.budgeting_enabled,
        }
    }
}

// Request DTOs
#[derive(Debug, Clone, Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub currency: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateWorkspaceRequest {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ShareWorkspaceRequest {
    pub username: String,
    #[serde(default = "default_permission")]
    pub permission: Permission,
}

fn default_permission() -> Permission {
    Permission::Read
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetBudgetingRequest {
    pub enabled: bool,
}

