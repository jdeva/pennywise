use std::path::Path;
use std::process::Command;

use crate::utils::error::AppError;
use crate::utils::validation::is_valid_username_char;

pub struct LedgerCli;

impl LedgerCli {
    pub fn balance(
        ledger_path: &Path,
        pivot_user: bool,
        filter_user: Option<&str>,
    ) -> Result<String, AppError> {
        let mut cmd = Command::new("ledger");
        cmd.arg("balance").arg("-f").arg(ledger_path);
        if pivot_user {
            cmd.arg("--pivot").arg("User");
        }
        if let Some(user) = filter_user {
            if user.is_empty() || !user.chars().all(is_valid_username_char) {
                return Err(AppError::BadRequest(
                    "Invalid user filter: must contain only alphanumeric characters, hyphens, or underscores".to_string(),
                ));
            }
            cmd.arg("--limit")
                .arg(format!("tag('User') =~ /{}/", user));
        }
        Self::execute(cmd)
    }

    pub fn register(
        ledger_path: &Path,
        filter_user: Option<&str>,
        filter_payee: Option<&str>,
        begin: Option<&str>,
        end: Option<&str>,
    ) -> Result<String, AppError> {
        let mut cmd = Command::new("ledger");
        // Wide account column so ledger doesn't truncate names like
        // `Expenses:Transport:Fuel` → `Exp:Transpor:Fuel`, which would break
        // the frontend's top-level-segment aggregation.
        cmd.arg("--account-width=80").arg("register").arg("-f").arg(ledger_path);
        if let Some(user) = filter_user {
            // Must match `validate_username`'s char set — anything else could
            // terminate the regex literal and inject a ledger value-expression.
            if user.is_empty() || !user.chars().all(is_valid_username_char) {
                return Err(AppError::BadRequest(
                    "Invalid user filter: must contain only alphanumeric characters, hyphens, or underscores".to_string(),
                ));
            }
            cmd.arg("--limit")
                .arg(format!("tag('User') =~ /{}/", user));
        }
        if let Some(payee) = filter_payee {
            // Accept only safe payee-search characters to prevent ledger
            // value-expression injection via regex specials.
            if payee.is_empty()
                || !payee
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' || c == '.')
            {
                return Err(AppError::BadRequest(
                    "Invalid payee filter: letters, digits, spaces, hyphens, underscores, or dots only".to_string(),
                ));
            }
            cmd.arg("--limit")
                .arg(format!("payee =~ /(?i){}/", payee));
        }
        if let Some(b) = begin {
            cmd.arg("--begin").arg(b);
        }
        if let Some(e) = end {
            cmd.arg("--end").arg(e);
        }
        Self::execute(cmd)
    }

    pub fn budget_balance(
        ledger_path: &Path,
        begin: Option<&str>,
        end: Option<&str>,
    ) -> Result<String, AppError> {
        let mut cmd = Command::new("ledger");
        cmd.arg("--budget").arg("balance").arg("-f").arg(ledger_path);
        if let Some(b) = begin {
            cmd.arg("--begin").arg(b);
        }
        if let Some(e) = end {
            cmd.arg("--end").arg(e);
        }
        Self::execute(cmd)
    }

    pub fn unbudgeted_balance(
        ledger_path: &Path,
        begin: Option<&str>,
        end: Option<&str>,
    ) -> Result<String, AppError> {
        let mut cmd = Command::new("ledger");
        cmd.arg("--unbudgeted").arg("balance").arg("-f").arg(ledger_path);
        if let Some(b) = begin {
            cmd.arg("--begin").arg(b);
        }
        if let Some(e) = end {
            cmd.arg("--end").arg(e);
        }
        Self::execute(cmd)
    }

    pub fn forecast_register(
        ledger_path: &Path,
        end_date: &str,
    ) -> Result<String, AppError> {
        let mut cmd = Command::new("ledger");
        cmd.arg("--account-width=80")
            .arg("--forecast")
            .arg(format!("d<[{}]", end_date))
            .arg("register")
            .arg("-f")
            .arg(ledger_path);
        Self::execute(cmd)
    }

