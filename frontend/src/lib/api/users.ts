import apiClient from './client'
import type { UserPublic } from '@/lib/types'

export const usersApi = {
  getProfile: () =>
    apiClient.get<UserPublic>('/users/me'),

  updateProfile: (data: { username?: string; email?: string }) =>
    apiClient.put<UserPublic>('/users/me', data),

  changePassword: (currentPassword: string, newPassword: string) =>
    apiClient.post<{ message: string }>('/users/me/password', {
      current_password: currentPassword,
      new_password: newPassword,
    }),

  deactivate: (password: string) =>
    apiClient.post<{ message: string }>('/users/me/deactivate', { password }),
}
