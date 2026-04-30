# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What Pennywise Is

Personal finance tracker that is a **UI/API layer on top of ledger-cli** — we do NOT reinvent accounting logic. "Workspaces" are shared expense collections (e.g., "Home", "Vacation"), not bank accounts. Users can share workspaces with roommates at read-only or read-write permissions.

Stack: Rust/Actix-web backend, React/TypeScript/Vite frontend, Redis cache, flat-file storage (JSON metadata + ledger files), Docker Compose deployment.

Note: both `README.md` and `docs/ARCHITECTURE.md` describe the old Svelte frontend and a simpler "accounts" model. Ignore those where they conflict with the current code — the frontend is now React, and "account" means "workspace" (renamed). `.kiro/steering/project-context.md` is the most up-to-date high-level narrative.

## Common Commands

### Full stack (Docker)
```bash
docker-compose up -d                        # Start redis, backend, worker, frontend
docker-compose up -d --build backend        # Rebuild one service
docker logs -f pennywise-backend            # Tail logs
```
Frontend: http://localhost:3000 · Backend: http://localhost:8080 · API prefix: `/api/v1`

### Backend (Rust, `backend/`)
```bash
cargo build
cargo run                                   # needs REDIS_URL, LEDGER_DATA_PATH, JWT_SECRET (>=32 chars)
cargo test                                  # runs unit + proptest suite
cargo test <test_name>                      # single test
cargo fmt && cargo clippy
```
Required env vars: `REDIS_URL`, `LEDGER_DATA_PATH`, `JWT_SECRET` (min 32 chars), optional `PORT` (default 8080), `RUST_LOG`.

### Frontend (`frontend/`, uses pnpm)
```bash
pnpm install
pnpm dev                                    # vite dev server on :3000, proxies /api → :8080
pnpm build                                  # tsc -b && vite build
pnpm test                                   # vitest run (single pass)
pnpm test:watch
```

### Worker (`worker/`)
```bash
cargo run                                   # polls Redis every SYNC_INTERVAL_SECONDS (default 300)
```
Known: worker currently drains `pending_writes` without actually writing to files (see BACKLOG.md).

## Architecture Essentials

### Three-level ledger file hierarchy (backend)
Transactions are never written directly to a single file. Routing is by date into period files:
```
/data/ledger/
├── users/
│   ├── user-{uuid}.json                    # UserProfile
│   ├── user-{uuid}-auth.json               # UserAuth (chmod 600)
│   ├── user-{uuid}-master.ledger           # !include all workspace ledgers
│   └── user-{uuid}-chart-of-accounts.json
└── workspaces/
    ├── workspace-{uuid}.json
    └── workspace-{uuid}/
        ├── workspace-{uuid}.ledger         # !include period files
        └── workspace-{uuid}-2026-Q1.ledger # actual transactions
```
`accounts/` also exists for legacy single-file workspaces — migration code supports both.

### Backend layout (`backend/src/`)
- `main.rs` — wires `Cache`, `FileStore`, and services (`UserService`, `WorkspaceService`, `TransactionService`, `BudgetService`) into Actix app data.
- `models/v1/` and `api/v1/` — versioned; all routes served under `/api/v1`.
- `services/file_store.rs` — atomic writes (temp file → rename) for JSON; append-only for ledger files.
- `services/ledger_cli.rs` — subprocess wrapper; all balance/register queries shell out to `ledger`.
- `services/cache.rs` — Redis wrapper, also stores refresh tokens.
- `middleware/auth.rs` — JWT middleware; skips paths containing `/auth/` or `/health`. Access tokens 15min, refresh 7 days, bcrypt cost 14.
- `utils/error.rs` — `AppError` enum (`BadRequest | Unauthorized | Forbidden | NotFound | Conflict | Validation | Internal`); use this, not anyhow/unwrap.
- `utils/validation.rs` — input validation returning `Vec<ValidationDetail>` for structured 400s.
- OpenAPI spec at `docs/openapi/pennywise.yaml` is the API source of truth; keep it in sync when changing `api/v1/*.rs` or `models/v1/*.rs`.

### Frontend layout (`frontend/src/`)
React 18 + TanStack Router (file-ish routes under `routes/`) + TanStack Query + React Hook Form + Zod + Tailwind + Radix UI primitives in `components/ui/`. Path alias `@/` → `src/`.
- `main.tsx` — composes `ThemeProvider`, `AuthProvider`, `WorkspaceProvider` around the router.
- `context/` — auth, workspace selection, theme.
- `features/<domain>/` — feature-scoped screens and components.
- `lib/api/` — axios client + per-resource modules (auth, workspaces, transactions, budgets, chart-of-accounts, etc.).

### Read this before designing anything
`.kiro/steering/project-context.md` has the up-to-date narrative (current specs completed, design choices, ledger-first principles). `.kiro/specs/<feature>/` folders hold the spec-driven development trail (requirements → design → tasks) for each feature.

## Project Conventions

- **Ledger-first**: never modify ledger files for bookkeeping outside of ledger's format. Full paths always — `Assets:Bank:Revolut`, not `Bank:Revolut`. Account type is derived from the first segment. Files must remain usable with `ledger` CLI directly.
- **Versioned API**: new endpoints go under `api/v1/` and `models/v1/`. Update `docs/openapi/pennywise.yaml` when routes or schemas change (there's a Kiro hook that reminds).
- **No `.unwrap()` in production code** — return `AppError`.
- **Atomic file writes** for JSON (temp + rename); append-only for ledger files.
- **Property-based tests** with `proptest` are expected for validation and parsing logic; regressions live in `backend/proptest-regressions/`.
- "Account" in older code/docs = "Workspace" in current code (renamed).
- FOSS/MIT — keep code minimal and readable; avoid junk comments.
