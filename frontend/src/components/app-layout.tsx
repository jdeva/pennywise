import { Link, useRouterState } from '@tanstack/react-router'
import { useAuth } from '@/context/auth-context'
import { WorkspaceDropdown } from '@/components/workspace-dropdown'
import { useTheme } from '@/context/theme-context'
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
import { Button } from '@/components/ui/button'

const navItems = [
  { to: '/' as const, label: 'Dashboard', icon: LayoutDashboard },
  { to: '/transactions' as const, label: 'Transactions', icon: ArrowLeftRight },
  { to: '/budgets' as const, label: 'Budgets', icon: PiggyBank },
]

export function AppLayout({ children }: { children: React.ReactNode }) {
  const { user, logout } = useAuth()
  const { theme, setTheme } = useTheme()
  const router = useRouterState()
  const currentPath = router.location.pathname

  return (
    <div className="flex h-screen bg-background">
      {/* Sidebar */}
      <aside className="hidden w-64 flex-col border-r border-sidebar-border bg-sidebar md:flex">
        <div className="flex h-16 items-center gap-2 px-5">
          <div className="flex h-9 w-9 items-center justify-center rounded-full bg-primary text-primary-foreground">
            <span className="font-display text-lg font-semibold">P</span>
          </div>
          <span className="font-display text-xl font-semibold tracking-tight text-sidebar-foreground">
            Pennywise
          </span>
        </div>
        <nav className="flex-1 space-y-1 px-3">
          {navItems.map(({ to, label, icon: Icon }) => {
            const isActive =
              to === '/' ? currentPath === '/' : currentPath.startsWith(to)
            return (
              <Link
                key={to}
                to={to}
                className={cn(
                  'group flex items-center gap-3 rounded-xl px-3 py-2.5 text-sm transition-all',
                  isActive
                    ? 'bg-primary/10 font-semibold text-primary'
                    : 'text-sidebar-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground',
                )}
              >
                <Icon className={cn('h-4 w-4 transition-transform group-hover:scale-110', isActive && 'text-primary')} />
                {label}
              </Link>
            )
          })}
        </nav>
        <div className="border-t border-sidebar-border p-3">
          <div className="mb-2 flex items-center gap-2 px-2">
            <div className="flex h-7 w-7 items-center justify-center rounded-full bg-accent-lavender/20 text-xs font-semibold text-accent-lavender">
              {user?.username?.[0]?.toUpperCase() ?? '?'}
            </div>
            <span className="truncate text-sm text-sidebar-foreground">{user?.username}</span>
          </div>
          <Button
            variant="ghost"
            size="sm"
            className="w-full justify-start gap-2 text-sidebar-foreground"
            onClick={logout}
          >
            <LogOut className="h-4 w-4" /> Log out
          </Button>
        </div>
      </aside>

      {/* Main content */}
      <div className="flex flex-1 flex-col overflow-hidden">
        {/* Header */}
        <header className="flex h-16 items-center justify-end gap-2 border-b border-border/60 bg-background/80 px-6 backdrop-blur">
          <WorkspaceDropdown />
          <Link to="/settings">
            <Button variant="ghost" size="icon" aria-label="Settings">
              <Settings className="h-4 w-4" />
            </Button>
          </Link>
          <Button
            variant="ghost"
            size="icon"
            onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}
            aria-label="Toggle theme"
          >
            {theme === 'dark' ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
          </Button>
        </header>

        {/* Page content */}
        <main className="flex-1 overflow-auto p-6 lg:p-8">{children}</main>
      </div>
    </div>
  )
}
