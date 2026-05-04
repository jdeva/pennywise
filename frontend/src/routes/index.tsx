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
import {
  CashflowReport,
  CategoriesReport,
  TrendReport,
  MerchantsReport,
} from '@/features/reports'
import { RecurringPage } from '@/features/recurring'

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

const reportsIndexRoute = createRoute({
  getParentRoute: () => protectedRoute,
  path: '/reports',
  component: CashflowReport,
})

const reportsCashflowRoute = createRoute({
  getParentRoute: () => protectedRoute,
  path: '/reports/cashflow',
  component: CashflowReport,
})

const reportsCategoriesRoute = createRoute({
  getParentRoute: () => protectedRoute,
  path: '/reports/categories',
  component: CategoriesReport,
})

const reportsTrendRoute = createRoute({
  getParentRoute: () => protectedRoute,
  path: '/reports/trend',
  component: TrendReport,
})

const reportsMerchantsRoute = createRoute({
  getParentRoute: () => protectedRoute,
  path: '/reports/merchants',
  component: MerchantsReport,
})

const recurringRoute = createRoute({
  getParentRoute: () => protectedRoute,
  path: '/recurring',
  component: RecurringPage,
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
    reportsIndexRoute,
    reportsCashflowRoute,
    reportsCategoriesRoute,
    reportsTrendRoute,
    reportsMerchantsRoute,
    recurringRoute,
    settingsRoute,
    ledgerFilesRoute,
  ]),
])
