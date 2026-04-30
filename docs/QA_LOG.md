# QA Log — 2026-04-30

Verification pass after UI rewrite. All critical issues triaged and fixed; results below.

## Issues & Resolutions

### I-1  Stale bundle served at deep-linked routes — FIXED
**Symptom:** Navigated to `/transactions` after rebuilding the frontend container. Browser loaded a hashed JS bundle filename that no longer existed, causing 404s and a blank page.

**RCA:** `nginx.conf` had no cache headers. Browsers cached `index.html` via the freshness heuristic, so SPA route loads served the pre-deploy index.html, which referenced hashed JS that had been purged in the new image.

**Fix:** `frontend/nginx.conf`:
- `/assets/*` → `Cache-Control: public, immutable; expires 1y` (hashed filenames, safe to cache)
- `/` and `/index.html` → `Cache-Control: no-store, no-cache, must-revalidate`

**Verified:** `curl -sI http://localhost:5173/` returns the new headers; fresh navigations pick up the latest bundle.

### I-2  401 on workspace switch — CLOSED (cosmetic)
**Symptom:** Switching workspaces sometimes logged `401 Unauthorized` on `/workspaces/{id}/register`.

**RCA:** Access-token (15min) expired mid-session; axios response interceptor successfully refreshes + retries the request. The original failed request's 401 still shows in the browser console because it logs at the network layer before the interceptor's retry runs.

**Decision:** Not a functional bug — the retry succeeds, user sees data. Silencing requires replacing `interceptors.response` with a custom fetch wrapper or using `validateStatus`, which adds surface for every caller. Not worth it.

### I-3  Empty-state currency on dashboard — NOT A BUG
Verified: a fresh EUR workspace shows `€0.00`, not `$0.00`. My earlier test run had submitted the create dialog with `$` before the `€` button click registered (Playwright automation timing), so the workspace was stored with `$`.

### I-4  Playwright MCP screenshot staleness — WORKAROUND
Screenshot tool occasionally returned prior page's render even after navigation. Inspecting the live DOM via `browser_evaluate` was authoritative. No product change needed.

### I-14  /budgets returns 400 when budgeting disabled — FIXED
**Symptom:** Landing on `/budgets` with a workspace that has `budgeting_enabled=false` logged `GET /api/v1/workspaces/{id}/budgets → 400` in the console.

**RCA:** Backend's `ensure_budgeting_enabled` intentionally returns 400 when the feature is off. Frontend gate was `enabled: !!activeWorkspace && budgetingStatus?.budgeting_enabled`. When `budgetingStatus` is `undefined` (still loading), the expression evaluates to `undefined`, which react-query does NOT treat as "disabled" the same way `false` does in some code paths — and once `budgetingStatus` resolved to `{budgeting_enabled: false}`, the initial query had already fired.

**Fix:** `frontend/src/features/budgets/index.tsx:56` — change gate to `=== true` for explicit boolean narrowing.

**Verified:** navigated to `/budgets` on a no-budget workspace. No network request made, console clean.

### I-16  Shared users list shows UUID — FIXED
**Symptom:** Settings → Workspaces → Expand Home → "Shared Users: 3da5133c-4a44-... (write)".

**RCA:** Backend `SharedUser` struct only stores `{user_id, permission}`. Frontend rendered `su.user_id` raw.

**Fix:**
- `backend/src/models/v1/workspace.rs` — introduced `SharedUserPublic { user_id, username, permission }`. Changed `WorkspacePublic.shared_with` from `Vec<SharedUser>` → `Vec<SharedUserPublic>`. Default `From<Workspace>` conversion leaves `username` empty.
- `backend/src/services/workspace.rs` — new `to_public(Workspace)` method that resolves usernames via `UserService::get_profile` for each shared user.
- `backend/src/api/v1/workspaces.rs` and `budgets.rs` — swapped `WorkspacePublic::from(w)` → `ws_service.to_public(w)` in all handlers.
- `frontend/src/lib/types.ts` — added `username: string` to `SharedUser`.
- `frontend/src/features/settings/workspaces.tsx` — renders `su.username || su.user_id` (fallback if lookup failed).

**Tests:** 200/200 backend tests still green after the model change.
**Verified:** UI now shows "mate1 (write)".

### I-17  POST /auth/logout returns 404 — FIXED
**Symptom:** Logout from any page logged `404 Not Found` on `/api/v1/auth/logout`.

**RCA:** Frontend called the endpoint; backend never implemented it.

**Fix:**
- `backend/src/api/v1/users.rs` — added `POST /auth/logout` handler that deletes the refresh-token entry from Redis. Accepts `{refresh_token}` body.
- `frontend/src/lib/api/auth.ts` — `logout(refreshToken)` now passes the token.
- `frontend/src/context/auth-context.tsx` — reads `refresh_token` from localStorage before calling `authApi.logout(...)`. Swallows errors regardless (localStorage is cleared either way).

**Verified:** logout → redirect to `/sign-in`, tokens cleared, zero console errors.

## Summary

| ID | Status | Severity |
|----|--------|----------|
| I-1 nginx cache | FIXED | high (blocked deploys) |
| I-2 401 on switch | closed (cosmetic) | low |
| I-3 empty-state currency | non-bug | — |
| I-4 Playwright screenshot staleness | workaround | tooling |
| I-14 /budgets 400 | FIXED | medium (console noise) |
| I-16 shared users UUID | FIXED | high (real UX bug) |
| I-17 logout 404 | FIXED | medium |

Full end-to-end passed: sign-up/in/out, workspace CRUD, transaction CRUD via sheet, two-user sharing with write permission round-trip, theme toggle, Dashboard/Budgets/Settings pages all render clean.
