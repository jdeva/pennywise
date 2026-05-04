use chrono::Utc;
use uuid::Uuid;

use crate::models::v1::recurring::{
    RecurringDefinition, RecurringDefinitionResponse, RecurringForecastResponse,
};
use crate::services::cache::Cache;
use crate::services::file_store::FileStore;
use crate::services::ledger_cli::LedgerCli;
use crate::services::workspace::WorkspaceService;
use crate::utils::error::AppError;

/// Ledger periodic-transaction block for a recurring entry. A payee (when set)
/// lands on the primary posting as `; Payee: <name>` so the frontend can
/// render forecasts with the payee label alongside the amount.
pub fn format_periodic_transaction(
    period: &str,
    account: &str,
    counter_account: &str,
    amount: f64,
    currency: &str,
    payee: Option<&str>,
) -> String {
    let payee_comment = match payee {
        Some(p) if !p.trim().is_empty() => format!("  ; Payee: {}", p.trim()),
        _ => String::new(),
    };
    format!(
        "~ {period}\n    {account}  {currency}{amount:.2}{payee_comment}\n    {counter}",
        period = period,
        account = account,
        currency = currency,
        amount = amount,
        payee_comment = payee_comment,
        counter = counter_account,
    )
}

pub fn parse_recurring_file(content: &str) -> Vec<RecurringDefinition> {
    let mut out = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        if let Some(period) = line.strip_prefix("~ ") {
            let period = period.trim().to_string();
            // Primary posting
            i += 1;
            if i >= lines.len() {
                break;
            }
            let primary = lines[i];
            let (primary_part, payee) = split_off_comment(primary);
            let primary_parsed = parse_posting_line(primary_part);
            // Counter posting on the next line
            i += 1;
            let counter = if i < lines.len() {
                let (c_part, _) = split_off_comment(lines[i]);
                c_part.trim().to_string()
            } else {
                String::new()
            };
            if let Some((account, currency, amount)) = primary_parsed {
                if !counter.is_empty() {
                    out.push(RecurringDefinition {
                        period,
                        account,
                        counter_account: counter,
                        amount,
                        currency,
                        payee,
                    });
                }
            }
        }
        i += 1;
    }

    out
}

fn split_off_comment(line: &str) -> (&str, Option<String>) {
    // Payee comment format written by format_periodic_transaction: `  ; Payee: <name>`.
    if let Some(idx) = line.find(';') {
        let (before, after) = line.split_at(idx);
        let after_trim = after.trim_start_matches(';').trim();
        let payee = after_trim
            .strip_prefix("Payee:")
            .map(|v| v.trim().to_string());
        (before.trim_end(), payee)
    } else {
        (line, None)
    }
}

fn parse_posting_line(posting: &str) -> Option<(String, String, f64)> {
    let trimmed = posting.trim();
    let parts: Vec<&str> = trimmed.splitn(2, "  ").collect();
    if parts.len() != 2 {
        return None;
    }
    let account = parts[0].trim().to_string();
    let amount_str = parts[1].trim();
    if amount_str.is_empty() {
        return None;
    }
    let numeric_start = amount_str
        .find(|c: char| c.is_ascii_digit() || c == '-')
        .unwrap_or(amount_str.len());
    let currency = if numeric_start > 0 {
        amount_str[..numeric_start].to_string()
    } else {
        "$".to_string()
    };
    let amount: f64 = amount_str[numeric_start..].parse().ok()?;
    Some((account, currency, amount))
}

pub fn serialize_recurring_file(definitions: &[RecurringDefinition], workspace_id: &Uuid) -> String {
    let mut output = format!(
        "; Recurring transactions\n; Workspace ID: {}\n",
        workspace_id
    );
    for def in definitions {
        output.push('\n');
        output.push_str(&format_periodic_transaction(
            &def.period,
            &def.account,
            &def.counter_account,
            def.amount,
            &def.currency,
            def.payee.as_deref(),
        ));
        output.push('\n');
    }
    output
}

#[derive(Clone)]
pub struct RecurringService {
    file_store: FileStore,
    #[allow(dead_code)]
    cache: Cache,
    workspace_service: WorkspaceService,
    #[allow(dead_code)]
    cache_ttl: usize,
}

