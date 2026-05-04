import { useMemo, useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { budgetsApi } from '@/lib/api/budgets'
import { transactionsApi } from '@/lib/api/transactions'
import { chartOfAccountsApi } from '@/lib/api/chart-of-accounts'
import { useWorkspace } from '@/context/workspace-context'
import { useDateRange } from '@/context/date-range-context'
import { parseRegister, normaliseDate, formatAmount } from '@/lib/ledger-parser'
import { Card, CardContent } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetBody,
  SheetTitle,
  SheetDescription,
} from '@/components/ui/sheet'
import { Plus, Pencil, Trash2, PiggyBank } from 'lucide-react'
import type { BudgetDefinition } from '@/lib/types'
import { cn } from '@/lib/utils'

const PERIOD_OPTIONS = ['Monthly', 'Weekly', 'Biweekly', 'Quarterly', 'Yearly', 'Daily']

export function BudgetsPage() {
  const { activeWorkspace } = useWorkspace()
  const { range } = useDateRange()
  const queryClient = useQueryClient()

  const [sheetOpen, setSheetOpen] = useState(false)
  const [editIndex, setEditIndex] = useState<number | null>(null)
  const [form, setForm] = useState({ period: 'Monthly', account: '', amount: '' })
  const [error, setError] = useState<string | null>(null)
  const [accountFilter, setAccountFilter] = useState('')
  const [showAccountSuggestions, setShowAccountSuggestions] = useState(false)

  const { data: expenseAccounts = [] } = useQuery({
    queryKey: ['all-accounts-budget'],
    queryFn: async () => {
      const { data } = await chartOfAccountsApi.list('expenses')
      const all = data.map((name) => `Expenses:${name}`)
      return all.filter((a) => !all.some((other) => other !== a && other.startsWith(a + ':')))
    },
  })

  const filteredAccounts = useMemo(
    () => expenseAccounts.filter((a) => a.toLowerCase().includes(accountFilter.toLowerCase())),
    [expenseAccounts, accountFilter],
  )

  const { data: budgetingStatus } = useQuery({
    queryKey: ['budgeting', activeWorkspace?.id],
    queryFn: async () => (await budgetsApi.getBudgeting(activeWorkspace!.id)).data,
    enabled: !!activeWorkspace,
  })

  const enabled = budgetingStatus?.budgeting_enabled ?? false

  const { data: budgets = [], isLoading } = useQuery({
    queryKey: ['budgets', activeWorkspace?.id],
    queryFn: async () => (await budgetsApi.list(activeWorkspace!.id)).data,
    enabled: !!activeWorkspace && enabled,
  })

  // Register for the active range — used to compute actuals per budget.
  const { data: register } = useQuery({
    queryKey: ['register', activeWorkspace?.id, range.begin, range.end],
    queryFn: async () => {
      const { data } = await transactionsApi.getRegister(activeWorkspace!.id, {
        begin: range.begin,
        end: range.end,
      })
      return data
    },
    enabled: !!activeWorkspace && enabled,
  })

  const entries = useMemo(() => {
    if (!register?.output) return []
    return parseRegister(register.output).filter((e) => {
      const d = normaliseDate(e.date)
      return d >= range.begin && d <= range.end
    })
  }, [register?.output, range.begin, range.end])

  /**
   * Spend against a budget account includes both exact matches and any
   * descendants (e.g. budget on `Expenses:Food` counts `Expenses:Food:Coffee`).
   */
  const spendFor = (account: string): number => {
    const prefix = account + ':'
    let total = 0
    for (const e of entries) {
      for (const p of e.postings) {
        if (p.account === account || p.account.startsWith(prefix)) {
          total += Math.abs(p.amount)
        }
      }
    }
    return total
  }

  const toggleMutation = useMutation({
    mutationFn: (next: boolean) => budgetsApi.setBudgeting(activeWorkspace!.id, next),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['budgeting', activeWorkspace?.id] })
      queryClient.invalidateQueries({ queryKey: ['budgets', activeWorkspace?.id] })
    },
  })

  const createMutation = useMutation({
    mutationFn: () =>
      budgetsApi.create(activeWorkspace!.id, {
        period: form.period,
        account: form.account,
        amount: form.amount,
        currency: activeWorkspace!.currency,
      }),
    onSuccess: () => {
      closeSheet()
      queryClient.invalidateQueries({ queryKey: ['budgets', activeWorkspace?.id] })
    },
    onError: (err: any) => setError(err?.response?.data?.error || 'Failed to create budget'),
  })

  const updateMutation = useMutation({
    mutationFn: () =>
      budgetsApi.update(activeWorkspace!.id, editIndex!, {
        period: form.period,
        account: form.account,
        amount: form.amount,
        currency: activeWorkspace!.currency,
      }),
    onSuccess: () => {
      closeSheet()
      queryClient.invalidateQueries({ queryKey: ['budgets', activeWorkspace?.id] })
    },
    onError: (err: any) => setError(err?.response?.data?.error || 'Failed to update budget'),
  })

  const deleteMutation = useMutation({
    mutationFn: (index: number) => budgetsApi.delete(activeWorkspace!.id, index),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['budgets', activeWorkspace?.id] })
    },
  })

  const openSheetForNew = () => {
    setForm({ period: 'Monthly', account: '', amount: '' })
    setEditIndex(null)
    setError(null)
    setSheetOpen(true)
  }

  const openSheetForEdit = (index: number) => {
    const b = budgets[index]
    setForm({ period: b.period, account: b.account, amount: String(b.amount) })
    setEditIndex(index)
    setError(null)
    setSheetOpen(true)
  }

  const closeSheet = () => {
    setSheetOpen(false)
    setEditIndex(null)
    setError(null)
  }

  if (!activeWorkspace) {
    return (
      <div className="flex min-h-[50vh] flex-col items-center justify-center gap-3 rounded-xl border border-dashed border-border py-12 text-center">
        <h2 className="font-display text-xl font-semibold">No lair selected</h2>
        <p className="max-w-sm text-sm text-muted-foreground">Pick a lair from the sidebar.</p>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="font-display text-3xl font-semibold tracking-tight">Budgets</h1>
        <p className="text-sm text-muted-foreground">Keep each category on its leash.</p>
      </div>

      <Card>
        <CardContent className="flex items-center justify-between p-5">
          <div>
            <p className="text-sm font-medium">
              Budgeting is{' '}
              <span className={enabled ? 'text-accent-mint' : 'text-muted-foreground'}>
                {enabled ? 'on' : 'off'}
              </span>
            </p>
            <p className="mt-1 text-xs text-muted-foreground">
              {enabled
                ? 'Set targets per category and track progress against them.'
                : 'Enable to set spending targets for your expense categories.'}
            </p>
          </div>
          <Button
            size="sm"
            variant={enabled ? 'outline' : 'default'}
            onClick={() => toggleMutation.mutate(!enabled)}
            disabled={toggleMutation.isPending}
          >
            {enabled ? 'Turn off' : 'Turn on'}
          </Button>
        </CardContent>
      </Card>

      {enabled && (
        <>
          <div className="flex items-center justify-between">
            <h2 className="text-sm font-semibold uppercase tracking-wider text-muted-foreground">
              Targets
            </h2>
            <Button size="sm" onClick={openSheetForNew} className="gap-1.5">
              <Plus className="h-4 w-4" />
              Add budget
            </Button>
          </div>

          {isLoading ? (
            <div className="space-y-3">
              {Array.from({ length: 3 }).map((_, i) => (
                <div key={i} className="h-20 animate-pulse rounded-xl bg-muted" />
              ))}
            </div>
          ) : budgets.length === 0 ? (
            <div className="rounded-xl border border-dashed border-border p-10 text-center">
              <PiggyBank className="mx-auto mb-2 h-6 w-6 text-muted-foreground" />
              <p className="text-sm text-muted-foreground">No targets yet.</p>
              <p className="mt-1 text-xs text-muted-foreground">
                Add one to track spending against it.
              </p>
            </div>
          ) : (
            <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
              {budgets.map((b, i) => (
                <BudgetCard
                  key={i}
                  budget={b}
                  spent={spendFor(b.account)}
                  onEdit={() => openSheetForEdit(i)}
                  onDelete={() => deleteMutation.mutate(i)}
                  currency={activeWorkspace.currency}
                />
              ))}
            </div>
          )}
        </>
      )}

      <Sheet open={sheetOpen} onOpenChange={(v) => (v ? null : closeSheet())}>
        <SheetContent side="right">
          <SheetHeader>
            <SheetTitle>{editIndex !== null ? 'Edit budget' : 'New budget'}</SheetTitle>
            <SheetDescription>
              Target monthly (or custom) spend for an expense category.
            </SheetDescription>
          </SheetHeader>
          <SheetBody>
            <form
              className="flex h-full flex-col"
              onSubmit={(e) => {
                e.preventDefault()
                if (!form.period || !form.account.trim() || !form.amount.trim()) return
                editIndex !== null ? updateMutation.mutate() : createMutation.mutate()
              }}
            >
              <div className="flex-1 space-y-4 overflow-y-auto">
                {error && (
                  <div role="alert" className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">
                    {error}
                  </div>
                )}

                <div className="space-y-2">
                  <Label htmlFor="b-period">Period</Label>
                  <select
                    id="b-period"
                    value={form.period}
                    onChange={(e) => setForm({ ...form, period: e.target.value })}
                    className="flex h-10 w-full rounded-lg border border-border bg-background px-3 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  >
                    {PERIOD_OPTIONS.map((p) => (
                      <option key={p} value={p}>{p}</option>
                    ))}
                  </select>
                </div>

                <div className="relative space-y-2">
                  <Label htmlFor="b-account">Category</Label>
                  <Input
                    id="b-account"
                    value={form.account}
                    onChange={(e) => {
                      setForm({ ...form, account: e.target.value })
                      setAccountFilter(e.target.value)
                    }}
                    onFocus={() => {
                      setAccountFilter(form.account)
                      setShowAccountSuggestions(true)
                    }}
                    onBlur={() => setTimeout(() => setShowAccountSuggestions(false), 200)}
                    placeholder="Expenses:Food"
                    autoComplete="off"
                  />
                  {showAccountSuggestions && filteredAccounts.length > 0 && (
                    <ul className="absolute z-10 mt-1 max-h-48 w-full overflow-auto rounded-md border border-border bg-popover p-1 text-sm shadow-md">
                      {filteredAccounts.slice(0, 10).map((a) => (
                        <li
                          key={a}
                          className="cursor-pointer truncate rounded px-2 py-1.5 hover:bg-accent"
                          onMouseDown={() => {
                            setForm({ ...form, account: a })
                            setShowAccountSuggestions(false)
                          }}
                        >
                          {a}
                        </li>
                      ))}
                    </ul>
                  )}
                </div>

                <div className="space-y-2">
                  <Label htmlFor="b-amount">Amount ({activeWorkspace.currency})</Label>
                  <Input
                    id="b-amount"
                    inputMode="decimal"
                    value={form.amount}
                    onChange={(e) => setForm({ ...form, amount: e.target.value })}
                    placeholder="500.00"
                  />
                </div>
              </div>
              <div className="mt-5 flex items-center justify-end gap-2 border-t border-border pt-4">
                <Button type="button" variant="outline" onClick={closeSheet}>
                  Cancel
                </Button>
                <Button
                  type="submit"
                  disabled={
                    createMutation.isPending ||
                    updateMutation.isPending ||
                    !form.period ||
                    !form.account.trim() ||
                    !form.amount.trim()
                  }
                >
                  {editIndex !== null ? 'Save changes' : 'Create budget'}
                </Button>
              </div>
            </form>
          </SheetBody>
        </SheetContent>
      </Sheet>
    </div>
  )
}

