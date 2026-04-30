export interface UserPublic {
  id: string
  username: string
  email: string
  master_ledger: string
  owned_accounts: string[]
  shared_accounts: string[]
  is_active: boolean
  is_admin: boolean
  created_at: string
  updated_at: string
}

export interface WorkspacePublic {
  id: string
  name: string
  owner_id: string
  currency: string
  shared_with: SharedUser[]
  is_active: boolean
  created_at: string
  updated_at: string
  ledger_dir: string | null
  rotation_period: 'quarterly' | 'semiannual' | 'yearly'
  budgeting_enabled: boolean
}

export interface SharedUser {
  user_id: string
  permission: 'read' | 'write'
}

export interface PostTransactionRequest {
  date: string
  payee: string
  debit_account: string
  credit_account: string
  amount: string
}

export interface TransactionResponse {
  formatted_text: string
}

export interface BalanceResponse {
  output: string
}

export interface RegisterResponse {
  output: string
}

export interface BudgetDefinition {
  period: string
  account: string
  amount: number
  currency: string
}

export interface CreateBudgetRequest {
  period: string
  account: string
  amount: string
  currency?: string
}

export interface BudgetReportResponse {
  output: string
}

export interface AddAccountRequest {
  name: string
  account_type: 'assets' | 'expenses' | 'income' | 'liabilities' | 'equity'
}

export interface AddCategoryRequest {
  name: string
  category_type: 'expense' | 'income'
}

export interface AuthResponse {
  access_token: string
  refresh_token: string
  user: UserPublic
}

export interface ApiError {
  error: string
}

export interface ValidationError {
  error: string
  details: { field: string; message: string }[]
}
