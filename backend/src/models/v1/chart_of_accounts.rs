use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The five standard ledger account types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AccountType {
    Assets,
    Expenses,
    Income,
    Liabilities,
    Equity,
}

/// Per-user chart of accounts — persisted as user-{uuid}-chart-of-accounts.json
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChartOfAccounts {
    pub user_id: Uuid,
    pub assets: Vec<String>,
    pub expenses: Vec<String>,
    pub income: Vec<String>,
    pub liabilities: Vec<String>,
    pub equity: Vec<String>,
}

impl ChartOfAccounts {
    pub fn empty(user_id: Uuid) -> Self {
        Self {
            user_id,
            assets: vec![],
            expenses: vec![],
            income: vec![],
            liabilities: vec![],
            equity: vec![],
        }
    }

    pub fn get_list(&self, account_type: &AccountType) -> &Vec<String> {
        match account_type {
            AccountType::Assets => &self.assets,
            AccountType::Expenses => &self.expenses,
            AccountType::Income => &self.income,
            AccountType::Liabilities => &self.liabilities,
            AccountType::Equity => &self.equity,
        }
    }

    pub fn get_list_mut(&mut self, account_type: &AccountType) -> &mut Vec<String> {
        match account_type {
            AccountType::Assets => &mut self.assets,
            AccountType::Expenses => &mut self.expenses,
            AccountType::Income => &mut self.income,
            AccountType::Liabilities => &mut self.liabilities,
            AccountType::Equity => &mut self.equity,
        }
    }

    /// Detect the account type from a ledger account name prefix
    pub fn detect_account_type(account_name: &str) -> AccountType {
        if account_name.starts_with("Assets:") {
            AccountType::Assets
        } else if account_name.starts_with("Expenses:") {
            AccountType::Expenses
        } else if account_name.starts_with("Income:") {
            AccountType::Income
        } else if account_name.starts_with("Liabilities:") {
            AccountType::Liabilities
        } else if account_name.starts_with("Equity:") {
            AccountType::Equity
        } else {
            AccountType::Expenses // fallback
        }
    }
}

// Request/Response DTOs for chart of accounts API

#[derive(Debug, Clone, Deserialize)]
pub struct ListAccountsQuery {
    pub r#type: AccountType,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AddAccountRequest {
    pub name: String,
    pub account_type: AccountType,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeleteAccountRequest {
    pub name: String,
    pub account_type: AccountType,
}
