import { useQuery, useQueryClient, useMutation } from '@tanstack/react-query'
import { useMemo, useState } from 'react'
import { transactionsApi } from '@/lib/api/transactions'
import { useWorkspace } from '@/context/workspace-context'
import { parseRegister, formatAmount, normaliseDate } from '@/lib/ledger-parser'
import type { TransactionEntry } from '@/lib/types'
import { ArrowRight, MoreVertical, Pencil, Trash2 } from 'lucide-react'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'

export interface RegisterFilters {
  user?: string
  payee?: string
  begin?: string
  end?: string
}

interface Props {
  filters?: RegisterFilters
  onEdit?: (entry: TransactionEntry) => void
}

function matchKey(date: string, payee: string, absAmount: number): string {
  return `${date}${payee.trim().toLowerCase()}${absAmount.toFixed(2)}`
}

function parseAmountToAbs(raw: string | undefined): number {
  if (!raw) return NaN
  const cleaned = raw.replace(/[^0-9.-]/g, '')
  const n = Number(cleaned)
  return Number.isFinite(n) ? Math.abs(n) : NaN
}

export function RegisterView({ filters = {}, onEdit }: Props) {
  const { activeWorkspace } = useWorkspace()
  const queryClient = useQueryClient()
  const [pendingDelete, setPendingDelete] = useState<TransactionEntry | null>(null)

  const { data: register, isLoading } = useQuery({
    queryKey: ['register', activeWorkspace?.id, filters],
    queryFn: async () => {
      const { data } = await transactionsApi.getRegister(activeWorkspace!.id, filters)
      return data
    },
    enabled: !!activeWorkspace,
  })

  // Structured list gives us the IDs; ledger-cli register output gives the
  // filtered/sorted display rows. We match on date+payee+|amount| because the
  // ledger output groups by entry, and within one workspace the combination is
  // overwhelmingly unique. Ambiguous keys (same date/payee/amount posted
  // twice) collapse to one id — the last one wins — which is an acceptable
  // trade-off given the UX cost of exposing both rows.
  const { data: structured = [] } = useQuery<TransactionEntry[]>({
    queryKey: ['transactions', activeWorkspace?.id],
    queryFn: async () => {
      const { data } = await transactionsApi.list(activeWorkspace!.id)
      return data
    },
    enabled: !!activeWorkspace,
  })

  const idMap = useMemo(() => {
    const m = new Map<string, TransactionEntry>()
    for (const e of structured) {
      const amount = parseAmountToAbs(e.postings[0]?.amount)
      if (!Number.isFinite(amount)) continue
      m.set(matchKey(e.date, e.payee, amount), e)
    }
    return m
  }, [structured])

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

  const deleteMutation = useMutation({
    mutationFn: async (tx: TransactionEntry) => {
      if (!activeWorkspace) throw new Error('No workspace')
      await transactionsApi.delete(activeWorkspace.id, tx.id)
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['register', activeWorkspace?.id] })
      queryClient.invalidateQueries({ queryKey: ['balance', activeWorkspace?.id] })
      queryClient.invalidateQueries({ queryKey: ['transactions', activeWorkspace?.id] })
      setPendingDelete(null)
    },
  })

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
    <>
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
                const key = matchKey(date, e.payee, Math.abs(primary?.amount ?? 0))
                const matched = idMap.get(key)
                return (
                  <li key={i} className="flex items-start justify-between gap-3 px-4 py-3">
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
                    {matched ? (
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <button
                            type="button"
                            aria-label="Row actions"
                            className="-mr-2 -my-1 flex h-8 w-8 shrink-0 items-center justify-center rounded-md text-muted-foreground hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                          >
                            <MoreVertical className="h-4 w-4" />
                          </button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end" className="w-36">
                          <DropdownMenuItem onSelect={() => onEdit?.(matched)}>
                            <Pencil className="mr-2 h-3.5 w-3.5" /> Edit
                          </DropdownMenuItem>
                          <DropdownMenuItem
                            className="text-destructive focus:text-destructive"
                            onSelect={() => setPendingDelete(matched)}
                          >
                            <Trash2 className="mr-2 h-3.5 w-3.5" /> Delete
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    ) : (
                      <div className="w-8 shrink-0" aria-hidden />
                    )}
                  </li>
                )
              })}
            </ul>
          </section>
        ))}
      </div>

      <Dialog
        open={!!pendingDelete}
        onOpenChange={(open) => {
          if (!open) setPendingDelete(null)
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete this transaction?</DialogTitle>
            <DialogDescription>
              {pendingDelete
                ? `${pendingDelete.payee || 'Transaction'} on ${pendingDelete.date}. This can't be undone.`
                : null}
            </DialogDescription>
          </DialogHeader>
          {deleteMutation.isError && (
            <p role="alert" className="text-sm text-destructive">
              Couldn't delete. Try again?
            </p>
          )}
          <DialogFooter className="gap-2">
            <Button
              type="button"
              variant="outline"
              onClick={() => setPendingDelete(null)}
              disabled={deleteMutation.isPending}
            >
              Cancel
            </Button>
            <Button
              type="button"
              variant="destructive"
              disabled={deleteMutation.isPending}
              onClick={() => pendingDelete && deleteMutation.mutate(pendingDelete)}
            >
              {deleteMutation.isPending ? 'Deleting…' : 'Delete'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  )
}
