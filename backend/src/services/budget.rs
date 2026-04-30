use chrono::Utc;
use uuid::Uuid;

use crate::models::v1::budget::{BudgetDefinition, BudgetDefinitionResponse, BudgetReportResponse};
use crate::models::Workspace;
use crate::services::cache::Cache;
use crate::services::file_store::FileStore;
use crate::services::ledger_cli::LedgerCli;
use crate::services::workspace::WorkspaceService;
use crate::utils::error::AppError;

/// Formats a single periodic transaction in ledger-cli format.
///
/// Returns:
/// ```text
/// ~ {period}
///     {account}  {currency}{amount:.2}
///     Assets:Checking
/// ```
pub fn format_periodic_transaction(period: &str, account: &str, amount: f64, currency: &str) -> String {
    format!(
        "~ {period}\n    {account}  {currency}{amount:.2}\n    Assets:Checking",
        period = period,
        account = account,
        currency = currency,
        amount = amount,
    )
}

/// Parses budget file content into a list of `BudgetDefinition` objects.
///
/// Scans for lines starting with `~ `, reads subsequent indented posting lines,
/// and extracts account name, currency symbol, and amount from the first posting.
pub fn parse_budget_file(content: &str) -> Vec<BudgetDefinition> {
    let mut definitions = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        if let Some(period) = line.strip_prefix("~ ") {
            let period = period.trim().to_string();
            // Read subsequent indented posting lines
            i += 1;
            if i < lines.len() {
                let posting_line = lines[i].trim();
                // Parse the first posting: "Account  CurrencyAmount"
                // Find the last group of whitespace that separates account from amount
                if let Some(def) = parse_posting_line(&period, posting_line) {
                    definitions.push(def);
                }
            }
        }
        i += 1;
    }

    definitions
}

/// Parses a single posting line like "Expenses:Food  $500.00" into a BudgetDefinition.
fn parse_posting_line(period: &str, posting: &str) -> Option<BudgetDefinition> {
    // Split on two-or-more spaces to separate account from amount portion
    let parts: Vec<&str> = posting.splitn(2, "  ").collect();
    if parts.len() != 2 {
        return None;
    }

    let account = parts[0].trim().to_string();
    let amount_str = parts[1].trim();

    if amount_str.is_empty() {
        return None;
    }

    // The currency symbol is the leading non-numeric, non-dot, non-minus characters
    let numeric_start = amount_str
        .find(|c: char| c.is_ascii_digit() || c == '-')
        .unwrap_or(amount_str.len());

    let currency = if numeric_start > 0 {
        amount_str[..numeric_start].to_string()
    } else {
        "$".to_string()
    };

    let amount: f64 = amount_str[numeric_start..].parse().ok()?;

    Some(BudgetDefinition {
        period: period.to_string(),
        account,
        amount,
        currency,
    })
}

/// Serializes a list of `BudgetDefinition` objects into ledger-cli budget file format.
///
/// Produces header comments and formatted periodic transactions separated by blank lines.
pub fn serialize_budget_file(definitions: &[BudgetDefinition], workspace_id: &Uuid) -> String {
    let mut output = format!(
        "; Budget definitions\n; Workspace ID: {}\n",
        workspace_id
    );

    for def in definitions {
        output.push('\n');
        output.push_str(&format_periodic_transaction(
            &def.period,
            &def.account,
            def.amount,
            &def.currency,
        ));
        output.push('\n');
    }

    output
}

#[derive(Clone)]
pub struct BudgetService {
    file_store: FileStore,
    #[allow(dead_code)]
    cache: Cache,
    workspace_service: WorkspaceService,
    #[allow(dead_code)]
    cache_ttl: usize,
}

impl BudgetService {
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

    /// Checks that budgeting is enabled for the workspace; returns HTTP 400 if not.
    fn ensure_budgeting_enabled(&self, workspace: &Workspace) -> Result<(), AppError> {
        if !workspace.budgeting_enabled {
            return Err(AppError::BadRequest(
                "Budgeting is not enabled for this workspace".to_string(),
            ));
        }
        Ok(())
    }

