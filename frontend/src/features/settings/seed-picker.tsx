import { useState } from 'react'
import { Check, Palette } from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { useSeed } from '@/context/seed-context'
import { useWorkspace } from '@/context/workspace-context'
import { SEED_SWATCHES } from '@/lib/palette'
import { cn } from '@/lib/utils'

export function SeedPicker() {
  const { seed, setSeed, saving } = useSeed()
  const { activeWorkspace } = useWorkspace()
  const [customOpen, setCustomOpen] = useState(false)
  const [customValue, setCustomValue] = useState(seed)

  if (!activeWorkspace) {
    return null
  }

  const isPreset = SEED_SWATCHES.some((s) => s.hex.toLowerCase() === seed.toLowerCase())
  const displayName = activeWorkspace.name

  return (
    <Card>
      <CardHeader>
        <CardTitle>Lair colour</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <p className="text-sm text-muted-foreground">
          Pick a seed and <span className="font-medium text-foreground">{displayName}</span> wears a matching palette for everyone who opens it.
          {saving && <span className="ml-1 italic">Saving…</span>}
        </p>
        <div
          role="radiogroup"
          aria-label="Seed colour"
          className="grid grid-cols-6 gap-3 sm:grid-cols-8"
        >
          {SEED_SWATCHES.map((s) => {
            const selected = seed.toLowerCase() === s.hex.toLowerCase()
            return (
              <button
                key={s.hex}
                type="button"
                role="radio"
                aria-checked={selected}
                aria-label={s.label}
                title={s.label}
                disabled={saving}
                onClick={() => setSeed(s.hex)}
                className={cn(
                  'group flex aspect-square items-center justify-center rounded-full ring-offset-2 ring-offset-background transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50',
                  selected ? 'ring-2 ring-foreground' : 'ring-1 ring-border hover:ring-foreground/40',
                )}
                style={{ backgroundColor: s.hex }}
              >
                {selected && <Check className="h-4 w-4 text-white drop-shadow" />}
              </button>
            )
          })}

          <button
            type="button"
            aria-label="Custom colour"
            title="Custom"
            onClick={() => setCustomOpen((v) => !v)}
            className={cn(
              'group flex aspect-square items-center justify-center rounded-full ring-offset-2 ring-offset-background transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
              !isPreset ? 'ring-2 ring-foreground' : 'ring-1 ring-border hover:ring-foreground/40',
            )}
            style={{
              background:
                'conic-gradient(from 0deg, #ff6b6b, #f59e0b, #facc15, #4ade80, #22d3ee, #3b82f6, #8b5cf6, #ec4899, #ff6b6b)',
            }}
          >
            <Palette className="h-4 w-4 text-white drop-shadow" />
          </button>
        </div>

        {customOpen && (
          <div className="flex items-center gap-3 rounded-lg border border-border p-3">
            <input
              type="color"
              value={customValue}
              onChange={(e) => setCustomValue(e.target.value)}
              className="h-10 w-12 cursor-pointer rounded border border-border bg-transparent"
              aria-label="Colour wheel"
            />
            <code className="flex-1 rounded bg-muted px-2 py-1 text-sm">{customValue.toUpperCase()}</code>
            <button
              type="button"
              disabled={saving || customValue.toLowerCase() === seed.toLowerCase()}
              onClick={() => setSeed(customValue)}
              className="inline-flex h-9 items-center rounded-md bg-primary px-3 text-sm font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
            >
              Apply
            </button>
          </div>
        )}
      </CardContent>
    </Card>
  )
}
