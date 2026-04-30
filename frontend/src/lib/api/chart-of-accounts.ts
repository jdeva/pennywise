import apiClient from './client'
import type { AddAccountRequest } from '@/lib/types'

type AccountType = 'assets' | 'expenses' | 'income' | 'liabilities' | 'equity'

export const chartOfAccountsApi = {
  list: (type: AccountType) =>
    apiClient.get<string[]>('/chart-of-accounts', { params: { type } }),

  add: (data: AddAccountRequest) =>
    apiClient.post('/chart-of-accounts', data),

  delete: (name: string, accountType: AccountType) =>
    apiClient.delete('/chart-of-accounts', { data: { name, account_type: accountType } }),
}
