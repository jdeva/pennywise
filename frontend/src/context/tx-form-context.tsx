import React, { createContext, useCallback, useContext, useMemo, useState } from 'react'
import type { TransactionEntry } from '@/lib/types'

interface TxFormContextType {
  open: boolean
  editing: TransactionEntry | null
  openForNew: () => void
  openForEdit: (tx: TransactionEntry) => void
  close: () => void
}

const TxFormContext = createContext<TxFormContextType | undefined>(undefined)

export function TxFormProvider({ children }: { children: React.ReactNode }) {
  const [open, setOpen] = useState(false)
  const [editing, setEditing] = useState<TransactionEntry | null>(null)

  const openForNew = useCallback(() => {
    setEditing(null)
    setOpen(true)
  }, [])

  const openForEdit = useCallback((tx: TransactionEntry) => {
    setEditing(tx)
    setOpen(true)
  }, [])

  const close = useCallback(() => {
    setOpen(false)
    setEditing(null)
  }, [])

  const value = useMemo(
    () => ({ open, editing, openForNew, openForEdit, close }),
    [open, editing, openForNew, openForEdit, close],
  )

  return <TxFormContext.Provider value={value}>{children}</TxFormContext.Provider>
}

export function useTxForm() {
  const ctx = useContext(TxFormContext)
  if (!ctx) throw new Error('useTxForm must be used within TxFormProvider')
  return ctx
}
