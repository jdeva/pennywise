use chrono::NaiveDate;

use crate::models::{
    AccountType, AddAccountRequest, AddCategoryRequest, ChangePasswordRequest,
    CreateBudgetRequest, CreateWorkspaceRequest, DeleteAccountRequest, OpeningBalanceRequest,
    PostTransactionRequest, RegisterRequest, ShareWorkspaceRequest, UpdateBudgetRequest,
    UpdateWorkspaceRequest, UpdateProfileRequest, ValidationDetail,
};

pub fn is_valid_username_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

fn has_forbidden_control_chars(s: &str, extra_forbidden: &[char]) -> bool {
    s.chars().any(|c| {
        extra_forbidden.contains(&c)
            || c == '\n'
            || c == '\r'
            || (c.is_control() && c != '\t')
    })
}

pub fn validate_username(username: &str) -> Result<(), Vec<ValidationDetail>> {
    let trimmed = username.trim();
    let mut errors = Vec::new();

    if trimmed.len() < 3 || trimmed.len() > 32 {
        errors.push(ValidationDetail {
            field: "username".to_string(),
            message: "Must be 3-32 characters".to_string(),
        });
    }

    if !trimmed.is_empty() && !trimmed.chars().all(is_valid_username_char) {
        errors.push(ValidationDetail {
            field: "username".to_string(),
            message: "Must contain only alphanumeric characters, hyphens, or underscores"
                .to_string(),
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn validate_email(email: &str) -> Result<(), Vec<ValidationDetail>> {
    let trimmed = email.trim();
    let mut errors = Vec::new();

    let valid = if let Some(at_pos) = trimmed.find('@') {
        let local = &trimmed[..at_pos];
        let domain = &trimmed[at_pos + 1..];
        !local.is_empty() && !domain.is_empty() && domain.contains('.') && !trimmed[at_pos + 1..].contains('@')
    } else {
        false
    };

    if !valid {
        errors.push(ValidationDetail {
            field: "email".to_string(),
            message: "Invalid email format".to_string(),
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn validate_password(password: &str) -> Result<(), Vec<ValidationDetail>> {
    let mut errors = Vec::new();

    if password.len() < 8 {
        errors.push(ValidationDetail {
            field: "password".to_string(),
            message: "Must be at least 8 characters".to_string(),
        });
    }

    if !password.chars().any(|c| c.is_ascii_uppercase()) {
        errors.push(ValidationDetail {
            field: "password".to_string(),
            message: "Must contain at least one uppercase letter".to_string(),
        });
    }

    if !password.chars().any(|c| c.is_ascii_lowercase()) {
        errors.push(ValidationDetail {
            field: "password".to_string(),
            message: "Must contain at least one lowercase letter".to_string(),
        });
    }

    if !password.chars().any(|c| c.is_ascii_digit()) {
        errors.push(ValidationDetail {
            field: "password".to_string(),
            message: "Must contain at least one digit".to_string(),
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn validate_register(req: &RegisterRequest) -> Result<(), Vec<ValidationDetail>> {
    let mut errors = Vec::new();

    if let Err(mut e) = validate_username(&req.username) {
        errors.append(&mut e);
    }
    if let Err(mut e) = validate_email(&req.email) {
        errors.append(&mut e);
    }
    if let Err(mut e) = validate_password(&req.password) {
        errors.append(&mut e);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn validate_update_profile(req: &UpdateProfileRequest) -> Result<(), Vec<ValidationDetail>> {
    let mut errors = Vec::new();

    if let Some(ref username) = req.username {
        if let Err(mut e) = validate_username(username) {
            errors.append(&mut e);
        }
    }
    if let Some(ref email) = req.email {
        if let Err(mut e) = validate_email(email) {
            errors.append(&mut e);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn validate_change_password(req: &ChangePasswordRequest) -> Result<(), Vec<ValidationDetail>> {
    validate_password(&req.new_password)
}

pub fn validate_workspace_name(name: &str) -> Result<(), Vec<ValidationDetail>> {
    let trimmed = name.trim();
    let mut errors = Vec::new();

    if trimmed.is_empty() {
        errors.push(ValidationDetail {
            field: "name".to_string(),
            message: "Must not be empty".to_string(),
        });
    } else if trimmed.len() > 128 {
        errors.push(ValidationDetail {
            field: "name".to_string(),
            message: "Must be 128 characters or fewer".to_string(),
        });
    }

    // Workspace name is written into ledger file header comments — a newline
    // could break out of the comment and inject fake transactions.
    if has_forbidden_control_chars(trimmed, &[]) {
        errors.push(ValidationDetail {
            field: "name".to_string(),
            message: "Must not contain newlines or other control characters".to_string(),
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn validate_create_workspace(req: &CreateWorkspaceRequest) -> Result<(), Vec<ValidationDetail>> {
    validate_workspace_name(&req.name)
}

pub fn validate_update_workspace(req: &UpdateWorkspaceRequest) -> Result<(), Vec<ValidationDetail>> {
    validate_workspace_name(&req.name)
}

pub fn validate_share_workspace(req: &ShareWorkspaceRequest) -> Result<(), Vec<ValidationDetail>> {
    validate_username(&req.username)
}

fn is_valid_category_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == ':' || c == ' ' || c == '-' || c == '_'
}

pub fn validate_category_name(name: &str) -> Result<(), Vec<ValidationDetail>> {
    let trimmed = name.trim();
    let mut errors = Vec::new();

    if trimmed.is_empty() {
        errors.push(ValidationDetail {
            field: "category".to_string(),
            message: "Must not be empty".to_string(),
        });
    } else {
        if trimmed.len() > 256 {
            errors.push(ValidationDetail {
                field: "category".to_string(),
                message: "Must be 256 characters or fewer".to_string(),
            });
        }
        if !trimmed.chars().all(is_valid_category_char) {
            errors.push(ValidationDetail {
                field: "category".to_string(),
                message: "Must contain only alphanumeric characters, colons, spaces, hyphens, or underscores".to_string(),
            });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn validate_transaction_date(date: &str) -> Result<NaiveDate, Vec<ValidationDetail>> {
    let trimmed = date.trim();
    match NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        Ok(d) => Ok(d),
        Err(_) => Err(vec![ValidationDetail {
            field: "date".to_string(),
            message: "Must be a valid date in YYYY-MM-DD format".to_string(),
        }]),
    }
}

pub fn validate_payee(payee: &str) -> Result<(), Vec<ValidationDetail>> {
    let trimmed = payee.trim();
    let mut errors = Vec::new();

    if trimmed.is_empty() {
        errors.push(ValidationDetail {
            field: "payee".to_string(),
            message: "Must not be empty".to_string(),
        });
    } else if trimmed.len() > 256 {
        errors.push(ValidationDetail {
            field: "payee".to_string(),
            message: "Must be 256 characters or fewer".to_string(),
        });
    }

    // Payee is appended verbatim to the ledger transaction header line;
    // newline starts a new entry, semicolon starts a comment.
    if has_forbidden_control_chars(trimmed, &[';']) {
        errors.push(ValidationDetail {
            field: "payee".to_string(),
            message: "Must not contain newlines, semicolons, or other control characters".to_string(),
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn validate_amount(amount: &str) -> Result<f64, Vec<ValidationDetail>> {
    let trimmed = amount.trim();

    let parsed: f64 = match trimmed.parse() {
        Ok(v) => v,
        Err(_) => {
            return Err(vec![ValidationDetail {
                field: "amount".to_string(),
                message: "Must be a valid decimal number".to_string(),
            }]);
        }
    };

    if !parsed.is_finite() {
        return Err(vec![ValidationDetail {
            field: "amount".to_string(),
            message: "Must be a valid finite number".to_string(),
        }]);
    }

    if parsed <= 0.0 {
        return Err(vec![ValidationDetail {
            field: "amount".to_string(),
            message: "Must be a positive number".to_string(),
        }]);
    }

    // Check at most 2 decimal places by inspecting the string
    if let Some(dot_pos) = trimmed.find('.') {
        let decimals = &trimmed[dot_pos + 1..];
        if decimals.len() > 2 {
            return Err(vec![ValidationDetail {
                field: "amount".to_string(),
                message: "Must have at most 2 decimal places".to_string(),
            }]);
        }
    }

    Ok(parsed)
}

pub fn validate_ledger_account_name(
    name: &str,
    field: &str,
) -> Result<(), Vec<ValidationDetail>> {
    let trimmed = name.trim();
    let mut errors = Vec::new();

    if trimmed.is_empty() {
        errors.push(ValidationDetail {
            field: field.to_string(),
            message: "Must not be empty".to_string(),
        });
    } else {
        if trimmed.len() > 256 {
            errors.push(ValidationDetail {
                field: field.to_string(),
                message: "Must be 256 characters or fewer".to_string(),
            });
        }
        if !trimmed.chars().all(is_valid_category_char) {
            errors.push(ValidationDetail {
                field: field.to_string(),
                message: "Must contain only alphanumeric characters, colons, spaces, hyphens, or underscores".to_string(),
            });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn validate_chart_account_name(name: &str) -> Result<(), Vec<ValidationDetail>> {
    validate_ledger_account_name(name, "account_name")
}

pub fn validate_account_type(type_str: &str) -> Result<AccountType, Vec<ValidationDetail>> {
    match type_str {
        "assets" => Ok(AccountType::Assets),
        "expenses" => Ok(AccountType::Expenses),
        "income" => Ok(AccountType::Income),
        "liabilities" => Ok(AccountType::Liabilities),
        "equity" => Ok(AccountType::Equity),
        _ => Err(vec![ValidationDetail {
            field: "account_type".to_string(),
            message: "Must be one of: assets, expenses, income, liabilities, equity".to_string(),
        }]),
    }
}

pub fn validate_add_chart_account(req: &AddAccountRequest) -> Result<(), Vec<ValidationDetail>> {
    let mut errors = Vec::new();

    if let Err(mut e) = validate_chart_account_name(&req.name) {
        errors.append(&mut e);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn validate_delete_chart_account(req: &DeleteAccountRequest) -> Result<(), Vec<ValidationDetail>> {
    let mut errors = Vec::new();

    if let Err(mut e) = validate_chart_account_name(&req.name) {
        errors.append(&mut e);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn validate_post_transaction(
    req: &PostTransactionRequest,
) -> Result<(), Vec<ValidationDetail>> {
    let mut errors = Vec::new();

    if let Err(mut e) = validate_transaction_date(&req.date) {
        errors.append(&mut e);
    }
    if let Err(mut e) = validate_payee(&req.payee) {
        errors.append(&mut e);
    }
    if let Err(mut e) = validate_amount(&req.amount) {
        errors.append(&mut e);
    }
    if let Err(mut e) = validate_ledger_account_name(&req.debit_account, "debit_account") {
        errors.append(&mut e);
    }
    if let Err(mut e) = validate_ledger_account_name(&req.credit_account, "credit_account") {
        errors.append(&mut e);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn validate_add_category(req: &AddCategoryRequest) -> Result<(), Vec<ValidationDetail>> {
    validate_category_name(&req.name)
}

/// Validates opening balance amount: non-negative decimal with up to 2 decimal places.
/// Unlike validate_amount (which requires > 0), this accepts zero.
pub fn validate_opening_balance_amount(amount: &str) -> Result<f64, Vec<ValidationDetail>> {
    let trimmed = amount.trim();
    match trimmed.parse::<f64>() {
        Ok(val) if val < 0.0 => Err(vec![ValidationDetail {
            field: "amount".to_string(),
            message: "Amount must be non-negative".to_string(),
        }]),
        Ok(val) if !val.is_finite() => Err(vec![ValidationDetail {
            field: "amount".to_string(),
            message: "Amount must be a valid number".to_string(),
        }]),
        Ok(val) => {
            // Check at most 2 decimal places
            if let Some(dot_pos) = trimmed.find('.') {
                let decimals = trimmed.len() - dot_pos - 1;
                if decimals > 2 {
                    return Err(vec![ValidationDetail {
                        field: "amount".to_string(),
                        message: "Amount must have at most 2 decimal places".to_string(),
                    }]);
                }
            }
            Ok(val)
        }
        Err(_) => Err(vec![ValidationDetail {
            field: "amount".to_string(),
            message: "Amount must be a valid decimal number".to_string(),
        }]),
    }
}

pub fn validate_opening_balance(req: &OpeningBalanceRequest) -> Result<(), Vec<ValidationDetail>> {
    let mut errors = vec![];

    // Validate amount (required) — non-negative, up to 2 decimal places
    if let Err(mut details) = validate_opening_balance_amount(&req.amount) {
        errors.append(&mut details);
    }

    // Validate date (optional) — reuse existing validate_transaction_date
    if let Some(ref date) = req.date {
        if let Err(mut details) = validate_transaction_date(date) {
            errors.append(&mut details);
        }
    }

    // Validate account_name (optional) — reuse existing validate_ledger_account_name
    if let Some(ref account) = req.account_name {
        if let Err(mut details) = validate_ledger_account_name(account, "account_name") {
            errors.append(&mut details);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validates a budget period expression.
/// Accepts: "Monthly", "Quarterly", "Yearly", "Weekly", "Biweekly", "Daily",
/// or "Every N days/weeks/months/years" (case-insensitive, N is a positive integer).
pub fn validate_period_expression(period: &str) -> Result<(), Vec<ValidationDetail>> {
    let trimmed = period.trim();
    let lower = trimmed.to_lowercase();

    let valid = matches!(
        lower.as_str(),
        "monthly" | "quarterly" | "yearly" | "weekly" | "biweekly" | "daily"
    ) || {
        // Check "every N unit" pattern
        if let Some(rest) = lower.strip_prefix("every ") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            parts.len() == 2
                && parts[0].parse::<u64>().map_or(false, |n| n > 0)
                && matches!(parts[1], "days" | "weeks" | "months" | "years")
        } else {
            false
        }
    };

    if valid {
        Ok(())
    } else {
        Err(vec![ValidationDetail {
            field: "period".to_string(),
            message: "Must be one of: Monthly, Quarterly, Yearly, Weekly, Biweekly, Daily, or 'Every N days/weeks/months/years'".to_string(),
        }])
    }
}

pub fn validate_create_budget(req: &CreateBudgetRequest) -> Result<(), Vec<ValidationDetail>> {
    let mut errors = Vec::new();

    if let Err(mut e) = validate_period_expression(&req.period) {
        errors.append(&mut e);
    }
    if let Err(mut e) = validate_ledger_account_name(&req.account, "account") {
        errors.append(&mut e);
    }
    if let Err(mut e) = validate_amount(&req.amount) {
        errors.append(&mut e);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn validate_update_budget(req: &UpdateBudgetRequest) -> Result<(), Vec<ValidationDetail>> {
    let mut errors = Vec::new();

    if let Err(mut e) = validate_period_expression(&req.period) {
        errors.append(&mut e);
    }
    if let Err(mut e) = validate_ledger_account_name(&req.account, "account") {
        errors.append(&mut e);
    }
    if let Err(mut e) = validate_amount(&req.amount) {
        errors.append(&mut e);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_username_accepted() {
        assert!(validate_username("alice").is_ok());
        assert!(validate_username("Bob_123").is_ok());
        assert!(validate_username("a-b").is_ok());
        assert!(validate_username("abc").is_ok());
    }

    #[test]
    fn username_too_short_rejected() {
        assert!(validate_username("ab").is_err());
        assert!(validate_username("").is_err());
    }

    #[test]
    fn username_too_long_rejected() {
        let long = "a".repeat(33);
        assert!(validate_username(&long).is_err());
    }

    #[test]
    fn username_invalid_chars_rejected() {
        assert!(validate_username("no spaces").is_err());
        assert!(validate_username("bad@char").is_err());
    }

    #[test]
    fn username_whitespace_trimmed() {
        assert!(validate_username("  alice  ").is_ok());
    }

    #[test]
    fn valid_email_accepted() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("a@b.c").is_ok());
    }

    #[test]
    fn email_missing_at_rejected() {
        assert!(validate_email("noatsign.com").is_err());
    }

    #[test]
    fn email_empty_local_rejected() {
        assert!(validate_email("@domain.com").is_err());
    }

    #[test]
    fn email_empty_domain_rejected() {
        assert!(validate_email("user@").is_err());
    }

    #[test]
    fn email_domain_no_dot_rejected() {
        assert!(validate_email("user@domain").is_err());
    }

    #[test]
    fn email_whitespace_trimmed() {
        assert!(validate_email("  user@example.com  ").is_ok());
    }

    #[test]
    fn valid_password_accepted() {
        assert!(validate_password("Abcdefg1").is_ok());
        assert!(validate_password("P@ssw0rd").is_ok());
    }

    #[test]
    fn password_too_short_rejected() {
        assert!(validate_password("Ab1").is_err());
    }

    #[test]
    fn password_no_uppercase_rejected() {
        assert!(validate_password("abcdefg1").is_err());
    }

    #[test]
    fn password_no_lowercase_rejected() {
        assert!(validate_password("ABCDEFG1").is_err());
    }

    #[test]
    fn password_no_digit_rejected() {
        assert!(validate_password("Abcdefgh").is_err());
    }

    #[test]
    fn validate_register_collects_all_errors() {
        let req = RegisterRequest {
            username: "".to_string(),
            email: "bad".to_string(),
            password: "short".to_string(),
        };
        let errs = validate_register(&req).unwrap_err();
        assert!(errs.len() >= 3);
    }

    #[test]
    fn validate_update_profile_skips_none_fields() {
        let req = UpdateProfileRequest {
            username: None,
            email: None,
        };
        assert!(validate_update_profile(&req).is_ok());
    }

    #[test]
    fn validate_change_password_checks_new_password() {
        let req = ChangePasswordRequest {
            current_password: "anything".to_string(),
            new_password: "weak".to_string(),
        };
        assert!(validate_change_password(&req).is_err());
    }

    #[test]
    fn valid_account_name_accepted() {
        assert!(validate_workspace_name("Checking").is_ok());
        assert!(validate_workspace_name("a").is_ok());
        assert!(validate_workspace_name(&"x".repeat(128)).is_ok());
    }

    #[test]
    fn account_name_empty_rejected() {
        assert!(validate_workspace_name("").is_err());
        assert!(validate_workspace_name("   ").is_err());
    }

    #[test]
    fn account_name_too_long_rejected() {
        assert!(validate_workspace_name(&"x".repeat(129)).is_err());
    }

    #[test]
    fn account_name_whitespace_trimmed() {
        assert!(validate_workspace_name("  Savings  ").is_ok());
    }

    // --- validate_opening_balance_amount tests ---

    #[test]
    fn opening_balance_amount_zero_accepted() {
        assert!(validate_opening_balance_amount("0").is_ok());
        assert!(validate_opening_balance_amount("0.00").is_ok());
        assert!(validate_opening_balance_amount("0.0").is_ok());
    }

    #[test]
    fn opening_balance_amount_positive_accepted() {
        let result = validate_opening_balance_amount("100.50");
        assert!(result.is_ok());
        assert!((result.unwrap() - 100.50).abs() < f64::EPSILON);
    }

    #[test]
    fn opening_balance_amount_integer_accepted() {
        assert!(validate_opening_balance_amount("42").is_ok());
    }

    #[test]
    fn opening_balance_amount_negative_rejected() {
        let errs = validate_opening_balance_amount("-1.00").unwrap_err();
        assert_eq!(errs[0].field, "amount");
        assert!(errs[0].message.contains("non-negative"));
    }

    #[test]
    fn opening_balance_amount_non_numeric_rejected() {
        let errs = validate_opening_balance_amount("abc").unwrap_err();
        assert_eq!(errs[0].field, "amount");
        assert!(errs[0].message.contains("valid decimal"));
    }

    #[test]
    fn opening_balance_amount_too_many_decimals_rejected() {
        let errs = validate_opening_balance_amount("10.123").unwrap_err();
        assert_eq!(errs[0].field, "amount");
        assert!(errs[0].message.contains("2 decimal"));
    }

    #[test]
    fn opening_balance_amount_whitespace_trimmed() {
        assert!(validate_opening_balance_amount("  50.00  ").is_ok());
    }

    // --- validate_opening_balance tests ---

    #[test]
    fn opening_balance_valid_all_fields() {
        let req = OpeningBalanceRequest {
            amount: "100.00".to_string(),
            date: Some("2025-01-01".to_string()),
            account_name: Some("Assets:Checking".to_string()),
        };
        assert!(validate_opening_balance(&req).is_ok());
    }

    #[test]
    fn opening_balance_valid_amount_only() {
        let req = OpeningBalanceRequest {
            amount: "0.00".to_string(),
            date: None,
            account_name: None,
        };
        assert!(validate_opening_balance(&req).is_ok());
    }

    #[test]
    fn opening_balance_invalid_amount_rejected() {
        let req = OpeningBalanceRequest {
            amount: "-5".to_string(),
            date: None,
            account_name: None,
        };
        let errs = validate_opening_balance(&req).unwrap_err();
        assert!(errs.iter().any(|e| e.field == "amount"));
    }

    #[test]
    fn opening_balance_invalid_date_rejected() {
        let req = OpeningBalanceRequest {
            amount: "10.00".to_string(),
            date: Some("not-a-date".to_string()),
            account_name: None,
        };
        let errs = validate_opening_balance(&req).unwrap_err();
        assert!(errs.iter().any(|e| e.field == "date"));
    }

    #[test]
    fn opening_balance_invalid_account_name_rejected() {
        let req = OpeningBalanceRequest {
            amount: "10.00".to_string(),
            date: None,
            account_name: Some("   ".to_string()),
        };
        let errs = validate_opening_balance(&req).unwrap_err();
        assert!(errs.iter().any(|e| e.field == "account_name"));
    }

    #[test]
    fn opening_balance_collects_all_errors() {
        let req = OpeningBalanceRequest {
            amount: "abc".to_string(),
            date: Some("bad".to_string()),
            account_name: Some("".to_string()),
        };
        let errs = validate_opening_balance(&req).unwrap_err();
        assert!(errs.len() >= 3);
        assert!(errs.iter().any(|e| e.field == "amount"));
        assert!(errs.iter().any(|e| e.field == "date"));
        assert!(errs.iter().any(|e| e.field == "account_name"));
    }

    // --- Chart account name validation tests ---

    #[test]
    fn chart_account_name_valid_accepted() {
        assert!(validate_chart_account_name("Expenses:Food:Groceries").is_ok());
        assert!(validate_chart_account_name("a").is_ok());
        assert!(validate_chart_account_name("Assets:Checking").is_ok());
        assert!(validate_chart_account_name("Income:My-Job_2025").is_ok());
        assert!(validate_chart_account_name(&"x".repeat(256)).is_ok());
    }

    #[test]
    fn chart_account_name_empty_rejected() {
        let errs = validate_chart_account_name("").unwrap_err();
        assert_eq!(errs[0].field, "account_name");
    }

    #[test]
    fn chart_account_name_whitespace_only_rejected() {
        let errs = validate_chart_account_name("   ").unwrap_err();
        assert_eq!(errs[0].field, "account_name");
    }

    #[test]
    fn chart_account_name_too_long_rejected() {
        let errs = validate_chart_account_name(&"x".repeat(257)).unwrap_err();
        assert_eq!(errs[0].field, "account_name");
    }

    #[test]
    fn chart_account_name_invalid_chars_rejected() {
        let errs = validate_chart_account_name("Expenses:Food@Home").unwrap_err();
        assert_eq!(errs[0].field, "account_name");
    }

    #[test]
    fn chart_account_name_whitespace_trimmed() {
        assert!(validate_chart_account_name("  Expenses:Food  ").is_ok());
    }

    // --- Account type validation tests ---

    #[test]
    fn account_type_valid_values_accepted() {
        assert!(validate_account_type("assets").is_ok());
        assert!(validate_account_type("expenses").is_ok());
        assert!(validate_account_type("income").is_ok());
        assert!(validate_account_type("liabilities").is_ok());
        assert!(validate_account_type("equity").is_ok());
    }

    #[test]
    fn account_type_returns_correct_variant() {
        assert_eq!(validate_account_type("assets").unwrap(), AccountType::Assets);
        assert_eq!(validate_account_type("expenses").unwrap(), AccountType::Expenses);
        assert_eq!(validate_account_type("income").unwrap(), AccountType::Income);
        assert_eq!(validate_account_type("liabilities").unwrap(), AccountType::Liabilities);
        assert_eq!(validate_account_type("equity").unwrap(), AccountType::Equity);
    }

    #[test]
    fn account_type_invalid_rejected() {
        let errs = validate_account_type("savings").unwrap_err();
        assert_eq!(errs[0].field, "account_type");
    }

    #[test]
    fn account_type_case_sensitive() {
        assert!(validate_account_type("Assets").is_err());
        assert!(validate_account_type("EXPENSES").is_err());
    }

    #[test]
    fn account_type_empty_rejected() {
        let errs = validate_account_type("").unwrap_err();
        assert_eq!(errs[0].field, "account_type");
    }

    // --- Composite chart account validation tests ---

    #[test]
    fn validate_add_chart_account_valid() {
        let req = AddAccountRequest {
            name: "Expenses:Food".to_string(),
            account_type: AccountType::Expenses,
        };
        assert!(validate_add_chart_account(&req).is_ok());
    }

    #[test]
    fn validate_add_chart_account_invalid_name() {
        let req = AddAccountRequest {
            name: "".to_string(),
            account_type: AccountType::Expenses,
        };
        let errs = validate_add_chart_account(&req).unwrap_err();
        assert!(errs.iter().any(|e| e.field == "account_name"));
    }

    #[test]
    fn validate_delete_chart_account_valid() {
        let req = DeleteAccountRequest {
            name: "Expenses:Food".to_string(),
            account_type: AccountType::Expenses,
        };
        assert!(validate_delete_chart_account(&req).is_ok());
    }

    #[test]
    fn validate_delete_chart_account_invalid_name() {
        let req = DeleteAccountRequest {
            name: "   ".to_string(),
            account_type: AccountType::Income,
        };
        let errs = validate_delete_chart_account(&req).unwrap_err();
        assert!(errs.iter().any(|e| e.field == "account_name"));
    }

    // --- Budget period expression validation tests ---

    #[test]
    fn period_expression_simple_keywords_accepted() {
        assert!(validate_period_expression("Monthly").is_ok());
        assert!(validate_period_expression("Quarterly").is_ok());
        assert!(validate_period_expression("Yearly").is_ok());
        assert!(validate_period_expression("Weekly").is_ok());
        assert!(validate_period_expression("Biweekly").is_ok());
        assert!(validate_period_expression("Daily").is_ok());
    }

    #[test]
    fn period_expression_case_insensitive() {
        assert!(validate_period_expression("monthly").is_ok());
        assert!(validate_period_expression("MONTHLY").is_ok());
        assert!(validate_period_expression("mOnThLy").is_ok());
    }

    #[test]
    fn period_expression_every_n_accepted() {
        assert!(validate_period_expression("Every 2 weeks").is_ok());
        assert!(validate_period_expression("Every 1 days").is_ok());
        assert!(validate_period_expression("Every 3 months").is_ok());
        assert!(validate_period_expression("Every 12 years").is_ok());
    }

    #[test]
    fn period_expression_every_n_case_insensitive() {
        assert!(validate_period_expression("every 2 Weeks").is_ok());
        assert!(validate_period_expression("EVERY 3 MONTHS").is_ok());
    }

    #[test]
    fn period_expression_whitespace_trimmed() {
        assert!(validate_period_expression("  Monthly  ").is_ok());
        assert!(validate_period_expression("  Every 2 weeks  ").is_ok());
    }

    #[test]
    fn period_expression_empty_rejected() {
        assert!(validate_period_expression("").is_err());
        assert!(validate_period_expression("   ").is_err());
    }

    #[test]
    fn period_expression_invalid_keyword_rejected() {
        let errs = validate_period_expression("Annually").unwrap_err();
        assert_eq!(errs[0].field, "period");
    }

    #[test]
    fn period_expression_every_zero_rejected() {
        assert!(validate_period_expression("Every 0 days").is_err());
    }

    #[test]
    fn period_expression_every_negative_rejected() {
        assert!(validate_period_expression("Every -1 days").is_err());
    }

    #[test]
    fn period_expression_every_invalid_unit_rejected() {
        assert!(validate_period_expression("Every 2 fortnights").is_err());
    }

    #[test]
    fn period_expression_every_missing_number_rejected() {
        assert!(validate_period_expression("Every weeks").is_err());
    }

    // --- Budget composite validation tests ---

    #[test]
    fn validate_create_budget_valid() {
        let req = CreateBudgetRequest {
            period: "Monthly".to_string(),
            account: "Expenses:Food".to_string(),
            amount: "500.00".to_string(),
            currency: None,
        };
        assert!(validate_create_budget(&req).is_ok());
    }

    #[test]
    fn validate_create_budget_collects_all_errors() {
        let req = CreateBudgetRequest {
            period: "".to_string(),
            account: "   ".to_string(),
            amount: "-5".to_string(),
            currency: None,
        };
        let errs = validate_create_budget(&req).unwrap_err();
        assert!(errs.iter().any(|e| e.field == "period"));
        assert!(errs.iter().any(|e| e.field == "account"));
        assert!(errs.iter().any(|e| e.field == "amount"));
    }

    #[test]
    fn validate_update_budget_valid() {
        let req = UpdateBudgetRequest {
            period: "Every 2 weeks".to_string(),
            account: "Expenses:Rent".to_string(),
            amount: "1500.00".to_string(),
            currency: Some("$".to_string()),
        };
        assert!(validate_update_budget(&req).is_ok());
    }

    #[test]
    fn validate_update_budget_collects_all_errors() {
        let req = UpdateBudgetRequest {
            period: "invalid".to_string(),
            account: "".to_string(),
            amount: "abc".to_string(),
            currency: None,
        };
        let errs = validate_update_budget(&req).unwrap_err();
        assert!(errs.iter().any(|e| e.field == "period"));
        assert!(errs.iter().any(|e| e.field == "account"));
        assert!(errs.iter().any(|e| e.field == "amount"));
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use chrono::Datelike;
    use proptest::prelude::*;

    // ---------------------------------------------------------------
    // Feature: transaction-ledger-api, Property 5: Category and ledger account name validation
    // **Validates: Requirements 2.5, 2.6, 2.7, 5.7, 5.8**
    // ---------------------------------------------------------------

    /// Strategy for generating valid category characters.
    fn valid_category_char_strategy() -> impl Strategy<Value = char> {
        prop::char::ranges(
            vec![
                'a'..='z',
                'A'..='Z',
                '0'..='9',
                ':'..=':',
                ' '..=' ',
                '-'..='-',
                '_'..='_',
            ]
            .into(),
        )
    }

    /// Strategy for generating valid category names (1-256 valid chars, not all whitespace).
    fn valid_category_name_strategy() -> impl Strategy<Value = String> {
        // Generate at least one non-space valid char to avoid whitespace-only strings
        (
            prop::collection::vec(valid_category_char_strategy(), 0..=127),
            prop::char::ranges(
                vec![
                    'a'..='z',
                    'A'..='Z',
                    '0'..='9',
                    ':'..=':',
                    '-'..='-',
                    '_'..='_',
                ]
                .into(),
            ),
            prop::collection::vec(valid_category_char_strategy(), 0..=127),
        )
            .prop_map(|(prefix, mid, suffix)| {
                let mut s: String = prefix.into_iter().collect();
                s.push(mid);
                s.extend(suffix.into_iter());
                s
            })
            .prop_filter("must be 1-256 chars after trim", |s| {
                let t = s.trim();
                !t.is_empty() && t.len() <= 256
            })
    }

    proptest! {
        #[test]
        fn prop_valid_category_name_accepted(name in valid_category_name_strategy()) {
            prop_assert!(validate_category_name(&name).is_ok(),
                "Expected valid category name to be accepted: {:?}", name);
        }

        #[test]
        fn prop_empty_or_whitespace_category_name_rejected(
            spaces in " {0,20}"
        ) {
            prop_assert!(validate_category_name(&spaces).is_err(),
                "Expected empty/whitespace category name to be rejected");
        }

        #[test]
        fn prop_too_long_category_name_rejected(
            extra in prop::collection::vec(valid_category_char_strategy(), 255..=298)
        ) {
            // Bookend with non-space chars so trim() cannot shrink below 257
            let middle: String = extra.into_iter().collect();
            let name = format!("A{}A", middle);
            prop_assert!(validate_category_name(&name).is_err(),
                "Expected too-long category name to be rejected");
        }

        #[test]
        fn prop_invalid_char_category_name_rejected(
            prefix in "[a-zA-Z]{1,5}",
            bad_char in prop::char::ranges(vec![
                '!'..='!', '@'..='@', '#'..='#', '$'..='$', '%'..='%',
                '&'..='&', '*'..='*', '('..='(', ')'..=')', '+'..='+',
                '='..='=', '['..='[', ']'..=']', '{'..='{', '}'..='}',
                '|'..='|', '\\'..='\\', '/'..='/', '<'..='<', '>'..='>',
                '?'..='?', '~'..='~', '`'..='`', '"'..='"', '\''..='\'',
                ','..=',', '.'..='.', ';'..=';',
            ].into()),
            suffix in "[a-zA-Z]{0,5}",
        ) {
            let name = format!("{}{}{}", prefix, bad_char, suffix);
            prop_assert!(validate_category_name(&name).is_err(),
                "Expected category name with invalid char '{}' to be rejected", bad_char);
        }

        // Same rules apply to validate_ledger_account_name
        #[test]
        fn prop_valid_ledger_account_name_accepted(name in valid_category_name_strategy()) {
            prop_assert!(validate_ledger_account_name(&name, "debit_account").is_ok(),
                "Expected valid ledger account name to be accepted: {:?}", name);
        }

        #[test]
        fn prop_empty_ledger_account_name_rejected(
            spaces in " {0,20}"
        ) {
            prop_assert!(validate_ledger_account_name(&spaces, "debit_account").is_err(),
                "Expected empty/whitespace ledger account name to be rejected");
        }

        #[test]
        fn prop_ledger_account_name_uses_field_param(
            field in "(debit_account|credit_account)"
        ) {
            let result = validate_ledger_account_name("", &field);
            let errors = result.unwrap_err();
            prop_assert!(errors.iter().all(|e| e.field == field),
                "Expected error field to match parameter: {}", field);
        }
    }

    // ---------------------------------------------------------------
    // Feature: transaction-ledger-api, Property 13: Transaction date validation
    // **Validates: Requirements 5.1, 5.2**
    // ---------------------------------------------------------------

    proptest! {
        #[test]
        fn prop_valid_date_accepted(
            y in 1900u32..=2100u32,
            m in 1u32..=12u32,
            d in 1u32..=28u32,  // 1-28 always valid for any month
        ) {
            let date_str = format!("{:04}-{:02}-{:02}", y, m, d);
            let result = validate_transaction_date(&date_str);
            prop_assert!(result.is_ok(), "Expected valid date to be accepted: {}", date_str);
            let parsed = result.unwrap();
            prop_assert_eq!(parsed.year() as u32, y);
            prop_assert_eq!(parsed.month(), m);
            prop_assert_eq!(parsed.day(), d);
        }

        #[test]
        fn prop_invalid_date_format_rejected(
            date in "[a-zA-Z0-9]{1,20}"
        ) {
            // Filter out strings that happen to be valid YYYY-MM-DD
            prop_assume!(NaiveDate::parse_from_str(date.trim(), "%Y-%m-%d").is_err());
            prop_assert!(validate_transaction_date(&date).is_err(),
                "Expected invalid date format to be rejected: {}", date);
        }

        #[test]
        fn prop_empty_date_rejected(spaces in " {0,10}") {
            prop_assert!(validate_transaction_date(&spaces).is_err(),
                "Expected empty/whitespace date to be rejected");
        }

        #[test]
        fn prop_wrong_separator_date_rejected(
            y in 2000u32..=2030u32,
            m in 1u32..=12u32,
            d in 1u32..=28u32,
            sep in "[/\\.]",
        ) {
            let date_str = format!("{:04}{}{:02}{}{:02}", y, sep, m, sep, d);
            prop_assert!(validate_transaction_date(&date_str).is_err(),
                "Expected wrong-separator date to be rejected: {}", date_str);
        }
    }

    // ---------------------------------------------------------------
    // Feature: transaction-ledger-api, Property 15: Amount validation
    // **Validates: Requirements 5.5, 5.6**
    // ---------------------------------------------------------------

    proptest! {
        #[test]
        fn prop_valid_integer_amount_accepted(n in 1u32..=999999u32) {
            let amount_str = n.to_string();
            let result = validate_amount(&amount_str);
            prop_assert!(result.is_ok(), "Expected valid integer amount to be accepted: {}", amount_str);
            let val = result.unwrap();
            prop_assert!((val - n as f64).abs() < f64::EPSILON);
        }

        #[test]
        fn prop_valid_one_decimal_amount_accepted(
            whole in 0u32..=99999u32,
            frac in 1u32..=9u32,
        ) {
            let amount_str = format!("{}.{}", whole, frac);
            // whole.frac must be > 0
            prop_assume!(whole > 0 || frac > 0);
            let result = validate_amount(&amount_str);
            prop_assert!(result.is_ok(), "Expected valid 1-decimal amount to be accepted: {}", amount_str);
        }

        #[test]
        fn prop_valid_two_decimal_amount_accepted(
            whole in 0u32..=99999u32,
            frac in 1u32..=99u32,
        ) {
            let amount_str = format!("{}.{:02}", whole, frac);
            prop_assume!(whole > 0 || frac > 0);
            let result = validate_amount(&amount_str);
            prop_assert!(result.is_ok(), "Expected valid 2-decimal amount to be accepted: {}", amount_str);
        }

        #[test]
        fn prop_three_or_more_decimal_places_rejected(
            whole in 1u32..=9999u32,
            frac in 1u32..=999u32,
        ) {
            let amount_str = format!("{}.{:03}", whole, frac);
            prop_assert!(validate_amount(&amount_str).is_err(),
                "Expected 3+ decimal places to be rejected: {}", amount_str);
        }

        #[test]
        fn prop_zero_amount_rejected(zeros in "(0|0\\.0|0\\.00)") {
            prop_assert!(validate_amount(&zeros).is_err(),
                "Expected zero amount to be rejected: {}", zeros);
        }

        #[test]
        fn prop_negative_amount_rejected(n in 1u32..=99999u32) {
            let amount_str = format!("-{}", n);
            prop_assert!(validate_amount(&amount_str).is_err(),
                "Expected negative amount to be rejected: {}", amount_str);
        }

        #[test]
        fn prop_non_numeric_amount_rejected(s in "[a-zA-Z]{1,10}") {
            prop_assert!(validate_amount(&s).is_err(),
                "Expected non-numeric amount to be rejected: {}", s);
        }
    }

    // ---------------------------------------------------------------
    // Feature: transaction-ledger-api, Property 14: Payee validation
    // **Validates: Requirements 5.3, 5.4**
    // ---------------------------------------------------------------

    proptest! {
        #[test]
        fn prop_valid_payee_accepted(
            payee in "[a-zA-Z0-9 ]{1,256}"
        ) {
            prop_assume!(!payee.trim().is_empty());
            prop_assume!(payee.trim().len() <= 256);
            prop_assert!(validate_payee(&payee).is_ok(),
                "Expected valid payee to be accepted: {:?}", payee);
        }

        #[test]
        fn prop_empty_or_whitespace_payee_rejected(
            spaces in " {0,20}"
        ) {
            prop_assert!(validate_payee(&spaces).is_err(),
                "Expected empty/whitespace payee to be rejected");
        }

        #[test]
        fn prop_too_long_payee_rejected(
            payee in "[a-zA-Z]{257,300}"
        ) {
            prop_assert!(validate_payee(&payee).is_err(),
                "Expected too-long payee to be rejected: len={}", payee.len());
        }
    }

    // ---------------------------------------------------------------
    // Feature: transaction-ledger-api, Property 16: Validation error response structure
    // **Validates: Requirements 5.9**
    // ---------------------------------------------------------------

    use crate::utils::error::AppError;
    use crate::models::CategoryType;
    use actix_web::error::ResponseError;

    /// Strategy for generating invalid PostTransactionRequest objects.
    /// At least one field is invalid to guarantee validation failure.
    fn invalid_post_transaction_strategy() -> impl Strategy<Value = PostTransactionRequest> {
        prop_oneof![
            // Invalid date
            Just(PostTransactionRequest {
                date: "not-a-date".to_string(),
                payee: "Valid Payee".to_string(),
                debit_account: "Expenses:Food".to_string(),
                credit_account: "Assets:Checking".to_string(),
                amount: "10.00".to_string(),
            }),
            // Empty payee
            Just(PostTransactionRequest {
                date: "2025-01-15".to_string(),
                payee: "   ".to_string(),
                debit_account: "Expenses:Food".to_string(),
                credit_account: "Assets:Checking".to_string(),
                amount: "10.00".to_string(),
            }),
            // Invalid amount
            Just(PostTransactionRequest {
                date: "2025-01-15".to_string(),
                payee: "Valid Payee".to_string(),
                debit_account: "Expenses:Food".to_string(),
                credit_account: "Assets:Checking".to_string(),
                amount: "-5.00".to_string(),
            }),
            // Invalid debit account
            Just(PostTransactionRequest {
                date: "2025-01-15".to_string(),
                payee: "Valid Payee".to_string(),
                debit_account: "".to_string(),
                credit_account: "Assets:Checking".to_string(),
                amount: "10.00".to_string(),
            }),
            // Multiple invalid fields
            Just(PostTransactionRequest {
                date: "bad".to_string(),
                payee: "".to_string(),
                debit_account: "".to_string(),
                credit_account: "".to_string(),
                amount: "abc".to_string(),
            }),
        ]
    }

    /// Strategy for generating invalid AddCategoryRequest objects.
    fn invalid_add_category_strategy() -> impl Strategy<Value = AddCategoryRequest> {
        prop_oneof![
            // Empty name
            Just(AddCategoryRequest {
                name: "".to_string(),
                category_type: CategoryType::Expense,
            }),
            // Whitespace-only name
            Just(AddCategoryRequest {
                name: "   ".to_string(),
                category_type: CategoryType::Income,
            }),
            // Invalid characters
            Just(AddCategoryRequest {
                name: "Bad@Category!".to_string(),
                category_type: CategoryType::Expense,
            }),
        ]
    }

    proptest! {
        #[test]
        fn prop_validation_error_response_structure_post_transaction(
            req in invalid_post_transaction_strategy()
        ) {
            let validation_result = validate_post_transaction(&req);
            prop_assert!(validation_result.is_err(), "Expected validation to fail");

            let details = validation_result.unwrap_err();
            let error = AppError::Validation(details);
            let response = error.error_response();
            let body = response.into_body();
            let bytes = actix_web::body::to_bytes(body);
            // Use a runtime to resolve the future
            let rt = tokio::runtime::Runtime::new().unwrap();
            let body_bytes = rt.block_on(bytes).unwrap();
            let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

            // Verify top-level "error" field is a string
            prop_assert!(json.get("error").is_some(), "Response must have 'error' field");
            prop_assert!(json["error"].is_string(), "'error' field must be a string");

            // Verify "details" field is an array
            prop_assert!(json.get("details").is_some(), "Response must have 'details' field");
            prop_assert!(json["details"].is_array(), "'details' field must be an array");

            let details_arr = json["details"].as_array().unwrap();
            prop_assert!(!details_arr.is_empty(), "'details' array must not be empty");

            // Each element must have "field" (string) and "message" (string)
            for detail in details_arr {
                prop_assert!(detail.get("field").is_some(), "Each detail must have 'field'");
                prop_assert!(detail["field"].is_string(), "'field' must be a string");
                prop_assert!(detail.get("message").is_some(), "Each detail must have 'message'");
                prop_assert!(detail["message"].is_string(), "'message' must be a string");
            }
        }

        #[test]
        fn prop_validation_error_response_structure_add_category(
            req in invalid_add_category_strategy()
        ) {
            let validation_result = validate_add_category(&req);
            prop_assert!(validation_result.is_err(), "Expected validation to fail");

            let details = validation_result.unwrap_err();
            let error = AppError::Validation(details);
            let response = error.error_response();
            let body = response.into_body();
            let bytes = actix_web::body::to_bytes(body);
            let rt = tokio::runtime::Runtime::new().unwrap();
            let body_bytes = rt.block_on(bytes).unwrap();
            let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

            // Verify top-level "error" field is a string
            prop_assert!(json.get("error").is_some(), "Response must have 'error' field");
            prop_assert!(json["error"].is_string(), "'error' field must be a string");

            // Verify "details" field is an array
            prop_assert!(json.get("details").is_some(), "Response must have 'details' field");
            prop_assert!(json["details"].is_array(), "'details' field must be an array");

            let details_arr = json["details"].as_array().unwrap();
            prop_assert!(!details_arr.is_empty(), "'details' array must not be empty");

            // Each element must have "field" (string) and "message" (string)
            for detail in details_arr {
                prop_assert!(detail.get("field").is_some(), "Each detail must have 'field'");
                prop_assert!(detail["field"].is_string(), "'field' must be a string");
                prop_assert!(detail.get("message").is_some(), "Each detail must have 'message'");
                prop_assert!(detail["message"].is_string(), "'message' must be a string");
            }
        }
    }
}
