import { useState } from 'react'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Button } from '@/components/ui/button'
import { Calendar, Check } from 'lucide-react'
import {
  PRESET_LABELS,
  useDateRange,
  type DateRangePreset,
} from '@/context/date-range-context'
import { cn } from '@/lib/utils'

const PRESETS: Exclude<DateRangePreset, 'custom'>[] = [
  'this-month',
  'last-month',
  'last-30d',
  'last-90d',
  'ytd',
]

function formatShort(iso: string): string {
  const [y, m, d] = iso.split('-')
  return `${d}/${m}/${y.slice(2)}`
}

export function DateRangePill() {
  const { range, setPreset, setCustom } = useDateRange()
  const [open, setOpen] = useState(false)
  const [customOpen, setCustomOpen] = useState(false)
  const [customBegin, setCustomBegin] = useState(range.begin)
  const [customEnd, setCustomEnd] = useState(range.end)

  const label =
    range.preset === 'custom'
      ? `${formatShort(range.begin)} – ${formatShort(range.end)}`
      : PRESET_LABELS[range.preset]

  const applyCustom = () => {
    if (!customBegin || !customEnd) return
    setCustom(customBegin, customEnd)
    setCustomOpen(false)
    setOpen(false)
  }

  return (
    <DropdownMenu open={open} onOpenChange={setOpen}>
      <DropdownMenuTrigger asChild>
        <button
          type="button"
          className="flex h-9 items-center gap-2 rounded-full border border-border bg-background px-3 text-sm font-medium text-foreground hover:bg-accent"
          aria-label={`Date range: ${label}`}
        >
          <Calendar className="h-3.5 w-3.5 text-muted-foreground" />
          <span className="truncate max-w-[10rem]">{label}</span>
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-56">
        {PRESETS.map((p) => (
          <DropdownMenuItem
            key={p}
            onSelect={(e) => {
              e.preventDefault()
              setPreset(p)
              setOpen(false)
            }}
            className="gap-2"
          >
            <Check
              className={cn('h-4 w-4', range.preset === p ? 'opacity-100' : 'opacity-0')}
            />
            {PRESET_LABELS[p]}
          </DropdownMenuItem>
        ))}
        <DropdownMenuSeparator />
        <DropdownMenuItem
          onSelect={(e) => {
            e.preventDefault()
            setCustomOpen((v) => !v)
          }}
          className="gap-2"
        >
          <Check
            className={cn('h-4 w-4', range.preset === 'custom' ? 'opacity-100' : 'opacity-0')}
          />
          Custom…
        </DropdownMenuItem>
        {customOpen && (
          <div className="space-y-2 border-t border-border px-2 py-3">
            <div className="space-y-1">
              <Label htmlFor="dr-begin" className="text-xs">From</Label>
              <Input
                id="dr-begin"
                type="date"
                value={customBegin}
                onChange={(e) => setCustomBegin(e.target.value)}
              />
            </div>
            <div className="space-y-1">
              <Label htmlFor="dr-end" className="text-xs">To</Label>
              <Input
                id="dr-end"
                type="date"
                value={customEnd}
                onChange={(e) => setCustomEnd(e.target.value)}
              />
            </div>
            <Button size="sm" className="w-full" onClick={applyCustom}>
              Apply
            </Button>
          </div>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
