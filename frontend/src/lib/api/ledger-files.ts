import apiClient from './client'

export interface LedgerFileEntry {
  path: string
  label: string
  workspace_id: string | null
  workspace_name: string | null
  bytes: number
}

export interface LedgerFileContent {
  path: string
  content: string
}

export interface LedgerValidateResult {
  ok: boolean
  output: string
}

export const ledgerFilesApi = {
  list: () =>
    apiClient.get<LedgerFileEntry[]>('/ledger-files').then((r) => r.data),

  read: (path: string) =>
    apiClient
      .get<LedgerFileContent>('/ledger-files/content', { params: { path } })
      .then((r) => r.data),

  write: (path: string, content: string) =>
    apiClient
      .put('/ledger-files/content', { content }, { params: { path } })
      .then((r) => r.data),

  validate: (content: string) =>
    apiClient
      .post<LedgerValidateResult>('/ledger-files/validate', { content })
      .then((r) => r.data),
}
