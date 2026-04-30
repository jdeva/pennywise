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
