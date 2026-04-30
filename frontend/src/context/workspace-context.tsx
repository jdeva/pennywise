import React, { createContext, useContext, useState, useEffect, useCallback, useMemo } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import type { WorkspacePublic } from '@/lib/types'
import { workspacesApi } from '@/lib/api/workspaces'
import { useAuth } from '@/context/auth-context'

interface WorkspaceContextType {
  workspaces: WorkspacePublic[]
  activeWorkspace: WorkspacePublic | null
  setActiveWorkspaceId: (id: string) => void
  createWorkspace: (name: string, currency?: string) => Promise<void>
  isLoading: boolean
}

const WorkspaceContext = createContext<WorkspaceContextType | undefined>(undefined)

export function WorkspaceProvider({ children }: { children: React.ReactNode }) {
  const { isAuthenticated } = useAuth()
  const queryClient = useQueryClient()
  const [activeId, setActiveId] = useState<string | null>(
    () => localStorage.getItem('active_workspace_id'),
  )

  const { data: workspaces = [], isLoading } = useQuery({
    queryKey: ['workspaces'],
    queryFn: async () => {
      const { data } = await workspacesApi.list()
      return data
    },
    enabled: isAuthenticated,
  })

  const activeWorkspace = useMemo(
    () => workspaces.find((w) => w.id === activeId) ?? workspaces[0] ?? null,
    [workspaces, activeId],
  )

  // Sync activeId when workspaces load and current selection is invalid
  useEffect(() => {
    if (workspaces.length > 0 && !workspaces.find((w) => w.id === activeId)) {
      setActiveId(workspaces[0].id)
      localStorage.setItem('active_workspace_id', workspaces[0].id)
    }
  }, [workspaces, activeId])

  const setActiveWorkspaceId = useCallback(
    (id: string) => {
      setActiveId(id)
      localStorage.setItem('active_workspace_id', id)
      // Invalidate workspace-scoped queries
      queryClient.invalidateQueries({ queryKey: ['balance'] })
      queryClient.invalidateQueries({ queryKey: ['register'] })
      queryClient.invalidateQueries({ queryKey: ['budgets'] })
      queryClient.invalidateQueries({ queryKey: ['budgeting'] })
    },
    [queryClient],
  )

  const createWorkspace = useCallback(
    async (name: string, currency?: string) => {
      const { data } = await workspacesApi.create(name, currency)
      await queryClient.invalidateQueries({ queryKey: ['workspaces'] })
      setActiveWorkspaceId(data.id)
    },
    [queryClient, setActiveWorkspaceId],
  )

  return (
    <WorkspaceContext.Provider
      value={{ workspaces, activeWorkspace, setActiveWorkspaceId, createWorkspace, isLoading }}
    >
      {children}
    </WorkspaceContext.Provider>
  )
}

export function useWorkspace() {
  const context = useContext(WorkspaceContext)
  if (!context) throw new Error('useWorkspace must be used within WorkspaceProvider')
  return context
}
