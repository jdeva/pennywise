import { useQuery, useQueries } from '@tanstack/react-query'
import { useWorkspace } from '@/context/workspace-context'
import { useAuth } from '@/context/auth-context'
import { useDateRange } from '@/context/date-range-context'
import { transactionsApi } from '@/lib/api/transactions'
import { budgetsApi } from '@/lib/api/budgets'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { parseBalance, parseRegister, formatAmount, normaliseDate, type RegisterEntry } from '@/lib/ledger-parser'
import { ArrowDownLeft, ArrowUpRight, ArrowRight } from 'lucide-react'
import { useMemo } from 'react'
import {
  Bar,
  BarChart,
  ReferenceLine,
  ResponsiveContainer,
  Tooltip,
  XAxis,
} from 'recharts'

interface Totals {
  income: number
  expenses: number
  net: number
}

function totalsFromEntries(entries: RegisterEntry[]): Totals {
  // Sum postings by top-level account to derive income/expenses within range.
  let income = 0
  let expenses = 0
  for (const e of entries) {
    for (const p of e.postings) {
      const top = p.account.split(':')[0].toLowerCase()
      // Income postings in ledger are negative (credit side); expenses positive.
      if (top === 'income') income += Math.abs(p.amount)
      else if (top === 'expenses') expenses += Math.abs(p.amount)
    }
  }
  return { income, expenses, net: income - expenses }
}

function entriesInRange(entries: RegisterEntry[], begin: string, end: string): RegisterEntry[] {
  return entries.filter((e) => {
    const d = normaliseDate(e.date)
    return d >= begin && d <= end
  })
}

function daysInRange(begin: string, end: string): string[] {
  const out: string[] = []
  const b = new Date(begin)
  const e = new Date(end)
  for (let d = new Date(b); d <= e; d.setDate(d.getDate() + 1)) {
    out.push(d.toISOString().slice(0, 10))
  }
  return out
}

function cashflowByDay(entries: RegisterEntry[], days: string[]): Array<{ date: string; net: number; income: number; expenses: number }> {
  const map = new Map<string, { income: number; expenses: number }>()
  for (const d of days) map.set(d, { income: 0, expenses: 0 })
  for (const e of entries) {
    const d = normaliseDate(e.date)
    const slot = map.get(d)
    if (!slot) continue
    for (const p of e.postings) {
      const top = p.account.split(':')[0].toLowerCase()
      if (top === 'income') slot.income += Math.abs(p.amount)
      else if (top === 'expenses') slot.expenses += Math.abs(p.amount)
    }
  }
  return days.map((d) => {
    const s = map.get(d)!
    return { date: d, net: s.income - s.expenses, income: s.income, expenses: s.expenses }
  })
}

function topCategories(
  entries: RegisterEntry[],
  limit = 5,
): Array<{ name: string; amount: number; currency: string }> {
  const totals = new Map<string, { amount: number; currency: string }>()
  for (const e of entries) {
    for (const p of e.postings) {
      const top = p.account.split(':')[0].toLowerCase()
      if (top !== 'expenses') continue
      // Group by first two segments if available (Expenses:Food). Fallback to full.
      const segments = p.account.split(':')
      const key = segments.length >= 2 ? `${segments[0]}:${segments[1]}` : p.account
      const cur = totals.get(key) ?? { amount: 0, currency: p.currency }
      cur.amount += Math.abs(p.amount)
      totals.set(key, cur)
    }
  }
  return Array.from(totals.entries())
    .map(([name, v]) => ({ name, amount: v.amount, currency: v.currency }))
    .sort((a, b) => b.amount - a.amount)
    .slice(0, limit)
}

function displayCategory(full: string): string {
  // "Expenses:Food" → "Food". "Expenses:Food:Coffee" would never happen (we cap at 2 segments).
  const segs = full.split(':')
  return segs.length > 1 ? segs.slice(1).join(':') : full
}

