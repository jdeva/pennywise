import React, { createContext, useContext, useEffect, useState, useCallback, useMemo } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useWorkspace } from '@/context/workspace-context'
import { workspacesApi } from '@/lib/api/workspaces'
import { applyPalette, DEFAULT_SEED_HEX } from '@/lib/palette'

interface SeedContextType {
  /** Effective seed hex for the active lair — returns DEFAULT_SEED_HEX when unset. */
  seed: string
  /** Persist a new seed to the active lair (server round-trip). */
  setSeed: (hex: string) => Promise<void>
  /** Saving indicator for UI feedback. */
  saving: boolean
}

const SeedContext = createContext<SeedContextType | undefined>(undefined)

function isDarkNow(): boolean {
  return document.documentElement.classList.contains('dark')
}

export function SeedProvider({ children }: { children: React.ReactNode }) {
  const { activeWorkspace } = useWorkspace()
  const queryClient = useQueryClient()
  const [dark, setDark] = useState<boolean>(() => {
    if (typeof document === 'undefined') return false
    return isDarkNow()
  })
  const [saving, setSaving] = useState(false)

  const activeSeed = activeWorkspace?.seed_color || DEFAULT_SEED_HEX

  // React to theme class changes on <html>.
  useEffect(() => {
    const root = document.documentElement
    const obs = new MutationObserver(() => setDark(isDarkNow()))
    obs.observe(root, { attributes: true, attributeFilter: ['class'] })
    return () => obs.disconnect()
  }, [])

  // Apply palette whenever seed or theme flips.
  useEffect(() => {
    applyPalette(activeSeed, dark)
  }, [activeSeed, dark])

  const setSeed = useCallback(
    async (hex: string) => {
      if (!activeWorkspace) return
      setSaving(true)
      try {
        // Optimistically update the cached workspace list so the palette flips instantly.
        queryClient.setQueryData<import('@/lib/types').WorkspacePublic[] | undefined>(
          ['workspaces'],
          (prev) => prev?.map((w) => (w.id === activeWorkspace.id ? { ...w, seed_color: hex } : w)),
        )
        await workspacesApi.update(activeWorkspace.id, activeWorkspace.name, hex)
        await queryClient.invalidateQueries({ queryKey: ['workspaces'] })
      } finally {
        setSaving(false)
      }
    },
    [activeWorkspace, queryClient],
  )

  const value = useMemo(() => ({ seed: activeSeed, setSeed, saving }), [activeSeed, setSeed, saving])

  return <SeedContext.Provider value={value}>{children}</SeedContext.Provider>
}

export function useSeed() {
  const context = useContext(SeedContext)
  if (!context) throw new Error('useSeed must be used within SeedProvider')
  return context
}
