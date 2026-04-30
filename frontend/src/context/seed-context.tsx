import React, { createContext, useContext, useEffect, useState } from 'react'

export const SEEDS = ['coral', 'ocean', 'emerald', 'plum', 'butter'] as const
export type Seed = (typeof SEEDS)[number]

export const SEED_LABELS: Record<Seed, string> = {
  coral: 'Coral',
  ocean: 'Ocean',
  emerald: 'Emerald',
  plum: 'Plum',
  butter: 'Butter',
}

// Swatch shown in the picker — roughly matches the light-mode --primary for that seed.
export const SEED_SWATCHES: Record<Seed, string> = {
  coral: 'hsl(2 80% 65%)',
  ocean: 'hsl(199 75% 52%)',
  emerald: 'hsl(158 55% 45%)',
  plum: 'hsl(278 50% 58%)',
  butter: 'hsl(36 88% 55%)',
}

const DEFAULT_SEED: Seed = 'coral'
const STORAGE_KEY = 'seed'

function isSeed(v: unknown): v is Seed {
  return typeof v === 'string' && (SEEDS as readonly string[]).includes(v)
}

interface SeedContextType {
  seed: Seed
  setSeed: (seed: Seed) => void
}

const SeedContext = createContext<SeedContextType | undefined>(undefined)

export function SeedProvider({ children }: { children: React.ReactNode }) {
  const [seed, setSeed] = useState<Seed>(() => {
    const stored = localStorage.getItem(STORAGE_KEY)
    return isSeed(stored) ? stored : DEFAULT_SEED
  })

  useEffect(() => {
    document.documentElement.setAttribute('data-seed', seed)
    localStorage.setItem(STORAGE_KEY, seed)
  }, [seed])

  return <SeedContext.Provider value={{ seed, setSeed }}>{children}</SeedContext.Provider>
}

export function useSeed() {
  const context = useContext(SeedContext)
  if (!context) throw new Error('useSeed must be used within SeedProvider')
  return context
}
