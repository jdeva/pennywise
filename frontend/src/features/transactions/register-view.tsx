import { useQuery } from '@tanstack/react-query'
import { useMemo } from 'react'
import { transactionsApi } from '@/lib/api/transactions'
import { useWorkspace } from '@/context/workspace-context'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Pill } from '@/components/ui/pill'
import { parseRegister, formatAmount, normaliseDate } from '@/lib/ledger-parser'

export function RegisterView() {
  const { activeWorkspace } = useWorkspace()

  const { data: register, isLoading } = useQuery({
    queryKey: ['register', activeWorkspace?.id],
    queryFn: async () => {
      const { data } = await transactionsApi.getRegister(activeWorkspace!.id)
      return data
    },
    enabled: !!activeWorkspace,
  })

  const entries = useMemo(() => (register?.output ? parseRegister(register.output) : []), [register?.output])

  const grouped = useMemo(() => {
    const byDate = new Map<string, typeof entries>()
    for (const e of [...entries].reverse()) {
      const d = normaliseDate(e.date)
      const bucket = byDate.get(d) || []
      bucket.push(e)
      byDate.set(d, bucket)
    }
    return Array.from(byDate.entries())
  }, [entries])

  return (
    <Card>
      <CardHeader>
        <CardTitle>Transaction register</CardTitle>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="space-y-3">
            {Array.from({ length: 5 }).map((_, i) => (
              <div key={i} className="h-12 animate-pulse rounded-lg bg-muted" />
            ))}
          </div>
        ) : entries.length === 0 ? (
          <p className="py-6 text-center text-sm text-muted-foreground">
            Nothing posted yet. Add your first transaction above.
          </p>
        ) : (
          <div className="space-y-6">
            {grouped.map(([date, dayEntries]) => (
              <section key={date}>
                <h4 className="num mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                  {date}
                </h4>
                <ul className="divide-y divide-border/60 rounded-xl bg-muted/30">
                  {dayEntries.map((e, i) => {
                    const primary = e.postings.find((p) => p.amount > 0) || e.postings[0]
                    const other = e.postings.find((p) => p.account !== primary?.account)
                    return (
                      <li key={i} className="flex items-start justify-between gap-3 px-4 py-3">
                        <div className="min-w-0 flex-1">
                          <p className="truncate text-sm font-medium">{e.payee || '—'}</p>
                          <div className="mt-1 flex flex-wrap items-center gap-1.5">
                            {primary && <Pill account={primary.account}>{primary.account}</Pill>}
                            {other && (
                              <>
                                <span className="text-xs text-muted-foreground">from</span>
                                <Pill account={other.account}>{other.account}</Pill>
                              </>
                            )}
                          </div>
                        </div>
                        {primary && (
                          <span className="num shrink-0 font-display text-base font-semibold">
                            {formatAmount(primary.amount, primary.currency)}
                          </span>
                        )}
                      </li>
                    )
                  })}
                </ul>
              </section>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  )
}
