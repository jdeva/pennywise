import apiClient from './client'
import type {
  PostTransactionRequest,
  TransactionEntry,
  TransactionResponse,
  BalanceResponse,
  RegisterResponse,
  UpdateTransactionRequest,
} from '@/lib/types'

export const transactionsApi = {
  post: (workspaceId: string, data: PostTransactionRequest) =>
    apiClient.post<TransactionResponse>(`/workspaces/${workspaceId}/transactions`, data),

  list: (workspaceId: string) =>
    apiClient.get<TransactionEntry[]>(`/workspaces/${workspaceId}/transactions`),

  update: (workspaceId: string, txId: string, data: UpdateTransactionRequest) =>
    apiClient.put<TransactionResponse>(
      `/workspaces/${workspaceId}/transactions/${txId}`,
      data,
    ),

  delete: (workspaceId: string, txId: string) =>
    apiClient.delete(`/workspaces/${workspaceId}/transactions/${txId}`),

  getBalance: (workspaceId: string, opts?: { pivotUser?: boolean; user?: string }) =>
    apiClient.get<BalanceResponse>(`/workspaces/${workspaceId}/balance`, {
      params: {
        ...(opts?.pivotUser ? { pivot_user: true } : {}),
        ...(opts?.user ? { user: opts.user } : {}),
      },
    }),

  getRegister: (
    workspaceId: string,
    opts?: { user?: string; payee?: string; begin?: string; end?: string },
  ) =>
    apiClient.get<RegisterResponse>(`/workspaces/${workspaceId}/register`, {
      params: {
        ...(opts?.user ? { user: opts.user } : {}),
        ...(opts?.payee ? { payee: opts.payee } : {}),
        ...(opts?.begin ? { begin: opts.begin } : {}),
        ...(opts?.end ? { end: opts.end } : {}),
      },
    }),
}
