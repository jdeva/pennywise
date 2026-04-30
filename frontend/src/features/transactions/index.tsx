import { useQueryClient } from '@tanstack/react-query'
import { useWorkspace } from '@/context/workspace-context'
import { TransactionForm } from './transaction-form'
import { RegisterView } from './register-view'

export function TransactionsPage() {
  const { activeWorkspace } = useWorkspace()
  const queryClient = useQueryClient()

  const handleSuccess = () => {
    queryClient.invalidateQueries({ queryKey: ['register', activeWorkspace?.id] })
    queryClient.invalidateQueries({ queryKey: ['balance', activeWorkspace?.id] })
  }

  if (!activeWorkspace) {
    return (
      <div className="flex min-h-[60vh] flex-col items-center justify-center gap-4 rounded-2xl bg-warm-gradient p-12 text-center">
        <h2 className="font-display text-2xl font-semibold">No workspace selected</h2>
        <p className="max-w-sm text-muted-foreground">Pick a workspace from the menu above to post transactions.</p>
      </div>
    )
  }

  return (
    <div className="space-y-8">
      <div>
        <p className="text-sm font-medium uppercase tracking-wider text-muted-foreground">Activity</p>
        <h1 className="mt-1 font-display text-4xl font-semibold tracking-tight">Transactions</h1>
      </div>
      <TransactionForm onSuccess={handleSuccess} />
      <RegisterView />
    </div>
  )
}
