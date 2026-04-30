import { useWorkspace } from '@/context/workspace-context'
import { cn } from '@/lib/utils'

export function WorkspaceSelector() {
  const { workspaces, activeWorkspace, setActiveWorkspaceId, isLoading } = useWorkspace()

  if (isLoading) {
    return (
      <div className="h-9 w-48 animate-pulse rounded-md bg-muted" />
    )
  }

  if (workspaces.length === 0) {
    return (
      <span className="text-sm text-muted-foreground">No workspaces</span>
    )
  }

  return (
    <select
      value={activeWorkspace?.id ?? ''}
      onChange={(e) => setActiveWorkspaceId(e.target.value)}
      className={cn(
        'h-9 rounded-md border border-input bg-background px-3 text-sm',
        'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
      )}
      aria-label="Select workspace"
    >
      {workspaces.map((ws) => (
        <option key={ws.id} value={ws.id}>
          {ws.name}
        </option>
      ))}
    </select>
  )
}
