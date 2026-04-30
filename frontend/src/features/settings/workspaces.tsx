import { useState } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { workspacesApi } from '@/lib/api/workspaces'
import { useWorkspace } from '@/context/workspace-context'
import { useAuth } from '@/context/auth-context'
import type { WorkspacePublic } from '@/lib/types'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { ChevronDown, ChevronRight, Trash2, UserMinus, Power } from 'lucide-react'

function WorkspaceDetail({ workspace }: { workspace: WorkspacePublic }) {
  const { user } = useAuth()
  const queryClient = useQueryClient()
  const isOwner = user?.id === workspace.owner_id

  // Name edit state
  const [editName, setEditName] = useState(workspace.name)
  const [nameMsg, setNameMsg] = useState<string | null>(null)
  const [nameError, setNameError] = useState<string | null>(null)

  // Share form state
  const [shareUsername, setShareUsername] = useState('')
  const [sharePermission, setSharePermission] = useState<'read' | 'write'>('read')
  const [shareError, setShareError] = useState<string | null>(null)

  // Deactivate state
  const [deactivateError, setDeactivateError] = useState<string | null>(null)

  const updateMutation = useMutation({
    mutationFn: () => workspacesApi.update(workspace.id, editName.trim()),
    onSuccess: () => {
      setNameMsg('Name updated.')
      setNameError(null)
      queryClient.invalidateQueries({ queryKey: ['workspaces'] })
    },
    onError: (err: any) => {
      setNameMsg(null)
      setNameError(err?.response?.data?.error || 'Failed to update name')
    },
  })

  const shareMutation = useMutation({
    mutationFn: () =>
      workspacesApi.share(workspace.id, shareUsername.trim(), sharePermission),
    onSuccess: () => {
      setShareUsername('')
      setSharePermission('read')
      setShareError(null)
      queryClient.invalidateQueries({ queryKey: ['workspaces'] })
    },
    onError: (err: any) => {
      setShareError(err?.response?.data?.error || 'Failed to invite user')
    },
  })

  const unshareMutation = useMutation({
    mutationFn: (userId: string) => workspacesApi.unshare(workspace.id, userId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['workspaces'] })
    },
  })

  const deactivateMutation = useMutation({
    mutationFn: () => workspacesApi.deactivate(workspace.id),
    onSuccess: () => {
      setDeactivateError(null)
      queryClient.invalidateQueries({ queryKey: ['workspaces'] })
    },
    onError: (err: any) => {
      setDeactivateError(
        err?.response?.data?.error || 'Failed to deactivate lair'
      )
    },
  })

  const handleDeactivate = () => {
    if (!confirm('Deactivate this lair? This cannot be undone.')) return
    deactivateMutation.mutate()
  }

  return (
    <div className="space-y-4 border-t pt-4">
      {/* Name Edit */}
      <div className="space-y-2">
        <Label htmlFor={`name-${workspace.id}`}>Lair name</Label>
        <form
          className="flex gap-2"
          onSubmit={(e) => {
            e.preventDefault()
            if (editName.trim() && editName.trim() !== workspace.name) {
              updateMutation.mutate()
            }
          }}
        >
          <Input
            id={`name-${workspace.id}`}
            value={editName}
            onChange={(e) => setEditName(e.target.value)}
            className="flex-1"
          />
          <Button
            type="submit"
            size="sm"
            disabled={
              updateMutation.isPending ||
              !editName.trim() ||
              editName.trim() === workspace.name
            }
          >
            Save
          </Button>
        </form>
        {nameMsg && (
          <p className="text-sm text-green-700">{nameMsg}</p>
        )}
        {nameError && (
          <p role="alert" className="text-sm text-destructive">{nameError}</p>
        )}
      </div>

      {/* Shared Users */}
      <div className="space-y-2">
        <Label>Roommates</Label>
        {workspace.shared_with.length === 0 ? (
          <p className="text-sm text-muted-foreground">You're alone in this lair — invite someone below.</p>
        ) : (
          <ul className="space-y-1">
            {workspace.shared_with.map((su) => (
              <li
                key={su.user_id}
                className="flex items-center justify-between rounded px-2 py-1 hover:bg-muted"
              >
                <span className="text-sm">
                  <span className="font-medium">{su.username || su.user_id}</span>{' '}
                  <span className="text-muted-foreground">({su.permission})</span>
                </span>
                {isOwner && (
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-7 w-7"
                    onClick={() => unshareMutation.mutate(su.user_id)}
                    disabled={unshareMutation.isPending}
                    aria-label={`Unshare from ${su.username || su.user_id}`}
                  >
                    <UserMinus className="h-4 w-4" />
                  </Button>
                )}
              </li>
            ))}
          </ul>
        )}
      </div>

      {/* Share Form */}
      {isOwner && (
        <div className="space-y-2">
          <Label>Invite into this lair</Label>
          {shareError && (
            <div role="alert" className="rounded-md bg-destructive/10 p-2 text-sm text-destructive">
              {shareError}
            </div>
          )}
          <form
            className="flex gap-2"
            onSubmit={(e) => {
              e.preventDefault()
              if (shareUsername.trim()) shareMutation.mutate()
            }}
          >
            <Input
              value={shareUsername}
              onChange={(e) => setShareUsername(e.target.value)}
              placeholder="Their username"
              className="flex-1"
            />
            <select
              value={sharePermission}
              onChange={(e) => setSharePermission(e.target.value as 'read' | 'write')}
              className="rounded-md border border-input bg-background px-3 py-2 text-sm"
              aria-label="Permission level"
            >
              <option value="read">Read</option>
              <option value="write">Write</option>
            </select>
            <Button
              type="submit"
              size="sm"
              disabled={shareMutation.isPending || !shareUsername.trim()}
            >
              Invite
            </Button>
          </form>
        </div>
      )}

      {/* Deactivate */}
      {isOwner && workspace.is_active && (
        <div className="space-y-2 border-t pt-4">
          {deactivateError && (
            <div role="alert" className="rounded-md bg-destructive/10 p-2 text-sm text-destructive">
              {deactivateError}
            </div>
          )}
          <Button
            variant="destructive"
            size="sm"
            onClick={handleDeactivate}
            disabled={deactivateMutation.isPending}
          >
            <Power className="mr-2 h-4 w-4" />
            Deactivate lair
          </Button>
        </div>
      )}
    </div>
  )
}

