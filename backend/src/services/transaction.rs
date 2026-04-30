use std::path::Path;

use chrono::{NaiveDate, Utc};
use log::warn;
use uuid::Uuid;

use crate::models::{
    AccountType, BalanceResponse, CategoryType, ChartOfAccounts, OpeningBalanceRequest,
    PostTransactionRequest, RegisterResponse, TransactionEntry, TransactionPosting,
    TransactionResponse, UpdateTransactionRequest, UserCategories,
};
use crate::services::ledger_cli::LedgerCli;
use crate::services::ledger_parser;
use crate::services::workspace::WorkspaceService;
use crate::services::cache::Cache;
use crate::services::file_store::FileStore;
use crate::services::user::UserService;
use crate::utils::error::AppError;

pub struct TransactionService {
    file_store: FileStore,
    cache: Cache,
    workspace_service: WorkspaceService,
    user_service: UserService,
    cache_ttl: usize,
}

impl TransactionService {
    pub fn new(
        file_store: FileStore,
        cache: Cache,
        account_service: WorkspaceService,
        user_service: UserService,
        cache_ttl: usize,
    ) -> Self {
        Self {
            file_store,
            cache,
            workspace_service: account_service,
            user_service,
            cache_ttl,
        }
    }

    fn cache_key(category_type: &CategoryType, user_id: &Uuid) -> String {
        let type_str = match category_type {
            CategoryType::Expense => "expense",
            CategoryType::Income => "income",
        };
        format!("categories:{}:{}", type_str, user_id)
    }