function displayCategory(account: string): string {
  return account.startsWith('Expenses:') ? account.slice('Expenses:'.length) : account
}

function BudgetCard({
  budget,
  spent,
  currency,
  onEdit,
  onDelete,
}: {
  budget: BudgetDefinition
  spent: number
  currency: string
  onEdit: () => void
  onDelete: () => void
}) {
  const target = Number(budget.amount) || 0
  const pct = target > 0 ? Math.min(200, (spent / target) * 100) : 0
  const over = spent > target
  const remaining = target - spent
  // Tone: under-budget = primary bar (lair accent); ≥90% = amber warning; over = destructive red.
  let barClass = 'bg-primary'
  if (over) barClass = 'bg-destructive'
  else if (pct >= 90) barClass = 'bg-accent-butter'

  return (
    <Card>
      <CardContent className="space-y-3 p-4">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0 flex-1">
            <p className="truncate font-medium">{displayCategory(budget.account)}</p>
            <p className="text-xs text-muted-foreground">{budget.period}</p>
          </div>
          <div className="flex items-center gap-1">
            <button
              type="button"
              onClick={onEdit}
              aria-label="Edit"
              className="flex h-8 w-8 items-center justify-center rounded-md text-muted-foreground hover:bg-accent hover:text-foreground"
            >
              <Pencil className="h-3.5 w-3.5" />
            </button>
            <button
              type="button"
              onClick={onDelete}
              aria-label="Delete"
              className="flex h-8 w-8 items-center justify-center rounded-md text-muted-foreground hover:bg-accent hover:text-destructive"
            >
              <Trash2 className="h-3.5 w-3.5" />
            </button>
          </div>
        </div>

        <div className="h-2 overflow-hidden rounded-full bg-muted">
          <div
            className={cn('h-full rounded-full transition-all', barClass)}
            style={{ width: `${Math.max(2, Math.min(100, pct))}%` }}
          />
        </div>

        <div className="flex items-baseline justify-between text-sm">
          <span className="num tabular-nums">
            <span className="font-semibold">{formatAmount(spent, currency)}</span>{' '}
            <span className="text-muted-foreground">of {formatAmount(target, currency)}</span>
          </span>
          <span
            className={cn(
              'num shrink-0 text-xs font-medium tabular-nums',
              over ? 'text-destructive' : 'text-muted-foreground',
            )}
          >
            {over
              ? `${formatAmount(Math.abs(remaining), currency)} over`
              : `${formatAmount(remaining, currency)} left`}
          </span>
        </div>
      </CardContent>
    </Card>
  )
}