function WorkspaceItem({ workspace }: { workspace: WorkspacePublic }) {
  const { user } = useAuth()
  const [expanded, setExpanded] = useState(false)
  const isOwner = user?.id === workspace.owner_id

  return (
    <Card>
      <CardContent className="py-4">
        <button
          type="button"
          className="flex w-full items-center justify-between text-left"
          onClick={() => setExpanded(!expanded)}
          aria-expanded={expanded}
        >
          <div className="flex items-center gap-3">
            {expanded ? (
              <ChevronDown className="h-4 w-4 text-muted-foreground" />
            ) : (
              <ChevronRight className="h-4 w-4 text-muted-foreground" />
            )}
            <div>
              <span className="font-medium">{workspace.name}</span>
              <div className="flex gap-2 text-xs text-muted-foreground">
                <span>{workspace.currency}</span>
                <span>·</span>
                <span>{isOwner ? 'Owner' : 'Shared'}</span>
                <span>·</span>
                <span className={workspace.is_active ? 'text-green-600' : 'text-destructive'}>
                  {workspace.is_active ? 'Active' : 'Deactivated'}
                </span>
              </div>
            </div>
          </div>
        </button>
        {expanded && <WorkspaceDetail workspace={workspace} />}
      </CardContent>
    </Card>
  )
}

export function WorkspacesTab() {
  const { workspaces, isLoading } = useWorkspace()

  if (isLoading) {
    return (
      <div className="max-w-2xl space-y-3">
        {[1, 2].map((i) => (
          <div key={i} className="h-16 animate-pulse rounded bg-muted" />
        ))}
      </div>
    )
  }

  if (workspaces.length === 0) {
    return (
      <div className="max-w-2xl">
        <Card>
          <CardContent className="py-6">
            <p className="text-sm text-muted-foreground">
              No lairs yet. Create one from the lair switcher in the sidebar.
            </p>
          </CardContent>
        </Card>
      </div>
    )
  }

  return (
    <div className="max-w-2xl space-y-3">
      {workspaces.map((ws) => (
        <WorkspaceItem key={ws.id} workspace={ws} />
      ))}
    </div>
  )
}
