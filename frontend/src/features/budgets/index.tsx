import { useState, useMemo } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { budgetsApi } from '@/lib/api/budgets'
import { chartOfAccountsApi } from '@/lib/api/chart-of-accounts'
import { useWorkspace } from '@/context/workspace-context'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Pill } from '@/components/ui/pill'
import { Trash2, Pencil } from 'lucide-react'
import { BudgetReports } from './budget-reports'

const PERIOD_OPTIONS = ['Monthly', 'Weekly', 'Biweekly', 'Quarterly', 'Yearly', 'Daily']

export function BudgetsPage() {
  const { activeWorkspace } = useWorkspace()
  const queryClient = useQueryClient()
  const [showForm, setShowForm] = useState(false)
  const [editIndex, setEditIndex] = useState<number | null>(null)
  const [form, setForm] = useState({ period: '', account: '', amount: '' })
  const [error, setError] = useState<string | null>(null)
  const [accountFilter, setAccountFilter] = useState('')
  const [showAccountSuggestions, setShowAccountSuggestions] = useState(false)

  // Fetch expense accounts for budget account dropdown
  const { data: expenseAccounts = [] } = useQuery({
    queryKey: ['all-accounts-budget'],
    queryFn: async () => {
      const { data } = await chartOfAccountsApi.list('expenses')
      const all = data.map((name) => `Expenses:${name}`)
      return all.filter((a) => !all.some((other) => other !== a && other.startsWith(a + ':')))
    },
  })

  const filteredAccounts = useMemo(() =>
    expenseAccounts.filter((a) => a.toLowerCase().includes(accountFilter.toLowerCase())),
    [expenseAccounts, accountFilter],
  )

  const { data: budgetingStatus } = useQuery({
    queryKey: ['budgeting', activeWorkspace?.id],
    queryFn: async () => {
      const { data } = await budgetsApi.getBudgeting(activeWorkspace!.id)
      return data
    },
    enabled: !!activeWorkspace,
  })

  const { data: budgets = [], isLoading } = useQuery({
    queryKey: ['budgets', activeWorkspace?.id],
    queryFn: async () => {
      const { data } = await budgetsApi.list(activeWorkspace!.id)
      return data
    },
    enabled: !!activeWorkspace && budgetingStatus?.budgeting_enabled === true,
  })

  const toggleMutation = useMutation({
    mutationFn: (enabled: boolean) => budgetsApi.setBudgeting(activeWorkspace!.id, enabled),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['budgeting', activeWorkspace?.id] })
      queryClient.invalidateQueries({ queryKey: ['budgets', activeWorkspace?.id] })
    },
  })

  const createMutation = useMutation({
    mutationFn: () => budgetsApi.create(activeWorkspace!.id, {
      period: form.period, account: form.account, amount: form.amount,
      currency: activeWorkspace!.currency,
    }),
    onSuccess: () => {
      resetForm()
      queryClient.invalidateQueries({ queryKey: ['budgets', activeWorkspace?.id] })
    },
    onError: (err: any) => setError(err?.response?.data?.error || 'Failed to create budget'),
  })

  const updateMutation = useMutation({
    mutationFn: () => budgetsApi.update(activeWorkspace!.id, editIndex!, {
      period: form.period, account: form.account, amount: form.amount,
      currency: activeWorkspace!.currency,
    }),
    onSuccess: () => {
      resetForm()
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

  const resetForm = () => {
    setForm({ period: '', account: '', amount: '' })
    setShowForm(false)
    setEditIndex(null)
    setError(null)
  }

  const startEdit = (index: number) => {
    const b = budgets[index]
    setForm({ period: b.period, account: b.account, amount: String(b.amount) })
    setEditIndex(index)
    setShowForm(true)
  }

  if (!activeWorkspace) {
    return (
      <div className="flex min-h-[60vh] flex-col items-center justify-center gap-4 rounded-2xl bg-warm-gradient p-12 text-center">
        <h2 className="font-display text-2xl font-semibold">No workspace selected</h2>
      </div>
    )
  }

  const enabled = budgetingStatus?.budgeting_enabled ?? false

  return (
    <div className="space-y-8">
      <div>
        <p className="text-sm font-medium uppercase tracking-wider text-muted-foreground">Plan</p>
        <h1 className="mt-1 font-display text-4xl font-semibold tracking-tight">Budgets</h1>
      </div>

      <Card className={enabled ? '' : 'bg-gradient-to-br from-accent-butter/10 to-transparent'}>
        <CardContent className="flex items-center justify-between p-5">
          <div>
            <p className="text-sm font-medium">
              Budgeting is <span className={enabled ? 'text-accent-mint' : 'text-muted-foreground'}>{enabled ? 'on' : 'off'}</span>
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
          <Card>
            <CardHeader className="flex flex-row items-center justify-between">
              <CardTitle>Budget Definitions</CardTitle>
              <Button size="sm" onClick={() => { resetForm(); setShowForm(true) }}>
                Add Budget
              </Button>
            </CardHeader>
            <CardContent>
              {showForm && (
                <div className="mb-4 space-y-3 rounded-md border p-4">
                  {error && (
                    <div role="alert" className="rounded-md bg-destructive/10 p-2 text-sm text-destructive">{error}</div>
                  )}
                  <div className="grid grid-cols-3 gap-3">
                    <div className="space-y-1">
                      <Label>Period</Label>
                      <select
                        value={form.period}
                        onChange={(e) => setForm({ ...form, period: e.target.value })}
                        className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
                      >
                        <option value="">Select period…</option>
                        {PERIOD_OPTIONS.map((p) => (
                          <option key={p} value={p}>{p}</option>
                        ))}
                      </select>
                    </div>
                    <div className="space-y-1 relative">
                      <Label>Account</Label>
                      <Input
                        value={form.account}
                        onChange={(e) => { setForm({ ...form, account: e.target.value }); setAccountFilter(e.target.value) }}
                        onFocus={() => { setAccountFilter(form.account); setShowAccountSuggestions(true) }}
                        onBlur={() => setTimeout(() => setShowAccountSuggestions(false), 200)}
                        placeholder="Search accounts…"
                        autoComplete="off"
                      />
                      {showAccountSuggestions && filteredAccounts.length > 0 && (
                        <ul className="absolute z-10 mt-1 max-h-40 w-full overflow-auto rounded-md border bg-popover p-1 text-sm shadow-md">
                          {filteredAccounts.slice(0, 10).map((a) => (
                            <li
                              key={a}
                              className="cursor-pointer rounded px-2 py-1 hover:bg-accent"
                              onMouseDown={() => { setForm({ ...form, account: a }); setShowAccountSuggestions(false) }}
                            >
                              {a}
                            </li>
                          ))}
                        </ul>
                      )}
                    </div>
                    <div className="space-y-1">
                      <Label>Amount</Label>
                      <Input value={form.amount} onChange={(e) => setForm({ ...form, amount: e.target.value })} placeholder="500.00" />
                    </div>
                  </div>
                  <div className="flex gap-2">
                    <Button
                      size="sm"
                      onClick={() => editIndex !== null ? updateMutation.mutate() : createMutation.mutate()}
                      disabled={createMutation.isPending || updateMutation.isPending}
                    >
                      {editIndex !== null ? 'Update' : 'Create'}
                    </Button>
                    <Button size="sm" variant="outline" onClick={resetForm}>Cancel</Button>
                  </div>
                </div>
              )}
              {isLoading ? (
                <div className="h-20 animate-pulse rounded bg-muted" />
              ) : budgets.length === 0 ? (
                <p className="text-sm text-muted-foreground">No budget definitions.</p>
              ) : (
                <ul className="divide-y divide-border/60">
                  {budgets.map((b, i) => (
                    <li key={i} className="flex items-center justify-between gap-3 py-3">
                      <div className="flex items-center gap-3">
                        <Pill color="butter">{b.period}</Pill>
                        <Pill account={b.account}>{b.account}</Pill>
                      </div>
                      <div className="flex items-center gap-1">
                        <span className="num mr-3 font-display text-base font-semibold">
                          {activeWorkspace?.currency}{b.amount}
                        </span>
                        <Button variant="ghost" size="icon" className="h-8 w-8" onClick={() => startEdit(i)} aria-label="Edit">
                          <Pencil className="h-3.5 w-3.5" />
                        </Button>
                        <Button variant="ghost" size="icon" className="h-8 w-8" onClick={() => deleteMutation.mutate(i)} aria-label="Delete">
                          <Trash2 className="h-3.5 w-3.5" />
                        </Button>
                      </div>
                    </li>
                  ))}
                </ul>
              )}
            </CardContent>
          </Card>

          <BudgetReports />
        </>
      )}
    </div>
  )
}
