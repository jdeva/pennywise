import apiClient from './client'
import type { WorkspacePublic } from '@/lib/types'

export const workspacesApi = {
  create: (name: string, currency?: string) =>
    apiClient.post<WorkspacePublic>('/workspaces', { name, ...(currency ? { currency } : {}) }),

  list: () =>
    apiClient.get<WorkspacePublic[]>('/workspaces'),

  get: (id: string) =>
    apiClient.get<WorkspacePublic>(`/workspaces/${id}`),

  update: (id: string, name: string) =>
    apiClient.put<WorkspacePublic>(`/workspaces/${id}`, { name }),

  deactivate: (id: string) =>
    apiClient.post(`/workspaces/${id}/deactivate`),

  share: (id: string, username: string, permission: 'read' | 'write' = 'read') =>
    apiClient.post<WorkspacePublic>(`/workspaces/${id}/share`, { username, permission }),

  unshare: (id: string, userId: string) =>
    apiClient.delete<WorkspacePublic>(`/workspaces/${id}/share/${userId}`),
}
