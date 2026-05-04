import { useQueryClient } from '@tanstack/react-query'
import { useWorkspace } from '@/context/workspace-context'
import { useTxForm } from '@/context/tx-form-context'
import { TransactionForm } from '@/features/transactions/transaction-form'
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetBody,
  SheetTitle,
  SheetDescription,
} from '@/components/ui/sheet'

export function TxFormSheet() {
  const { activeWorkspace } = useWorkspace()
  const { open, editing, close } = useTxForm()
  const queryClient = useQueryClient()

  const handleSuccess = () => {
    queryClient.invalidateQueries({ queryKey: ['register', activeWorkspace?.id] })
    queryClient.invalidateQueries({ queryKey: ['balance', activeWorkspace?.id] })
    queryClient.invalidateQueries({ queryKey: ['transactions', activeWorkspace?.id] })
    close()
  }

  return (
    <Sheet open={open} onOpenChange={(v) => (v ? null : close())}>
      <SheetContent side="right">
        <SheetHeader>
          <SheetTitle>{editing ? 'Edit transaction' : 'New transaction'}</SheetTitle>
          <SheetDescription>
            {editing
              ? 'Adjust any field — history stays intact.'
              : 'Record an expense, income, or transfer.'}
          </SheetDescription>
        </SheetHeader>
        <SheetBody>
          <TransactionForm
            key={editing?.id ?? 'new'}
            editing={editing}
            onSuccess={handleSuccess}
            onCancel={close}
          />
        </SheetBody>
      </SheetContent>
    </Sheet>
  )
}
