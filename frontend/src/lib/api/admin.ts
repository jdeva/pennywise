import apiClient from './client'
import type { UserPublic } from '@/lib/types'

export const adminApi = {
  listUsers: () =>
    apiClient.get<UserPublic[]>('/admin/users'),

  setUserActive: (id: string, active: boolean) =>
    apiClient.put<UserPublic>(`/admin/users/${id}/active`, { is_active: active }),

  setUserRole: (id: string, isAdmin: boolean) =>
    apiClient.put<UserPublic>(`/admin/users/${id}/role`, { is_admin: isAdmin }),
}
