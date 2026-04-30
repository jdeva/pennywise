import { useState, useEffect, useMemo } from 'react'
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { useQuery } from '@tanstack/react-query'
import { transactionsApi } from '@/lib/api/transactions'
import { chartOfAccountsApi } from '@/lib/api/chart-of-accounts'
import { useWorkspace } from '@/context/workspace-context'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { transactionSchema, type TransactionForm } from '@/lib/schemas'

type TxType = 'expense' | 'income' | 'transfer'

interface Props {
  onSuccess: () => void
}

// Account type prefixes for filtering
const DEBIT_TYPES: Record<TxType, string[]> = {
  expense: ['Expenses:'],
  income: ['Assets:'],
  transfer: ['Assets:', 'Liabilities:'],
}

const CREDIT_TYPES: Record<TxType, string[]> = {
  expense: ['Assets:', 'Liabilities:'],
  income: ['Income:'],
  transfer: ['Assets:', 'Liabilities:'],
}

const TX_TYPE_LABELS: { value: TxType; label: string }[] = [
  { value: 'expense', label: 'Expense' },
  { value: 'income', label: 'Income' },
  { value: 'transfer', label: 'Transfer' },
]

export function TransactionForm({ onSuccess }: Props) {
  const { activeWorkspace } = useWorkspace()
  const [error, setError] = useState<string | null>(null)
  const [txType, setTxType] = useState<TxType>('expense')
  const [debitFilter, setDebitFilter] = useState('')
  const [creditFilter, setCreditFilter] = useState('')
  const [showDebitSuggestions, setShowDebitSuggestions] = useState(false)
  const [showCreditSuggestions, setShowCreditSuggestions] = useState(false)

  // Fetch all accounts with type prefix, leaf-only
  const { data: accountsByPrefix = [] } = useQuery({
    queryKey: ['all-accounts'],
    queryFn: async () => {
      const types = ['assets', 'expenses', 'income', 'liabilities', 'equity'] as const
      const typeLabels: Record<string, string> = {
        assets: 'Assets', expenses: 'Expenses', income: 'Income',
        liabilities: 'Liabilities', equity: 'Equity',
      }
      const results = await Promise.all(
        types.map((t) => chartOfAccountsApi.list(t).then((r) =>
          r.data.map((name) => `${typeLabels[t]}:${name}`)
        )),
      )
      const all = results.flat()
      return all.filter((a) => !all.some((other) => other !== a && other.startsWith(a + ':')))
    },
  })

  const {
    register, handleSubmit, reset, setValue, watch,
    formState: { errors, isSubmitting },
  } = useForm<TransactionForm>({
    resolver: zodResolver(transactionSchema),
    defaultValues: {
      date: new Date().toISOString().slice(0, 10),
      payee: '', debit_account: '', credit_account: '', amount: '',
    },
  })

  const debitValue = watch('debit_account')
  const creditValue = watch('credit_account')

  useEffect(() => { setDebitFilter(debitValue) }, [debitValue])
  useEffect(() => { setCreditFilter(creditValue) }, [creditValue])

  // Filter accounts by transaction type + text filter
  const debitAccounts = useMemo(() => {
    const prefixes = DEBIT_TYPES[txType]
    return accountsByPrefix
      .filter((a) => prefixes.some((p) => a.startsWith(p)))
      .filter((a) => a.toLowerCase().includes(debitFilter.toLowerCase()))
  }, [accountsByPrefix, txType, debitFilter])

  const creditAccounts = useMemo(() => {
    const prefixes = CREDIT_TYPES[txType]
    return accountsByPrefix
      .filter((a) => prefixes.some((p) => a.startsWith(p)))
      .filter((a) => a.toLowerCase().includes(creditFilter.toLowerCase()))
  }, [accountsByPrefix, txType, creditFilter])

  // Clear account fields when transaction type changes
  const handleTxTypeChange = (type: TxType) => {
    setTxType(type)
    setValue('debit_account', '')
    setValue('credit_account', '')
  }

  // Dynamic labels based on transaction type
  const debitLabel = txType === 'expense' ? 'Category' : txType === 'income' ? 'To Account' : 'From Account'
  const creditLabel = txType === 'expense' ? 'From Account' : txType === 'income' ? 'Income Source' : 'To Account'
  const debitPlaceholder = txType === 'expense' ? 'Expenses:Food' : txType === 'income' ? 'Assets:Checking' : 'Assets:Savings'
  const creditPlaceholder = txType === 'expense' ? 'Assets:Checking' : txType === 'income' ? 'Income:Salary' : 'Assets:Checking'

  const onSubmit = async (data: TransactionForm) => {
    if (!activeWorkspace) return
    setError(null)
    try {
      await transactionsApi.post(activeWorkspace.id, data)
      reset()
      onSuccess()
    } catch (err: any) {
      setError(err?.response?.data?.error || err?.error || 'Failed to post transaction')
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Post Transaction</CardTitle>
      </CardHeader>
      <form onSubmit={handleSubmit(onSubmit)}>
        <CardContent className="space-y-4">
          {error && (
            <div role="alert" className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">
              {error}
            </div>
          )}

          {/* Transaction Type Selector */}
          <div className="space-y-2">
            <Label>Type</Label>
            <div className="flex gap-2">
              {TX_TYPE_LABELS.map(({ value, label }) => {
                const active = txType === value
                const activeClass =
                  value === 'expense'
                    ? 'bg-accent-rose/20 text-accent-rose ring-accent-rose/40 shadow-soft'
                    : value === 'income'
                    ? 'bg-accent-mint/20 text-accent-mint ring-accent-mint/40 shadow-soft'
                    : 'bg-accent-sky/20 text-accent-sky ring-accent-sky/40 shadow-soft'
                return (
                  <button
                    key={value}
                    type="button"
                    onClick={() => handleTxTypeChange(value)}
                    className={[
                      'flex-1 rounded-full px-4 py-2 text-sm font-medium transition-all ring-1',
                      active ? activeClass : 'bg-transparent text-muted-foreground ring-border hover:bg-muted/60',
                    ].join(' ')}
                  >
                    {label}
                  </button>
                )
              })}
            </div>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="date">Date</Label>
              <Input id="date" type="date" {...register('date')} />
              {errors.date && <p className="text-sm text-destructive">{errors.date.message}</p>}
            </div>
            <div className="space-y-2">
              <Label htmlFor="amount">Amount</Label>
              <Input id="amount" placeholder="42.50" {...register('amount')} />
              {errors.amount && <p className="text-sm text-destructive">{errors.amount.message}</p>}
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="payee">Description</Label>
            <Input id="payee" placeholder="Grocery shopping" {...register('payee')} />
            {errors.payee && <p className="text-sm text-destructive">{errors.payee.message}</p>}
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2 relative">
              <Label htmlFor="debit_account">{debitLabel}</Label>
              <Input
                id="debit_account"
                placeholder={debitPlaceholder}
                autoComplete="off"
                {...register('debit_account')}
                onFocus={() => setShowDebitSuggestions(true)}
                onBlur={() => setTimeout(() => setShowDebitSuggestions(false), 200)}
              />
              {errors.debit_account && <p className="text-sm text-destructive">{errors.debit_account.message}</p>}
              {showDebitSuggestions && debitAccounts.length > 0 && (
                <ul className="absolute z-10 mt-1 max-h-40 w-full overflow-auto rounded-md border bg-popover p-1 text-sm shadow-md">
                  {debitAccounts.slice(0, 10).map((a) => (
                    <li
                      key={a}
                      className="cursor-pointer rounded px-2 py-1 hover:bg-accent"
                      onMouseDown={() => { setValue('debit_account', a); setShowDebitSuggestions(false) }}
                    >
                      {a}
                    </li>
                  ))}
                </ul>
              )}
            </div>
            <div className="space-y-2 relative">
              <Label htmlFor="credit_account">{creditLabel}</Label>
              <Input
                id="credit_account"
                placeholder={creditPlaceholder}
                autoComplete="off"
                {...register('credit_account')}
                onFocus={() => setShowCreditSuggestions(true)}
                onBlur={() => setTimeout(() => setShowCreditSuggestions(false), 200)}
              />
              {errors.credit_account && <p className="text-sm text-destructive">{errors.credit_account.message}</p>}
              {showCreditSuggestions && creditAccounts.length > 0 && (
                <ul className="absolute z-10 mt-1 max-h-40 w-full overflow-auto rounded-md border bg-popover p-1 text-sm shadow-md">
                  {creditAccounts.slice(0, 10).map((a) => (
                    <li
                      key={a}
                      className="cursor-pointer rounded px-2 py-1 hover:bg-accent"
                      onMouseDown={() => { setValue('credit_account', a); setShowCreditSuggestions(false) }}
                    >
                      {a}
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </div>

          <Button type="submit" disabled={isSubmitting || !activeWorkspace}>
            {isSubmitting ? 'Posting…' : 'Post Transaction'}
          </Button>
        </CardContent>
      </form>
    </Card>
  )
}
