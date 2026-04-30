import { cn } from '@/lib/utils'

export type PillColor = 'mint' | 'butter' | 'lavender' | 'sky' | 'rose' | 'peach' | 'coral' | 'neutral'

// Map each account type / semantic category to a stable accent color.
const ACCOUNT_TYPE_COLOR: Record<string, PillColor> = {
  expenses: 'rose',
  income: 'mint',
  assets: 'sky',
  liabilities: 'peach',
  equity: 'lavender',
}

const COLOR_STYLES: Record<PillColor, string> = {
  mint: 'bg-accent-mint/15 text-accent-mint dark:bg-accent-mint/20 dark:text-accent-mint ring-accent-mint/30',
  butter: 'bg-accent-butter/20 text-[hsl(35_80%_35%)] dark:text-accent-butter ring-accent-butter/40',
  lavender: 'bg-accent-lavender/15 text-accent-lavender dark:bg-accent-lavender/20 ring-accent-lavender/30',
  sky: 'bg-accent-sky/15 text-accent-sky dark:bg-accent-sky/20 ring-accent-sky/30',
  rose: 'bg-accent-rose/15 text-accent-rose dark:bg-accent-rose/20 ring-accent-rose/30',
  peach: 'bg-accent-peach/20 text-[hsl(18_70%_40%)] dark:text-accent-peach ring-accent-peach/40',
  coral: 'bg-primary/12 text-primary ring-primary/30',
  neutral: 'bg-muted text-muted-foreground ring-border',
}

interface PillProps {
  children: React.ReactNode
  color?: PillColor
  /** If the value looks like `Expenses:Food:Groceries`, auto-picks a color from the first segment. */
  account?: string
  className?: string
}

/** Pill for labels and categories — color encodes semantic type. */
export function Pill({ children, color, account, className }: PillProps) {
  const resolved: PillColor =
    color ?? (account ? ACCOUNT_TYPE_COLOR[account.split(':')[0]?.toLowerCase()] ?? 'neutral' : 'neutral')
  return (
    <span
      className={cn(
        'inline-flex items-center gap-1 rounded-full px-2.5 py-0.5 text-xs font-medium ring-1 ring-inset',
        COLOR_STYLES[resolved],
        className,
      )}
    >
      {children}
    </span>
  )
}
