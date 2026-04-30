# Pennywise TODO

Carried over from 2026-04-30 session. All items are agreed-on but not yet built.
Next session: read this file first, then work through in order.

## 1. Seed-colour picker in Settings → Profile

Let users pick the accent/primary colour. Derives the palette at runtime via CSS vars.

- Add an "Appearance" section at the top of Settings → Profile tab
- 5 presets: coral (current), ocean, emerald, plum, butter
- Store choice in `localStorage.theme_seed`
- On app mount (ThemeProvider or a new AppearanceProvider), set
  `document.documentElement.style.setProperty('--seed-h', hue)` and friends
- Dark/light themes should derive from the same seed so both modes work
- No backend round-trip — pure client preference
- Test at 390×844 and 1440; switch between presets, refresh, confirm persistence

## 2. Backend: transaction delete + update

Ledger files are append-only, so edit/delete means rewriting a period file.

**Stable tx identifier scheme**
- On POST /transactions, generate a UUID and write it as `; Id: <uuid>` on the
  first posting line alongside the existing `; User:` tag
- All returned `TransactionResponse` objects gain an `id` field

**Endpoints**
- `DELETE /api/v1/workspaces/{id}/transactions/{tx_id}` — locate the entry
  (date header + `; Id:` match), remove it + its posting lines, write the
  period file atomically
- `PUT /api/v1/workspaces/{id}/transactions/{tx_id}` — same locate logic,
  replace the entry in place with the new formatted content
- Both need write-permission check (`has_write_access`)
- Both need period-file resolution (new period file if date moved)

**Tests**
- Proptest that delete-after-create yields the same balance as if we'd never
  created it
- Delete the only tx in a period file — file should still be valid ledger
- Permission: read-only user can't delete

**OpenAPI** (`docs/openapi/pennywise.yaml`) needs the new endpoints documented.

## 3. Frontend: row-level edit + delete UI

Depends on (2) shipping first.

- Each register row gets an overflow menu (`...` Radix DropdownMenu) with Edit / Delete
- Edit: opens the existing TransactionForm Sheet prefilled from the row's data
- Delete: confirm via AlertDialog, then call `transactionsApi.delete(wsId, txId)`
  and invalidate `['register', wsId]` + `['balance', wsId, ...]` queries
- The TransactionForm component needs an optional `initialValues` prop + a mode
  flag (`create` vs `edit`) so the submit handler routes to POST vs PUT
- Test in Playwright at 390×844 and 1440: delete a tx → gone from register →
  balance updates → no console errors

## 4. Verify + push

- Full E2E pass at both viewports (per `feedback_e2e_before_push.md`)
- Commit in logical chunks, push to origin/main via gh
- Update `docs/QA_LOG.md` if any bugs surface

## Context pointers for next session

- Memory files in `~/.claude/projects/-Users-jdevv-Documents-workspace-pennywise/memory/`
  already capture: mobile-first rule, playful tone, autonomous-work permission,
  E2E-before-push rule, local dev port mapping, Playwright MCP setup
- `docs/QA_LOG.md` has RCAs for the bugs fixed last session
- `git log --oneline` shows the 4 commits that landed — don't redo that work
- Stack runs via `finch compose`; frontend rebuild is slow so batch changes
