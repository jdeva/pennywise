import {
  createRootRoute,
  createRoute,
  Outlet,
} from '@tanstack/react-router'
import { AuthGuard } from '@/components/auth-guard'
import { AppLayout } from '@/components/app-layout'
import { SignInPage } from '@/features/auth/sign-in'
import { SignUpPage } from '@/features/auth/sign-up'
import { DashboardPage } from '@/features/dashboard'
import { TransactionsPage } from '@/features/transactions'
import { BudgetsPage } from '@/features/budgets'
import { SettingsPage } from '@/features/settings'
import { LedgerFilesPage } from '@/features/ledger-files'

const rootRoute = createRootRoute({
  component: () => <Outlet />,
})

// Public routes
const signInRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/sign-in',
  component: SignInPage,
})

const signUpRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/sign-up',
  component: SignUpPage,
})

// Protected layout wrapper
function ProtectedLayout() {
  return (
    <AuthGuard>
      <AppLayout>
        <Outlet />
      </AppLayout>
    </AuthGuard>
  )
}

const protectedRoute = createRoute({
  getParentRoute: () => rootRoute,
  id: 'protected',
  component: ProtectedLayout,
})

// Protected routes
const dashboardRoute = createRoute({
  getParentRoute: () => protectedRoute,
  path: '/',
  component: DashboardPage,
})

const transactionsRoute = createRoute({
  getParentRoute: () => protectedRoute,
  path: '/transactions',
  component: TransactionsPage,
})

const budgetsRoute = createRoute({
  getParentRoute: () => protectedRoute,
  path: '/budgets',
  component: BudgetsPage,
})

const settingsRoute = createRoute({
  getParentRoute: () => protectedRoute,
  path: '/settings',
  component: SettingsPage,
})

const ledgerFilesRoute = createRoute({
  getParentRoute: () => protectedRoute,
  path: '/ledger-files',
  component: LedgerFilesPage,
})

export const routeTree = rootRoute.addChildren([
  signInRoute,
  signUpRoute,
  protectedRoute.addChildren([
    dashboardRoute,
    transactionsRoute,
    budgetsRoute,
    settingsRoute,
    ledgerFilesRoute,
  ]),
])
