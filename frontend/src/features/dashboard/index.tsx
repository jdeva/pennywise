import { useQuery, useQueries } from '@tanstack/react-query'
import { useWorkspace } from '@/context/workspace-context'
import { useAuth } from '@/context/auth-context'
import { transactionsApi } from '@/lib/api/transactions'
import { Button } from '@/components/ui/button'
import { Pill } from '@/components/ui/pill'
import { parseBalance, parseRegister, formatAmount, normaliseDate } from '@/lib/ledger-parser'
import { ArrowDownLeft, ArrowUpRight, ArrowRight, ChevronDown, ChevronRight } from 'lucide-react'
import { useMemo, useState } from 'react'

function totalsFrom(rows: ReturnType<typeof parseBalance>) {
  let income = 0
  let expenses = 0
  for (const r of rows) {
    if (r.depth > 0) continue
    const key = r.account.toLowerCase()
    if (key.startsWith('income')) income += Math.abs(r.amount)
    if (key.startsWith('expenses')) expenses += Math.abs(r.amount)
  }
  return { income, expenses, net: income - expenses }
}

export function DashboardPage() {
  const { activeWorkspace } = useWorkspace()
  const { user } = useAuth()

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

  const isShared = !!activeWorkspace && activeWorkspace.shared_with.length > 0

  // Collect every member (owner + shared) by username so we can fetch each one's balance.
  const members = useMemo(() => {
    if (!activeWorkspace || !user) return [] as string[]
    const names = new Set<string>()
    // Current user is always a member (owner or shared)
    if (user.username) names.add(user.username)
    for (const s of activeWorkspace.shared_with) {
      if (s.username) names.add(s.username)
    }
    return Array.from(names)
  }, [activeWorkspace, user])

  const memberBalances = useQueries({
    queries: members.map((username) => ({
      queryKey: ['balance', activeWorkspace?.id, 'user', username],
      queryFn: async () => {
        const { data } = await transactionsApi.getBalance(activeWorkspace!.id, { user: username })
        return { username, rows: parseBalance(data.output) }
      },
      enabled: !!activeWorkspace && isShared,
    })),
  })

  const {
    data: register,
    isLoading: registerLoading,
    error: registerError,
    refetch: refetchRegister,
  } = useQuery({
    queryKey: ['register', activeWorkspace?.id],
    queryFn: async () => {
      const { data } = await transactionsApi.getRegister(activeWorkspace!.id)
      return data
    },
    enabled: !!activeWorkspace,
  })

  const rows = useMemo(() => (balance?.output ? parseBalance(balance.output) : []), [balance?.output])
  const entries = useMemo(() => (register?.output ? parseRegister(register.output) : []), [register?.output])
  const totals = useMemo(() => totalsFrom(rows), [rows])

  const perUser = useMemo(() => {
    return memberBalances
      .filter((q) => q.data)
      .map((q) => ({ username: q.data!.username, expenses: totalsFrom(q.data!.rows).expenses }))
      .sort((a, b) => b.expenses - a.expenses)
  }, [memberBalances])

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
  const currency = rows[0]?.currency || activeWorkspace.currency || ''

  return (
    <div className="space-y-10">
      <section className="space-y-1">
        {isShared ? (
          <>
            <p className="text-xs font-medium uppercase tracking-wider text-muted-foreground">Your spending</p>
            {balanceLoading ? (
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
            {balanceLoading ? (
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
          <p className="text-sm text-destructive">Couldn't load workspace data.</p>
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

      <div className="grid grid-cols-1 gap-8 lg:grid-cols-5">
        <section className="space-y-3 lg:col-span-2">
          <h2 className="text-sm font-semibold uppercase tracking-wider text-muted-foreground">Accounts</h2>
          {balanceLoading ? (
            <SkeletonList rows={5} />
          ) : rows.length === 0 ? (
            <EmptyHint>No balance data yet.</EmptyHint>
          ) : (
            <AccountsTree rows={rows} />
          )}
        </section>

        <section className="space-y-3 lg:col-span-3">
          <h2 className="text-sm font-semibold uppercase tracking-wider text-muted-foreground">Recent activity</h2>
          {registerLoading ? (
            <SkeletonList rows={6} />
          ) : recentEntries.length === 0 ? (
            <EmptyHint>Nothing posted yet.</EmptyHint>
          ) : (
            <ul className="divide-y divide-border/60 rounded-lg border border-border">
              {recentEntries.map((e, i) => {
                const debit = e.postings.find((p) => p.amount > 0) || e.postings[0]
                const credit = e.postings.find((p) => p.account !== debit?.account)
                const primary = debit
                const isIncome = primary?.account.toLowerCase().startsWith('income')
                return (
                  <li key={i} className="flex items-start justify-between gap-4 px-4 py-3">
                    <div className="min-w-0 flex-1">
                      <p className="truncate text-sm font-medium">{e.payee || '—'}</p>
                      <div className="mt-1 flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground">
                        <time className="num shrink-0">{normaliseDate(e.date)}</time>
                        <span>·</span>
                        {debit && (
                          <span className="truncate">
                            {isIncome ? credit?.account : debit.account}
                          </span>
                        )}
                        {credit && debit && debit.account !== credit.account && (
                          <>
                            <ArrowRight className="h-3 w-3 shrink-0" />
                            <span className="truncate">
                              {isIncome ? debit.account : credit.account}
                            </span>
                          </>
                        )}
                      </div>
                    </div>
                    {primary && (
                      <span className={`num shrink-0 text-sm font-semibold tabular-nums ${isIncome ? 'text-accent-mint' : 'text-foreground'}`}>
                        {formatAmount(primary.amount, primary.currency)}
                      </span>
                    )}
                  </li>
                )
              })}
            </ul>
          )}
        </section>
      </div>

      {isShared && perUser.length > 0 && (
        <section className="max-w-sm rounded-lg border border-border p-4">
          <h2 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Roommate split</h2>
          <ul className="mt-3 space-y-1.5">
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
        </section>
      )}
    </div>
  )
}

function AccountsTree({ rows }: { rows: ReturnType<typeof parseBalance> }) {
  // Group rows by their top-level account type (depth 0). Each group is collapsible,
  // open by default. Nested accounts below a group stay nested as an indented list.
  const groups = useMemo(() => {
    const out: Array<{
      header: (typeof rows)[number]
      children: typeof rows
    }> = []
    for (const r of rows) {
      if (r.depth === 0) {
        out.push({ header: r, children: [] })
      } else if (out.length > 0) {
        out[out.length - 1].children.push(r)
      }
    }
    return out
  }, [rows])

  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({})

  return (
    <ul className="divide-y divide-border/60 rounded-lg border border-border">
      {groups.map((g) => {
        const isCollapsed = !!collapsed[g.header.account]
        const hasChildren = g.children.length > 0
        return (
          <li key={g.header.account}>
            <button
              type="button"
              className="flex w-full items-center justify-between px-4 py-2.5 text-left hover:bg-muted/40"
              onClick={() =>
                hasChildren && setCollapsed((c) => ({ ...c, [g.header.account]: !c[g.header.account] }))
              }
              disabled={!hasChildren}
            >
              <div className="flex min-w-0 items-center gap-2">
                {hasChildren ? (
                  isCollapsed ? (
                    <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />
                  ) : (
                    <ChevronDown className="h-3.5 w-3.5 text-muted-foreground" />
                  )
                ) : (
                  <span className="inline-block h-3.5 w-3.5" />
                )}
                <Pill account={g.header.account}>{g.header.account}</Pill>
              </div>
              <span
                className={`num shrink-0 text-sm font-semibold ${
                  g.header.amount < 0 ? 'text-destructive' : 'text-foreground'
                }`}
              >
                {formatAmount(g.header.amount, g.header.currency)}
              </span>
            </button>
            {!isCollapsed && hasChildren && (
              <ul className="divide-y divide-border/60 border-t border-border/60">
                {g.children.map((r, i) => (
                  <li key={i} className="flex items-center justify-between px-4 py-2">
                    <div className="flex min-w-0 items-center gap-2" style={{ paddingLeft: `${(r.depth - 0.5) * 0.75}rem` }}>
                      <span className="text-sm text-muted-foreground">{r.account.split(':').pop()}</span>
                    </div>
                    <span
                      className={`num shrink-0 text-sm ${
                        r.amount < 0 ? 'text-destructive' : 'text-foreground'
                      }`}
                    >
                      {formatAmount(r.amount, r.currency)}
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </li>
        )
      })}
    </ul>
  )
}

function SkeletonList({ rows }: { rows: number }) {
  return (
    <div className="space-y-2 rounded-lg border border-border p-3">
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