export function DashboardPage() {
  const { activeWorkspace } = useWorkspace()
  const { user } = useAuth()
  const { range } = useDateRange()

  const {
    data: register,
    isLoading: registerLoading,
    error: registerError,
    refetch: refetchRegister,
  } = useQuery({
    queryKey: ['register', activeWorkspace?.id, range.begin, range.end],
    queryFn: async () => {
      const { data } = await transactionsApi.getRegister(activeWorkspace!.id, {
        begin: range.begin,
        end: range.end,
      })
      return data
    },
    enabled: !!activeWorkspace,
  })

  const {
    data: balance,
    isLoading: balanceLoading,
    error: balanceError,
    refetch: refetchBalance,
  } = useQuery({
    queryKey: ['balance', activeWorkspace?.id],
    queryFn: async () => {
      const { data } = await transactionsApi.getBalance(activeWorkspace!.id)
      return data
    },
    enabled: !!activeWorkspace,
  })

  const { data: budgetingState } = useQuery({
    queryKey: ['budgeting', activeWorkspace?.id],
    queryFn: async () => (await budgetsApi.getBudgeting(activeWorkspace!.id)).data,
    enabled: !!activeWorkspace,
  })

  const budgetingEnabled = budgetingState?.budgeting_enabled ?? false

  const isShared = !!activeWorkspace && activeWorkspace.shared_with.length > 0

  const members = useMemo(() => {
    if (!activeWorkspace || !user) return [] as string[]
    const names = new Set<string>()
    if (user.username) names.add(user.username)
    for (const s of activeWorkspace.shared_with) {
      if (s.username) names.add(s.username)
    }
    return Array.from(names)
  }, [activeWorkspace, user])

  const memberRegisters = useQueries({
    queries: members.map((username) => ({
      queryKey: ['register', activeWorkspace?.id, range.begin, range.end, 'user', username],
      queryFn: async () => {
        const { data } = await transactionsApi.getRegister(activeWorkspace!.id, {
          user: username,
          begin: range.begin,
          end: range.end,
        })
        return { username, entries: parseRegister(data.output) }
      },
      enabled: !!activeWorkspace && isShared,
    })),
  })

  const entries = useMemo(
    () => (register?.output ? entriesInRange(parseRegister(register.output), range.begin, range.end) : []),
    [register?.output, range.begin, range.end],
  )
  const totals = useMemo(() => totalsFromEntries(entries), [entries])
  const balanceRows = useMemo(() => (balance?.output ? parseBalance(balance.output) : []), [balance?.output])
  const currency = balanceRows[0]?.currency || activeWorkspace?.currency || ''
  const days = useMemo(() => daysInRange(range.begin, range.end), [range.begin, range.end])
  const cashflow = useMemo(() => cashflowByDay(entries, days), [entries, days])
  const categories = useMemo(() => topCategories(entries), [entries])

  const perUser = useMemo(() => {
    return memberRegisters
      .filter((q) => q.data)
      .map((q) => ({ username: q.data!.username, expenses: totalsFromEntries(q.data!.entries).expenses }))
      .sort((a, b) => b.expenses - a.expenses)
  }, [memberRegisters])

  const myExpenses = useMemo(() => {
    return perUser.find((p) => p.username === user?.username)?.expenses ?? 0
  }, [perUser, user])

  const recentEntries = entries.slice(-6).reverse()

  if (!activeWorkspace) {
    return (
      <div className="flex min-h-[50vh] flex-col items-center justify-center gap-3 rounded-xl border border-dashed border-border py-12 text-center">
        <h2 className="font-display text-xl font-semibold">No lair selected</h2>
        <p className="max-w-sm text-sm text-muted-foreground">
          Pick or create a lair from the sidebar to start tracking.
        </p>
      </div>
    )
  }

  const hasError = balanceError || registerError
  const hasCashflow = cashflow.some((c) => c.income > 0 || c.expenses > 0)
  const topCategoryAmount = categories[0]?.amount ?? 0

  return (
    <div className="space-y-6">
      <section className="space-y-1">
        {isShared ? (
          <>
            <p className="text-xs font-medium uppercase tracking-wider text-muted-foreground">Your spending</p>
            {registerLoading ? (
              <div className="h-14 w-64 animate-pulse rounded-lg bg-muted" />
            ) : (
              <h1 className="num font-display text-4xl font-semibold tracking-tight md:text-5xl">
                {formatAmount(myExpenses, currency)}
              </h1>
            )}
            <p className="pt-2 text-sm text-muted-foreground">
              Lair total:{' '}
              <span className="num font-medium text-foreground">{formatAmount(totals.expenses, currency)}</span> spent ·{' '}
              <span className="num font-medium text-foreground">{formatAmount(totals.income, currency)}</span> in
            </p>
          </>
        ) : (
          <>
            <p className="text-xs font-medium uppercase tracking-wider text-muted-foreground">Net this period</p>
            {registerLoading ? (
              <div className="h-14 w-64 animate-pulse rounded-lg bg-muted" />
            ) : (
              <h1
                className={`num font-display text-4xl font-semibold tracking-tight md:text-5xl ${
                  totals.net < 0 ? 'text-destructive' : 'text-foreground'
                }`}
              >
                {formatAmount(totals.net, currency)}
              </h1>
            )}
            <div className="flex flex-wrap items-center gap-x-6 gap-y-2 pt-2 text-sm">
              <span className="flex items-center gap-1.5 text-muted-foreground">
                <ArrowDownLeft className="h-3.5 w-3.5 text-accent-mint" />
                Income
                <span className="num font-medium text-foreground">{formatAmount(totals.income, currency)}</span>
              </span>
              <span className="flex items-center gap-1.5 text-muted-foreground">
                <ArrowUpRight className="h-3.5 w-3.5 text-accent-rose" />
                Expenses
                <span className="num font-medium text-foreground">{formatAmount(totals.expenses, currency)}</span>
              </span>
            </div>
          </>
        )}
      </section>

      {hasError && (
        <div className="flex items-center justify-between rounded-lg border border-destructive/40 bg-destructive/5 px-4 py-3">
          <p className="text-sm text-destructive">Couldn't load lair data.</p>
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              refetchBalance()
              refetchRegister()
            }}
          >
            Try again
          </Button>
        </div>
      )}

      {/* Cashflow sparkline — full-width card */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-semibold uppercase tracking-wider text-muted-foreground">
            Cashflow
          </CardTitle>
        </CardHeader>
        <CardContent>
          {registerLoading ? (
            <div className="h-28 w-full animate-pulse rounded-lg bg-muted" />
          ) : !hasCashflow ? (
            <EmptyHint>Nothing to chart in this range.</EmptyHint>
          ) : (
            <CashflowBars data={cashflow} currency={currency} />
          )}
        </CardContent>
      </Card>

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-5">
        {/* Top categories */}
        <Card className="lg:col-span-2">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-semibold uppercase tracking-wider text-muted-foreground">
              Top categories
            </CardTitle>
          </CardHeader>
          <CardContent>
            {registerLoading ? (
              <SkeletonList rows={5} />
            ) : categories.length === 0 ? (
              <EmptyHint>No spending in this range.</EmptyHint>
            ) : (
              <ul className="space-y-2.5">
                {categories.map((c) => {
                  const pct = topCategoryAmount > 0 ? (c.amount / topCategoryAmount) * 100 : 0
                  return (
                    <li key={c.name} className="space-y-1">
                      <div className="flex items-baseline justify-between gap-2">
                        <span className="truncate text-sm font-medium">{displayCategory(c.name)}</span>
                        <span className="num shrink-0 text-sm tabular-nums text-foreground">
                          {formatAmount(c.amount, c.currency)}
                        </span>
                      </div>
                      <div className="h-1.5 overflow-hidden rounded-full bg-muted">
                        <div
                          className="h-full rounded-full bg-primary"
                          style={{ width: `${Math.max(4, pct)}%` }}
                        />
                      </div>
                    </li>
                  )
                })}
              </ul>
            )}
          </CardContent>
        </Card>

        {/* Recent transactions */}
        <Card className="lg:col-span-3">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-semibold uppercase tracking-wider text-muted-foreground">
              Recent activity
            </CardTitle>
          </CardHeader>
          <CardContent>
            {registerLoading ? (
              <SkeletonList rows={6} />
            ) : recentEntries.length === 0 ? (
              <EmptyHint>Nothing posted in this range.</EmptyHint>
            ) : (
              <ul className="divide-y divide-border/60">
                {recentEntries.map((e, i) => {
                  const debit = e.postings.find((p) => p.amount > 0) || e.postings[0]
                  const credit = e.postings.find((p) => p.account !== debit?.account)
                  const primary = debit
                  const isIncome = primary?.account.toLowerCase().startsWith('income')
                  return (
                    <li key={i} className="flex items-start justify-between gap-4 py-2.5 first:pt-0 last:pb-0">
                      <div className="min-w-0 flex-1">
                        <p className="truncate text-sm font-medium">{e.payee || '—'}</p>
                        <div className="mt-0.5 flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground">
                          <time className="num shrink-0">{normaliseDate(e.date)}</time>
                          <span>·</span>
                          {debit && (
                            <span className="truncate">{isIncome ? credit?.account : debit.account}</span>
                          )}
                          {credit && debit && debit.account !== credit.account && (
                            <>
                              <ArrowRight className="h-3 w-3 shrink-0" />
                              <span className="truncate">{isIncome ? debit.account : credit.account}</span>
                            </>
                          )}
                        </div>
                      </div>
                      {primary && (
                        <span
                          className={`num shrink-0 text-sm font-semibold tabular-nums ${
                            isIncome ? 'text-accent-mint' : 'text-foreground'
                          }`}
                        >
                          {formatAmount(primary.amount, primary.currency)}
                        </span>
                      )}
                    </li>
                  )
                })}
              </ul>
            )}
          </CardContent>
        </Card>
      </div>

      {budgetingEnabled && <BudgetProgressCard workspaceId={activeWorkspace.id} range={range} />}

      {isShared && perUser.length > 0 && (
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-semibold uppercase tracking-wider text-muted-foreground">
              Roommate split
            </CardTitle>
          </CardHeader>
          <CardContent>
            <ul className="space-y-1.5">
              {perUser.map((p) => (
                <li key={p.username} className="flex items-center justify-between text-sm">
                  <span>
                    {p.username}
                    {p.username === user?.username && (
                      <span className="ml-1.5 text-xs text-muted-foreground">(you)</span>
                    )}
                  </span>
                  <span className="num font-medium">{formatAmount(p.expenses, currency)}</span>
                </li>
              ))}
            </ul>
          </CardContent>
        </Card>
      )}
    </div>
  )
}

