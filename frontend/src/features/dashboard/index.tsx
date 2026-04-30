import { useQuery } from '@tanstack/react-query'
import { useWorkspace } from '@/context/workspace-context'
import { transactionsApi } from '@/lib/api/transactions'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Pill } from '@/components/ui/pill'
import { parseBalance, parseRegister, formatAmount, normaliseDate } from '@/lib/ledger-parser'
import { ArrowDownLeft, ArrowUpRight, Sparkles } from 'lucide-react'
import { useMemo } from 'react'

export function DashboardPage() {
  const { activeWorkspace } = useWorkspace()

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

  const totals = useMemo(() => {
    let income = 0
    let expenses = 0
    for (const r of rows) {
      if (r.depth > 0) continue
      const key = r.account.toLowerCase()
      if (key.startsWith('income')) income += Math.abs(r.amount)
      if (key.startsWith('expenses')) expenses += Math.abs(r.amount)
    }
    return { income, expenses, net: income - expenses }
  }, [rows])

  const recentEntries = entries.slice(-8).reverse()

  if (!activeWorkspace) {
    return (
      <div className="flex min-h-[60vh] flex-col items-center justify-center gap-4 rounded-2xl bg-warm-gradient p-12 text-center">
        <Sparkles className="h-10 w-10 text-primary" />
        <h2 className="font-display text-2xl font-semibold">No workspace selected</h2>
        <p className="max-w-sm text-muted-foreground">
          Create or select a workspace from the menu above to start tracking your money.
        </p>
      </div>
    )
  }

  const hasError = balanceError || registerError
  const currency = rows[0]?.currency || activeWorkspace.currency || ''

  return (
    <div className="space-y-8">
      {/* Hero */}
      <div>
        <p className="text-sm font-medium uppercase tracking-wider text-muted-foreground">Overview</p>
        <h1 className="mt-1 font-display text-4xl font-semibold tracking-tight">{activeWorkspace.name}</h1>
      </div>

      {hasError && (
        <Card className="border-destructive/40 bg-destructive/5">
          <CardContent className="flex items-center justify-between p-4">
            <p className="text-sm text-destructive">Couldn't load workspace data.</p>
            <Button
              variant="outline"
              size="sm"
              onClick={() => { refetchBalance(); refetchRegister() }}
            >
              Try again
            </Button>
          </CardContent>
        </Card>
      )}

      {/* Summary cards */}
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
        <SummaryCard
          label="Net"
          value={totals.net}
          currency={currency}
          loading={balanceLoading}
          accent="coral"
        />
        <SummaryCard
          label="Income"
          value={totals.income}
          currency={currency}
          loading={balanceLoading}
          accent="mint"
          icon={<ArrowDownLeft className="h-4 w-4" />}
        />
        <SummaryCard
          label="Expenses"
          value={totals.expenses}
          currency={currency}
          loading={balanceLoading}
          accent="rose"
          icon={<ArrowUpRight className="h-4 w-4" />}
        />
      </div>

      {/* Two-column: balance breakdown + recent transactions */}
      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Balance breakdown</CardTitle>
          </CardHeader>
          <CardContent>
            {balanceLoading ? (
              <SkeletonList rows={5} />
            ) : rows.length === 0 ? (
              <EmptyHint>No balance data yet. Post a transaction to see it here.</EmptyHint>
            ) : (
              <ul className="divide-y divide-border/60">
                {rows.map((r, i) => (
                  <li key={i} className="flex items-center justify-between py-3">
                    <div className="flex items-center gap-3">
                      <span style={{ paddingLeft: `${r.depth * 0.75}rem` }} />
                      <Pill account={r.account}>{r.account.split(':').pop()}</Pill>
                      <span className="text-sm text-muted-foreground">
                        {r.account.includes(':') ? r.account.split(':').slice(0, -1).join(' / ') : ''}
                      </span>
                    </div>
                    <span className={`num font-display text-base font-semibold ${r.amount < 0 ? 'text-destructive' : 'text-foreground'}`}>
                      {formatAmount(r.amount, r.currency)}
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Recent transactions</CardTitle>
          </CardHeader>
          <CardContent>
            {registerLoading ? (
              <SkeletonList rows={5} />
            ) : recentEntries.length === 0 ? (
              <EmptyHint>No transactions yet.</EmptyHint>
            ) : (
              <ul className="divide-y divide-border/60">
                {recentEntries.map((e, i) => {
                  const first = e.postings[0]
                  const counter = e.postings.find((p) => p.account !== first?.account)
                  const debit = [first, counter].find((p) => p && p.amount > 0)
                  const primary = debit || first
                  return (
                    <li key={i} className="flex items-center justify-between gap-3 py-3">
                      <div className="min-w-0 flex-1">
                        <p className="truncate text-sm font-medium">{e.payee || '—'}</p>
                        <div className="mt-1 flex items-center gap-2">
                          <time className="num text-xs text-muted-foreground">{normaliseDate(e.date)}</time>
                          {primary && <Pill account={primary.account}>{primary.account.split(':').slice(0, 2).join(':')}</Pill>}
                        </div>
                      </div>
                      {primary && (
                        <span className={`num font-display text-base font-semibold ${primary.account.toLowerCase().startsWith('income') ? 'text-accent-mint' : 'text-foreground'}`}>
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
    </div>
  )
}

function SummaryCard({
  label,
  value,
  currency,
  loading,
  accent,
  icon,
}: {
  label: string
  value: number
  currency: string
  loading: boolean
  accent: 'coral' | 'mint' | 'rose'
  icon?: React.ReactNode
}) {
  const tint: Record<string, string> = {
    coral: 'from-primary/10 to-primary/0 text-primary',
    mint: 'from-accent-mint/15 to-accent-mint/0 text-accent-mint',
    rose: 'from-accent-rose/15 to-accent-rose/0 text-accent-rose',
  }
  return (
    <Card className={`bg-gradient-to-br ${tint[accent].replace(/text-\S+/, '')}`}>
      <CardContent className="p-5">
        <div className="flex items-center justify-between">
          <span className="text-sm font-medium text-muted-foreground">{label}</span>
          {icon && <span className={tint[accent].match(/text-\S+/)?.[0]}>{icon}</span>}
        </div>
        {loading ? (
          <div className="mt-3 h-8 animate-pulse rounded-lg bg-muted" />
        ) : (
          <p className={`mt-2 font-display text-3xl font-semibold tracking-tight ${tint[accent].match(/text-\S+/)?.[0]} num`}>
            {formatAmount(value, currency)}
          </p>
        )}
      </CardContent>
    </Card>
  )
}

function SkeletonList({ rows }: { rows: number }) {
  return (
    <div className="space-y-3">
      {Array.from({ length: rows }).map((_, i) => (
        <div key={i} className="h-10 animate-pulse rounded-lg bg-muted" />
      ))}
    </div>
  )
}

function EmptyHint({ children }: { children: React.ReactNode }) {
  return <p className="py-4 text-center text-sm text-muted-foreground">{children}</p>
}