    /// Adds `!include workspace-{uuid}-budget.ledger` to the workspace ledger if not already present.
    fn ensure_budget_include(&self, workspace: &Workspace) -> Result<(), AppError> {
        let budget_filename = format!("workspace-{}-budget.ledger", workspace.id);
        if !self
            .file_store
            .workspace_ledger_has_include(workspace, &budget_filename)?
        {
            self.file_store
                .add_include_to_workspace_ledger(workspace, &budget_filename)?;
        }
        Ok(())
    }

    /// Creates a new budget definition and appends it to the workspace's budget file.
    pub fn create_budget_definition(
        &self,
        workspace_id: &Uuid,
        user_id: &Uuid,
        period: &str,
        account: &str,
        amount: f64,
        currency: &str,
    ) -> Result<BudgetDefinitionResponse, AppError> {
        let workspace = self
            .workspace_service
            .get_workspace_authorized(workspace_id, user_id)?;

        if !workspace.has_write_access(user_id) {
            return Err(AppError::Forbidden(
                "You don't have write access to this account".to_string(),
            ));
        }

        self.ensure_budgeting_enabled(&workspace)?;

        if !workspace.is_active {
            return Err(AppError::BadRequest(
                "Workspace is deactivated".to_string(),
            ));
        }

        let formatted = format_periodic_transaction(period, account, amount, currency);

        // Read existing budget file or start fresh
        let existing = self.file_store.read_budget_file(&workspace)?;
        let mut definitions = match &existing {
            Some(content) => parse_budget_file(content),
            None => Vec::new(),
        };

        let definition = BudgetDefinition {
            period: period.to_string(),
            account: account.to_string(),
            amount,
            currency: currency.to_string(),
        };
        definitions.push(definition.clone());

        let content = serialize_budget_file(&definitions, workspace_id);
        self.file_store.write_budget_file(&workspace, &content)?;

        // On first creation, ensure the workspace ledger includes the budget file
        if existing.is_none() {
            self.ensure_budget_include(&workspace)?;
        }

        Ok(BudgetDefinitionResponse {
            formatted_text: formatted,
            definition,
        })
    }

