import apiClient from './client'
import type { AuthResponse, UserPublic } from '@/lib/types'

interface LoginRequest {
  username: string
  password: string
}

interface RegisterRequest {
  username: string
  email: string
  password: string
}

export const authApi = {
  login: (data: LoginRequest) =>
    apiClient.post<AuthResponse>('/auth/login', data),

  register: (data: RegisterRequest) =>
    apiClient.post<AuthResponse>('/auth/register', data),

  refresh: (refreshToken: string) =>
    apiClient.post<AuthResponse>('/auth/refresh', { refresh_token: refreshToken }),

  logout: () =>
    apiClient.post('/auth/logout'),

  getProfile: () =>
    apiClient.get<UserPublic>('/users/me'),
}
