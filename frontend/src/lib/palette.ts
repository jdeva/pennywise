import {
  argbFromHex,
  hexFromArgb,
  Hct,
  SchemeTonalSpot,
} from '@material/material-color-utilities'

/** Swatch grid shown in the seed picker — balanced Material You-friendly seeds. */
export const SEED_SWATCHES = [
  { hex: '#FF6B6B', label: 'Coral' },
  { hex: '#F97373', label: 'Salmon' },
  { hex: '#F59E0B', label: 'Amber' },
  { hex: '#FBBF24', label: 'Honey' },
  { hex: '#FACC15', label: 'Butter' },
  { hex: '#A3E635', label: 'Lime' },
  { hex: '#4ADE80', label: 'Mint' },
  { hex: '#2DD4BF', label: 'Teal' },
  { hex: '#22D3EE', label: 'Cyan' },
  { hex: '#38BDF8', label: 'Sky' },
  { hex: '#3B82F6', label: 'Ocean' },
  { hex: '#6366F1', label: 'Indigo' },
  { hex: '#8B5CF6', label: 'Violet' },
  { hex: '#A78BFA', label: 'Lavender' },
  { hex: '#C084FC', label: 'Lilac' },
  { hex: '#D946EF', label: 'Fuchsia' },
  { hex: '#EC4899', label: 'Pink' },
  { hex: '#F472B6', label: 'Rose' },
  { hex: '#E11D48', label: 'Cherry' },
  { hex: '#DC2626', label: 'Ruby' },
  { hex: '#B45309', label: 'Bronze' },
  { hex: '#78716C', label: 'Stone' },
  { hex: '#64748B', label: 'Slate' },
  { hex: '#0F766E', label: 'Forest' },
] as const

export const DEFAULT_SEED_HEX = '#FF6B6B'

// Tonal key we write for each derived slot. Values are tones 0–100 fed into
// Material's SchemeTonalSpot. We follow the M3 guidance but keep values that
// play well with the existing warm-cream / deep-aubergine base.
// See https://m3.material.io/styles/color/roles for reference.

function hexToArgb(hex: string): number {
  return argbFromHex(hex)
}

/** Convert an ARGB int to HSL H/S/L triplet so we can emit CSS `H S% L%` — matching the existing var format. */
function argbToHsl(argb: number): { h: number; s: number; l: number } {
  const hex = hexFromArgb(argb)
  const r = parseInt(hex.slice(1, 3), 16) / 255
  const g = parseInt(hex.slice(3, 5), 16) / 255
  const b = parseInt(hex.slice(5, 7), 16) / 255
  const max = Math.max(r, g, b)
  const min = Math.min(r, g, b)
  const l = (max + min) / 2
  let h = 0
  let s = 0
  if (max !== min) {
    const d = max - min
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min)
    switch (max) {
      case r:
        h = (g - b) / d + (g < b ? 6 : 0)
        break
      case g:
        h = (b - r) / d + 2
        break
      case b:
        h = (r - g) / d + 4
        break
    }
    h *= 60
  }
  return { h: Math.round(h), s: Math.round(s * 100), l: Math.round(l * 100) }
}

function fmt(argb: number): string {
  const { h, s, l } = argbToHsl(argb)
  return `${h} ${s}% ${l}%`
}

export interface PaletteVars {
  [key: string]: string
}

/**
 * Derive a coherent palette from one seed colour and return CSS variable
 * values (HSL components, matching existing shadcn-style tokens).
 *
 * We map Material 3's SchemeTonalSpot onto the app's existing var names so
 * every existing Tailwind class keeps working.
 */
export function palette(seedHex: string, isDark: boolean): PaletteVars {
  const source = Hct.fromInt(hexToArgb(seedHex))
  const scheme = new SchemeTonalSpot(source, isDark, 0)

  return {
    '--primary': fmt(scheme.primary),
    '--primary-foreground': fmt(scheme.onPrimary),
    '--ring': fmt(scheme.primary),
    '--accent': fmt(scheme.secondaryContainer),
    '--accent-foreground': fmt(scheme.onSecondaryContainer),
    '--secondary': fmt(scheme.secondaryContainer),
    '--secondary-foreground': fmt(scheme.onSecondaryContainer),
  }
}

/** Apply palette vars to :root. Pass `null` to clear and fall back to app defaults. */
export function applyPalette(seedHex: string | null | undefined, isDark: boolean) {
  const root = document.documentElement
  const vars = ['--primary', '--primary-foreground', '--ring', '--accent', '--accent-foreground', '--secondary', '--secondary-foreground']
  if (!seedHex) {
    for (const v of vars) root.style.removeProperty(v)
    return
  }
  const p = palette(seedHex, isDark)
  for (const [k, v] of Object.entries(p)) {
    root.style.setProperty(k, v)
  }
}
