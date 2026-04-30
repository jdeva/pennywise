import { useState } from 'react'
import { useWorkspace } from '@/context/workspace-context'
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
import { Button } from '@/components/ui/button'
import { ChevronsUpDown, Check, Plus } from 'lucide-react'
import { cn } from '@/lib/utils'

const currencies = ['$', '€', '£', '¥', '₹', '₩', 'R$', 'CHF', 'kr', 'zł']

export function WorkspacePicker() {
  const { workspaces, activeWorkspace, setActiveWorkspaceId, createWorkspace, isLoading } = useWorkspace()
  const [createOpen, setCreateOpen] = useState(false)
  const [newName, setNewName] = useState('')
  const [newCurrency, setNewCurrency] = useState('$')
  const [creating, setCreating] = useState(false)

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
    return <div className="h-9 w-full animate-pulse rounded-lg bg-muted" />
  }

  const label = activeWorkspace?.name ?? (workspaces.length === 0 ? 'Create a lair' : 'Select lair')

  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <button
            type="button"
            className="flex h-9 w-full items-center justify-between gap-2 rounded-lg border border-sidebar-border bg-background/50 px-3 text-sm text-sidebar-foreground hover:bg-sidebar-accent"
          >
            <span className="truncate text-left font-medium">{label}</span>
            <ChevronsUpDown className="h-3.5 w-3.5 shrink-0 opacity-50" />
          </button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start" className="w-[15rem]">
          {workspaces.length === 0 ? (
            <DropdownMenuItem disabled>No lairs yet</DropdownMenuItem>
          ) : (
            workspaces.map((ws) => (
              <DropdownMenuItem
                key={ws.id}
                onClick={() => setActiveWorkspaceId(ws.id)}
                className="gap-2"
              >
                <Check className={cn('h-4 w-4', activeWorkspace?.id === ws.id ? 'opacity-100' : 'opacity-0')} />
                <span className="truncate">{ws.name}</span>
              </DropdownMenuItem>
            ))
          )}
          <DropdownMenuSeparator />
          <DropdownMenuItem onClick={() => setCreateOpen(true)} className="gap-2">
            <Plus className="h-4 w-4" />
            New lair
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>

      <Dialog open={createOpen} onOpenChange={setCreateOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create lair</DialogTitle>
          </DialogHeader>
          <div className="space-y-4 py-2">
            <div className="space-y-2">
              <Label htmlFor="ws-name">Name</Label>
              <Input
                id="ws-name"
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                placeholder="Home, Vacation, Side-hustle…"
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    e.preventDefault()
                    handleCreate()
                  }
                }}
                autoFocus
              />
            </div>
            <div className="space-y-2">
              <Label>Currency</Label>
              <div className="flex flex-wrap gap-1.5">
                {currencies.map((c) => (
                  <button
                    key={c}
                    type="button"
                    onClick={() => setNewCurrency(c)}
                    className={cn(
                      'rounded-md border px-2.5 py-1 text-sm transition-colors',
                      newCurrency === c
                        ? 'border-primary bg-primary text-primary-foreground'
                        : 'border-input bg-background hover:bg-accent hover:text-accent-foreground',
                    )}
                  >
                    {c}
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
