import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { useAdvancedMode } from '@/lib/use-advanced-mode'
import { cn } from '@/lib/utils'

export function AdvancedPanel() {
  const [enabled, setEnabled] = useAdvancedMode()
  const toggle = () => setEnabled(!enabled)

  return (
    <Card>
      <CardHeader>
        <CardTitle>Advanced</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="flex items-start justify-between gap-4">
          <div className="space-y-1">
            <div className="font-medium">Ledger file editor</div>
            <p className="text-sm text-muted-foreground">
              Edit raw <code className="font-mono text-xs">.ledger</code> files for lairs you own. Adds a{' '}
              <span className="font-medium">Ledger files</span> item to the sidebar. For advanced users —
              malformed ledger syntax breaks balances until fixed.
            </p>
          </div>
          <button
            type="button"
            role="switch"
            aria-checked={enabled}
            aria-label="Enable ledger file editor"
            onClick={toggle}
            className={cn(
              'relative h-6 w-11 shrink-0 rounded-full transition-colors',
              enabled ? 'bg-primary' : 'bg-muted',
            )}
          >
            <span
              className={cn(
                'absolute top-0.5 h-5 w-5 rounded-full bg-background shadow transition-transform',
                enabled ? 'translate-x-5' : 'translate-x-0.5',
              )}
            />
          </button>
        </div>
      </CardContent>
    </Card>
  )
}
