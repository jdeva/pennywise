import { useState } from 'react'
import { useWorkspace } from '@/context/workspace-context'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { ChevronsUpDown, Check, Plus } from 'lucide-react'
import { cn } from '@/lib/utils'

export function WorkspaceDropdown() {
  const { workspaces, activeWorkspace, setActiveWorkspaceId, createWorkspace, isLoading } = useWorkspace()
  const [createOpen, setCreateOpen] = useState(false)
  const [newName, setNewName] = useState('')
  const [newCurrency, setNewCurrency] = useState('$')
  const [creating, setCreating] = useState(false)

  const currencies = [
    { symbol: '$', label: '$' },
    { symbol: '€', label: '€' },
    { symbol: '£', label: '£' },
    { symbol: '¥', label: '¥' },
    { symbol: '₹', label: '₹' },
    { symbol: '₩', label: '₩' },
    { symbol: 'R$', label: 'R$' },
    { symbol: 'CHF', label: 'CHF' },
    { symbol: 'kr', label: 'kr' },
    { symbol: 'zł', label: 'zł' },
  ]

  const handleCreate = async () => {
    if (!newName.trim()) return
    setCreating(true)
    try {
      await createWorkspace(newName.trim(), newCurrency)
      setNewName('')
      setNewCurrency('$')
      setCreateOpen(false)
    } finally {
      setCreating(false)
    }
  }

  if (isLoading) {
    return <div className="h-9 w-40 animate-pulse rounded-md bg-muted" />
  }

  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="outline" size="sm" className="gap-2">
            <span className="max-w-[150px] truncate">
              {activeWorkspace?.name ?? 'Select workspace'}
            </span>
            <ChevronsUpDown className="h-4 w-4 opacity-50" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" className="w-56">
          {workspaces.length === 0 ? (
            <DropdownMenuItem disabled>No workspaces</DropdownMenuItem>
          ) : (
            workspaces.map((ws) => (
              <DropdownMenuItem
                key={ws.id}
                onClick={() => setActiveWorkspaceId(ws.id)}
                className="gap-2"
              >
                <Check
                  className={cn(
                    'h-4 w-4',
                    activeWorkspace?.id === ws.id ? 'opacity-100' : 'opacity-0',
                  )}
                />
                <span className="truncate">{ws.name}</span>
              </DropdownMenuItem>
            ))
          )}
          <DropdownMenuSeparator />
          <DropdownMenuItem onClick={() => setCreateOpen(true)} className="gap-2">
            <Plus className="h-4 w-4" />
            New workspace
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>

      <Dialog open={createOpen} onOpenChange={setCreateOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create Workspace</DialogTitle>
          </DialogHeader>
          <div className="space-y-4 py-2">
            <div className="space-y-2">
              <Label htmlFor="ws-name">Workspace name</Label>
              <Input
                id="ws-name"
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                placeholder="My workspace"
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    e.preventDefault()
                    handleCreate()
                  }
                }}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="ws-currency">Currency</Label>
              <div className="flex flex-wrap gap-2">
                {currencies.map((c) => (
                  <button
                    key={c.symbol}
                    type="button"
                    onClick={() => setNewCurrency(c.symbol)}
                    className={cn(
                      'rounded-md border px-3 py-1.5 text-sm transition-colors',
                      newCurrency === c.symbol
                        ? 'border-primary bg-primary text-primary-foreground'
                        : 'border-input bg-background hover:bg-accent hover:text-accent-foreground',
                    )}
                  >
                    {c.label}
                  </button>
                ))}
              </div>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setCreateOpen(false)}>
              Cancel
            </Button>
            <Button onClick={handleCreate} disabled={creating || !newName.trim()}>
              {creating ? 'Creating…' : 'Create'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  )
}
