import { useState } from 'react'
import { Link, useRouterState } from '@tanstack/react-router'
import { useAuth } from '@/context/auth-context'
import { useTheme } from '@/context/theme-context'
import { useTxForm } from '@/context/tx-form-context'
import { WorkspacePicker } from '@/components/workspace-picker'
import { DateRangePill } from '@/components/date-range-pill'
import { TxFormSheet } from '@/components/tx-form-sheet'
import { useAdvancedMode } from '@/lib/use-advanced-mode'
import { cn } from '@/lib/utils'
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetBody,
  SheetTitle,
} from '@/components/ui/sheet'
import {
  LayoutDashboard,
  ArrowLeftRight,
  PiggyBank,
  BarChart3,
  Repeat,
  Settings,
  FileCode2,
  LogOut,
  Sun,
  Moon,
  Plus,
  MoreHorizontal,
} from 'lucide-react'

type NavItem = {
  to: string
  label: string
  icon: React.ComponentType<{ className?: string }>
}

const primaryNav: NavItem[] = [
  { to: '/', label: 'Dashboard', icon: LayoutDashboard },
  { to: '/transactions', label: 'Transactions', icon: ArrowLeftRight },
  { to: '/budgets', label: 'Budgets', icon: PiggyBank },
  { to: '/reports', label: 'Reports', icon: BarChart3 },
  { to: '/recurring', label: 'Recurring', icon: Repeat },
]

const secondaryNav: NavItem[] = [
  { to: '/settings', label: 'Settings', icon: Settings },
]

const ledgerFilesItem: NavItem = {
  to: '/ledger-files',
  label: 'Ledger files',
  icon: FileCode2,
}

// Mobile bottom tabs: 4 items around a centre "+" FAB. The rest go into "More".
const mobileTabLeft: NavItem[] = [
  { to: '/', label: 'Home', icon: LayoutDashboard },
  { to: '/transactions', label: 'Transactions', icon: ArrowLeftRight },
]
const mobileTabRight: NavItem[] = [
  { to: '/budgets', label: 'Budgets', icon: PiggyBank },
  { to: '/reports', label: 'Reports', icon: BarChart3 },
]

