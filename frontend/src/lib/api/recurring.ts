import apiClient from './client'

export interface RecurringDefinition {
  period: string
  account: string
  counter_account: string
  amount: number
  currency: string
  payee?: string | null
}

export interface CreateRecurringRequest {
  period: string
  account: string
  counter_account: string
  amount: string
  currency?: string
  payee?: string
}

export interface RecurringDefinitionResponse {
  formatted_text: string
  definition: RecurringDefinition
}

export interface RecurringForecastResponse {
  output: string
}

export const recurringApi = {
  list: (workspaceId: string) =>
    apiClient.get<RecurringDefinition[]>(`/workspaces/${workspaceId}/recurring`),

  create: (workspaceId: string, data: CreateRecurringRequest) =>
    apiClient.post<RecurringDefinitionResponse>(`/workspaces/${workspaceId}/recurring`, data),

  update: (workspaceId: string, index: number, data: CreateRecurringRequest) =>
    apiClient.put<RecurringDefinitionResponse>(
      `/workspaces/${workspaceId}/recurring/${index}`,
      data,
    ),

  delete: (workspaceId: string, index: number) =>
    apiClient.delete(`/workspaces/${workspaceId}/recurring/${index}`),

  forecast: (workspaceId: string, endDate?: string) =>
    apiClient.get<RecurringForecastResponse>(`/workspaces/${workspaceId}/recurring/forecast`, {
      params: endDate ? { end_date: endDate } : undefined,
    }),
}
