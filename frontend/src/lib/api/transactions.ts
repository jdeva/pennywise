import apiClient from './client'
import type {
  PostTransactionRequest,
  TransactionResponse,
  BalanceResponse,
  RegisterResponse,
} from '@/lib/types'

export const transactionsApi = {
  post: (workspaceId: string, data: PostTransactionRequest) =>
    apiClient.post<TransactionResponse>(`/workspaces/${workspaceId}/transactions`, data),

  getBalance: (workspaceId: string, pivotUser?: boolean) =>
    apiClient.get<BalanceResponse>(`/workspaces/${workspaceId}/balance`, {
      params: pivotUser ? { pivot_user: true } : undefined,
    }),

  getRegister: (workspaceId: string, user?: string) =>
    apiClient.get<RegisterResponse>(`/workspaces/${workspaceId}/register`, {
      params: user ? { user } : undefined,
    }),
}
