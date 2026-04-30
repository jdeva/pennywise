import { useMemo, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useWorkspace } from '@/context/workspace-context'
import { useAuth } from '@/context/auth-context'
import { TransactionForm } from './transaction-form'
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
import { Plus, X, SlidersHorizontal } from 'lucide-react'

export function TransactionsPage() {
  const { activeWorkspace } = useWorkspace()
  const { user } = useAuth()
  const queryClient = useQueryClient()
  const [formOpen, setFormOpen] = useState(false)
  const [filterOpen, setFilterOpen] = useState(false)

  const [payee, setPayee] = useState('')
  const [filterUser, setFilterUser] = useState('')
  const [begin, setBegin] = useState('')
  const [end, setEnd] = useState('')

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
      begin: begin || undefined,
      end: end || undefined,
    }),
    [payee, filterUser, begin, end],
  )

  const activeFilterCount = Object.values(filters).filter(Boolean).length

  const clearFilters = () => {
    setPayee('')
    setFilterUser('')
    setBegin('')
    setEnd('')
  }

  const handleSuccess = () => {
    queryClient.invalidateQueries({ queryKey: ['register', activeWorkspace?.id] })
    queryClient.invalidateQueries({ queryKey: ['balance', activeWorkspace?.id] })
    setFormOpen(false)
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

  const fieldsBlock = (
    <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
      <div className="space-y-1">
        <Label htmlFor="f-payee" className="text-xs">Payee</Label>
        <Input
          id="f-payee"
          value={payee}
          onChange={(e) => setPayee(e.target.value)}
          placeholder="Search…"
        />
      </div>
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
      <div className="space-y-1">
        <Label htmlFor="f-begin" className="text-xs">From</Label>
        <Input id="f-begin" type="date" value={begin} onChange={(e) => setBegin(e.target.value)} />
      </div>
      <div className="space-y-1">
        <Label htmlFor="f-end" className="text-xs">To</Label>
        <Input id="f-end" type="date" value={end} onChange={(e) => setEnd(e.target.value)} />
      </div>
    </div>
  )

  return (
    <div className="space-y-5">
      <div className="flex items-end justify-between gap-3">
        <h1 className="font-display text-3xl font-semibold tracking-tight">Transactions</h1>
        <Button onClick={() => setFormOpen(true)} className="gap-1.5">
          <Plus className="h-4 w-4" /> <span className="hidden sm:inline">New transaction</span><span className="sm:hidden">New</span>
        </Button>
      </div>

      {/* Desktop: inline filters */}
      <div className="hidden md:block">
        {fieldsBlock}
        {activeFilterCount > 0 && (
          <div className="mt-3 flex items-center gap-2">
            <Button variant="ghost" size="sm" onClick={clearFilters} className="gap-1.5">
              <X className="h-3.5 w-3.5" /> Clear filters
            </Button>
          </div>
        )}
      </div>

      {/* Mobile: single Filters button */}
      <div className="flex gap-2 md:hidden">
        <Button
          variant="outline"
          size="sm"
          onClick={() => setFilterOpen(true)}
          className="gap-1.5"
        >
          <SlidersHorizontal className="h-3.5 w-3.5" />
          Filters
          {activeFilterCount > 0 && (
            <span className="rounded-full bg-primary px-1.5 text-[11px] font-semibold text-primary-foreground">
              {activeFilterCount}
            </span>
          )}
        </Button>
        {activeFilterCount > 0 && (
          <Button variant="ghost" size="sm" onClick={clearFilters} className="gap-1">
            <X className="h-3.5 w-3.5" /> Clear
          </Button>
        )}
      </div>

      <RegisterView filters={filters} />

      <Sheet open={formOpen} onOpenChange={setFormOpen}>
        <SheetContent side="right">
          <SheetHeader>
            <SheetTitle>New transaction</SheetTitle>
            <SheetDescription>Record an expense, income, or transfer.</SheetDescription>
          </SheetHeader>
          <SheetBody>
            <TransactionForm onSuccess={handleSuccess} onCancel={() => setFormOpen(false)} />
          </SheetBody>
        </SheetContent>
      </Sheet>

      <Sheet open={filterOpen} onOpenChange={setFilterOpen}>
        <SheetContent side="right">
          <SheetHeader>
            <SheetTitle>Filters</SheetTitle>
            <SheetDescription>Narrow down the register.</SheetDescription>
          </SheetHeader>
          <SheetBody>{fieldsBlock}</SheetBody>
        </SheetContent>
      </Sheet>
    </div>
  )
}