    fn execute(mut cmd: Command) -> Result<String, AppError> {
        let output = cmd.output().map_err(|e| {
            AppError::Internal(format!("Failed to execute ledger-cli: {}", e))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::Internal(format!(
                "ledger-cli exited with {}: {}",
                output.status, stderr
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

#[cfg(test)]
impl LedgerCli {
    /// Builds the balance command without executing it — for test inspection.
    fn build_balance_command(
        ledger_path: &Path,
        pivot_user: bool,
        filter_user: Option<&str>,
    ) -> Command {
        let mut cmd = Command::new("ledger");
        cmd.arg("balance").arg("-f").arg(ledger_path);
        if pivot_user {
            cmd.arg("--pivot").arg("User");
        }
        if let Some(user) = filter_user {
            cmd.arg("--limit")
                .arg(format!("tag('User') =~ /{}/", user));
        }
        cmd
    }

    /// Builds the register command without executing it — for test inspection.
    fn build_register_command(
        ledger_path: &Path,
        filter_user: Option<&str>,
        filter_payee: Option<&str>,
        begin: Option<&str>,
        end: Option<&str>,
    ) -> Command {
        let mut cmd = Command::new("ledger");
        cmd.arg("--account-width=80").arg("register").arg("-f").arg(ledger_path);
        if let Some(user) = filter_user {
            cmd.arg("--limit")
                .arg(format!("tag('User') =~ /{}/", user));
        }
        if let Some(payee) = filter_payee {
            cmd.arg("--limit")
                .arg(format!("payee =~ /(?i){}/", payee));
        }
        if let Some(b) = begin {
            cmd.arg("--begin").arg(b);
        }
        if let Some(e) = end {
            cmd.arg("--end").arg(e);
        }
        cmd
    }

    /// Builds the budget balance command without executing it — for test inspection.
    pub fn build_budget_balance_command(
        ledger_path: &Path,
        begin: Option<&str>,
        end: Option<&str>,
    ) -> Command {
        let mut cmd = Command::new("ledger");
        cmd.arg("--budget").arg("balance").arg("-f").arg(ledger_path);
        if let Some(b) = begin {
            cmd.arg("--begin").arg(b);
        }
        if let Some(e) = end {
            cmd.arg("--end").arg(e);
        }
        cmd
    }

    /// Builds the unbudgeted balance command without executing it — for test inspection.
    pub fn build_unbudgeted_balance_command(
        ledger_path: &Path,
        begin: Option<&str>,
        end: Option<&str>,
    ) -> Command {
        let mut cmd = Command::new("ledger");
        cmd.arg("--unbudgeted").arg("balance").arg("-f").arg(ledger_path);
        if let Some(b) = begin {
            cmd.arg("--begin").arg(b);
        }
        if let Some(e) = end {
            cmd.arg("--end").arg(e);
        }
        cmd
    }

    /// Builds the forecast register command without executing it — for test inspection.
    pub fn build_forecast_register_command(
        ledger_path: &Path,
        end_date: &str,
    ) -> Command {
        let mut cmd = Command::new("ledger");
        cmd.arg("--account-width=80")
            .arg("--forecast")
            .arg(format!("d<[{}]", end_date))
            .arg("register")
            .arg("-f")
            .arg(ledger_path);
        cmd
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn get_args(cmd: &Command) -> Vec<String> {
        let debug = format!("{:?}", cmd);
        // Command debug format: "ledger" "balance" "-f" "/path"
        // Parse quoted args from the debug output
        debug
            .split('"')
            .enumerate()
            .filter(|(i, _)| i % 2 == 1) // odd indices are inside quotes
            .map(|(_, s)| s.replace("\\'", "'")) // unescape single quotes
            .collect()
    }

    #[test]
    fn test_balance_command_basic() {
        let path = PathBuf::from("/tmp/test.ledger");
        let cmd = LedgerCli::build_balance_command(&path, false, None);
        let args = get_args(&cmd);
        assert_eq!(args[0], "ledger");
        assert_eq!(args[1], "balance");
        assert_eq!(args[2], "-f");
        assert_eq!(args[3], "/tmp/test.ledger");
        assert_eq!(args.len(), 4);
    }

    #[test]
    fn test_balance_command_with_pivot() {
        let path = PathBuf::from("/tmp/test.ledger");
        let cmd = LedgerCli::build_balance_command(&path, true, None);
        let args = get_args(&cmd);
        assert_eq!(args[0], "ledger");
        assert_eq!(args[1], "balance");
        assert_eq!(args[2], "-f");
        assert_eq!(args[3], "/tmp/test.ledger");
        assert_eq!(args[4], "--pivot");
        assert_eq!(args[5], "User");
        assert_eq!(args.len(), 6);
    }

    #[test]
    fn test_register_command_basic() {
        let path = PathBuf::from("/tmp/test.ledger");
        let cmd = LedgerCli::build_register_command(&path, None, None, None, None);
        let args = get_args(&cmd);
        assert_eq!(args[0], "ledger");
        assert_eq!(args[1], "--account-width=80");
        assert_eq!(args[2], "register");
        assert_eq!(args[3], "-f");
        assert_eq!(args[4], "/tmp/test.ledger");
        assert_eq!(args.len(), 5);
    }

    #[test]
    fn test_register_command_with_user_filter() {
        let path = PathBuf::from("/tmp/test.ledger");
        let cmd = LedgerCli::build_register_command(&path, Some("alice"), None, None, None);
        let args = get_args(&cmd);
        assert_eq!(args[0], "ledger");
        assert_eq!(args[1], "--account-width=80");
        assert_eq!(args[2], "register");
        assert_eq!(args[3], "-f");
        assert_eq!(args[4], "/tmp/test.ledger");
        assert_eq!(args[5], "--limit");
        assert_eq!(args[6], "tag('User') =~ /alice/");
        assert_eq!(args.len(), 7);
    }

    #[test]
    fn test_budget_balance_command_no_dates() {
        let path = PathBuf::from("/tmp/test.ledger");
        let cmd = LedgerCli::build_budget_balance_command(&path, None, None);
        let args = get_args(&cmd);
        assert_eq!(args[0], "ledger");
        assert_eq!(args[1], "--budget");
        assert_eq!(args[2], "balance");
        assert_eq!(args[3], "-f");
        assert_eq!(args[4], "/tmp/test.ledger");
        assert_eq!(args.len(), 5);
    }

    #[test]
    fn test_budget_balance_command_with_dates() {
        let path = PathBuf::from("/tmp/test.ledger");
        let cmd = LedgerCli::build_budget_balance_command(&path, Some("2025-01-01"), Some("2025-06-30"));
        let args = get_args(&cmd);
        assert_eq!(args[0], "ledger");
        assert_eq!(args[1], "--budget");
        assert_eq!(args[2], "balance");
        assert_eq!(args[3], "-f");
        assert_eq!(args[4], "/tmp/test.ledger");
        assert_eq!(args[5], "--begin");
        assert_eq!(args[6], "2025-01-01");
        assert_eq!(args[7], "--end");
        assert_eq!(args[8], "2025-06-30");
        assert_eq!(args.len(), 9);
    }

    #[test]
    fn test_budget_balance_command_begin_only() {
        let path = PathBuf::from("/tmp/test.ledger");
        let cmd = LedgerCli::build_budget_balance_command(&path, Some("2025-01-01"), None);
        let args = get_args(&cmd);
        assert_eq!(args[0], "ledger");
        assert_eq!(args[1], "--budget");
        assert_eq!(args[2], "balance");
        assert_eq!(args[3], "-f");
        assert_eq!(args[4], "/tmp/test.ledger");
        assert_eq!(args[5], "--begin");
        assert_eq!(args[6], "2025-01-01");
        assert_eq!(args.len(), 7);
    }

    #[test]
    fn test_unbudgeted_balance_command_no_dates() {
        let path = PathBuf::from("/tmp/test.ledger");
        let cmd = LedgerCli::build_unbudgeted_balance_command(&path, None, None);
        let args = get_args(&cmd);
        assert_eq!(args[0], "ledger");
        assert_eq!(args[1], "--unbudgeted");
        assert_eq!(args[2], "balance");
        assert_eq!(args[3], "-f");
        assert_eq!(args[4], "/tmp/test.ledger");
        assert_eq!(args.len(), 5);
    }

    #[test]
    fn test_unbudgeted_balance_command_with_dates() {
        let path = PathBuf::from("/tmp/test.ledger");
        let cmd = LedgerCli::build_unbudgeted_balance_command(&path, Some("2025-01-01"), Some("2025-12-31"));
        let args = get_args(&cmd);
        assert_eq!(args[0], "ledger");
        assert_eq!(args[1], "--unbudgeted");
        assert_eq!(args[2], "balance");
        assert_eq!(args[3], "-f");
        assert_eq!(args[4], "/tmp/test.ledger");
        assert_eq!(args[5], "--begin");
        assert_eq!(args[6], "2025-01-01");
        assert_eq!(args[7], "--end");
        assert_eq!(args[8], "2025-12-31");
        assert_eq!(args.len(), 9);
    }

    #[test]
    fn test_unbudgeted_balance_command_end_only() {
        let path = PathBuf::from("/tmp/test.ledger");
        let cmd = LedgerCli::build_unbudgeted_balance_command(&path, None, Some("2025-12-31"));
        let args = get_args(&cmd);
        assert_eq!(args[0], "ledger");
        assert_eq!(args[1], "--unbudgeted");
        assert_eq!(args[2], "balance");
        assert_eq!(args[3], "-f");
        assert_eq!(args[4], "/tmp/test.ledger");
        assert_eq!(args[5], "--end");
        assert_eq!(args[6], "2025-12-31");
        assert_eq!(args.len(), 7);
    }

    #[test]
    fn test_forecast_register_command() {
        let path = PathBuf::from("/tmp/test.ledger");
        let cmd = LedgerCli::build_forecast_register_command(&path, "2025-12-31");
        let args = get_args(&cmd);
        assert_eq!(args[0], "ledger");
        assert_eq!(args[1], "--account-width=80");
        assert_eq!(args[2], "--forecast");
        assert_eq!(args[3], "d<[2025-12-31]");
        assert_eq!(args[4], "register");
        assert_eq!(args[5], "-f");
        assert_eq!(args[6], "/tmp/test.ledger");
        assert_eq!(args.len(), 7);
    }
}
