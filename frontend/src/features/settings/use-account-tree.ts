import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { chartOfAccountsApi } from '@/lib/api/chart-of-accounts'
import {
  type AccountType,
  type AccountTypeTree,
  buildAccountTypeTree,
} from '@/lib/account-tree'

const ACCOUNT_TYPES: AccountType[] = [
  'assets',
  'expenses',
  'income',
  'liabilities',
  'equity',
]

function useAccountTypeQuery(type: AccountType) {
  return useQuery({
    queryKey: ['chart-of-accounts', type],
    queryFn: async () => {
      const { data } = await chartOfAccountsApi.list(type)
      return data
    },
  })
}

export interface UseAccountTreeReturn {
  trees: AccountTypeTree[]
  isLoading: boolean
  addAccount: (
    parentPath: string,
    leafName: string,
    accountType: AccountType,
  ) => Promise<string>
  deleteAccount: (fullPath: string, accountType: AccountType) => Promise<void>
}

export function useAccountTree(): UseAccountTreeReturn {
  const queryClient = useQueryClient()

  const queries = {
    assets: useAccountTypeQuery('assets'),
    expenses: useAccountTypeQuery('expenses'),
    income: useAccountTypeQuery('income'),
    liabilities: useAccountTypeQuery('liabilities'),
    equity: useAccountTypeQuery('equity'),
  }

  const isLoading = ACCOUNT_TYPES.some((t) => queries[t].isLoading)

  const trees: AccountTypeTree[] = ACCOUNT_TYPES.map((type) =>
    buildAccountTypeTree(type, queries[type].data ?? []),
  )

  const addMutation = useMutation({
    mutationFn: async ({
      fullPath,
      accountType,
    }: {
      fullPath: string
      accountType: AccountType
    }) => {
      await chartOfAccountsApi.add({ name: fullPath, account_type: accountType })
    },
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({
        queryKey: ['chart-of-accounts', variables.accountType],
      })
    },
  })

  const deleteMutation = useMutation({
    mutationFn: async ({
      fullPath,
      accountType,
    }: {
      fullPath: string
      accountType: AccountType
    }) => {
      await chartOfAccountsApi.delete(fullPath, accountType)
    },
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({
        queryKey: ['chart-of-accounts', variables.accountType],
      })
    },
  })

  const addAccount = async (
    parentPath: string,
    leafName: string,
    accountType: AccountType,
  ): Promise<string> => {
    const trimmed = leafName.trim()
    if (!trimmed) {
      throw new Error('Account name cannot be empty')
    }
    const typeLabel: Record<AccountType, string> = {
      assets: 'Assets', expenses: 'Expenses', income: 'Income',
      liabilities: 'Liabilities', equity: 'Equity',
    }
    const prefix = typeLabel[accountType]
    // Always ensure the full path starts with the type prefix
    let fullPath: string
    if (!parentPath) {
      fullPath = `${prefix}:${trimmed}`
    } else if (parentPath.startsWith(`${prefix}:`)) {
      fullPath = `${parentPath}:${trimmed}`
    } else {
      fullPath = `${prefix}:${parentPath}:${trimmed}`
    }
    await addMutation.mutateAsync({ fullPath, accountType })
    return fullPath
  }

  const deleteAccount = async (
    fullPath: string,
    accountType: AccountType,
  ): Promise<void> => {
    const typeLabel: Record<AccountType, string> = {
      assets: 'Assets', expenses: 'Expenses', income: 'Income',
      liabilities: 'Liabilities', equity: 'Equity',
    }
    const prefix = typeLabel[accountType]
    const apiPath = fullPath.startsWith(`${prefix}:`) ? fullPath : `${prefix}:${fullPath}`
    await deleteMutation.mutateAsync({ fullPath: apiPath, accountType })
  }

  return { trees, isLoading, addAccount, deleteAccount }
}
