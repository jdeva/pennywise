import { useMemo, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useWorkspace } from '@/context/workspace-context'
import { recurringApi, type RecurringDefinition } from '@/lib/api/recurring'
import { chartOfAccountsApi } from '@/lib/api/chart-of-accounts'
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
import { CalendarClock, Pencil, Plus, Repeat, Trash2 } from 'lucide-react'
import { cn } from '@/lib/utils'

const PERIOD_OPTIONS = ['Daily', 'Weekly', 'Biweekly', 'Monthly', 'Quarterly', 'Yearly']

interface FormState {
  period: string
  account: string
  counter_account: string
  amount: string
  payee: string
}

const EMPTY_FORM: FormState = {
  period: 'Monthly',
  account: '',
  counter_account: '',
  amount: '',
  payee: '',
}

export function RecurringPage() {
  const { activeWorkspace } = useWorkspace()
  const queryClient = useQueryClient()

  const [sheetOpen, setSheetOpen] = useState(false)
  const [editIndex, setEditIndex] = useState<number | null>(null)
  const [form, setForm] = useState<FormState>(EMPTY_FORM)
  const [error, setError] = useState<string | null>(null)

  const { data: allAccounts = [] } = useQuery({
    queryKey: ['all-accounts-recurring'],
    queryFn: async () => {
      const types = ['expenses', 'assets', 'liabilities', 'income'] as const
      const prefixMap: Record<string, string> = {
        expenses: 'Expenses',
        assets: 'Assets',
        liabilities: 'Liabilities',
        income: 'Income',
      }
      const results = await Promise.all(
        types.map(async (t) => {
          const { data } = await chartOfAccountsApi.list(t)
          return data.map((name) => `${prefixMap[t]}:${name}`)
        }),
      )
      const all = results.flat()
      return all.filter((a) => !all.some((other) => other !== a && other.startsWith(a + ':')))
    },
  })

  const expenseAccounts = useMemo(() => allAccounts.filter((a) => a.startsWith('Expenses:')), [allAccounts])
  const counterAccounts = useMemo(
    () => allAccounts.filter((a) => a.startsWith('Assets:') || a.startsWith('Liabilities:')),
    [allAccounts],
  )

  const { data: items = [], isLoading } = useQuery({
    queryKey: ['recurring', activeWorkspace?.id],
    queryFn: async () => (await recurringApi.list(activeWorkspace!.id)).data,
    enabled: !!activeWorkspace,
  })

  const { data: forecastResponse } = useQuery({
    queryKey: ['recurring-forecast', activeWorkspace?.id],
    queryFn: async () => (await recurringApi.forecast(activeWorkspace!.id)).data,
    enabled: !!activeWorkspace && items.length > 0,
  })

  const upcoming = useMemo(() => {
    if (!forecastResponse?.output) return []
    const today = new Date().toISOString().slice(0, 10)
    // `ledger --forecast register` prints one row per *posting*, stamping the
    // counter-posting with a synthetic `Forecast transaction` payee. Drop those —
    // we only want one row per occurrence, labelled with the real payee.
    return parseRegister(forecastResponse.output)
      .filter((entry) => entry.payee !== 'Forecast transaction')
      .flatMap((entry) => {
        const date = normaliseDate(entry.date)
        // Take the Expenses (or non-Assets) posting if present — that's the "intent" side.
        const primary = entry.postings.find((p) => !p.account.toLowerCase().startsWith('assets') && !p.account.toLowerCase().startsWith('liabilities'))
          || entry.postings[0]
        return primary ? [{ date, payee: entry.payee, account: primary.account, amount: Math.abs(primary.amount), currency: primary.currency }] : []
      })
      .filter((o) => o.date >= today)
      .sort((a, b) => (a.date < b.date ? -1 : 1))
      .slice(0, 30)
  }, [forecastResponse?.output])

  const createMutation = useMutation({
    mutationFn: () =>
      recurringApi.create(activeWorkspace!.id, {
        period: form.period,
        account: form.account,
        counter_account: form.counter_account,
        amount: form.amount,
        currency: activeWorkspace!.currency,
        payee: form.payee.trim() || undefined,
      }),
    onSuccess: () => {
      closeSheet()
      queryClient.invalidateQueries({ queryKey: ['recurring', activeWorkspace?.id] })
      queryClient.invalidateQueries({ queryKey: ['recurring-forecast', activeWorkspace?.id] })
    },
    onError: (err: any) => setError(err?.response?.data?.error || 'Failed to create recurring'),
  })

  const updateMutation = useMutation({
    mutationFn: () =>
      recurringApi.update(activeWorkspace!.id, editIndex!, {
        period: form.period,
        account: form.account,
        counter_account: form.counter_account,
        amount: form.amount,
        currency: activeWorkspace!.currency,
        payee: form.payee.trim() || undefined,
      }),
    onSuccess: () => {
      closeSheet()
      queryClient.invalidateQueries({ queryKey: ['recurring', activeWorkspace?.id] })
      queryClient.invalidateQueries({ queryKey: ['recurring-forecast', activeWorkspace?.id] })
    },
    onError: (err: any) => setError(err?.response?.data?.error || 'Failed to update recurring'),
  })

  const deleteMutation = useMutation({
    mutationFn: (index: number) => recurringApi.delete(activeWorkspace!.id, index),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['recurring', activeWorkspace?.id] })
      queryClient.invalidateQueries({ queryKey: ['recurring-forecast', activeWorkspace?.id] })
    },
  })

  const openSheetForNew = () => {
    setForm(EMPTY_FORM)
    setEditIndex(null)
    setError(null)
    setSheetOpen(true)
  }

  const openSheetForEdit = (index: number) => {
    const item = items[index]
    setForm({
      period: item.period,
      account: item.account,
      counter_account: item.counter_account,
      amount: String(item.amount),
      payee: item.payee ?? '',
    })
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
      <div className="rounded-xl border border-dashed border-border p-10 text-center">
        <p className="text-sm text-muted-foreground">Pick a lair from the sidebar.</p>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="font-display text-3xl font-semibold tracking-tight">Recurring</h1>
        <p className="text-sm text-muted-foreground">
          Declare repeat transactions — rent, subscriptions, salary — and we'll forecast them.
        </p>
      </div>

      <div className="flex items-center justify-between">
        <h2 className="text-sm font-semibold uppercase tracking-wider text-muted-foreground">
          Schedules
        </h2>
        <Button size="sm" onClick={openSheetForNew} className="gap-1.5">
          <Plus className="h-4 w-4" />
          Add recurring
        </Button>
      </div>

      {isLoading ? (
        <div className="space-y-3">
          {Array.from({ length: 3 }).map((_, i) => (
            <div key={i} className="h-20 animate-pulse rounded-xl bg-muted" />
          ))}
        </div>
      ) : items.length === 0 ? (
        <div className="rounded-xl border border-dashed border-border p-10 text-center">
          <Repeat className="mx-auto mb-2 h-6 w-6 text-muted-foreground" />
          <p className="text-sm text-muted-foreground">No schedules yet.</p>
          <p className="mt-1 text-xs text-muted-foreground">
            Add one to have us remind you before it's due.
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
          {items.map((item, i) => (
            <ScheduleCard
              key={i}
              item={item}
              onEdit={() => openSheetForEdit(i)}
              onDelete={() => deleteMutation.mutate(i)}
            />
          ))}
        </div>
      )}

      {upcoming.length > 0 && (
        <section className="space-y-3">
          <h2 className="text-sm font-semibold uppercase tracking-wider text-muted-foreground">
            Upcoming (next 90 days)
          </h2>
          <Card>
            <CardContent className="py-2">
              <ul className="divide-y divide-border/60">
                {upcoming.map((u, i) => (
                  <UpcomingRow key={`${u.date}-${i}`} item={u} />
                ))}
              </ul>
            </CardContent>
          </Card>
        </section>
      )}

      <Sheet open={sheetOpen} onOpenChange={(v) => (v ? null : closeSheet())}>
        <SheetContent side="right">
          <SheetHeader>
            <SheetTitle>{editIndex !== null ? 'Edit recurring' : 'New recurring'}</SheetTitle>
            <SheetDescription>
              Posts automatically in forecasts — you can convert each occurrence into a real
              transaction when it hits.
            </SheetDescription>
          </SheetHeader>
          <SheetBody>
            <form
              className="flex h-full flex-col"
              onSubmit={(e) => {
                e.preventDefault()
                if (!form.period || !form.account.trim() || !form.counter_account.trim() || !form.amount.trim())
                  return
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
                  <Label htmlFor="r-payee">Payee (optional)</Label>
                  <Input
                    id="r-payee"
                    value={form.payee}
                    onChange={(e) => setForm({ ...form, payee: e.target.value })}
                    placeholder="Netflix"
                  />
                </div>

                <div className="space-y-2">
                  <Label htmlFor="r-period">Cadence</Label>
                  <select
                    id="r-period"
                    value={form.period}
                    onChange={(e) => setForm({ ...form, period: e.target.value })}
                    className="flex h-10 w-full rounded-lg border border-border bg-background px-3 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  >
                    {PERIOD_OPTIONS.map((p) => (
                      <option key={p} value={p}>{p}</option>
                    ))}
                  </select>
                </div>

                <AccountAutocomplete
                  id="r-account"
                  label="Category"
                  value={form.account}
                  options={expenseAccounts}
                  placeholder="Expenses:Subscriptions:Netflix"
                  onChange={(v) => setForm({ ...form, account: v })}
                />

                <AccountAutocomplete
                  id="r-counter"
                  label="From account"
                  value={form.counter_account}
                  options={counterAccounts}
                  placeholder="Assets:Bank:Revolut"
                  onChange={(v) => setForm({ ...form, counter_account: v })}
                />

                <div className="space-y-2">
                  <Label htmlFor="r-amount">Amount ({activeWorkspace.currency})</Label>
                  <Input
                    id="r-amount"
                    inputMode="decimal"
                    value={form.amount}
                    onChange={(e) => setForm({ ...form, amount: e.target.value })}
                    placeholder="15.99"
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
                    !form.counter_account.trim() ||
                    !form.amount.trim()
                  }
                >
                  {editIndex !== null ? 'Save changes' : 'Create'}
                </Button>
              </div>
            </form>
          </SheetBody>
        </SheetContent>
      </Sheet>
    </div>
  )
}