function CashflowBars({
  data,
  currency,
}: {
  data: Array<{ date: string; net: number; income: number; expenses: number }>
  currency: string
}) {
  // Same treatment as /reports/cashflow — income above, expenses below the axis.
  const charted = data.map((d) => ({ ...d, expensesNeg: -d.expenses }))
  return (
    <div className="h-32 w-full">
      <ResponsiveContainer width="100%" height="100%">
        <BarChart data={charted} margin={{ top: 8, right: 4, bottom: 0, left: 4 }}>
          <XAxis dataKey="date" hide />
          <ReferenceLine y={0} stroke="hsl(var(--border))" />
          <Tooltip
            cursor={{ fill: 'hsl(var(--accent) / 0.35)' }}
            contentStyle={{
              backgroundColor: 'hsl(var(--popover))',
              border: '1px solid hsl(var(--border))',
              borderRadius: 8,
              fontSize: 12,
              padding: '8px 10px',
            }}
            formatter={(value, name, item) => {
              const key = (item as { dataKey?: string }).dataKey
              if (key === 'expensesNeg') {
                return [formatAmount(Math.abs(Number(value) || 0), currency), 'Expenses']
              }
              return [formatAmount(Number(value) || 0, currency), String(name)]
            }}
            labelFormatter={(l) => String(l ?? '')}
          />
          <Bar dataKey="income" name="Income" fill="hsl(var(--accent-mint))" radius={[3, 3, 0, 0]} maxBarSize={12} />
          <Bar dataKey="expensesNeg" name="Expenses" fill="hsl(var(--primary))" radius={[0, 0, 3, 3]} maxBarSize={12} />
        </BarChart>
      </ResponsiveContainer>
    </div>
  )
}