    /// Lists all budget definitions for a workspace.
    pub fn list_budget_definitions(
        &self,
        workspace_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<Vec<BudgetDefinition>, AppError> {
        let workspace = self
            .workspace_service
            .get_workspace_authorized(workspace_id, user_id)?;

        self.ensure_budgeting_enabled(&workspace)?;

        match self.file_store.read_budget_file(&workspace)? {
            Some(content) => Ok(parse_budget_file(&content)),
            None => Ok(Vec::new()),
        }
    }

    /// Updates a budget definition at the given index.
    pub fn update_budget_definition(
        &self,
        workspace_id: &Uuid,
        user_id: &Uuid,
        index: usize,
        period: &str,
        account: &str,
        amount: f64,
        currency: &str,
    ) -> Result<BudgetDefinitionResponse, AppError> {
        let workspace = self
            .workspace_service
            .get_workspace_authorized(workspace_id, user_id)?;

        if !workspace.has_write_access(user_id) {
            return Err(AppError::Forbidden(
                "You don't have write access to this account".to_string(),
            ));
        }

        self.ensure_budgeting_enabled(&workspace)?;

        if !workspace.is_active {
            return Err(AppError::BadRequest(
                "Workspace is deactivated".to_string(),
            ));
        }

        let content = self
            .file_store
            .read_budget_file(&workspace)?
            .unwrap_or_default();
        let mut definitions = parse_budget_file(&content);

        if index >= definitions.len() {
            return Err(AppError::NotFound(format!(
                "Budget definition not found at index {}",
                index
            )));
        }

        let definition = BudgetDefinition {
            period: period.to_string(),
            account: account.to_string(),
            amount,
            currency: currency.to_string(),
        };
        definitions[index] = definition.clone();

        let new_content = serialize_budget_file(&definitions, workspace_id);
        self.file_store.write_budget_file(&workspace, &new_content)?;

        let formatted = format_periodic_transaction(period, account, amount, currency);
        Ok(BudgetDefinitionResponse {
            formatted_text: formatted,
            definition,
        })
    }

    /// Deletes a budget definition at the given index.
    pub fn delete_budget_definition(
        &self,
        workspace_id: &Uuid,
        user_id: &Uuid,
        index: usize,
    ) -> Result<(), AppError> {
        let workspace = self
            .workspace_service
            .get_workspace_authorized(workspace_id, user_id)?;

        if !workspace.has_write_access(user_id) {
            return Err(AppError::Forbidden(
                "You don't have write access to this account".to_string(),
            ));
        }

        self.ensure_budgeting_enabled(&workspace)?;

        let content = self
            .file_store
            .read_budget_file(&workspace)?
            .unwrap_or_default();
        let mut definitions = parse_budget_file(&content);

        if index >= definitions.len() {
            return Err(AppError::NotFound(format!(
                "Budget definition not found at index {}",
                index
            )));
        }

        definitions.remove(index);

        let new_content = serialize_budget_file(&definitions, workspace_id);
        self.file_store.write_budget_file(&workspace, &new_content)?;

        Ok(())
    }

    /// Returns a budget vs actual report for the workspace.
    ///
    /// Invokes `ledger --budget balance` against the workspace ledger.
    /// Returns empty output if no budget file exists.
    pub fn budget_report(
        &self,
        workspace_id: &Uuid,
        user_id: &Uuid,
        begin: Option<&str>,
        end: Option<&str>,
    ) -> Result<BudgetReportResponse, AppError> {
        let workspace = self
            .workspace_service
            .get_workspace_authorized(workspace_id, user_id)?;

        self.ensure_budgeting_enabled(&workspace)?;

        // If no budget file exists, return empty output
        if self.file_store.read_budget_file(&workspace)?.is_none() {
            return Ok(BudgetReportResponse {
                output: String::new(),
            });
        }

        let ledger_path = self.file_store.get_workspace_ledger_path(&workspace);
        let output = LedgerCli::budget_balance(&ledger_path, begin, end)?;

        Ok(BudgetReportResponse { output })
    }

    /// Returns an unbudgeted expenses report for the workspace.
    ///
    /// Invokes `ledger --unbudgeted balance` against the workspace ledger.
    /// Returns empty output if no budget file exists.
    pub fn unbudgeted_report(
        &self,
        workspace_id: &Uuid,
        user_id: &Uuid,
        begin: Option<&str>,
        end: Option<&str>,
    ) -> Result<BudgetReportResponse, AppError> {
        let workspace = self
            .workspace_service
            .get_workspace_authorized(workspace_id, user_id)?;

        self.ensure_budgeting_enabled(&workspace)?;

        // If no budget file exists, return empty output
        if self.file_store.read_budget_file(&workspace)?.is_none() {
            return Ok(BudgetReportResponse {
                output: String::new(),
            });
        }

        let ledger_path = self.file_store.get_workspace_ledger_path(&workspace);
        let output = LedgerCli::unbudgeted_balance(&ledger_path, begin, end)?;

        Ok(BudgetReportResponse { output })
    }

    /// Returns a budget forecast report for the workspace.
    ///
    /// Invokes `ledger --forecast register` against the workspace ledger.
    /// Defaults end date to 3 months from now if not specified.
    /// Returns empty output if no budget file exists.
    pub fn forecast_report(
        &self,
        workspace_id: &Uuid,
        user_id: &Uuid,
        end_date: Option<&str>,
    ) -> Result<BudgetReportResponse, AppError> {
        let workspace = self
            .workspace_service
            .get_workspace_authorized(workspace_id, user_id)?;

        self.ensure_budgeting_enabled(&workspace)?;

        // If no budget file exists, return empty output
        if self.file_store.read_budget_file(&workspace)?.is_none() {
            return Ok(BudgetReportResponse {
                output: String::new(),
            });
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

        Ok(BudgetReportResponse { output })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_periodic_transaction_basic() {
        let result = format_periodic_transaction("Monthly", "Expenses:Food", 500.0, "$");
        assert_eq!(
            result,
            "~ Monthly\n    Expenses:Food  $500.00\n    Assets:Checking"
        );
    }

    #[test]
    fn format_periodic_transaction_different_currency() {
        let result = format_periodic_transaction("Yearly", "Expenses:Insurance", 2400.50, "€");
        assert_eq!(
            result,
            "~ Yearly\n    Expenses:Insurance  €2400.50\n    Assets:Checking"
        );
    }

    #[test]
    fn parse_budget_file_empty_content() {
        let defs = parse_budget_file("");
        assert!(defs.is_empty());
    }

    #[test]
    fn parse_budget_file_comments_only() {
        let content = "; Budget definitions\n; Workspace ID: abc-123\n";
        let defs = parse_budget_file(content);
        assert!(defs.is_empty());
    }

    #[test]
    fn parse_budget_file_single_definition() {
        let content = "\
; Budget definitions
; Workspace ID: abc-123

~ Monthly
    Expenses:Food  $500.00
    Assets:Checking
";
        let defs = parse_budget_file(content);
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].period, "Monthly");
        assert_eq!(defs[0].account, "Expenses:Food");
        assert_eq!(defs[0].amount, 500.0);
        assert_eq!(defs[0].currency, "$");
    }

    #[test]
    fn parse_budget_file_multiple_definitions() {
        let content = "\
; Budget definitions
; Workspace ID: abc-123

~ Monthly
    Expenses:Food  $500.00
    Assets:Checking

~ Yearly
    Expenses:Insurance  $2400.00
    Assets:Checking
";
        let defs = parse_budget_file(content);
        assert_eq!(defs.len(), 2);
        assert_eq!(defs[0].period, "Monthly");
        assert_eq!(defs[0].account, "Expenses:Food");
        assert_eq!(defs[1].period, "Yearly");
        assert_eq!(defs[1].account, "Expenses:Insurance");
        assert_eq!(defs[1].amount, 2400.0);
    }

    #[test]
    fn parse_budget_file_non_dollar_currency() {
        let content = "~ Monthly\n    Expenses:Rent  €1500.00\n    Assets:Checking\n";
        let defs = parse_budget_file(content);
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].currency, "€");
        assert_eq!(defs[0].amount, 1500.0);
    }

    #[test]
    fn serialize_budget_file_empty_list() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").expect("valid uuid");
        let result = serialize_budget_file(&[], &id);
        assert!(result.contains("; Budget definitions"));
        assert!(result.contains(&id.to_string()));
        // No periodic transactions
        assert!(!result.contains("~ "));
    }

    #[test]
    fn serialize_budget_file_single_definition() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").expect("valid uuid");
        let defs = vec![BudgetDefinition {
            period: "Monthly".to_string(),
            account: "Expenses:Food".to_string(),
            amount: 500.0,
            currency: "$".to_string(),
        }];
        let result = serialize_budget_file(&defs, &id);
        assert!(result.contains("~ Monthly"));
        assert!(result.contains("Expenses:Food  $500.00"));
        assert!(result.contains("Assets:Checking"));
    }

    #[test]
    fn round_trip_single_definition() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").expect("valid uuid");
        let original = vec![BudgetDefinition {
            period: "Monthly".to_string(),
            account: "Expenses:Food".to_string(),
            amount: 500.0,
            currency: "$".to_string(),
        }];
        let serialized = serialize_budget_file(&original, &id);
        let parsed = parse_budget_file(&serialized);
        assert_eq!(original, parsed);
    }

    #[test]
    fn round_trip_multiple_definitions() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").expect("valid uuid");
        let original = vec![
            BudgetDefinition {
                period: "Monthly".to_string(),
                account: "Expenses:Food".to_string(),
                amount: 500.0,
                currency: "$".to_string(),
            },
            BudgetDefinition {
                period: "Monthly".to_string(),
                account: "Expenses:Rent".to_string(),
                amount: 1500.0,
                currency: "$".to_string(),
            },
            BudgetDefinition {
                period: "Yearly".to_string(),
                account: "Expenses:Insurance".to_string(),
                amount: 2400.0,
                currency: "$".to_string(),
            },
        ];
        let serialized = serialize_budget_file(&original, &id);
        let parsed = parse_budget_file(&serialized);
        assert_eq!(original, parsed);
    }

    #[test]
    fn round_trip_with_different_currencies() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").expect("valid uuid");
        let original = vec![
            BudgetDefinition {
                period: "Monthly".to_string(),
                account: "Expenses:Food".to_string(),
                amount: 500.50,
                currency: "$".to_string(),
            },
            BudgetDefinition {
                period: "Yearly".to_string(),
                account: "Expenses:Travel".to_string(),
                amount: 3000.00,
                currency: "€".to_string(),
            },
        ];
        let serialized = serialize_budget_file(&original, &id);
        let parsed = parse_budget_file(&serialized);
        assert_eq!(original, parsed);
    }
}