function displayAccount(account: string): string {
  const segs = account.split(':')
  return segs.length > 1 ? segs.slice(1).join(':') : account
}

function ScheduleCard({
  item,
  onEdit,
  onDelete,
}: {
  item: RecurringDefinition
  onEdit: () => void
  onDelete: () => void
}) {
  const label = item.payee?.trim() || displayAccount(item.account)
  return (
    <Card>
      <CardContent className="space-y-2 p-4">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0 flex-1">
            <p className="truncate font-medium">{label}</p>
            <p className="truncate text-xs text-muted-foreground">
              {item.period} · {displayAccount(item.account)} ← {displayAccount(item.counter_account)}
            </p>
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
        <div className="flex items-baseline justify-between">
          <span className="text-xs text-muted-foreground">Each occurrence</span>
          <span className="num font-display text-base font-semibold tabular-nums">
            {formatAmount(item.amount, item.currency)}
          </span>
        </div>
      </CardContent>
    </Card>
  )
}

function daysUntil(iso: string): number {
  const today = new Date()
  today.setHours(0, 0, 0, 0)
  return Math.round((new Date(iso).getTime() - today.getTime()) / (1000 * 60 * 60 * 24))
}

function dueLabel(iso: string): string {
  const d = daysUntil(iso)
  if (d === 0) return 'Today'
  if (d === 1) return 'Tomorrow'
  if (d <= 7) return `In ${d} days`
  return iso
}

