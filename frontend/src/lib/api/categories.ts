import apiClient from './client'
import type { AddCategoryRequest } from '@/lib/types'

type CategoryType = 'expense' | 'income'

export const categoriesApi = {
  list: (type: CategoryType) =>
    apiClient.get<string[]>('/categories', { params: { type } }),

  add: (data: AddCategoryRequest) =>
    apiClient.post('/categories', data),

  delete: (name: string, categoryType: CategoryType) =>
    apiClient.delete('/categories', { data: { name, category_type: categoryType } }),
}
