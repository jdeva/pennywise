import { Link, useRouterState } from '@tanstack/react-router'
import { useAuth } from '@/context/auth-context'
import { useTheme } from '@/context/theme-context'
import { WorkspacePicker } from '@/components/workspace-picker'
import { cn } from '@/lib/utils'
import {
  LayoutDashboard,
  ArrowLeftRight,
  PiggyBank,
  Settings,
  LogOut,
  Sun,
  Moon,
} from 'lucide-react'

const navItems = [
  { to: '/' as const, label: 'Dashboard', icon: LayoutDashboard },
  { to: '/transactions' as const, label: 'Transactions', icon: ArrowLeftRight },
  { to: '/budgets' as const, label: 'Budgets', icon: PiggyBank },
  { to: '/settings' as const, label: 'Settings', icon: Settings },
]

export function AppLayout({ children }: { children: React.ReactNode }) {
  const { user, logout } = useAuth()
  const { theme, setTheme } = useTheme()
  const router = useRouterState()
  const currentPath = router.location.pathname

  const isActive = (to: string) => (to === '/' ? currentPath === '/' : currentPath.startsWith(to))

  return (
    <div className="flex min-h-screen flex-col bg-background md:flex-row">
      {/* Mobile top bar: logo + lair picker + theme */}
      <header className="sticky top-0 z-20 flex items-center gap-3 border-b border-border bg-background/95 px-4 py-3 backdrop-blur md:hidden">
        <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-primary text-primary-foreground">
          <span className="font-display text-base font-semibold">P</span>
        </div>
        <div className="flex-1 min-w-0">
          <WorkspacePicker />
        </div>
        <button
          onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}
          aria-label="Toggle theme"
          className="flex h-11 w-11 items-center justify-center rounded-md text-foreground hover:bg-accent"
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
          {navItems.map(({ to, label, icon: Icon }) => (
            <Link
              key={to}
              to={to}
              className={cn(
                'flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition-colors',
                isActive(to)
                  ? 'bg-primary/10 font-medium text-primary'
                  : 'text-sidebar-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground',
              )}
            >
              <Icon className="h-4 w-4" />
              {label}
            </Link>
          ))}
        </nav>

        <div className="border-t border-sidebar-border px-3 py-3">
          <div className="flex items-center gap-2 px-2 py-1.5">
            <div className="flex h-7 w-7 items-center justify-center rounded-full bg-accent-lavender/20 text-xs font-semibold text-accent-lavender">
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

      {/* Main content */}
      <main className="flex-1 overflow-x-hidden pb-20 md:pb-0">
        <div className="mx-auto max-w-7xl px-4 py-6 sm:px-6 md:py-8 lg:px-10 lg:py-10">
          {children}
        </div>
      </main>

      {/* Mobile bottom tab bar */}
      <nav className="fixed inset-x-0 bottom-0 z-20 flex border-t border-border bg-background/95 pb-[env(safe-area-inset-bottom)] backdrop-blur md:hidden">
        {navItems.map(({ to, label, icon: Icon }) => (
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
          onClick={logout}
          aria-label="Log out"
          className="flex flex-1 flex-col items-center justify-center gap-0.5 py-2 text-[11px] font-medium text-muted-foreground hover:text-foreground"
        >
          <LogOut className="h-5 w-5" />
          Log out
        </button>
      </nav>
    </div>
  )
}
