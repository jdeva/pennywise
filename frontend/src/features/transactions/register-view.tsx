import { useQuery } from '@tanstack/react-query'
import { useMemo } from 'react'
import { transactionsApi } from '@/lib/api/transactions'
import { useWorkspace } from '@/context/workspace-context'
import { parseRegister, formatAmount, normaliseDate } from '@/lib/ledger-parser'
import { ArrowRight } from 'lucide-react'

export interface RegisterFilters {
  user?: string
  payee?: string
  begin?: string
  end?: string
}

interface Props {
  filters?: RegisterFilters
}

export function RegisterView({ filters = {} }: Props) {
  const { activeWorkspace } = useWorkspace()

  const { data: register, isLoading } = useQuery({
    queryKey: ['register', activeWorkspace?.id, filters],
    queryFn: async () => {
      const { data } = await transactionsApi.getRegister(activeWorkspace!.id, filters)
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

  const hasActiveFilters = !!(filters.user || filters.payee || filters.begin || filters.end)

  if (isLoading) {
    return (
      <div className="space-y-2">
        {Array.from({ length: 5 }).map((_, i) => (
          <div key={i} className="h-14 animate-pulse rounded-lg bg-muted" />
        ))}
      </div>
    )
  }

  if (entries.length === 0) {
    return (
      <div className="rounded-lg border border-dashed border-border px-4 py-10 text-center text-sm text-muted-foreground">
        {hasActiveFilters
          ? 'Nothing fits those filters.'
          : 'No transactions yet — lure one in with the New button above.'}
      </div>
    )
  }

  return (
    <div className="space-y-6">
      {grouped.map(([date, dayEntries]) => (
        <section key={date}>
          <h4 className="num mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            {date}
          </h4>
          <ul className="divide-y divide-border/60 rounded-lg border border-border">
            {dayEntries.map((e, i) => {
              // debit posting = where money went (expense/asset in); credit = source
              const debit = e.postings.find((p) => p.amount > 0) || e.postings[0]
              const credit = e.postings.find((p) => p.account !== debit?.account)
              const primary = debit || credit
              const isIncome = primary?.account.toLowerCase().startsWith('income')
              return (
                <li key={i} className="flex items-start justify-between gap-4 px-4 py-3">
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-sm font-medium">{e.payee || '—'}</p>
                    <div className="mt-1 flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground">
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
                    <span
                      className={`num shrink-0 font-display text-base font-semibold tabular-nums ${
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
        </section>
      ))}
    </div>
  )
}