    fn get_category_list<'a>(
        categories: &'a UserCategories,
        category_type: &CategoryType,
    ) -> &'a Vec<String> {
        match category_type {
            CategoryType::Expense => &categories.expense,
            CategoryType::Income => &categories.income,
        }
    }

    fn get_category_list_mut<'a>(
        categories: &'a mut UserCategories,
        category_type: &CategoryType,
    ) -> &'a mut Vec<String> {
        match category_type {
            CategoryType::Expense => &mut categories.expense,
            CategoryType::Income => &mut categories.income,
        }
    }

    fn load_or_create_categories(&self, user_id: &Uuid) -> Result<UserCategories, AppError> {
        match self.file_store.read_categories(user_id)? {
            Some(categories) => Ok(categories),
            None => Ok(UserCategories {
                user_id: *user_id,
                expense: vec![],
                income: vec![],
            }),
        }
    }

    pub fn list_categories(
        &self,
        user_id: &Uuid,
        category_type: &CategoryType,
    ) -> Result<Vec<String>, AppError> {
        // 1. Try cache first
        let cache_key = Self::cache_key(category_type, user_id);
        match self.cache.get::<Vec<String>>(&cache_key) {
            Ok(Some(cached)) => return Ok(cached),
            Ok(None) => {}
            Err(e) => warn!("Cache read failed for {}: {}", cache_key, e),
        }

        // 2. On miss: read from file
        let categories = match self.file_store.read_categories(user_id)? {
            Some(cats) => cats,
            None => return Ok(vec![]),
        };

        // 3. Repopulate cache for both types
        let expense_key = Self::cache_key(&CategoryType::Expense, user_id);
        self.cache.set_or_warn(&expense_key, &categories.expense, self.cache_ttl);
        let income_key = Self::cache_key(&CategoryType::Income, user_id);
        self.cache.set_or_warn(&income_key, &categories.income, self.cache_ttl);

        // 4. Return matching type list
        Ok(Self::get_category_list(&categories, category_type).clone())
    }

    pub fn add_category(
        &self,
        user_id: &Uuid,
        name: String,
        category_type: &CategoryType,
    ) -> Result<(), AppError> {
        // 1. Load or create UserCategories
        let mut categories = self.load_or_create_categories(user_id)?;

        // 2. Check for duplicate
        let list = Self::get_category_list(&categories, category_type);
        if list.contains(&name) {
            return Err(AppError::Conflict(format!(
                "Category '{}' already exists",
                name
            )));
        }

        // 3. Add to appropriate list
        Self::get_category_list_mut(&mut categories, category_type).push(name);

        // 4. Persist to file
        self.file_store.write_categories(&categories)?;

        // 5. Update cache
        let cache_key = Self::cache_key(category_type, user_id);
        self.cache.set_or_warn(
            &cache_key,
            Self::get_category_list(&categories, category_type),
            self.cache_ttl,
        );

        Ok(())
    }

    pub fn delete_category(
        &self,
        user_id: &Uuid,
        name: &str,
        category_type: &CategoryType,
    ) -> Result<(), AppError> {
        // 1. Load UserCategories — 404 if file missing or category not in list
        let mut categories = match self.file_store.read_categories(user_id)? {
            Some(cats) => cats,
            None => {
                return Err(AppError::NotFound(format!(
                    "Category '{}' not found",
                    name
                )));
            }
        };

        let list = Self::get_category_list(&categories, category_type);
        let pos = list.iter().position(|c| c == name).ok_or_else(|| {
            AppError::NotFound(format!("Category '{}' not found", name))
        })?;

        // 2. Remove from appropriate list
        Self::get_category_list_mut(&mut categories, category_type).remove(pos);

        // 3. Persist to file
        self.file_store.write_categories(&categories)?;

        // 4. Update cache
        let cache_key = Self::cache_key(category_type, user_id);
        self.cache.set_or_warn(
            &cache_key,
            Self::get_category_list(&categories, category_type),
            self.cache_ttl,
        );

        Ok(())
    }

    // --- Chart of Accounts ---

    fn chart_cache_key(user_id: &Uuid) -> String {
        format!("chart:{}", user_id)
    }

    /// Load the chart of accounts for a user, migrating from categories if needed.
    /// - If chart-of-accounts.json exists, return it
    /// - If only categories.json exists, migrate: expense→expenses, income→income, empty for assets/liabilities/equity
    /// - If neither exists, return empty chart
    pub fn load_or_create_chart(&self, user_id: &Uuid) -> Result<ChartOfAccounts, AppError> {
        // 1. Try reading existing chart file
        if let Some(chart) = self.file_store.read_chart_of_accounts(user_id)? {
            return Ok(chart);
        }

        // 2. Try migrating from categories
        if let Some(categories) = self.file_store.read_categories(user_id)? {
            let chart = ChartOfAccounts {
                user_id: *user_id,
                assets: vec![],
                expenses: categories.expense,
                income: categories.income,
                liabilities: vec![],
                equity: vec![],
            };
            self.file_store.write_chart_of_accounts(&chart)?;
            self.file_store.delete_categories_file(user_id)?;
            return Ok(chart);
        }

        // 3. Neither exists — return empty chart
        Ok(ChartOfAccounts::empty(*user_id))
    }

    /// List accounts for a given type, with cache-first and file fallback.
    pub fn list_accounts(
        &self,
        user_id: &Uuid,
        account_type: &AccountType,
    ) -> Result<Vec<String>, AppError> {
        // 1. Try cache first
        let cache_key = Self::chart_cache_key(user_id);
        match self.cache.get::<ChartOfAccounts>(&cache_key) {
            Ok(Some(cached)) => return Ok(cached.get_list(account_type).clone()),
            Ok(None) => {}
            Err(e) => warn!("Cache read failed for {}: {}", cache_key, e),
        }

        // 2. On miss: load from file (with migration)
        let chart = self.load_or_create_chart(user_id)?;

        // 3. Repopulate cache
        self.cache.set_or_warn(&cache_key, &chart, self.cache_ttl);

        // 4. Return the requested type's list
        Ok(chart.get_list(account_type).clone())
    }

    /// Add an account to the chart. Returns 409 if duplicate.
    pub fn add_account(
        &self,
        user_id: &Uuid,
        name: String,
        account_type: &AccountType,
    ) -> Result<(), AppError> {
        // 1. Load chart
        let mut chart = self.load_or_create_chart(user_id)?;

        // 2. Check for duplicate
        let list = chart.get_list(account_type);
        if list.contains(&name) {
            return Err(AppError::Conflict(format!(
                "Account '{}' already exists in {}",
                name,
                serde_json::to_string(account_type).unwrap_or_default().trim_matches('"')
            )));
        }

        // 3. Add to list
        chart.get_list_mut(account_type).push(name);

        // 4. Persist
        self.file_store.write_chart_of_accounts(&chart)?;

        // 5. Update cache
        let cache_key = Self::chart_cache_key(user_id);
        self.cache.set_or_warn(&cache_key, &chart, self.cache_ttl);

        Ok(())
    }

    /// Delete an account from the chart. Returns 404 if not found.
    pub fn delete_account(
        &self,
        user_id: &Uuid,
        name: &str,
        account_type: &AccountType,
    ) -> Result<(), AppError> {
        // 1. Load chart
        let mut chart = self.load_or_create_chart(user_id)?;

        // 2. Find and remove
        let list = chart.get_list(account_type);
        let pos = list.iter().position(|a| a == name).ok_or_else(|| {
            AppError::NotFound(format!(
                "Account '{}' not found in {}",
                name,
                serde_json::to_string(account_type).unwrap_or_default().trim_matches('"')
            ))
        })?;

        chart.get_list_mut(account_type).remove(pos);

        // 3. Persist
        self.file_store.write_chart_of_accounts(&chart)?;

        // 4. Update cache
        let cache_key = Self::chart_cache_key(user_id);
        self.cache.set_or_warn(&cache_key, &chart, self.cache_ttl);

        Ok(())
    }

    /// Auto-add accounts from a transaction's postings to the chart of accounts.
    /// Detects account type from prefix and silently skips if already exists.
    pub fn auto_add_from_transaction(
        &self,
        user_id: &Uuid,
        debit_account: &str,
        credit_account: &str,
    ) -> Result<(), AppError> {
        for account_name in &[debit_account, credit_account] {
            let account_type = ChartOfAccounts::detect_account_type(account_name);
            // Silently ignore conflicts (account already exists)
            match self.add_account(user_id, account_name.to_string(), &account_type) {
                Ok(()) => {}
                Err(AppError::Conflict(_)) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    // --- Transaction Posting ---

    pub fn format_transaction(
        &self,
        date: &NaiveDate,
        payee: &str,
        debit_account: &str,
        credit_account: &str,
        amount: f64,
        username: &str,
    ) -> String {
        self.format_transaction_with_id(
            Uuid::new_v4(),
            date,
            payee,
            debit_account,
            credit_account,
            amount,
            username,
        )
    }

    /// Same as `format_transaction` but emits a caller-specified ID. Used when
    /// rewriting an existing transaction to preserve its identity across edits.
    pub fn format_transaction_with_id(
        &self,
        id: Uuid,
        date: &NaiveDate,
        payee: &str,
        debit_account: &str,
        credit_account: &str,
        amount: f64,
        username: &str,
    ) -> String {
        format!(
            "{date} {payee}\n    ; Id: {id}\n    {debit}  ${amount:.2}\n    ; User: {user}\n    {credit}  -${amount:.2}\n    ; User: {user}",
            date = date.format("%Y-%m-%d"),
            payee = payee,
            id = id,
            debit = debit_account,
            credit = credit_account,
            amount = amount,
            user = username,
        )
    }

    pub fn post_transaction(
        &self,
        workspace_id: &Uuid,
        user_id: &Uuid,
        req: &PostTransactionRequest,
    ) -> Result<TransactionResponse, AppError> {
        let workspace = self
            .workspace_service
            .get_workspace(workspace_id)?
            .ok_or_else(|| AppError::NotFound("Workspace not found".to_string()))?;

        if !workspace.is_active {
            return Err(AppError::BadRequest(
                "Workspace is deactivated".to_string(),
            ));
        }

        // 404 (not 403) when user has no access — don't leak existence
        if !workspace.has_access(user_id) {
            return Err(AppError::NotFound("Workspace not found".to_string()));
        }

        if !workspace.has_write_access(user_id) {
            return Err(AppError::Forbidden(
                "You don't have write access to this workspace".to_string(),
            ));
        }

        let profile = self
            .user_service
            .get_profile(user_id)?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        let date = NaiveDate::parse_from_str(&req.date, "%Y-%m-%d")
            .map_err(|e| AppError::BadRequest(format!("Invalid date: {}", e)))?;
        let amount: f64 = req
            .amount
            .parse()
            .map_err(|e| AppError::BadRequest(format!("Invalid amount: {}", e)))?;

        let tx_id = Uuid::new_v4();
        let formatted = self.format_transaction_with_id(
            tx_id,
            &date,
            &req.payee,
            &req.debit_account,
            &req.credit_account,
            amount,
            &profile.username,
        );

        if workspace.ledger_dir.is_some() {
            let period_label = workspace.rotation_period.period_label(&date);
            self.file_store.append_to_period_file(&workspace, &period_label, &formatted)?;
        } else {
            self.file_store.append_to_ledger(workspace_id, &formatted)?;
        }

        self.auto_add_from_transaction(user_id, &req.debit_account, &req.credit_account)?;

        Ok(TransactionResponse {
            formatted_text: formatted,
            id: Some(tx_id),
        })
    }

    /// List all transactions with IDs across the workspace's period files.
    /// Legacy (pre-ID) entries are skipped — callers wanting every historical
    /// posting should keep using `query_register`, which goes through the
    /// ledger CLI.
    pub fn list_transactions(
        &self,
        workspace_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<Vec<TransactionEntry>, AppError> {
        let workspace = self
            .workspace_service
            .get_workspace_authorized(workspace_id, user_id)?;

        let paths = self.file_store.list_period_files(&workspace)?;
        let mut out = Vec::new();
        for path in paths {
            let contents = match self.file_store.read_ledger_file(&path)? {
                Some(c) => c,
                None => continue,
            };
            for entry in ledger_parser::parse_entries(&contents) {
                if let Some(id) = entry.id {
                    out.push(TransactionEntry {
                        id,
                        date: entry.date,
                        payee: entry.payee,
                        postings: entry
                            .postings
                            .into_iter()
                            .map(|p| TransactionPosting {
                                account: p.account,
                                amount: p.amount,
                            })
                            .collect(),
                        posted_by: entry.posted_by,
                    });
                }
            }
        }
        // Sort newest first — date descending, then payee for stability.
        out.sort_by(|a, b| b.date.cmp(&a.date).then_with(|| a.payee.cmp(&b.payee)));
        Ok(out)
    }

    /// Delete a transaction by ID. Requires write access. Returns 404 if the
    /// ID doesn't exist in any period file.
    pub fn delete_transaction(
        &self,
        workspace_id: &Uuid,
        user_id: &Uuid,
        tx_id: &Uuid,
    ) -> Result<(), AppError> {
        let workspace = self
            .workspace_service
            .get_workspace(workspace_id)?
            .ok_or_else(|| AppError::NotFound("Workspace not found".to_string()))?;

        if !workspace.is_active {
            return Err(AppError::BadRequest(
                "Workspace is deactivated".to_string(),
            ));
        }

        if !workspace.has_access(user_id) {
            return Err(AppError::NotFound("Workspace not found".to_string()));
        }

        if !workspace.has_write_access(user_id) {
            return Err(AppError::Forbidden(
                "You don't have write access to this workspace".to_string(),
            ));
        }

        let paths = self.file_store.list_period_files(&workspace)?;
        for path in paths {
            let contents = match self.file_store.read_ledger_file(&path)? {
                Some(c) => c,
                None => continue,
            };
            if let Some(new_contents) = ledger_parser::remove_entry(&contents, tx_id) {
                self.file_store.write_ledger_file(&path, &new_contents)?;
                return Ok(());
            }
        }

        Err(AppError::NotFound("Transaction not found".to_string()))
    }

    /// Update a transaction by ID. Preserves the ID and the original poster's
    /// username (audit trail). If the new date falls into a different period
    /// than the original, the entry is moved to the correct period file.
    pub fn update_transaction(
        &self,
        workspace_id: &Uuid,
        user_id: &Uuid,
        tx_id: &Uuid,
        req: &UpdateTransactionRequest,
    ) -> Result<TransactionResponse, AppError> {
        let workspace = self
            .workspace_service
            .get_workspace(workspace_id)?
            .ok_or_else(|| AppError::NotFound("Workspace not found".to_string()))?;

        if !workspace.is_active {
            return Err(AppError::BadRequest(
                "Workspace is deactivated".to_string(),
            ));
        }

        if !workspace.has_access(user_id) {
            return Err(AppError::NotFound("Workspace not found".to_string()));
        }

        if !workspace.has_write_access(user_id) {
            return Err(AppError::Forbidden(
                "You don't have write access to this workspace".to_string(),
            ));
        }

        let new_date = NaiveDate::parse_from_str(&req.date, "%Y-%m-%d")
            .map_err(|e| AppError::BadRequest(format!("Invalid date: {}", e)))?;
        let new_amount: f64 = req
            .amount
            .parse()
            .map_err(|e| AppError::BadRequest(format!("Invalid amount: {}", e)))?;

        // 1. Find the original entry and its file so we can preserve the
        //    original poster's name.
        let paths = self.file_store.list_period_files(&workspace)?;
        let mut original: Option<(std::path::PathBuf, String, Option<String>)> = None;
        for path in &paths {
            let contents = match self.file_store.read_ledger_file(path)? {
                Some(c) => c,
                None => continue,
            };
            for entry in ledger_parser::parse_entries(&contents) {
                if entry.id == Some(*tx_id) {
                    original = Some((path.clone(), contents.clone(), entry.posted_by));
                    break;
                }
            }
            if original.is_some() {
                break;
            }
        }
        let (original_path, original_contents, original_user) = original
            .ok_or_else(|| AppError::NotFound("Transaction not found".to_string()))?;

        // Fall back to the editor's username if the original tx had no User tag.
        let username = match original_user {
            Some(u) => u,
            None => {
                let profile = self
                    .user_service
                    .get_profile(user_id)?
                    .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
                profile.username
            }
        };

        let formatted = self.format_transaction_with_id(
            *tx_id,
            &new_date,
            &req.payee,
            &req.debit_account,
            &req.credit_account,
            new_amount,
            &username,
        );

        // 2. Decide whether the new date lives in the same period file.
        let target_period_path = if workspace.ledger_dir.is_some() {
            let period_label = workspace.rotation_period.period_label(&new_date);
            self.file_store.period_file_path(&workspace, &period_label)
        } else {
            // Legacy single-file workspaces always rewrite in place.
            original_path.clone()
        };

        if target_period_path == original_path {
            // Rewrite the original file in place.
            let new_contents = ledger_parser::replace_entry(
                &original_contents,
                tx_id,
                &formatted,
            )
            .ok_or_else(|| {
                AppError::Internal("Failed to locate transaction block during rewrite".to_string())
            })?;
            self.file_store
                .write_ledger_file(&original_path, &new_contents)?;
        } else {
            // Date crossed a period boundary — remove from the old file,
            // append to the new (creating it if needed).
            let old_contents_stripped = ledger_parser::remove_entry(&original_contents, tx_id)
                .ok_or_else(|| {
                    AppError::Internal(
                        "Failed to locate transaction block during cross-period move".to_string(),
                    )
                })?;
            self.file_store
                .write_ledger_file(&original_path, &old_contents_stripped)?;
            let period_label = workspace.rotation_period.period_label(&new_date);
            self.file_store
                .append_to_period_file(&workspace, &period_label, &formatted)?;
        }

        self.auto_add_from_transaction(user_id, &req.debit_account, &req.credit_account)?;

        Ok(TransactionResponse {
            formatted_text: formatted,
            id: Some(*tx_id),
        })
    }

    // --- Ledger Queries ---

    pub fn query_balance(
        &self,
        workspace_id: &Uuid,
        user_id: &Uuid,
        pivot_user: bool,
        filter_user: Option<&str>,
    ) -> Result<BalanceResponse, AppError> {
        let workspace = self.workspace_service
            .get_workspace_authorized(workspace_id, user_id)?;
        let ledger_path = self.file_store.get_workspace_ledger_path(&workspace);
        let output = LedgerCli::balance(&ledger_path, pivot_user, filter_user)?;
        Ok(BalanceResponse { output })
    }

    pub fn query_register(
        &self,
        workspace_id: &Uuid,
        user_id: &Uuid,
        filter_user: Option<&str>,
        filter_payee: Option<&str>,
        begin: Option<&str>,
        end: Option<&str>,
    ) -> Result<RegisterResponse, AppError> {
        let workspace = self.workspace_service
            .get_workspace_authorized(workspace_id, user_id)?;
        let ledger_path = self.file_store.get_workspace_ledger_path(&workspace);
        let output = LedgerCli::register(&ledger_path, filter_user, filter_payee, begin, end)?;
        Ok(RegisterResponse { output })
    }

    // --- Opening Balance ---

    /// Scans the workspace's ledger file(s) for an existing "Equity:Opening Balances" posting.
    pub fn has_opening_balance(&self, workspace_id: &Uuid) -> Result<bool, AppError> {
        let workspace = self
            .workspace_service
            .get_workspace(workspace_id)?
            .ok_or_else(|| AppError::NotFound("Workspace not found".to_string()))?;

        if workspace.ledger_dir.is_some() {
            // Migrated workspace: scan period files, skip the workspace-level ledger
            // which only contains !include directives.
            let ws_dir = Path::new(&self.file_store.data_path())
                .join("workspaces")
                .join(format!("workspace-{}", workspace.id));
            let workspace_ledger_name = format!("workspace-{}.ledger", workspace.id);

            match std::fs::read_dir(&ws_dir) {
                Ok(entries) => {
                    for entry in entries {
                        let entry = entry.map_err(|e| {
                            AppError::Internal(format!("Failed to read directory entry: {}", e))
                        })?;
                        let file_name = entry.file_name();
                        let name = file_name.to_string_lossy();
                        if name.ends_with(".ledger") && *name != workspace_ledger_name {
                            match std::fs::read_to_string(entry.path()) {
                                Ok(content) => {
                                    if content.contains("Equity:Opening Balances") {
                                        return Ok(true);
                                    }
                                }
                                Err(e) => {
                                    return Err(AppError::Internal(format!(
                                        "Failed to read period file {:?}: {}",
                                        entry.path(),
                                        e
                                    )));
                                }
                            }
                        }
                    }
                    Ok(false)
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
                Err(e) => Err(AppError::Internal(format!(
                    "Failed to read workspace directory: {}",
                    e
                ))),
            }
        } else {
            let ledger_path = self.file_store.get_ledger_path(&workspace.id);
            match std::fs::read_to_string(&ledger_path) {
                Ok(content) => Ok(content.contains("Equity:Opening Balances")),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
                Err(e) => Err(AppError::Internal(format!(
                    "Failed to read ledger file: {}",
                    e
                ))),
            }
        }
    }

    /// Owner-only, idempotent: 409 if an opening balance already exists.
    pub fn post_opening_balance(
        &self,
        workspace_id: &Uuid,
        user_id: &Uuid,
        req: &OpeningBalanceRequest,
    ) -> Result<TransactionResponse, AppError> {
        let workspace = self
            .workspace_service
            .get_workspace(workspace_id)?
            .ok_or_else(|| AppError::NotFound("Workspace not found".to_string()))?;

        if !workspace.is_active {
            return Err(AppError::BadRequest(
                "Workspace is deactivated".to_string(),
            ));
        }

        if !workspace.has_access(user_id) {
            return Err(AppError::NotFound("Workspace not found".to_string()));
        }

        if workspace.owner_id != *user_id {
            return Err(AppError::Forbidden(
                "Only the workspace owner can set the opening balance".to_string(),
            ));
        }

        let profile = self
            .user_service
            .get_profile(user_id)?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        if self.has_opening_balance(workspace_id)? {
            return Err(AppError::Conflict(
                "Opening balance already exists for this workspace".to_string(),
            ));
        }

        let date = match &req.date {
            Some(d) => NaiveDate::parse_from_str(d, "%Y-%m-%d")
                .map_err(|e| AppError::BadRequest(format!("Invalid date: {}", e)))?,
            None => Utc::now().date_naive(),
        };
        let asset_account = req
            .account_name
            .as_deref()
            .unwrap_or("Assets:Opening Balance");
        let amount: f64 = req
            .amount
            .parse()
            .map_err(|e| AppError::BadRequest(format!("Invalid amount: {}", e)))?;

        let ob_id = Uuid::new_v4();
        let formatted = self.format_transaction_with_id(
            ob_id,
            &date,
            "Opening Balance",
            asset_account,
            "Equity:Opening Balances",
            amount,
            &profile.username,
        );

        if workspace.ledger_dir.is_some() {
            let period_label = workspace.rotation_period.period_label(&date);
            self.file_store.append_to_period_file(&workspace, &period_label, &formatted)?;
        } else {
            self.file_store.append_to_ledger(workspace_id, &formatted)?;
        }

        self.auto_add_from_transaction(user_id, asset_account, "Equity:Opening Balances")?;

        Ok(TransactionResponse {
            formatted_text: formatted,
            id: Some(ob_id),
        })
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{RotationPeriod, Workspace, Permission, SharedUser, UserCategories, UserProfile};
    use crate::services::cache::Cache;
    use crate::services::file_store::FileStore;
    use crate::services::user::UserService;
    use crate::services::workspace::WorkspaceService;
    use chrono::{NaiveDate, Utc};
    use proptest::prelude::*;
    use tempfile::TempDir;

    /// Creates a TransactionService backed by a temp directory.
    /// Uses an invalid Redis URL so cache operations fail gracefully,
    /// exercising the file-fallback path.
    fn make_test_service(tmp: &TempDir) -> TransactionService {
        let data_path = tmp.path().to_str().unwrap().to_string();
        let file_store = FileStore::new(data_path.clone());
        let redis_client = redis::Client::open("redis://invalid-host:6379").unwrap();
        let cache = Cache::new(redis_client);
        let user_service = UserService::new(file_store.clone(), cache.clone(), 86400);
        let account_service = WorkspaceService::new(file_store.clone(), cache.clone(), user_service.clone(), 86400);
        TransactionService::new(file_store, cache, account_service, user_service, 86400)
    }

    /// Strategy for generating valid category name strings.
    fn category_name_strategy() -> impl Strategy<Value = String> {
        prop::collection::vec(
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
            1..=30,
        )
        .prop_map(|chars| chars.into_iter().collect::<String>())
    }

    /// Strategy for generating a vec of unique category names.
    fn unique_categories_strategy(max_len: usize) -> impl Strategy<Value = Vec<String>> {
        prop::collection::hash_set(category_name_strategy(), 0..=max_len)
            .prop_map(|set| set.into_iter().collect::<Vec<_>>())
    }

    fn category_type_strategy() -> impl Strategy<Value = CategoryType> {
        prop_oneof![Just(CategoryType::Expense), Just(CategoryType::Income)]
    }

    // Feature: transaction-ledger-api, Property 1: Category listing by type
    // **Validates: Requirements 1.1**
    proptest! {
        #[test]
        fn prop_category_listing_by_type(
            expense_cats in unique_categories_strategy(8),
            income_cats in unique_categories_strategy(8),
        ) {
            let tmp = TempDir::new().unwrap();
            let service = make_test_service(&tmp);
            let user_id = Uuid::new_v4();

            // Write categories to file
            let categories = UserCategories {
                user_id,
                expense: expense_cats.clone(),
                income: income_cats.clone(),
            };
            service.file_store.write_categories(&categories).unwrap();

            // List expense categories
            let mut result_expense = service.list_categories(&user_id, &CategoryType::Expense).unwrap();
            let mut expected_expense = expense_cats.clone();
            result_expense.sort();
            expected_expense.sort();
            prop_assert_eq!(result_expense.len(), expense_cats.len());
            prop_assert_eq!(result_expense, expected_expense);

            // List income categories
            let mut result_income = service.list_categories(&user_id, &CategoryType::Income).unwrap();
            let mut expected_income = income_cats.clone();
            result_income.sort();
            expected_income.sort();
            prop_assert_eq!(result_income.len(), income_cats.len());
            prop_assert_eq!(result_income, expected_income);
        }
    }

    // Feature: transaction-ledger-api, Property 2: Category cache-fallback correctness
    // **Validates: Requirements 1.2, 1.4, 9.3, 9.4**
    proptest! {
        #[test]
        fn prop_category_cache_fallback_correctness(
            expense_cats in unique_categories_strategy(8),
            income_cats in unique_categories_strategy(8),
        ) {
            let tmp = TempDir::new().unwrap();
            let service = make_test_service(&tmp);
            let user_id = Uuid::new_v4();

            // Write categories to file
            let categories = UserCategories {
                user_id,
                expense: expense_cats.clone(),
                income: income_cats.clone(),
            };
            service.file_store.write_categories(&categories).unwrap();

            // Cache is unavailable (invalid Redis URL), so this exercises file fallback
            let mut result = service.list_categories(&user_id, &CategoryType::Expense).unwrap();
            let mut expected = expense_cats.clone();
            result.sort();
            expected.sort();
            prop_assert_eq!(result, expected);

            // Verify income also works via fallback
            let mut result_income = service.list_categories(&user_id, &CategoryType::Income).unwrap();
            let mut expected_income = income_cats.clone();
            result_income.sort();
            expected_income.sort();
            prop_assert_eq!(result_income, expected_income);
        }
    }

    // Feature: transaction-ledger-api, Property 3: Add category grows list
    // **Validates: Requirements 2.1, 2.2, 2.3**
    proptest! {
        #[test]
        fn prop_add_category_grows_list(
            existing in unique_categories_strategy(5),
            new_cat in category_name_strategy(),
            cat_type in category_type_strategy(),
        ) {
            // Skip if new_cat already exists in the list
            if existing.contains(&new_cat) {
                return Ok(());
            }

            let tmp = TempDir::new().unwrap();
            let service = make_test_service(&tmp);
            let user_id = Uuid::new_v4();

            // Set up initial categories
            let categories = match cat_type {
                CategoryType::Expense => UserCategories {
                    user_id,
                    expense: existing.clone(),
                    income: vec![],
                },
                CategoryType::Income => UserCategories {
                    user_id,
                    expense: vec![],
                    income: existing.clone(),
                },
            };
            if !existing.is_empty() {
                service.file_store.write_categories(&categories).unwrap();
            }

            let before_len = existing.len();

            // Add the new category
            service.add_category(&user_id, new_cat.clone(), &cat_type).unwrap();

            // Verify list grew by 1
            let after = service.list_categories(&user_id, &cat_type).unwrap();
            prop_assert_eq!(after.len(), before_len + 1);
            prop_assert!(after.contains(&new_cat));
        }
    }

    // Feature: transaction-ledger-api, Property 4: Duplicate category rejection
    // **Validates: Requirements 2.4**
    proptest! {
        #[test]
        fn prop_duplicate_category_rejection(
            existing in unique_categories_strategy(3).prop_filter("need at least one", |v| !v.is_empty()),
            cat_type in category_type_strategy(),
        ) {
            let tmp = TempDir::new().unwrap();
            let service = make_test_service(&tmp);
            let user_id = Uuid::new_v4();

            // Pick the first existing category as the duplicate
            let dup_name = existing[0].clone();

            let categories = match cat_type {
                CategoryType::Expense => UserCategories {
                    user_id,
                    expense: existing.clone(),
                    income: vec![],
                },
                CategoryType::Income => UserCategories {
                    user_id,
                    expense: vec![],
                    income: existing.clone(),
                },
            };
            service.file_store.write_categories(&categories).unwrap();

            // Try to add duplicate
            let result = service.add_category(&user_id, dup_name, &cat_type);
            prop_assert!(matches!(result, Err(AppError::Conflict(_))));

            // List should remain unchanged
            let after = service.list_categories(&user_id, &cat_type).unwrap();
            prop_assert_eq!(after.len(), existing.len());
        }
    }

    // Feature: transaction-ledger-api, Property 6: Delete category removes from list
    // **Validates: Requirements 3.1, 3.2, 3.3**
    proptest! {
        #[test]
        fn prop_delete_category_removes_from_list(
            existing in unique_categories_strategy(5).prop_filter("need at least one", |v| !v.is_empty()),
            cat_type in category_type_strategy(),
        ) {
            let tmp = TempDir::new().unwrap();
            let service = make_test_service(&tmp);
            let user_id = Uuid::new_v4();

            // Pick the first category to delete
            let to_delete = existing[0].clone();

            let categories = match cat_type {
                CategoryType::Expense => UserCategories {
                    user_id,
                    expense: existing.clone(),
                    income: vec![],
                },
                CategoryType::Income => UserCategories {
                    user_id,
                    expense: vec![],
                    income: existing.clone(),
                },
            };
            service.file_store.write_categories(&categories).unwrap();

            // Delete the category
            service.delete_category(&user_id, &to_delete, &cat_type).unwrap();

            // Verify list shrunk by 1 and doesn't contain deleted category
            let after = service.list_categories(&user_id, &cat_type).unwrap();
            prop_assert_eq!(after.len(), existing.len() - 1);
            prop_assert!(!after.contains(&to_delete));
        }
    }

    // Feature: transaction-ledger-api, Property 7: Delete non-existent category returns error
    // **Validates: Requirements 3.4**
    proptest! {
        #[test]
        fn prop_delete_nonexistent_category_returns_error(
            existing in unique_categories_strategy(3),
            nonexistent in category_name_strategy(),
            cat_type in category_type_strategy(),
        ) {
            // Skip if nonexistent happens to be in the list
            if existing.contains(&nonexistent) {
                return Ok(());
            }

            let tmp = TempDir::new().unwrap();
            let service = make_test_service(&tmp);
            let user_id = Uuid::new_v4();

            if !existing.is_empty() {
                let categories = match cat_type {
                    CategoryType::Expense => UserCategories {
                        user_id,
                        expense: existing.clone(),
                        income: vec![],
                    },
                    CategoryType::Income => UserCategories {
                        user_id,
                        expense: vec![],
                        income: existing.clone(),
                    },
                };
                service.file_store.write_categories(&categories).unwrap();
            }

            // Try to delete non-existent category
            let result = service.delete_category(&user_id, &nonexistent, &cat_type);
            prop_assert!(matches!(result, Err(AppError::NotFound(_))));

            // List should remain unchanged
            let after = service.list_categories(&user_id, &cat_type).unwrap();
            prop_assert_eq!(after.len(), existing.len());
        }
    }

    // --- Helpers for transaction property tests ---

    /// Creates a user profile on disk and returns the profile.
    fn create_test_user(file_store: &FileStore, username: &str) -> UserProfile {
        let now = Utc::now();
        let profile = UserProfile {
            id: Uuid::new_v4(),
            username: username.to_string(),
            email: format!("{}@test.com", username),
            master_ledger: String::new(),
            owned_accounts: vec![],
            shared_accounts: vec![],
            is_active: true,
            is_admin: false,
            created_at: now,
            updated_at: now,
        };
        file_store.write_profile(&profile).unwrap();
        profile
    }

    /// Creates an account on disk with the given owner and shared users.
    fn create_test_account(
        file_store: &FileStore,
        owner_id: Uuid,
        shared_with: Vec<SharedUser>,
        is_active: bool,
    ) -> Workspace {
        let now = Utc::now();
        let account = Workspace {
            id: Uuid::new_v4(),
            name: "Test Account".to_string(),
            owner_id,
            currency: "USD".to_string(),
            shared_with,
            is_active,
            created_at: now,
            updated_at: now,
            ledger_dir: None,
            rotation_period: RotationPeriod::default(),
            budgeting_enabled: false,
        };
        file_store.write_account(&account).unwrap();
        file_store.create_account_ledger(&account).unwrap();
        account
    }

    /// Strategy for generating valid payee strings (non-empty, printable ASCII).
    fn payee_strategy() -> impl Strategy<Value = String> {
        prop::collection::vec(
            prop::char::ranges(vec!['a'..='z', 'A'..='Z', '0'..='9', ' '..=' '].into()),
            1..=30,
        )
        .prop_map(|chars| chars.into_iter().collect::<String>())
    }

    /// Strategy for generating valid ledger account names.
    fn account_name_strategy() -> impl Strategy<Value = String> {
        prop::collection::vec(
            prop::char::ranges(vec!['a'..='z', 'A'..='Z', ':'..=':'].into()),
            1..=30,
        )
        .prop_map(|chars| chars.into_iter().collect::<String>())
    }

    /// Strategy for generating valid positive amounts (0.01 to 99999.99).
    fn amount_strategy() -> impl Strategy<Value = f64> {
        (1u64..10_000_000u64).prop_map(|cents| cents as f64 / 100.0)
    }

    /// Strategy for generating valid dates.
    fn date_strategy() -> impl Strategy<Value = NaiveDate> {
        (2000i32..2030, 1u32..13, 1u32..29).prop_map(|(y, m, d)| {
            NaiveDate::from_ymd_opt(y, m, d).unwrap()
        })
    }

    /// Strategy for generating simple usernames.
    fn username_strategy() -> impl Strategy<Value = String> {
        prop::collection::vec(
            prop::char::range('a', 'z'),
            3..=10,
        )
        .prop_map(|chars| chars.into_iter().collect::<String>())
    }

    // Feature: transaction-ledger-api, Property 8: Transaction formatting correctness
    // **Validates: Requirements 4.1, 4.2, 8.1, 8.2, 8.3**
    proptest! {
        #[test]
        fn prop_transaction_formatting_correctness(
            date in date_strategy(),
            payee in payee_strategy(),
            debit in account_name_strategy(),
            credit in account_name_strategy(),
            amount in amount_strategy(),
            username in username_strategy(),
        ) {
            let tmp = TempDir::new().unwrap();
            let service = make_test_service(&tmp);

            let formatted = service.format_transaction(&date, &payee, &debit, &credit, amount, &username);
            let lines: Vec<&str> = formatted.lines().collect();

            // Must have exactly 6 lines: header, ; Id, debit, ; User, credit, ; User
            prop_assert_eq!(lines.len(), 6, "Expected 6 lines, got {}: {:?}", lines.len(), lines);

            // Line 1: date and payee
            let expected_date = date.format("%Y-%m-%d").to_string();
            prop_assert!(lines[0].starts_with(&expected_date), "First line should start with date");
            prop_assert!(lines[0].contains(&payee), "First line should contain payee");

            // Line 2: Id metadata tag — uuid v4 format
            prop_assert!(lines[1].trim_start().starts_with("; Id:"), "Second line should carry Id tag, got {:?}", lines[1]);
            prop_assert!(lines[1].starts_with("    "), "Second line should be indented");

            // Line 3: debit account with positive amount
            let amount_str = format!("${:.2}", amount);
            prop_assert!(lines[2].contains(&debit), "Third line should contain debit account");
            prop_assert!(lines[2].contains(&amount_str), "Third line should contain amount");
            prop_assert!(lines[2].starts_with("    "), "Third line should be indented");

            // Line 4: User metadata tag
            let user_tag = format!("; User: {}", username);
            prop_assert!(lines[3].contains(&user_tag), "Fourth line should contain user tag");
            prop_assert!(lines[3].starts_with("    "), "Fourth line should be indented");

            // Line 5: credit account with negative amount
            let neg_amount_str = format!("-${:.2}", amount);
            prop_assert!(lines[4].contains(&credit), "Fifth line should contain credit account");
            prop_assert!(lines[4].contains(&neg_amount_str), "Fifth line should contain negative amount");
            prop_assert!(lines[4].starts_with("    "), "Fifth line should be indented");

            // Line 6: User metadata tag
            prop_assert!(lines[5].contains(&user_tag), "Sixth line should contain user tag");
            prop_assert!(lines[5].starts_with("    "), "Sixth line should be indented");
        }
    }

    // Feature: transaction-ledger-api, Property 9: Auto-add categories on transaction post
    // **Validates: Requirements 4.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]
        #[test]
        fn prop_auto_add_categories_on_post(
            debit in account_name_strategy(),
            credit in account_name_strategy(),
            amount in amount_strategy(),
            date in date_strategy(),
            payee in payee_strategy(),
        ) {
            let tmp = TempDir::new().unwrap();
            let service = make_test_service(&tmp);

            // Create owner user
            let owner = create_test_user(&service.file_store, "testowner");

            // Create active account owned by this user
            let account = create_test_account(&service.file_store, owner.id, vec![], true);

            let req = PostTransactionRequest {
                date: date.format("%Y-%m-%d").to_string(),
                payee: payee.clone(),
                debit_account: debit.clone(),
                credit_account: credit.clone(),
                amount: format!("{:.2}", amount),
            };

            // Post the transaction
            service.post_transaction(&account.id, &owner.id, &req).unwrap();

            // Verify debit was auto-added to chart of accounts under the correct type
            let debit_type = ChartOfAccounts::detect_account_type(&debit);
            let debit_list = service.list_accounts(&owner.id, &debit_type).unwrap();
            prop_assert!(debit_list.contains(&debit), "Debit account should be auto-added to chart of accounts");

            // Verify credit was auto-added to chart of accounts under the correct type
            let credit_type = ChartOfAccounts::detect_account_type(&credit);
            let credit_list = service.list_accounts(&owner.id, &credit_type).unwrap();
            prop_assert!(credit_list.contains(&credit), "Credit account should be auto-added to chart of accounts");
        }
    }

    // Feature: transaction-ledger-api, Property 10: Inactive account rejects transactions
    // **Validates: Requirements 4.6**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]
        #[test]
        fn prop_inactive_account_rejects_transactions(
            date in date_strategy(),
            payee in payee_strategy(),
            debit in account_name_strategy(),
            credit in account_name_strategy(),
            amount in amount_strategy(),
        ) {
            let tmp = TempDir::new().unwrap();
            let service = make_test_service(&tmp);

            // Create owner user
            let owner = create_test_user(&service.file_store, "inactiveowner");

            // Create INACTIVE account
            let account = create_test_account(&service.file_store, owner.id, vec![], false);

            let req = PostTransactionRequest {
                date: date.format("%Y-%m-%d").to_string(),
                payee,
                debit_account: debit,
                credit_account: credit,
                amount: format!("{:.2}", amount),
            };

            let result = service.post_transaction(&account.id, &owner.id, &req);
            prop_assert!(matches!(result, Err(AppError::BadRequest(ref msg)) if msg.contains("deactivated")),
                "Inactive account should return BadRequest, got: {:?}", result);
        }
    }

    // Feature: transaction-ledger-api, Property 11: Write authorization for transaction posting
    // **Validates: Requirements 4.7, 4.8, 10.3, 10.5**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]
        #[test]
        fn prop_write_authorization_for_posting(
            date in date_strategy(),
            payee in payee_strategy(),
            debit in account_name_strategy(),
            credit in account_name_strategy(),
            amount in amount_strategy(),
        ) {
            let tmp = TempDir::new().unwrap();
            let service = make_test_service(&tmp);

            // Create users
            let owner = create_test_user(&service.file_store, "authowner");
            let write_user = create_test_user(&service.file_store, "writeuser");
            let read_user = create_test_user(&service.file_store, "readuser");
            let no_access_user = create_test_user(&service.file_store, "noaccess");

            // Create account with shared users
            let account = create_test_account(
                &service.file_store,
                owner.id,
                vec![
                    SharedUser { user_id: write_user.id, permission: Permission::Write },
                    SharedUser { user_id: read_user.id, permission: Permission::Read },
                ],
                true,
            );

            let req = PostTransactionRequest {
                date: date.format("%Y-%m-%d").to_string(),
                payee,
                debit_account: debit,
                credit_account: credit,
                amount: format!("{:.2}", amount),
            };

            // Owner should succeed
            let owner_result = service.post_transaction(&account.id, &owner.id, &req);
            prop_assert!(owner_result.is_ok(), "Owner should be able to post: {:?}", owner_result);

            // Write user should succeed
            let write_result = service.post_transaction(&account.id, &write_user.id, &req);
            prop_assert!(write_result.is_ok(), "Write user should be able to post: {:?}", write_result);

            // Read user should get Forbidden
            let read_result = service.post_transaction(&account.id, &read_user.id, &req);
            prop_assert!(matches!(read_result, Err(AppError::Forbidden(_))),
                "Read user should get Forbidden, got: {:?}", read_result);

            // No-access user should get NotFound
            let no_access_result = service.post_transaction(&account.id, &no_access_user.id, &req);
            prop_assert!(matches!(no_access_result, Err(AppError::NotFound(_))),
                "No-access user should get NotFound, got: {:?}", no_access_result);
        }
    }

    // Feature: transaction-ledger-api, Property 12: Read authorization for queries
    // **Validates: Requirements 6.5, 6.6, 7.5, 7.6, 10.4**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]
        #[test]
        fn prop_read_authorization_for_queries(
            pivot_user in proptest::bool::ANY,
            filter_user in proptest::option::of("[a-z]{3,8}"),
        ) {
            let tmp = TempDir::new().unwrap();
            let service = make_test_service(&tmp);

            // Create users
            let owner = create_test_user(&service.file_store, "queryowner");
            let write_user = create_test_user(&service.file_store, "querywrite");
            let read_user = create_test_user(&service.file_store, "queryread");
            let no_access_user = create_test_user(&service.file_store, "querynone");

            // Create account with shared users
            let account = create_test_account(
                &service.file_store,
                owner.id,
                vec![
                    SharedUser { user_id: write_user.id, permission: Permission::Write },
                    SharedUser { user_id: read_user.id, permission: Permission::Read },
                ],
                true,
            );

            // --- Balance queries ---

            // Owner can query balance — should NOT get NotFound
            let owner_bal = service.query_balance(&account.id, &owner.id, pivot_user, None);
            prop_assert!(!matches!(owner_bal, Err(AppError::NotFound(_))),
                "Owner balance query should not return NotFound, got: {:?}", owner_bal);

            // Write user can query balance — should NOT get NotFound
            let write_bal = service.query_balance(&account.id, &write_user.id, pivot_user, None);
            prop_assert!(!matches!(write_bal, Err(AppError::NotFound(_))),
                "Write user balance query should not return NotFound, got: {:?}", write_bal);

            // Read user can query balance — should NOT get NotFound
            let read_bal = service.query_balance(&account.id, &read_user.id, pivot_user, None);
            prop_assert!(!matches!(read_bal, Err(AppError::NotFound(_))),
                "Read user balance query should not return NotFound, got: {:?}", read_bal);

            // No-access user should get NotFound for balance
            let no_access_bal = service.query_balance(&account.id, &no_access_user.id, pivot_user, None);
            prop_assert!(matches!(no_access_bal, Err(AppError::NotFound(_))),
                "No-access user balance query should return NotFound, got: {:?}", no_access_bal);

            // --- Register queries ---

            // Owner can query register — should NOT get NotFound
            let owner_reg = service.query_register(&account.id, &owner.id, filter_user.as_deref(), None, None, None);
            prop_assert!(!matches!(owner_reg, Err(AppError::NotFound(_))),
                "Owner register query should not return NotFound, got: {:?}", owner_reg);

            // Write user can query register — should NOT get NotFound
            let write_reg = service.query_register(&account.id, &write_user.id, filter_user.as_deref(), None, None, None);
            prop_assert!(!matches!(write_reg, Err(AppError::NotFound(_))),
                "Write user register query should not return NotFound, got: {:?}", write_reg);

            // Read user can query register — should NOT get NotFound
            let read_reg = service.query_register(&account.id, &read_user.id, filter_user.as_deref(), None, None, None);
            prop_assert!(!matches!(read_reg, Err(AppError::NotFound(_))),
                "Read user register query should not return NotFound, got: {:?}", read_reg);

            // No-access user should get NotFound for register
            let no_access_reg = service.query_register(&account.id, &no_access_user.id, filter_user.as_deref(), None, None, None);
            prop_assert!(matches!(no_access_reg, Err(AppError::NotFound(_))),
                "No-access user register query should return NotFound, got: {:?}", no_access_reg);
        }
    }

    /// Returns true if the `ledger` CLI is available on this system.
    fn is_ledger_available() -> bool {
        std::process::Command::new("ledger")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    // Feature: transaction-ledger-api, Property 17: Ledger format round-trip
    // **Validates: Requirements 8.4**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(15))]
        #[test]
        fn prop_ledger_format_round_trip(
            date in date_strategy(),
            payee in payee_strategy(),
            debit in account_name_strategy(),
            credit in account_name_strategy(),
            amount in amount_strategy(),
            username in username_strategy(),
        ) {
            // Skip if ledger CLI is not installed
            if !is_ledger_available() {
                return Ok(());
            }

            let tmp = TempDir::new().unwrap();
            let service = make_test_service(&tmp);

            // Format the transaction using TransactionService
            let formatted = service.format_transaction(&date, &payee, &debit, &credit, amount, &username);

            // Write the formatted transaction to a temp ledger file
            let ledger_file = tmp.path().join("roundtrip.ledger");
            std::fs::write(&ledger_file, &formatted).unwrap();

            // Run `ledger register -f <temp_file>` to parse it
            let output = std::process::Command::new("ledger")
                .arg("register")
                .arg("-f")
                .arg(&ledger_file)
                .output()
                .expect("Failed to execute ledger register");

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            // ledger should parse the file successfully
            prop_assert!(
                output.status.success(),
                "ledger register failed with status {}: stderr={}",
                output.status,
                stderr
            );

            // Verify the output contains the original date
            let date_str = date.format("%y-%b-%d").to_string();
            let date_ymd = date.format("%Y-%m-%d").to_string();
            let date_short = date.format("%y-%m-%d").to_string();
            prop_assert!(
                stdout.contains(&date_str) || stdout.contains(&date_ymd) || stdout.contains(&date_short),
                "Output should contain date ({} or {} or {}), got: {}",
                date_str, date_ymd, date_short, stdout
            );

            // Verify the output contains the payee
            prop_assert!(
                stdout.contains(payee.trim()),
                "Output should contain payee '{}', got: {}",
                payee.trim(), stdout
            );

            // Verify the output contains both account names
            prop_assert!(
                stdout.contains(&debit),
                "Output should contain debit account '{}', got: {}",
                debit, stdout
            );
            prop_assert!(
                stdout.contains(&credit),
                "Output should contain credit account '{}', got: {}",
                credit, stdout
            );

            // Verify the output contains the amount (ledger may format differently,
            // but the numeric value should appear)
            let amount_str = format!("{:.2}", amount);
            prop_assert!(
                stdout.contains(&amount_str),
                "Output should contain amount '{}', got: {}",
                amount_str, stdout
            );
        }
    }

    // --- Chart of Accounts unit tests ---

    #[test]
    fn load_or_create_chart_returns_empty_when_no_files() {
        let tmp = TempDir::new().unwrap();
        let service = make_test_service(&tmp);
        let user_id = Uuid::new_v4();

        let chart = service.load_or_create_chart(&user_id).unwrap();
        assert_eq!(chart.user_id, user_id);
        assert!(chart.assets.is_empty());
        assert!(chart.expenses.is_empty());
        assert!(chart.income.is_empty());
        assert!(chart.liabilities.is_empty());
        assert!(chart.equity.is_empty());
    }

    #[test]
    fn load_or_create_chart_returns_existing_chart() {
        let tmp = TempDir::new().unwrap();
        let service = make_test_service(&tmp);
        let user_id = Uuid::new_v4();

        let chart = ChartOfAccounts {
            user_id,
            assets: vec!["Assets:Checking".to_string()],
            expenses: vec!["Expenses:Food".to_string()],
            income: vec!["Income:Salary".to_string()],
            liabilities: vec![],
            equity: vec![],
        };
        service.file_store.write_chart_of_accounts(&chart).unwrap();

        let loaded = service.load_or_create_chart(&user_id).unwrap();
        assert_eq!(loaded, chart);
    }

    #[test]
    fn load_or_create_chart_migrates_from_categories() {
        let tmp = TempDir::new().unwrap();
        let service = make_test_service(&tmp);
        let user_id = Uuid::new_v4();

        // Write categories file
        let categories = UserCategories {
            user_id,
            expense: vec!["Food".to_string(), "Transport".to_string()],
            income: vec!["Salary".to_string()],
        };
        service.file_store.write_categories(&categories).unwrap();

        let chart = service.load_or_create_chart(&user_id).unwrap();
        assert_eq!(chart.expenses, vec!["Food".to_string(), "Transport".to_string()]);
        assert_eq!(chart.income, vec!["Salary".to_string()]);
        assert!(chart.assets.is_empty());
        assert!(chart.liabilities.is_empty());
        assert!(chart.equity.is_empty());

        // Categories file should be deleted
        assert!(service.file_store.read_categories(&user_id).unwrap().is_none());
        // Chart file should now exist
        assert!(service.file_store.read_chart_of_accounts(&user_id).unwrap().is_some());
    }

    #[test]
    fn load_or_create_chart_prefers_chart_over_categories() {
        let tmp = TempDir::new().unwrap();
        let service = make_test_service(&tmp);
        let user_id = Uuid::new_v4();

        // Write both files
        let categories = UserCategories {
            user_id,
            expense: vec!["OldExpense".to_string()],
            income: vec!["OldIncome".to_string()],
        };
        service.file_store.write_categories(&categories).unwrap();

        let chart = ChartOfAccounts {
            user_id,
            assets: vec!["Assets:Checking".to_string()],
            expenses: vec!["Expenses:Food".to_string()],
            income: vec![],
            liabilities: vec![],
            equity: vec![],
        };
        service.file_store.write_chart_of_accounts(&chart).unwrap();

        // Chart should win
        let loaded = service.load_or_create_chart(&user_id).unwrap();
        assert_eq!(loaded, chart);
    }

    #[test]
    fn list_accounts_returns_correct_type() {
        let tmp = TempDir::new().unwrap();
        let service = make_test_service(&tmp);
        let user_id = Uuid::new_v4();

        let chart = ChartOfAccounts {
            user_id,
            assets: vec!["Assets:Checking".to_string()],
            expenses: vec!["Expenses:Food".to_string(), "Expenses:Gas".to_string()],
            income: vec!["Income:Salary".to_string()],
            liabilities: vec![],
            equity: vec![],
        };
        service.file_store.write_chart_of_accounts(&chart).unwrap();

        let expenses = service.list_accounts(&user_id, &AccountType::Expenses).unwrap();
        assert_eq!(expenses, vec!["Expenses:Food".to_string(), "Expenses:Gas".to_string()]);

        let assets = service.list_accounts(&user_id, &AccountType::Assets).unwrap();
        assert_eq!(assets, vec!["Assets:Checking".to_string()]);

        let liabilities = service.list_accounts(&user_id, &AccountType::Liabilities).unwrap();
        assert!(liabilities.is_empty());
    }

    #[test]
    fn add_account_adds_to_correct_type() {
        let tmp = TempDir::new().unwrap();
        let service = make_test_service(&tmp);
        let user_id = Uuid::new_v4();

        service.add_account(&user_id, "Assets:Checking".to_string(), &AccountType::Assets).unwrap();
        service.add_account(&user_id, "Expenses:Food".to_string(), &AccountType::Expenses).unwrap();

        let assets = service.list_accounts(&user_id, &AccountType::Assets).unwrap();
        assert_eq!(assets, vec!["Assets:Checking".to_string()]);

        let expenses = service.list_accounts(&user_id, &AccountType::Expenses).unwrap();
        assert_eq!(expenses, vec!["Expenses:Food".to_string()]);
    }

    #[test]
    fn add_account_rejects_duplicate() {
        let tmp = TempDir::new().unwrap();
        let service = make_test_service(&tmp);
        let user_id = Uuid::new_v4();

        service.add_account(&user_id, "Expenses:Food".to_string(), &AccountType::Expenses).unwrap();
        let result = service.add_account(&user_id, "Expenses:Food".to_string(), &AccountType::Expenses);
        assert!(matches!(result, Err(AppError::Conflict(_))));
    }

    #[test]
    fn delete_account_removes_from_list() {
        let tmp = TempDir::new().unwrap();
        let service = make_test_service(&tmp);
        let user_id = Uuid::new_v4();

        service.add_account(&user_id, "Expenses:Food".to_string(), &AccountType::Expenses).unwrap();
        service.add_account(&user_id, "Expenses:Gas".to_string(), &AccountType::Expenses).unwrap();

        service.delete_account(&user_id, "Expenses:Food", &AccountType::Expenses).unwrap();

        let expenses = service.list_accounts(&user_id, &AccountType::Expenses).unwrap();
        assert_eq!(expenses, vec!["Expenses:Gas".to_string()]);
    }

    #[test]
    fn delete_account_returns_404_for_nonexistent() {
        let tmp = TempDir::new().unwrap();
        let service = make_test_service(&tmp);
        let user_id = Uuid::new_v4();

        let result = service.delete_account(&user_id, "Expenses:Nope", &AccountType::Expenses);
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[test]
    fn auto_add_from_transaction_detects_types() {
        let tmp = TempDir::new().unwrap();
        let service = make_test_service(&tmp);
        let user_id = Uuid::new_v4();

        service.auto_add_from_transaction(
            &user_id,
            "Expenses:Food:Groceries",
            "Assets:Checking",
        ).unwrap();

        let expenses = service.list_accounts(&user_id, &AccountType::Expenses).unwrap();
        assert!(expenses.contains(&"Expenses:Food:Groceries".to_string()));

        let assets = service.list_accounts(&user_id, &AccountType::Assets).unwrap();
        assert!(assets.contains(&"Assets:Checking".to_string()));
    }

    #[test]
    fn auto_add_from_transaction_skips_existing() {
        let tmp = TempDir::new().unwrap();
        let service = make_test_service(&tmp);
        let user_id = Uuid::new_v4();

        // Add once
        service.auto_add_from_transaction(&user_id, "Expenses:Food", "Income:Salary").unwrap();
        // Add again — should not error
        service.auto_add_from_transaction(&user_id, "Expenses:Food", "Income:Salary").unwrap();

        let expenses = service.list_accounts(&user_id, &AccountType::Expenses).unwrap();
        assert_eq!(expenses.iter().filter(|e| *e == "Expenses:Food").count(), 1);
    }

    // --- Transaction ID / delete / update tests ---

    fn mk_ws_with_user(service: &TransactionService, username: &str) -> (UserProfile, Workspace) {
        let user = create_test_user(&service.file_store, username);
        // Use the migrated workspace layout so period files live in
        // workspaces/workspace-{id}/. Build one by hand so the rotation_period
        // and ledger_dir fields match what `TransactionService` expects.
        let now = Utc::now();
        let ws = Workspace {
            id: Uuid::new_v4(),
            name: "IdTest".to_string(),
            owner_id: user.id,
            currency: "USD".to_string(),
            shared_with: vec![],
            is_active: true,
            created_at: now,
            updated_at: now,
            ledger_dir: Some(format!("workspaces/workspace-{}/", Uuid::new_v4())),
            rotation_period: RotationPeriod::Quarterly,
            budgeting_enabled: false,
        };
        // Re-patch ledger_dir to use the same id we just generated for ws.
        let mut ws = ws;
        ws.ledger_dir = Some(format!("workspaces/workspace-{}/", ws.id));
        // Ensure the workspace dir and root ledger exist so
        // `append_to_period_file` has something to `add_include_to_workspace_ledger` into.
        service.file_store.write_workspace(&ws).unwrap();
        service.file_store.create_workspace_dir(&ws).unwrap();
        service.file_store.create_workspace_ledger(&ws).unwrap();
        (user, ws)
    }

    #[test]
    fn post_transaction_emits_id_and_id_tag_in_file() {
        let tmp = TempDir::new().unwrap();
        let service = make_test_service(&tmp);
        let (user, ws) = mk_ws_with_user(&service, "idtester");

        let req = PostTransactionRequest {
            date: "2026-05-01".to_string(),
            payee: "Lunch".to_string(),
            debit_account: "Expenses:Food".to_string(),
            credit_account: "Assets:Bank:Revolut".to_string(),
            amount: "12.50".to_string(),
        };
        let resp = service.post_transaction(&ws.id, &user.id, &req).unwrap();
        let tx_id = resp.id.expect("post_transaction should return an id");

        // The period file should contain the Id tag.
        let paths = service.file_store.list_period_files(&ws).unwrap();
        let mut found_id = false;
        for p in paths {
            let contents = std::fs::read_to_string(&p).unwrap();
            if contents.contains(&format!("; Id: {}", tx_id)) {
                found_id = true;
                break;
            }
        }
        assert!(found_id, "Posted transaction should include its Id tag in the period file");
    }

    #[test]
    fn list_transactions_returns_only_id_bearing_entries_sorted_newest_first() {
        let tmp = TempDir::new().unwrap();
        let service = make_test_service(&tmp);
        let (user, ws) = mk_ws_with_user(&service, "idlister");

        let mk_req = |date: &str, payee: &str| PostTransactionRequest {
            date: date.to_string(),
            payee: payee.to_string(),
            debit_account: "Expenses:Food".to_string(),
            credit_account: "Assets:Bank:Revolut".to_string(),
            amount: "1.00".to_string(),
        };
        service.post_transaction(&ws.id, &user.id, &mk_req("2026-04-01", "Old")).unwrap();
        service.post_transaction(&ws.id, &user.id, &mk_req("2026-05-15", "New")).unwrap();

        let entries = service.list_transactions(&ws.id, &user.id).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].date, "2026-05-15");
        assert_eq!(entries[0].payee, "New");
        assert_eq!(entries[1].date, "2026-04-01");
        assert_eq!(entries[0].posted_by.as_deref(), Some("idlister"));
    }

    #[test]
    fn delete_transaction_removes_block_and_leaves_others() {
        let tmp = TempDir::new().unwrap();
        let service = make_test_service(&tmp);
        let (user, ws) = mk_ws_with_user(&service, "iddeleter");

        let keep = service.post_transaction(
            &ws.id,
            &user.id,
            &PostTransactionRequest {
                date: "2026-05-01".to_string(),
                payee: "Keep".to_string(),
                debit_account: "Expenses:Food".to_string(),
                credit_account: "Assets:Bank:Revolut".to_string(),
                amount: "1.00".to_string(),
            },
        ).unwrap().id.unwrap();
        let drop = service.post_transaction(
            &ws.id,
            &user.id,
            &PostTransactionRequest {
                date: "2026-05-02".to_string(),
                payee: "Drop".to_string(),
                debit_account: "Expenses:Food".to_string(),
                credit_account: "Assets:Bank:Revolut".to_string(),
                amount: "2.00".to_string(),
            },
        ).unwrap().id.unwrap();

        service.delete_transaction(&ws.id, &user.id, &drop).unwrap();

        let entries = service.list_transactions(&ws.id, &user.id).unwrap();
        let ids: Vec<Uuid> = entries.iter().map(|e| e.id).collect();
        assert!(ids.contains(&keep));
        assert!(!ids.contains(&drop));
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn delete_transaction_missing_returns_404() {
        let tmp = TempDir::new().unwrap();
        let service = make_test_service(&tmp);
        let (user, ws) = mk_ws_with_user(&service, "idnone");
        let result = service.delete_transaction(&ws.id, &user.id, &Uuid::new_v4());
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[test]
    fn update_transaction_in_place_preserves_id_and_original_user() {
        let tmp = TempDir::new().unwrap();
        let service = make_test_service(&tmp);
        let (user, ws) = mk_ws_with_user(&service, "idupdater");

        let tx_id = service.post_transaction(
            &ws.id,
            &user.id,
            &PostTransactionRequest {
                date: "2026-05-01".to_string(),
                payee: "Coffee".to_string(),
                debit_account: "Expenses:Food:Coffee".to_string(),
                credit_account: "Assets:Bank:Revolut".to_string(),
                amount: "4.00".to_string(),
            },
        ).unwrap().id.unwrap();

        // Add a second write-permission user and have them do the edit — the
        // author metadata should stay as the original poster.
        let editor = create_test_user(&service.file_store, "editor");
        let mut ws_with_editor = ws.clone();
        ws_with_editor.shared_with.push(crate::models::SharedUser {
            user_id: editor.id,
            permission: crate::models::Permission::Write,
        });
        service.file_store.write_workspace(&ws_with_editor).unwrap();

        let resp = service.update_transaction(
            &ws.id,
            &editor.id,
            &tx_id,
            &UpdateTransactionRequest {
                date: "2026-05-02".to_string(),
                payee: "Coffee v2".to_string(),
                debit_account: "Expenses:Food:Coffee".to_string(),
                credit_account: "Assets:Bank:Revolut".to_string(),
                amount: "5.00".to_string(),
            },
        ).unwrap();
        assert_eq!(resp.id, Some(tx_id));

        let entries = service.list_transactions(&ws.id, &user.id).unwrap();
        let updated = entries.iter().find(|e| e.id == tx_id).unwrap();
        assert_eq!(updated.date, "2026-05-02");
        assert_eq!(updated.payee, "Coffee v2");
        assert_eq!(updated.postings[0].amount, "$5.00");
        // Original poster's username is preserved.
        assert_eq!(updated.posted_by.as_deref(), Some("idupdater"));
        // Still exactly one entry with this id.
        assert_eq!(entries.iter().filter(|e| e.id == tx_id).count(), 1);
    }

    #[test]
    fn update_transaction_moves_across_period_boundary() {
        let tmp = TempDir::new().unwrap();
        let service = make_test_service(&tmp);
        let (user, ws) = mk_ws_with_user(&service, "idmover");

        // Post in Q2 2026.
        let tx_id = service.post_transaction(
            &ws.id,
            &user.id,
            &PostTransactionRequest {
                date: "2026-05-01".to_string(),
                payee: "Move me".to_string(),
                debit_account: "Expenses:Food".to_string(),
                credit_account: "Assets:Bank:Revolut".to_string(),
                amount: "1.00".to_string(),
            },
        ).unwrap().id.unwrap();

        // Update to a Q4 2026 date → should land in the Q4 file, not Q2.
        service.update_transaction(
            &ws.id,
            &user.id,
            &tx_id,
            &UpdateTransactionRequest {
                date: "2026-11-15".to_string(),
                payee: "Moved".to_string(),
                debit_account: "Expenses:Food".to_string(),
                credit_account: "Assets:Bank:Revolut".to_string(),
                amount: "2.00".to_string(),
            },
        ).unwrap();

        let entries = service.list_transactions(&ws.id, &user.id).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, tx_id);
        assert_eq!(entries[0].date, "2026-11-15");
        assert_eq!(entries[0].payee, "Moved");

        // Verify placement on disk — the entry should be in the Q4 file.
        let q2 = service.file_store.period_file_path(&ws, "2026-Q2");
        let q4 = service.file_store.period_file_path(&ws, "2026-Q4");
        if q2.exists() {
            let c = std::fs::read_to_string(&q2).unwrap();
            assert!(!c.contains(&tx_id.to_string()), "Old period file should no longer contain the tx id");
        }
        assert!(q4.exists(), "Q4 period file should have been created");
        let c4 = std::fs::read_to_string(&q4).unwrap();
        assert!(c4.contains(&tx_id.to_string()));
        assert!(c4.contains("Moved"));
    }

    #[test]
    fn delete_transaction_requires_write_access() {
        let tmp = TempDir::new().unwrap();
        let service = make_test_service(&tmp);
        let (owner, ws) = mk_ws_with_user(&service, "owner");
        let reader = create_test_user(&service.file_store, "reader");
        let mut ws2 = ws.clone();
        ws2.shared_with.push(crate::models::SharedUser {
            user_id: reader.id,
            permission: crate::models::Permission::Read,
        });
        service.file_store.write_workspace(&ws2).unwrap();

        let tx_id = service.post_transaction(
            &ws.id,
            &owner.id,
            &PostTransactionRequest {
                date: "2026-05-01".to_string(),
                payee: "T".to_string(),
                debit_account: "Expenses:Food".to_string(),
                credit_account: "Assets:Bank:Revolut".to_string(),
                amount: "1.00".to_string(),
            },
        ).unwrap().id.unwrap();

        let result = service.delete_transaction(&ws.id, &reader.id, &tx_id);
        assert!(matches!(result, Err(AppError::Forbidden(_))));
    }

    #[test]
    fn auto_add_from_transaction_fallback_to_expenses() {
        let tmp = TempDir::new().unwrap();
        let service = make_test_service(&tmp);
        let user_id = Uuid::new_v4();

        service.auto_add_from_transaction(&user_id, "UnknownAccount", "Liabilities:CreditCard").unwrap();

        // Unknown prefix falls back to expenses
        let expenses = service.list_accounts(&user_id, &AccountType::Expenses).unwrap();
        assert!(expenses.contains(&"UnknownAccount".to_string()));

        let liabilities = service.list_accounts(&user_id, &AccountType::Liabilities).unwrap();
        assert!(liabilities.contains(&"Liabilities:CreditCard".to_string()));
    }
}
