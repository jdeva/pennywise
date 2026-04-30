import { Check } from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { SEEDS, SEED_LABELS, SEED_SWATCHES, useSeed } from '@/context/seed-context'
import { cn } from '@/lib/utils'

export function SeedPicker() {
  const { seed, setSeed } = useSeed()

  return (
    <Card>
      <CardHeader>
        <CardTitle>Accent colour</CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        <p className="text-sm text-muted-foreground">
          Pick the shade your lair wears.
        </p>
        <div
          role="radiogroup"
          aria-label="Accent colour"
          className="flex flex-wrap gap-3"
        >
          {SEEDS.map((s) => {
            const selected = seed === s
            return (
              <button
                key={s}
                type="button"
                role="radio"
                aria-checked={selected}
                aria-label={SEED_LABELS[s]}
                onClick={() => setSeed(s)}
                className={cn(
                  'group flex flex-col items-center gap-1.5 focus:outline-none',
                )}
              >
                <span
                  className={cn(
                    'flex h-11 w-11 items-center justify-center rounded-full ring-offset-2 ring-offset-background transition-all group-focus-visible:ring-2 group-focus-visible:ring-ring',
                    selected ? 'ring-2 ring-foreground' : 'ring-1 ring-border',
                  )}
                  style={{ backgroundColor: SEED_SWATCHES[s] }}
                >
                  {selected && <Check className="h-5 w-5 text-white drop-shadow" />}
                </span>
                <span className="text-xs text-muted-foreground">
                  {SEED_LABELS[s]}
                </span>
              </button>
            )
          })}
        </div>
      </CardContent>
    </Card>
  )
}
