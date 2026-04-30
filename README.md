# Pennywise

Personal finance tracker that is a UI/API layer on top of [ledger-cli](https://www.ledger-cli.org/). Workspaces are shared expense collections (e.g., "Home", "Vacation") — not bank accounts. Users can share a workspace with roommates at read-only or read-write permission.

Ledger-first: all bookkeeping goes through ledger-cli's format. Files remain usable with the `ledger` CLI directly.

## Stack

- Backend: Rust / Actix-web wrapping ledger-cli
- Frontend: React 18 + TanStack Router/Query + Tailwind + Radix
- Redis: cache for reads and refresh tokens
- Storage: flat JSON metadata + ledger files
- Deployment: Docker Compose

## Quick start

```bash
docker-compose up -d
```
Frontend at `http://localhost:3000`, API at `http://localhost:8080`, all routes under `/api/v1/`.

Required env vars: `REDIS_URL`, `LEDGER_DATA_PATH`, `JWT_SECRET` (≥32 chars).

## Local development

Backend:
```bash
cd backend
cargo run          # needs REDIS_URL, LEDGER_DATA_PATH, JWT_SECRET
cargo test
cargo fmt && cargo clippy
```

Frontend (uses pnpm):
```bash
cd frontend
pnpm install
pnpm dev           # :3000, proxies /api → :8080
pnpm test
pnpm build
```

## Documentation

- API: `docs/openapi/pennywise.yaml` (source of truth for endpoints)
- Project state and conventions: `.kiro/steering/project-context.md`
- Architectural context for Claude Code: `CLAUDE.md`

## License

MIT.
