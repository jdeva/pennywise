import { Link, useRouterState } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { useMemo } from 'react'
import { useWorkspace } from '@/context/workspace-context'
import { useDateRange } from '@/context/date-range-context'
import { transactionsApi } from '@/lib/api/transactions'
import { parseRegister, formatAmount, normaliseDate, type RegisterEntry } from '@/lib/ledger-parser'
import { Card, CardContent } from '@/components/ui/card'
import { cn } from '@/lib/utils'
import {
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Line,
  LineChart,
  Pie,
  PieChart,
  ResponsiveContainer,
  Sankey,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts'

const tabs = [
  { to: '/reports/cashflow' as const, label: 'Cashflow' },
  { to: '/reports/categories' as const, label: 'Categories' },
  { to: '/reports/trend' as const, label: 'Trend' },
  { to: '/reports/merchants' as const, label: 'Merchants' },
]

function useRegisterEntries() {
  const { activeWorkspace } = useWorkspace()
  const { range } = useDateRange()
  const { data, isLoading, error } = useQuery({
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
  const entries = useMemo(() => {
    if (!data?.output) return [] as RegisterEntry[]
    const parsed = parseRegister(data.output)
    return parsed.filter((e) => {
      const d = normaliseDate(e.date)
      return d >= range.begin && d <= range.end
    })
  }, [data?.output, range.begin, range.end])

  const currency = useMemo(() => {
    for (const e of entries) for (const p of e.postings) if (p.currency) return p.currency
    return activeWorkspace?.currency ?? ''
  }, [entries, activeWorkspace?.currency])

  return { entries, currency, isLoading, error, hasWorkspace: !!activeWorkspace, range }
}

function monthKey(iso: string): string {
  return iso.slice(0, 7) // YYYY-MM
}

function labelMonth(key: string): string {
  const [y, m] = key.split('-')
  const date = new Date(Number(y), Number(m) - 1, 1)
  return date.toLocaleString(undefined, { month: 'short', year: '2-digit' })
}

interface IncomeExpenseByMonth {
  month: string
  label: string
  income: number
  expenses: number
}

function bucketByMonth(entries: RegisterEntry[]): IncomeExpenseByMonth[] {
  const map = new Map<string, { income: number; expenses: number }>()
  for (const e of entries) {
    const d = normaliseDate(e.date)
    const key = monthKey(d)
    const cur = map.get(key) ?? { income: 0, expenses: 0 }
    for (const p of e.postings) {
      const top = p.account.split(':')[0].toLowerCase()
      if (top === 'income') cur.income += Math.abs(p.amount)
      else if (top === 'expenses') cur.expenses += Math.abs(p.amount)
    }
    map.set(key, cur)
  }
  return Array.from(map.entries())
    .sort(([a], [b]) => (a < b ? -1 : 1))
    .map(([month, v]) => ({ month, label: labelMonth(month), income: v.income, expenses: v.expenses }))
}

function spendByCategory(entries: RegisterEntry[]): Array<{ name: string; value: number }> {
  const map = new Map<string, number>()
  for (const e of entries) {
    for (const p of e.postings) {
      const top = p.account.split(':')[0].toLowerCase()
      if (top !== 'expenses') continue
      const segs = p.account.split(':')
      const key = segs.length >= 2 ? segs[1] : p.account
      map.set(key, (map.get(key) ?? 0) + Math.abs(p.amount))
    }
  }
  return Array.from(map.entries())
    .map(([name, value]) => ({ name, value }))
    .sort((a, b) => b.value - a.value)
}

function spendByDay(entries: RegisterEntry[]): Array<{ date: string; spend: number }> {
  const map = new Map<string, number>()
  for (const e of entries) {
    const d = normaliseDate(e.date)
    let spend = 0
    for (const p of e.postings) {
      if (p.account.split(':')[0].toLowerCase() === 'expenses') spend += Math.abs(p.amount)
    }
    map.set(d, (map.get(d) ?? 0) + spend)
  }
  return Array.from(map.entries())
    .sort(([a], [b]) => (a < b ? -1 : 1))
    .map(([date, spend]) => ({ date, spend }))
}

function spendByMonth(entries: RegisterEntry[]): Array<{ month: string; label: string; spend: number }> {
  return bucketByMonth(entries).map((m) => ({ month: m.month, label: m.label, spend: m.expenses }))
}

/**
 * Build a Monarch-style Sankey graph: income sources → "Income" hub → expense
 * top-level groups → subcategories. Savings (= net positive) appears as a
 * first-class sibling to expense groups.
 *
 * Each node carries a `kind` so the renderer can pick the right colour band
 * (income vs savings vs group vs leaf) and a `group` so sub-category nodes
 * inherit their parent group's colour.
 */
type NodeKind = 'source' | 'hub' | 'savings' | 'group' | 'sub'

interface SankeyNodeDatum {
  name: string
  kind: NodeKind
  group?: string
}

interface SankeyGraph {
  nodes: SankeyNodeDatum[]
  links: Array<{ source: number; target: number; value: number }>
  total: number
}

function buildCashflowSankey(entries: RegisterEntry[]): SankeyGraph {
  const incomeBySource = new Map<string, number>()
  const expenseByGroupSub = new Map<string, Map<string, number>>()
  let totalIncome = 0
  let totalExpenses = 0

  for (const e of entries) {
    for (const p of e.postings) {
      const segs = p.account.split(':')
      const top = segs[0].toLowerCase()
      const amt = Math.abs(p.amount)
      if (top === 'income') {
        const source = segs[1] ?? 'Other'
        incomeBySource.set(source, (incomeBySource.get(source) ?? 0) + amt)
        totalIncome += amt
      } else if (top === 'expenses') {
        const group = segs[1] ?? 'Other'
        const sub = segs[2] ?? group
        const inner = expenseByGroupSub.get(group) ?? new Map<string, number>()
        inner.set(sub, (inner.get(sub) ?? 0) + amt)
        expenseByGroupSub.set(group, inner)
        totalExpenses += amt
      }
    }
  }

  if (totalIncome === 0 && totalExpenses === 0) {
    return { nodes: [], links: [], total: 0 }
  }

  const nodes: SankeyNodeDatum[] = []
  const links: Array<{ source: number; target: number; value: number }> = []
  const nodeIdx = new Map<string, number>()
  const addNode = (datum: SankeyNodeDatum) => {
    const existing = nodeIdx.get(datum.name)
    if (existing !== undefined) return existing
    const i = nodes.length
    nodes.push(datum)
    nodeIdx.set(datum.name, i)
    return i
  }

  const incomeHub = addNode({ name: 'Income', kind: 'hub' })

  for (const [source, value] of [...incomeBySource.entries()].sort((a, b) => b[1] - a[1])) {
    const src = addNode({ name: source, kind: 'source' })
    links.push({ source: src, target: incomeHub, value })
  }

  for (const [group, subs] of [...expenseByGroupSub.entries()].sort(
    (a, b) => sumMap(b[1]) - sumMap(a[1]),
  )) {
    const groupTotal = sumMap(subs)
    const groupNode = addNode({ name: group, kind: 'group', group })
    links.push({ source: incomeHub, target: groupNode, value: groupTotal })
    const subEntries = [...subs.entries()].sort((a, b) => b[1] - a[1])
    // Skip the third column when a group has a single sub-category matching
    // the group name — avoids a redundant "Shopping → Shopping" hop.
    if (subEntries.length === 1 && subEntries[0][0] === group) continue
    for (const [sub, value] of subEntries) {
      const subNode = addNode({ name: `${group}: ${sub}`, kind: 'sub', group })
      links.push({ source: groupNode, target: subNode, value })
    }
  }

  const surplus = totalIncome - totalExpenses
  if (surplus > 0) {
    const savings = addNode({ name: 'Savings', kind: 'savings' })
    links.push({ source: incomeHub, target: savings, value: surplus })
  }

  return { nodes, links, total: totalIncome }
}

function sumMap(m: Map<string, number>): number {
  let s = 0
  for (const v of m.values()) s += v
  return s
}

function merchantLeaderboard(entries: RegisterEntry[]): Array<{ payee: string; count: number; spend: number }> {
  const map = new Map<string, { count: number; spend: number }>()
  for (const e of entries) {
    const payee = e.payee.trim() || '—'
    const cur = map.get(payee) ?? { count: 0, spend: 0 }
    cur.count += 1
    for (const p of e.postings) {
      if (p.account.split(':')[0].toLowerCase() === 'expenses') cur.spend += Math.abs(p.amount)
    }
    map.set(payee, cur)
  }
  return Array.from(map.entries())
    .map(([payee, v]) => ({ payee, count: v.count, spend: v.spend }))
    .filter((m) => m.spend > 0)
    .sort((a, b) => b.spend - a.spend)
}

export function ReportsShell({ children }: { children: React.ReactNode }) {
  const router = useRouterState()
  const current = router.location.pathname

  return (
    <div className="space-y-6">
      <div>
        <h1 className="font-display text-3xl font-semibold tracking-tight">Reports</h1>
        <p className="text-sm text-muted-foreground">Understand where your money goes.</p>
      </div>
      <nav className="-mx-4 flex gap-1 overflow-x-auto border-b border-border px-4 sm:mx-0 sm:px-0">
        {tabs.map((t) => {
          const active = current === t.to || (t.to === '/reports/cashflow' && current === '/reports')
          return (
            <Link
              key={t.to}
              to={t.to}
              className={cn(
                'shrink-0 rounded-t-md border-b-2 px-3 py-2 text-sm font-medium transition-colors',
                active
                  ? 'border-primary text-primary'
                  : 'border-transparent text-muted-foreground hover:text-foreground',
              )}
            >
              {t.label}
            </Link>
          )
        })}
      </nav>
      <div>{children}</div>
    </div>
  )
}

function NoWorkspace() {
  return (
    <div className="rounded-xl border border-dashed border-border p-10 text-center">
      <p className="text-sm text-muted-foreground">Pick a lair from the sidebar to see reports.</p>
    </div>
  )
}

function Empty({ children }: { children: React.ReactNode }) {
  return (
    <div className="rounded-xl border border-dashed border-border p-10 text-center text-sm text-muted-foreground">
      {children}
    </div>
  )
}

function SkeletonChart() {
  return <div className="h-64 w-full animate-pulse rounded-lg bg-muted" />
}

// Palette for donut / stacked slices — derived from the lair's accent plus
// complementary neutrals. Recharts accepts CSS vars just fine via hsl().
const SLICE_FILLS = [
  'hsl(var(--primary))',
  'hsl(var(--accent-mint))',
  'hsl(var(--accent-butter))',
  'hsl(var(--accent-lavender))',
  'hsl(var(--accent-sky))',
  'hsl(var(--accent-rose))',
  'hsl(var(--accent-peach))',
]

export function CashflowReport() {
  const { entries, currency, isLoading, hasWorkspace } = useRegisterEntries()
  const sankey = useMemo(() => buildCashflowSankey(entries), [entries])

  const totals = useMemo(() => {
    let income = 0
    let expenses = 0
    for (const e of entries) {
      for (const p of e.postings) {
        const top = p.account.split(':')[0].toLowerCase()
        if (top === 'income') income += Math.abs(p.amount)
        else if (top === 'expenses') expenses += Math.abs(p.amount)
      }
    }
    return { income, expenses, net: income - expenses }
  }, [entries])

  if (!hasWorkspace) return <ReportsShell><NoWorkspace /></ReportsShell>

  return (
    <ReportsShell>
      <div className="space-y-4">
        {/* Summary totals row — Monarch-style three-up */}
        <div className="grid grid-cols-3 gap-3">
          <SummaryTile label="Income" value={totals.income} currency={currency} tone="positive" />
          <SummaryTile label="Expenses" value={totals.expenses} currency={currency} tone="negative" />
          <SummaryTile label="Net" value={totals.net} currency={currency} tone="net" />
        </div>

        <Card>
          <CardContent className="space-y-3 py-4">
            {isLoading ? (
              <SkeletonChart />
            ) : sankey.nodes.length === 0 ? (
              <Empty>Nothing to chart in this range.</Empty>
            ) : (
              <div className="h-[min(75vh,44rem)] w-full">
                <ResponsiveContainer width="100%" height="100%">
                  <Sankey
                    data={sankey}
                    nodePadding={28}
                    nodeWidth={10}
                    margin={{ top: 16, right: 220, bottom: 16, left: 140 }}
                    link={<SankeyLink nodes={sankey.nodes} />}
                    node={<SankeyNode currency={currency} total={sankey.total} />}
                  >
                    <Tooltip
                      contentStyle={{
                        backgroundColor: 'hsl(var(--popover))',
                        border: '1px solid hsl(var(--border))',
                        borderRadius: 10,
                        fontSize: 12,
                        padding: '8px 12px',
                      }}
                      formatter={(value) =>
                        [formatAmount(Number(value) || 0, currency), 'Flow'] as [string, string]
                      }
                    />
                  </Sankey>
                </ResponsiveContainer>
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </ReportsShell>
  )
}

interface SankeyNodeProps {
  x?: number
  y?: number
  width?: number
  height?: number
  index?: number
  payload?: { name?: string; value?: number } & Partial<SankeyNodeDatum>
  containerWidth?: number
  currency: string
  total: number
}

// Solid accent palette for expense groups — order stable so the same category
// gets the same colour across re-renders (we hash the group name into it).
const GROUP_PALETTE = [
  'hsl(var(--accent-rose))',
  'hsl(var(--accent-peach))',
  'hsl(var(--accent-butter))',
  'hsl(var(--accent-lavender))',
  'hsl(var(--primary))',
]

function colourFor(node: SankeyNodeDatum | undefined): string {
  if (!node) return 'hsl(var(--muted-foreground))'
  if (node.kind === 'hub' || node.kind === 'source') return 'hsl(var(--accent-mint))'
  if (node.kind === 'savings') return 'hsl(var(--accent-sky))'
  // group / sub both keyed by their `group` name so sub-categories match parent
  const key = node.group ?? node.name
  let h = 0
  for (let i = 0; i < key.length; i++) h = (h * 31 + key.charCodeAt(i)) >>> 0
  return GROUP_PALETTE[h % GROUP_PALETTE.length]
}

function SankeyNode({ x = 0, y = 0, width = 0, height = 0, payload, containerWidth = 0, currency, total }: SankeyNodeProps) {
  const name = payload?.name ?? ''
  const value = payload?.value ?? 0
  const isLeft = x < containerWidth / 2
  const fill = colourFor(payload as SankeyNodeDatum)
  const pct = total > 0 ? (value / total) * 100 : 0
  // Labels sit in the chart margin; the anchor flips sides around mid-canvas.
  const anchorX = isLeft ? x - 10 : x + width + 10
  const textAnchor = isLeft ? 'end' : 'start'
  // Sub-category labels display "Food: Groceries" → just "Groceries" to
  // de-duplicate the group name that already sits one column left.
  const display = name.includes(': ') ? name.split(': ').slice(1).join(': ') : name
  return (
    <g>
      <rect x={x} y={y} width={width} height={height} fill={fill} rx={2} />
      <text
        x={anchorX}
        y={y + height / 2 - 6}
        textAnchor={textAnchor}
        dominantBaseline="middle"
        className="fill-foreground"
        fontSize={12}
        fontWeight={500}
      >
        {display}
      </text>
      <text
        x={anchorX}
        y={y + height / 2 + 9}
        textAnchor={textAnchor}
        dominantBaseline="middle"
        className="fill-muted-foreground"
        fontSize={11}
      >
        {formatAmount(value, currency)}
        {pct >= 0.5 ? ` · ${pct.toFixed(1)}%` : ''}
      </text>
    </g>
  )
}

interface SankeyLinkProps {
  sourceX?: number
  targetX?: number
  sourceY?: number
  targetY?: number
  sourceControlX?: number
  targetControlX?: number
  linkWidth?: number
  payload?: { source?: { name?: string } & Partial<SankeyNodeDatum>; target?: { name?: string } & Partial<SankeyNodeDatum> }
  nodes: SankeyNodeDatum[]
}

/**
 * Custom link renderer so each ribbon inherits its target's colour (Monarch
 * style — the "where the money goes" side is what reads). Recharts supplies
 * the Bezier control points via props, so we just hand-roll the path.
 */
function SankeyLink({
  sourceX = 0,
  targetX = 0,
  sourceY = 0,
  targetY = 0,
  sourceControlX = 0,
  targetControlX = 0,
  linkWidth = 0,
  payload,
}: SankeyLinkProps) {
  const target = payload?.target as SankeyNodeDatum | undefined
  const stroke = colourFor(target)
  const path = `M${sourceX},${sourceY}C${sourceControlX},${sourceY} ${targetControlX},${targetY} ${targetX},${targetY}`
  return (
    <path
      d={path}
      stroke={stroke}
      strokeOpacity={0.35}
      strokeWidth={Math.max(linkWidth, 1)}
      fill="none"
    />
  )
}

function SummaryTile({
  label,
  value,
  currency,
  tone,
}: {
  label: string
  value: number
  currency: string
  tone: 'positive' | 'negative' | 'net'
}) {
  const amountClass =
    tone === 'net'
      ? value < 0
        ? 'text-destructive'
        : 'text-foreground'
      : tone === 'positive'
      ? 'text-accent-mint'
      : 'text-foreground'

  return (
    <Card>
      <CardContent className="space-y-1 p-4">
        <p className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">{label}</p>
        <p className={cn('num font-display text-xl font-semibold tabular-nums sm:text-2xl', amountClass)}>
          {formatAmount(value, currency)}
        </p>
      </CardContent>
    </Card>
  )
}

export function CategoriesReport() {
  const { entries, currency, isLoading, hasWorkspace } = useRegisterEntries()
  const data = useMemo(() => spendByCategory(entries), [entries])
  const total = useMemo(() => data.reduce((sum, d) => sum + d.value, 0), [data])

  if (!hasWorkspace) return <ReportsShell><NoWorkspace /></ReportsShell>

  return (
    <ReportsShell>
      <div className="grid gap-6 lg:grid-cols-5">
        <Card className="lg:col-span-2">
          <CardContent className="py-4">
            {isLoading ? (
              <SkeletonChart />
            ) : data.length === 0 ? (
              <Empty>No spending in this range.</Empty>
            ) : (
              <div className="relative h-64 w-full">
                <ResponsiveContainer width="100%" height="100%">
                  <PieChart>
                    <Tooltip
                      contentStyle={{
                        backgroundColor: 'hsl(var(--popover))',
                        border: '1px solid hsl(var(--border))',
                        borderRadius: 8,
                        fontSize: 12,
                      }}
                      formatter={(value, name) => [formatAmount(Number(value) || 0, currency), String(name)]}
                    />
                    <Pie
                      data={data}
                      dataKey="value"
                      nameKey="name"
                      innerRadius="60%"
                      outerRadius="90%"
                      paddingAngle={1.5}
                      stroke="hsl(var(--background))"
                      strokeWidth={2}
                    >
                      {data.map((_, i) => (
                        <Cell key={i} fill={SLICE_FILLS[i % SLICE_FILLS.length]} />
                      ))}
                    </Pie>
                  </PieChart>
                </ResponsiveContainer>
                <div className="pointer-events-none absolute inset-0 flex flex-col items-center justify-center">
                  <span className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">Total</span>
                  <span className="num font-display text-xl font-semibold">{formatAmount(total, currency)}</span>
                </div>
              </div>
            )}
          </CardContent>
        </Card>

        <Card className="lg:col-span-3">
          <CardContent className="py-4">
            {isLoading ? (
              <SkeletonChart />
            ) : data.length === 0 ? (
              <Empty>No category breakdown yet.</Empty>
            ) : (
              <ul className="space-y-2.5">
                {data.map((c, i) => {
                  const pct = total > 0 ? (c.value / total) * 100 : 0
                  return (
                    <li key={c.name} className="space-y-1">
                      <div className="flex items-baseline justify-between gap-2">
                        <span className="flex min-w-0 items-center gap-2">
                          <span
                            className="h-2.5 w-2.5 shrink-0 rounded-sm"
                            style={{ backgroundColor: SLICE_FILLS[i % SLICE_FILLS.length] }}
                          />
                          <span className="truncate text-sm font-medium">{c.name}</span>
                        </span>
                        <span className="num shrink-0 text-sm tabular-nums">
                          {formatAmount(c.value, currency)}{' '}
                          <span className="text-muted-foreground">· {pct.toFixed(0)}%</span>
                        </span>
                      </div>
                      <div className="h-1.5 overflow-hidden rounded-full bg-muted">
                        <div
                          className="h-full rounded-full"
                          style={{
                            width: `${Math.max(2, pct)}%`,
                            backgroundColor: SLICE_FILLS[i % SLICE_FILLS.length],
                          }}
                        />
                      </div>
                    </li>
                  )
                })}
              </ul>
            )}
          </CardContent>
        </Card>
      </div>
    </ReportsShell>
  )
}

export function TrendReport() {
  const { entries, currency, isLoading, hasWorkspace, range } = useRegisterEntries()
  // If the range is ≤ 60 days, show daily; otherwise monthly.
  const dayCount = useMemo(() => {
    const b = new Date(range.begin).getTime()
    const e = new Date(range.end).getTime()
    return Math.round((e - b) / (1000 * 60 * 60 * 24)) + 1
  }, [range.begin, range.end])
  const daily = dayCount <= 60
  const data = useMemo(
    () => (daily ? spendByDay(entries).map((d) => ({ x: d.date, spend: d.spend })) : spendByMonth(entries).map((m) => ({ x: m.label, spend: m.spend }))),
    [entries, daily],
  )

  if (!hasWorkspace) return <ReportsShell><NoWorkspace /></ReportsShell>

  return (
    <ReportsShell>
      <Card>
        <CardContent className="py-4">
          {isLoading ? (
            <SkeletonChart />
          ) : data.length === 0 ? (
            <Empty>Nothing trending yet.</Empty>
          ) : (
            <div className="h-64 w-full">
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={data} margin={{ top: 8, right: 8, bottom: 4, left: 4 }}>
                  <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" vertical={false} />
                  <XAxis
                    dataKey="x"
                    tick={{ fontSize: 11 }}
                    stroke="hsl(var(--muted-foreground))"
                    // In daily mode the axis gets crowded — show every Nth label.
                    interval={daily ? Math.max(0, Math.floor(data.length / 10) - 1) : 0}
                  />
                  <YAxis
                    tick={{ fontSize: 11 }}
                    stroke="hsl(var(--muted-foreground))"
                    tickFormatter={(v) => formatAmount(Number(v) || 0, currency)}
                    width={70}
                  />
                  <Tooltip
                    contentStyle={{
                      backgroundColor: 'hsl(var(--popover))',
                      border: '1px solid hsl(var(--border))',
                      borderRadius: 8,
                      fontSize: 12,
                    }}
                    formatter={(value) => [formatAmount(Number(value) || 0, currency), 'Spend']}
                  />
                  <Line
                    type="monotone"
                    dataKey="spend"
                    stroke="hsl(var(--primary))"
                    strokeWidth={2}
                    dot={data.length < 20 ? { r: 3, strokeWidth: 0, fill: 'hsl(var(--primary))' } : false}
                    activeDot={{ r: 4 }}
                  />
                </LineChart>
              </ResponsiveContainer>
            </div>
          )}
        </CardContent>
      </Card>
    </ReportsShell>
  )
}

export function MerchantsReport() {
  const { entries, currency, isLoading, hasWorkspace } = useRegisterEntries()
  const data = useMemo(() => merchantLeaderboard(entries).slice(0, 20), [entries])
  const topSpend = data[0]?.spend ?? 0

  if (!hasWorkspace) return <ReportsShell><NoWorkspace /></ReportsShell>

  return (
    <ReportsShell>
      <Card>
        <CardContent className="py-4">
          {isLoading ? (
            <SkeletonChart />
          ) : data.length === 0 ? (
            <Empty>No merchant activity yet.</Empty>
          ) : (
            <ul className="divide-y divide-border/60">
              {data.map((m, i) => {
                const pct = topSpend > 0 ? (m.spend / topSpend) * 100 : 0
                return (
                  <li key={m.payee} className="flex items-center gap-3 py-2.5 first:pt-0 last:pb-0">
                    <span className="num w-6 shrink-0 text-xs font-semibold text-muted-foreground">{i + 1}</span>
                    <div className="min-w-0 flex-1">
                      <div className="flex items-baseline justify-between gap-2">
                        <span className="truncate text-sm font-medium">{m.payee}</span>
                        <span className="num shrink-0 text-sm tabular-nums">{formatAmount(m.spend, currency)}</span>
                      </div>
                      <div className="mt-1 flex items-center gap-2">
                        <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-muted">
                          <div className="h-full rounded-full bg-primary" style={{ width: `${Math.max(3, pct)}%` }} />
                        </div>
                        <span className="shrink-0 text-[11px] text-muted-foreground">
                          {m.count} {m.count === 1 ? 'visit' : 'visits'}
                        </span>
                      </div>
                    </div>
                  </li>
                )
              })}
            </ul>
          )}
        </CardContent>
      </Card>
    </ReportsShell>
  )
}
