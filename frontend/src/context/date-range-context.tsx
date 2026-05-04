import React, { createContext, useContext, useMemo, useState } from 'react'

export type DateRangePreset =
  | 'this-month'
  | 'last-month'
  | 'last-30d'
  | 'last-90d'
  | 'ytd'
  | 'custom'

export interface DateRange {
  preset: DateRangePreset
  begin: string // YYYY-MM-DD inclusive
  end: string   // YYYY-MM-DD inclusive
}

function iso(d: Date): string {
  return d.toISOString().slice(0, 10)
}

function startOfMonth(d: Date): Date {
  return new Date(d.getFullYear(), d.getMonth(), 1)
}

function endOfMonth(d: Date): Date {
  return new Date(d.getFullYear(), d.getMonth() + 1, 0)
}

function addDays(d: Date, n: number): Date {
  const out = new Date(d)
  out.setDate(out.getDate() + n)
  return out
}

export function resolvePreset(preset: Exclude<DateRangePreset, 'custom'>): { begin: string; end: string } {
  const today = new Date()
  switch (preset) {
    case 'this-month':
      return { begin: iso(startOfMonth(today)), end: iso(endOfMonth(today)) }
    case 'last-month': {
      const firstPrev = new Date(today.getFullYear(), today.getMonth() - 1, 1)
      return { begin: iso(firstPrev), end: iso(endOfMonth(firstPrev)) }
    }
    case 'last-30d':
      return { begin: iso(addDays(today, -29)), end: iso(today) }
    case 'last-90d':
      return { begin: iso(addDays(today, -89)), end: iso(today) }
    case 'ytd':
      return { begin: iso(new Date(today.getFullYear(), 0, 1)), end: iso(today) }
  }
}

export const PRESET_LABELS: Record<DateRangePreset, string> = {
  'this-month': 'This month',
  'last-month': 'Last month',
  'last-30d': 'Last 30 days',
  'last-90d': 'Last 90 days',
  'ytd': 'Year to date',
  'custom': 'Custom',
}

interface DateRangeContextType {
  range: DateRange
  setPreset: (p: Exclude<DateRangePreset, 'custom'>) => void
  setCustom: (begin: string, end: string) => void
}

const DateRangeContext = createContext<DateRangeContextType | undefined>(undefined)

const STORAGE_KEY = 'date_range_v1'

function load(): DateRange {
  try {
    const raw = sessionStorage.getItem(STORAGE_KEY)
    if (raw) {
      const parsed = JSON.parse(raw) as DateRange
      if (parsed?.preset && parsed.begin && parsed.end) return parsed
    }
  } catch {
    // ignore
  }
  const { begin, end } = resolvePreset('this-month')
  return { preset: 'this-month', begin, end }
}

export function DateRangeProvider({ children }: { children: React.ReactNode }) {
  const [range, setRange] = useState<DateRange>(load)

  const value = useMemo<DateRangeContextType>(() => {
    const commit = (r: DateRange) => {
      setRange(r)
      try {
        sessionStorage.setItem(STORAGE_KEY, JSON.stringify(r))
      } catch {
        // ignore
      }
    }
    return {
      range,
      setPreset: (p) => {
        const { begin, end } = resolvePreset(p)
        commit({ preset: p, begin, end })
      },
      setCustom: (begin, end) => {
        commit({ preset: 'custom', begin, end })
      },
    }
  }, [range])

  return <DateRangeContext.Provider value={value}>{children}</DateRangeContext.Provider>
}

export function useDateRange() {
  const ctx = useContext(DateRangeContext)
  if (!ctx) throw new Error('useDateRange must be used within DateRangeProvider')
  return ctx
}
