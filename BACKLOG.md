# Pennywise - Feature Backlog

Ideas and improvements to revisit later. Not prioritized — just captured so nothing gets lost.

## User Management
- Admin user creation flow: first user becomes admin, only admin can create other users
- Email-based self-service sign up with verification
- Password reset via email
- Email verification on registration
- Rate limiting on auth endpoints
- Refresh token revocation on password change
- Session management (list active sessions, revoke)
- User roles and permissions (admin, member, viewer)

## Account Management
- Account deletion (soft delete with archive)
- Account metadata persistence to JSON files (currently Redis-only)
- Multi-currency support per ledger file
- Account categories/grouping

## Ledger / Transactions
- Ledger file rotation (time-based splitting with !include directives in master ledger)
- Transaction editing/deletion
- Credit card account support
- Budgeting support (ledger budget directives)
- Recurring transactions
- **Ledger file import** — Upload a ledger file to a workspace, save it as-is to the workspace directory, and add `!include imported.ledger` to the workspace ledger. No parsing needed — ledger-cli handles it. Future transactions still go to period files normally.
- Import/export ledger files
- Reports and charts (spending patterns, trends)
- User-specific queries for shared accounts (filter by user_id)
- Master ledger balance endpoint

## Frontend
- Full redesign with proper auth flow
- Mobile responsiveness
- Account sharing UI
- Transaction history view
- Date picker for transactions
- Loading states and error handling

## DevOps
- HTTPS/TLS setup
- Backup strategy for ledger files
- Monitoring and logging
- Health checks for all services
- Non-root container user
- Production JWT secret management
- CI/CD pipeline

## Technical Debt
- Worker doesn't actually sync (just clears pending writes)
- Input validation on account/transaction endpoints
- Integration tests
- API documentation (OpenAPI/Swagger)
- Audit logging
