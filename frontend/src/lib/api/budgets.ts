import apiClient from './client'
import type {
  WorkspacePublic,
  BudgetDefinition,
  CreateBudgetRequest,
  BudgetReportResponse,
} from '@/lib/types'

interface BudgetDefinitionResponse {
  formatted_text: string
  definition: BudgetDefinition
}

export const budgetsApi = {
  getBudgeting: (workspaceId: string) =>
    apiClient.get<{ budgeting_enabled: boolean }>(`/workspaces/${workspaceId}/budgeting`),

  setBudgeting: (workspaceId: string, enabled: boolean) =>
    apiClient.put<WorkspacePublic>(`/workspaces/${workspaceId}/budgeting`, { enabled }),

  list: (workspaceId: string) =>
    apiClient.get<BudgetDefinition[]>(`/workspaces/${workspaceId}/budgets`),

  create: (workspaceId: string, data: CreateBudgetRequest) =>
    apiClient.post<BudgetDefinitionResponse>(`/workspaces/${workspaceId}/budgets`, data),

  update: (workspaceId: string, index: number, data: CreateBudgetRequest) =>
    apiClient.put<BudgetDefinitionResponse>(`/workspaces/${workspaceId}/budgets/${index}`, data),

  delete: (workspaceId: string, index: number) =>
    apiClient.delete(`/workspaces/${workspaceId}/budgets/${index}`),

  report: (workspaceId: string, begin?: string, end?: string) =>
    apiClient.get<BudgetReportResponse>(`/workspaces/${workspaceId}/budgets/report`, {
      params: { ...(begin && { begin }), ...(end && { end }) },
    }),

  unbudgeted: (workspaceId: string, begin?: string, end?: string) =>
    apiClient.get<BudgetReportResponse>(`/workspaces/${workspaceId}/budgets/unbudgeted`, {
      params: { ...(begin && { begin }), ...(end && { end }) },
    }),

  forecast: (workspaceId: string, endDate?: string) =>
    apiClient.get<BudgetReportResponse>(`/workspaces/${workspaceId}/budgets/forecast`, {
      params: endDate ? { end_date: endDate } : undefined,
    }),
}
