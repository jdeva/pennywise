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

## Playwright E2E protocol

Use these exact steps to verify after any UI/feature change. Do **not** skip.
The MCP screenshot tool occasionally returns stale images — always cross-check
with live DOM via `browser_evaluate` before declaring a failure.

### Setup
1. Make sure the stack is running: `finch compose ps` — all 4 containers must
   show `running`.
2. Rebuild the service you changed, then force-recreate the container:
   ```
   finch compose up -d --build frontend
   finch compose up -d --force-recreate frontend
   sleep 3
   ```
   (Swap `frontend` for `backend` if backend changed. Both if both changed.)
3. Close any existing Playwright tabs: `browser_close`.

### Desktop pass (1440×900)
1. `browser_resize` width=1440 height=900
2. `browser_navigate` http://localhost:5173/sign-in
3. `browser_wait_for` 1.5s
4. `browser_fill_form` #username=uitest2 #password=TestPass123
5. `browser_click` button[type=submit]
6. `browser_wait_for` 2.5s
7. For **every** page you touched: navigate, wait 2s, take a viewport
   screenshot (NOT fullPage — viewport is more reliable), call
   `browser_evaluate` to pull the key DOM facts, call
   `browser_console_messages` level=error.
8. Test the primary interaction (open sheet, click filter, switch lair, etc.)
   via `browser_evaluate` rather than `browser_click` where possible — click
   selectors can trip on Radix portals.

### Mobile pass (390×844)
1. `browser_resize` width=390 height=844
2. `browser_navigate` http://localhost:5173/ (already signed in from desktop pass)
3. For each page, verify via `browser_evaluate`:
   - `window.innerWidth === 390`
   - `getComputedStyle(document.querySelector('aside')).display === 'none'`
   - `!!document.querySelector('nav.fixed')` (bottom nav present)
   - `document.documentElement.scrollWidth <= window.innerWidth + 1` (no
     horizontal scroll)
   - `document.querySelectorAll('section > ul > li').length > 0` (content
     rendered, not just the shell)
4. Open the filters sheet: click the "Filters" button → assert
   `!!document.querySelector('[role=dialog]')` and the filter inputs render
   inside it.
5. Check console is clean: `browser_console_messages` level=error must return 0.

### Known quirks
- `browser_take_screenshot` fullPage can return stale images after navigation.
  Prefer viewport screenshots, and never judge a failure from a screenshot
  alone — `browser_evaluate` is authoritative.
- On stale-bundle errors (404 on /assets/*.js) the browser tab cached an old
  index.html. Bust with a query-string reload: `window.location.replace(url + '?v=' + Date.now())`.
- Radix tab triggers don't respond to programmatic `.click()` from evaluate;
  use `browser_click` with a CSS selector.
- `browser_fill_form` can silently fail if the input was re-rendered — verify
  with a follow-up `browser_evaluate` checking the input's `.value`.

### Data
Test users already exist on the running stack:
- `uitest2` / `TestPass123` (owner of lair "Home")
- `mate1` / `TestPass123` (shared into "Home" with write permission)

If the stack was wiped, re-register via `POST /api/v1/auth/register` then
use curl to seed a few transactions (see the session transcript or
`docs/QA_LOG.md` for the API shape).

## Context pointers for next session

- Memory files in `~/.claude/projects/-Users-jdevv-Documents-workspace-pennywise/memory/`
  already capture: mobile-first rule, playful tone, autonomous-work permission,
  E2E-before-push rule, local dev port mapping, Playwright MCP setup
- `docs/QA_LOG.md` has RCAs for the bugs fixed last session
- `git log --oneline` shows the 4 commits that landed — don't redo that work
- Stack runs via `finch compose`; frontend rebuild is slow so batch changes