export function AppLayout({ children }: { children: React.ReactNode }) {
  const { user, logout } = useAuth()
  const { theme, setTheme } = useTheme()
  const { openForNew } = useTxForm()
  const [advanced] = useAdvancedMode()
  const [moreOpen, setMoreOpen] = useState(false)
  const router = useRouterState()
  const currentPath = router.location.pathname

  const desktopMain: NavItem[] = advanced ? [...primaryNav] : primaryNav
  const desktopSecondary: NavItem[] = advanced
    ? [...secondaryNav, ledgerFilesItem]
    : secondaryNav

  const moreItems: NavItem[] = advanced
    ? [{ to: '/recurring', label: 'Recurring', icon: Repeat }, ...secondaryNav, ledgerFilesItem]
    : [{ to: '/recurring', label: 'Recurring', icon: Repeat }, ...secondaryNav]

  const isActive = (to: string) => (to === '/' ? currentPath === '/' : currentPath.startsWith(to))

  const handleNewTx = () => {
    openForNew()
  }

  const NavLink = ({ item, compact }: { item: NavItem; compact?: boolean }) => {
    const Icon = item.icon
    const active = isActive(item.to)
    return (
      <Link
        to={item.to}
        className={cn(
          'flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition-colors',
          compact && 'justify-center px-2',
          active
            ? 'bg-primary/10 font-medium text-primary'
            : 'text-sidebar-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground',
        )}
      >
        <Icon className="h-4 w-4 shrink-0" />
        {!compact && item.label}
      </Link>
    )
  }

  return (
    <div className="flex min-h-screen flex-col bg-background md:flex-row">
      {/* Mobile top bar */}
      <header className="sticky top-0 z-20 flex items-center gap-2 border-b border-border bg-background/95 px-3 py-2.5 backdrop-blur md:hidden">
        <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-primary text-primary-foreground">
          <span className="font-display text-base font-semibold">P</span>
        </div>
        <div className="min-w-0 flex-1">
          <WorkspacePicker />
        </div>
        <DateRangePill />
        <button
          onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}
          aria-label="Toggle theme"
          className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md text-foreground hover:bg-accent"
        >
          {theme === 'dark' ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
        </button>
      </header>

      {/* Desktop sidebar */}
      <aside className="hidden w-64 shrink-0 flex-col border-r border-sidebar-border bg-sidebar md:flex">
        <div className="flex items-center gap-2 px-5 pt-5 pb-3">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary text-primary-foreground">
            <span className="font-display text-base font-semibold">P</span>
          </div>
          <span className="font-display text-lg font-semibold tracking-tight text-sidebar-foreground">
            Pennywise
          </span>
        </div>

        <div className="px-3 pb-3">
          <WorkspacePicker />
        </div>

        <nav className="flex-1 space-y-0.5 px-3">
          {desktopMain.map((item) => (
            <NavLink key={item.to} item={item} />
          ))}

          <div className="pt-4" />
          <p className="px-3 pb-1 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
            Workspace
          </p>
          {desktopSecondary.map((item) => (
            <NavLink key={item.to} item={item} />
          ))}
        </nav>

        <div className="border-t border-sidebar-border px-3 py-3">
          <div className="flex items-center gap-2 px-2 py-1.5">
            <div className="flex h-7 w-7 items-center justify-center rounded-full bg-primary/15 text-xs font-semibold text-primary">
              {user?.username?.[0]?.toUpperCase() ?? '?'}
            </div>
            <span className="flex-1 truncate text-sm text-sidebar-foreground">{user?.username}</span>
            <button
              onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}
              aria-label="Toggle theme"
              className="flex h-7 w-7 items-center justify-center rounded-md text-sidebar-foreground hover:bg-sidebar-accent"
            >
              {theme === 'dark' ? <Sun className="h-3.5 w-3.5" /> : <Moon className="h-3.5 w-3.5" />}
            </button>
            <button
              onClick={logout}
              aria-label="Log out"
              className="flex h-7 w-7 items-center justify-center rounded-md text-sidebar-foreground hover:bg-sidebar-accent"
            >
              <LogOut className="h-3.5 w-3.5" />
            </button>
          </div>
        </div>
      </aside>

      {/* Main column */}
      <div className="flex min-w-0 flex-1 flex-col">
        {/* Desktop top bar */}
        <div className="sticky top-0 z-10 hidden items-center justify-end gap-3 border-b border-border bg-background/80 px-6 py-3 backdrop-blur md:flex lg:px-10">
          <DateRangePill />
          <button
            onClick={handleNewTx}
            className="inline-flex h-9 items-center gap-1.5 rounded-full bg-primary px-4 text-sm font-medium text-primary-foreground shadow-soft hover:bg-primary/90 transition-colors"
          >
            <Plus className="h-4 w-4" />
            New transaction
          </button>
        </div>

        {/* Main content */}
        <main className="flex-1 overflow-x-hidden pb-24 md:pb-0">
          <div className="px-4 py-5 sm:px-6 md:py-8 lg:px-10 lg:py-10">
            {children}
          </div>
        </main>
      </div>

      {/* Mobile bottom tab bar with centre FAB */}
      <nav className="fixed inset-x-0 bottom-0 z-20 flex items-stretch border-t border-border bg-background/95 pb-[env(safe-area-inset-bottom)] backdrop-blur md:hidden">
        {mobileTabLeft.map(({ to, label, icon: Icon }) => (
          <Link
            key={to}
            to={to}
            className={cn(
              'flex flex-1 flex-col items-center justify-center gap-0.5 py-2 text-[11px] font-medium transition-colors',
              isActive(to) ? 'text-primary' : 'text-muted-foreground hover:text-foreground',
            )}
          >
            <Icon className="h-5 w-5" />
            {label}
          </Link>
        ))}
        <div className="flex flex-1 items-center justify-center">
          <button
            onClick={handleNewTx}
            aria-label="New transaction"
            className="-translate-y-3 flex h-12 w-12 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-lg ring-4 ring-background hover:bg-primary/90 active:scale-95 transition-transform"
          >
            <Plus className="h-5 w-5" />
          </button>
        </div>
        {mobileTabRight.map(({ to, label, icon: Icon }) => (
          <Link
            key={to}
            to={to}
            className={cn(
              'flex flex-1 flex-col items-center justify-center gap-0.5 py-2 text-[11px] font-medium transition-colors',
              isActive(to) ? 'text-primary' : 'text-muted-foreground hover:text-foreground',
            )}
          >
            <Icon className="h-5 w-5" />
            {label}
          </Link>
        ))}
        <button
          onClick={() => setMoreOpen(true)}
          className="flex flex-1 flex-col items-center justify-center gap-0.5 py-2 text-[11px] font-medium text-muted-foreground hover:text-foreground"
        >
          <MoreHorizontal className="h-5 w-5" />
          More
        </button>
      </nav>

      <Sheet open={moreOpen} onOpenChange={setMoreOpen}>
        <SheetContent side="right">
          <SheetHeader>
            <SheetTitle>More</SheetTitle>
          </SheetHeader>
          <SheetBody>
            <nav className="space-y-1">
              {moreItems.map(({ to, label, icon: Icon }) => (
                <Link
                  key={to}
                  to={to}
                  onClick={() => setMoreOpen(false)}
                  className={cn(
                    'flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm transition-colors',
                    isActive(to)
                      ? 'bg-primary/10 font-medium text-primary'
                      : 'text-foreground hover:bg-accent',
                  )}
                >
                  <Icon className="h-4 w-4" />
                  {label}
                </Link>
              ))}
              <button
                onClick={() => {
                  setMoreOpen(false)
                  logout()
                }}
                className="flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left text-sm text-foreground hover:bg-accent"
              >
                <LogOut className="h-4 w-4" />
                Log out
              </button>
            </nav>
            <div className="mt-6 border-t border-border pt-4">
              <p className="px-3 text-xs text-muted-foreground">
                Signed in as <span className="font-medium text-foreground">{user?.username}</span>
              </p>
            </div>
          </SheetBody>
        </SheetContent>
      </Sheet>

      <TxFormSheet />
    </div>
  )
}