impl RecurringService {
    pub fn new(
        file_store: FileStore,
        cache: Cache,
        workspace_service: WorkspaceService,
        cache_ttl: usize,
    ) -> Self {
        Self {
            file_store,
            cache,
            workspace_service,
            cache_ttl,
        }
    }

    fn ensure_include(&self, workspace: &crate::models::Workspace) -> Result<(), AppError> {
        let filename = format!("workspace-{}-recurring.ledger", workspace.id);
        if !self
            .file_store
            .workspace_ledger_has_include(workspace, &filename)?
        {
            self.file_store
                .add_include_to_workspace_ledger(workspace, &filename)?;
        }
        Ok(())
    }

    pub fn list(
        &self,
        workspace_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<Vec<RecurringDefinition>, AppError> {
        let workspace = self
            .workspace_service
            .get_workspace_authorized(workspace_id, user_id)?;
        match self.file_store.read_recurring_file(&workspace)? {
            Some(content) => Ok(parse_recurring_file(&content)),
            None => Ok(Vec::new()),
        }
    }

    pub fn create(
        &self,
        workspace_id: &Uuid,
        user_id: &Uuid,
        period: &str,
        account: &str,
        counter_account: &str,
        amount: f64,
        currency: &str,
        payee: Option<&str>,
    ) -> Result<RecurringDefinitionResponse, AppError> {
        let workspace = self
            .workspace_service
            .get_workspace_authorized(workspace_id, user_id)?;
        if !workspace.has_write_access(user_id) {
            return Err(AppError::Forbidden(
                "You don't have write access to this workspace".to_string(),
            ));
        }
        if !workspace.is_active {
            return Err(AppError::BadRequest("Workspace is deactivated".to_string()));
        }

        let existing = self.file_store.read_recurring_file(&workspace)?;
        let mut definitions = match &existing {
            Some(content) => parse_recurring_file(content),
            None => Vec::new(),
        };

        let definition = RecurringDefinition {
            period: period.to_string(),
            account: account.to_string(),
            counter_account: counter_account.to_string(),
            amount,
            currency: currency.to_string(),
            payee: payee.filter(|s| !s.trim().is_empty()).map(|s| s.to_string()),
        };
        definitions.push(definition.clone());

        let content = serialize_recurring_file(&definitions, workspace_id);
        self.file_store.write_recurring_file(&workspace, &content)?;

        if existing.is_none() {
            self.ensure_include(&workspace)?;
        }

        let formatted = format_periodic_transaction(
            period,
            account,
            counter_account,
            amount,
            currency,
            payee,
        );
        Ok(RecurringDefinitionResponse {
            formatted_text: formatted,
            definition,
        })
    }

    pub fn update(
        &self,
        workspace_id: &Uuid,
        user_id: &Uuid,
        index: usize,
        period: &str,
        account: &str,
        counter_account: &str,
        amount: f64,
        currency: &str,
        payee: Option<&str>,
    ) -> Result<RecurringDefinitionResponse, AppError> {
        let workspace = self
            .workspace_service
            .get_workspace_authorized(workspace_id, user_id)?;
        if !workspace.has_write_access(user_id) {
            return Err(AppError::Forbidden(
                "You don't have write access to this workspace".to_string(),
            ));
        }
        let content = self
            .file_store
            .read_recurring_file(&workspace)?
            .unwrap_or_default();
        let mut definitions = parse_recurring_file(&content);
        if index >= definitions.len() {
            return Err(AppError::NotFound(format!(
                "Recurring definition not found at index {}",
                index
            )));
        }
        let definition = RecurringDefinition {
            period: period.to_string(),
            account: account.to_string(),
            counter_account: counter_account.to_string(),
            amount,
            currency: currency.to_string(),
            payee: payee.filter(|s| !s.trim().is_empty()).map(|s| s.to_string()),
        };
        definitions[index] = definition.clone();
        let new_content = serialize_recurring_file(&definitions, workspace_id);
        self.file_store.write_recurring_file(&workspace, &new_content)?;

        let formatted = format_periodic_transaction(
            period,
            account,
            counter_account,
            amount,
            currency,
            payee,
        );
        Ok(RecurringDefinitionResponse {
            formatted_text: formatted,
            definition,
        })
    }

    pub fn delete(&self, workspace_id: &Uuid, user_id: &Uuid, index: usize) -> Result<(), AppError> {
        let workspace = self
            .workspace_service
            .get_workspace_authorized(workspace_id, user_id)?;
        if !workspace.has_write_access(user_id) {
            return Err(AppError::Forbidden(
                "You don't have write access to this workspace".to_string(),
            ));
        }
        let content = self
            .file_store
            .read_recurring_file(&workspace)?
            .unwrap_or_default();
        let mut definitions = parse_recurring_file(&content);
        if index >= definitions.len() {
            return Err(AppError::NotFound(format!(
                "Recurring definition not found at index {}",
                index
            )));
        }
        definitions.remove(index);
        let new_content = serialize_recurring_file(&definitions, workspace_id);
        self.file_store.write_recurring_file(&workspace, &new_content)?;
        Ok(())
    }

    /// Shell out to `ledger --forecast register` against the workspace's
    /// consolidated ledger (which `!include`s the recurring file). Default
    /// horizon is 90 days.
    pub fn forecast(
        &self,
        workspace_id: &Uuid,
        user_id: &Uuid,
        end_date: Option<&str>,
    ) -> Result<RecurringForecastResponse, AppError> {
        let workspace = self
            .workspace_service
            .get_workspace_authorized(workspace_id, user_id)?;
        if self.file_store.read_recurring_file(&workspace)?.is_none() {
            return Ok(RecurringForecastResponse { output: String::new() });
        }
        let effective_end = match end_date {
            Some(d) => d.to_string(),
            None => {
                let future = Utc::now() + chrono::Duration::days(90);
                future.format("%Y-%m-%d").to_string()
            }
        };
        let ledger_path = self.file_store.get_workspace_ledger_path(&workspace);
        let output = LedgerCli::forecast_register(&ledger_path, &effective_end)?;
        Ok(RecurringForecastResponse { output })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_includes_payee_when_set() {
        let r = format_periodic_transaction(
            "Monthly",
            "Expenses:Subscriptions:Netflix",
            "Assets:Bank:Revolut",
            15.99,
            "$",
            Some("Netflix"),
        );
        assert!(r.contains("~ Monthly"));
        assert!(r.contains("Expenses:Subscriptions:Netflix  $15.99  ; Payee: Netflix"));
        assert!(r.contains("Assets:Bank:Revolut"));
    }

    #[test]
    fn format_without_payee_has_no_comment() {
        let r = format_periodic_transaction(
            "Weekly",
            "Expenses:Food",
            "Assets:Cash",
            50.0,
            "$",
            None,
        );
        assert!(!r.contains(";"));
    }

    #[test]
    fn round_trip_with_payee() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let original = vec![RecurringDefinition {
            period: "Monthly".to_string(),
            account: "Expenses:Subscriptions:Netflix".to_string(),
            counter_account: "Assets:Bank:Revolut".to_string(),
            amount: 15.99,
            currency: "$".to_string(),
            payee: Some("Netflix".to_string()),
        }];
        let serialized = serialize_recurring_file(&original, &id);
        let parsed = parse_recurring_file(&serialized);
        assert_eq!(original, parsed);
    }

    #[test]
    fn round_trip_multiple() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let original = vec![
            RecurringDefinition {
                period: "Monthly".to_string(),
                account: "Expenses:Rent".to_string(),
                counter_account: "Assets:Bank:Revolut".to_string(),
                amount: 1500.0,
                currency: "$".to_string(),
                payee: Some("Landlord".to_string()),
            },
            RecurringDefinition {
                period: "Yearly".to_string(),
                account: "Expenses:Insurance".to_string(),
                counter_account: "Assets:Bank:Checking".to_string(),
                amount: 1200.0,
                currency: "€".to_string(),
                payee: None,
            },
        ];
        let serialized = serialize_recurring_file(&original, &id);
        let parsed = parse_recurring_file(&serialized);
        assert_eq!(original, parsed);
    }
}