function BudgetProgressCard({
  workspaceId,
  range,
}: {
  workspaceId: string
  range: { begin: string; end: string }
}) {
  const { data: budgets = [] } = useQuery({
    queryKey: ['budgets', workspaceId],
    queryFn: async () => (await budgetsApi.list(workspaceId)).data,
  })
  const { data: report } = useQuery({
    queryKey: ['budgets', workspaceId, 'report', range.begin, range.end],
    queryFn: async () => (await budgetsApi.report(workspaceId, range.begin, range.end)).data,
    enabled: budgets.length > 0,
  })

  if (budgets.length === 0) return null

  // Very thin parser — the budget report output is a fixed-column ledger format.
  // For now we render the raw report in a mono block; Step 6 will upgrade this.
  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="text-sm font-semibold uppercase tracking-wider text-muted-foreground">
          Budget progress
        </CardTitle>
      </CardHeader>
      <CardContent>
        {report?.output ? (
          <pre className="max-h-60 overflow-auto rounded-md bg-muted/60 p-3 text-xs leading-relaxed">
            {report.output}
          </pre>
        ) : (
          <EmptyHint>Loading budget progress…</EmptyHint>
        )}
      </CardContent>
    </Card>
  )
}

function SkeletonList({ rows }: { rows: number }) {
  return (
    <div className="space-y-2">
      {Array.from({ length: rows }).map((_, i) => (
        <div key={i} className="h-8 animate-pulse rounded-md bg-muted" />
      ))}
    </div>
  )
}

function EmptyHint({ children }: { children: React.ReactNode }) {
  return (
    <div className="rounded-lg border border-dashed border-border px-4 py-6 text-center text-sm text-muted-foreground">
      {children}
    </div>
  )
}