function UpcomingRow({
  item,
}: {
  item: { date: string; payee: string; account: string; amount: number; currency: string }
}) {
  const soon = daysUntil(item.date) <= 7
  return (
    <li className="flex items-center justify-between gap-3 py-2.5 first:pt-0 last:pb-0">
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium">{item.payee || displayAccount(item.account)}</p>
        <p className="mt-0.5 truncate text-xs text-muted-foreground">{displayAccount(item.account)}</p>
      </div>
      <div className="flex shrink-0 flex-col items-end">
        <span className="num text-sm font-semibold tabular-nums">
          {formatAmount(item.amount, item.currency)}
        </span>
        <span
          className={cn(
            'flex items-center gap-1 text-[11px]',
            soon ? 'text-foreground' : 'text-muted-foreground',
          )}
        >
          <CalendarClock className="h-3 w-3" />
          {dueLabel(item.date)}
        </span>
      </div>
    </li>
  )
}

function AccountAutocomplete({
  id,
  label,
  value,
  options,
  placeholder,
  onChange,
}: {
  id: string
  label: string
  value: string
  options: string[]
  placeholder?: string
  onChange: (v: string) => void
}) {
  const [open, setOpen] = useState(false)
  const filtered = useMemo(
    () => options.filter((o) => o.toLowerCase().includes(value.toLowerCase())).slice(0, 10),
    [options, value],
  )

  return (
    <div className="relative space-y-2">
      <Label htmlFor={id}>{label}</Label>
      <Input
        id={id}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onFocus={() => setOpen(true)}
        onBlur={() => setTimeout(() => setOpen(false), 150)}
        placeholder={placeholder}
        autoComplete="off"
      />
      {open && filtered.length > 0 && (
        <ul className="absolute z-10 mt-1 max-h-48 w-full overflow-auto rounded-md border border-border bg-popover p-1 text-sm shadow-md">
          {filtered.map((a) => (
            <li
              key={a}
              className="cursor-pointer truncate rounded px-2 py-1.5 hover:bg-accent"
              onMouseDown={() => {
                onChange(a)
                setOpen(false)
              }}
            >
              {a}
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}
