import { useMemo, useState } from 'react'
import { useWorkspace } from '@/context/workspace-context'
import { useAuth } from '@/context/auth-context'
import { useDateRange } from '@/context/date-range-context'
import { useTxForm } from '@/context/tx-form-context'
import { RegisterView, type RegisterFilters } from './register-view'
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
import { Search, SlidersHorizontal, X, Calendar, UserRound } from 'lucide-react'

export function TransactionsPage() {
  const { activeWorkspace } = useWorkspace()
  const { user } = useAuth()
  const { range } = useDateRange()
  const { openForEdit } = useTxForm()
  const [filterOpen, setFilterOpen] = useState(false)

  const [payee, setPayee] = useState('')
  const [filterUser, setFilterUser] = useState('')
  // begin/end default to the global date range but can be overridden here
  const [begin, setBegin] = useState<string | null>(null)
  const [end, setEnd] = useState<string | null>(null)

  const effectiveBegin = begin ?? range.begin
  const effectiveEnd = end ?? range.end
  const usingGlobalRange = begin === null && end === null

  const members = useMemo(() => {
    if (!activeWorkspace || !user) return [] as string[]
    const names = new Set<string>()
    if (user.username) names.add(user.username)
    for (const s of activeWorkspace.shared_with) {
      if (s.username) names.add(s.username)
    }
    return Array.from(names)
  }, [activeWorkspace, user])

  const filters: RegisterFilters = useMemo(
    () => ({
      payee: payee.trim() || undefined,
      user: filterUser || undefined,
      begin: effectiveBegin || undefined,
      end: effectiveEnd || undefined,
    }),
    [payee, filterUser, effectiveBegin, effectiveEnd],
  )

  // Count user-controlled filters beyond the inherited date range.
  const activeCustomCount =
    (payee.trim() ? 1 : 0) + (filterUser ? 1 : 0) + (!usingGlobalRange ? 1 : 0)

  const resetDateRange = () => {
    setBegin(null)
    setEnd(null)
  }

  const clearFilters = () => {
    setPayee('')
    setFilterUser('')
    resetDateRange()
  }

  if (!activeWorkspace) {
    return (
      <div className="flex min-h-[50vh] flex-col items-center justify-center gap-3 rounded-xl border border-dashed border-border py-12 text-center">
        <h2 className="font-display text-xl font-semibold">No lair selected</h2>
        <p className="max-w-sm text-sm text-muted-foreground">
          Pick or create a lair from the sidebar to post transactions.
        </p>
      </div>
    )
  }

  const formatShortDate = (iso: string) => {
    const [y, m, d] = iso.split('-')
    return `${d}/${m}/${y.slice(2)}`
  }

  // Chip shown below the search bar. Each one is tappable to remove or edit.
  const chips: Array<{ key: string; icon?: React.ReactNode; label: string; onRemove?: () => void }> = []
  chips.push({
    key: 'range',
    icon: <Calendar className="h-3 w-3" />,
    label: usingGlobalRange
      ? 'This range'
      : `${formatShortDate(effectiveBegin)} – ${formatShortDate(effectiveEnd)}`,
    onRemove: usingGlobalRange ? undefined : resetDateRange,
  })
  if (payee.trim()) {
    chips.push({
      key: 'payee',
      icon: <Search className="h-3 w-3" />,
      label: `"${payee.trim()}"`,
      onRemove: () => setPayee(''),
    })
  }
  if (filterUser) {
    chips.push({
      key: 'user',
      icon: <UserRound className="h-3 w-3" />,
      label: filterUser,
      onRemove: () => setFilterUser(''),
    })
  }

  return (
    <div className="space-y-5">
      <div>
        <h1 className="font-display text-3xl font-semibold tracking-tight">Transactions</h1>
        <p className="text-sm text-muted-foreground">All the comings and goings in your lair.</p>
      </div>

      <div className="space-y-3">
        <div className="flex items-center gap-2">
          <div className="relative flex-1">
            <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              value={payee}
              onChange={(e) => setPayee(e.target.value)}
              placeholder="Search payees…"
              className="pl-9"
              aria-label="Search payees"
            />
            {payee && (
              <button
                type="button"
                aria-label="Clear search"
                onClick={() => setPayee('')}
                className="absolute right-2 top-1/2 flex h-7 w-7 -translate-y-1/2 items-center justify-center rounded-md text-muted-foreground hover:bg-accent hover:text-foreground"
              >
                <X className="h-3.5 w-3.5" />
              </button>
            )}
          </div>
          <Button
            variant="outline"
            onClick={() => setFilterOpen(true)}
            className="shrink-0 gap-1.5"
            aria-label="Open filters"
          >
            <SlidersHorizontal className="h-4 w-4" />
            <span className="hidden sm:inline">Filters</span>
            {activeCustomCount > 0 && (
              <span className="rounded-full bg-primary px-1.5 text-[11px] font-semibold text-primary-foreground">
                {activeCustomCount}
              </span>
            )}
          </Button>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          {chips.map((c) => (
            <FilterChip
              key={c.key}
              icon={c.icon}
              label={c.label}
              onRemove={c.onRemove}
            />
          ))}
          {activeCustomCount > 0 && (
            <button
              type="button"
              onClick={clearFilters}
              className="text-xs font-medium text-muted-foreground underline-offset-4 hover:text-foreground hover:underline"
            >
              Clear all
            </button>
          )}
        </div>
      </div>

      <RegisterView filters={filters} onEdit={openForEdit} />

      <Sheet open={filterOpen} onOpenChange={setFilterOpen}>
        <SheetContent side="right">
          <SheetHeader>
            <SheetTitle>Filters</SheetTitle>
            <SheetDescription>Narrow down the register.</SheetDescription>
          </SheetHeader>
          <SheetBody>
            <div className="space-y-4">
              {members.length > 1 && (
                <div className="space-y-1">
                  <Label htmlFor="f-user" className="text-xs">Posted by</Label>
                  <select
                    id="f-user"
                    value={filterUser}
                    onChange={(e) => setFilterUser(e.target.value)}
                    className="flex h-10 w-full rounded-lg border border-border bg-background px-3 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  >
                    <option value="">Anyone</option>
                    {members.map((m) => (
                      <option key={m} value={m}>
                        {m}
                        {m === user?.username ? ' (you)' : ''}
                      </option>
                    ))}
                  </select>
                </div>
              )}
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <Label className="text-xs">Date range override</Label>
                  {!usingGlobalRange && (
                    <button
                      type="button"
                      onClick={resetDateRange}
                      className="text-[11px] font-medium text-muted-foreground hover:text-foreground"
                    >
                      Use lair default
                    </button>
                  )}
                </div>
                <div className="grid grid-cols-2 gap-2">
                  <div className="space-y-1">
                    <Label htmlFor="f-begin" className="text-[11px] text-muted-foreground">
                      From
                    </Label>
                    <Input
                      id="f-begin"
                      type="date"
                      value={begin ?? range.begin}
                      onChange={(e) => setBegin(e.target.value)}
                    />
                  </div>
                  <div className="space-y-1">
                    <Label htmlFor="f-end" className="text-[11px] text-muted-foreground">
                      To
                    </Label>
                    <Input
                      id="f-end"
                      type="date"
                      value={end ?? range.end}
                      onChange={(e) => setEnd(e.target.value)}
                    />
                  </div>
                </div>
                {usingGlobalRange && (
                  <p className="text-[11px] text-muted-foreground">
                    Inheriting the lair's <span className="font-medium text-foreground">{formatShortDate(range.begin)} – {formatShortDate(range.end)}</span>. Change either date to override.
                  </p>
                )}
              </div>
            </div>
          </SheetBody>
        </SheetContent>
      </Sheet>
    </div>
  )
}

function FilterChip({
  icon,
  label,
  onRemove,
}: {
  icon?: React.ReactNode
  label: string
  onRemove?: () => void
}) {
  return (
    <span className="inline-flex items-center gap-1.5 rounded-full border border-border bg-muted/40 px-2.5 py-1 text-xs font-medium text-foreground">
      {icon && <span className="text-muted-foreground">{icon}</span>}
      <span className="truncate max-w-[12rem]">{label}</span>
      {onRemove && (
        <button
          type="button"
          aria-label={`Remove ${label}`}
          onClick={onRemove}
          className="-mr-1 flex h-4 w-4 items-center justify-center rounded-full text-muted-foreground hover:bg-border hover:text-foreground"
        >
          <X className="h-3 w-3" />
        </button>
      )}
    </span>
  )
}
