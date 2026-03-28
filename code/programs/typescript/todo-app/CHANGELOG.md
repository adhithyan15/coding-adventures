# Changelog

All notable changes to this package will be documented in this file.

## [0.4.0] - 2026-03-28

### Added

- **Audit log & event sourcing system** (`src/audit.ts`) — every entity mutation is
  now durably recorded as an `AuditEvent` in IndexedDB, enabling activity feeds,
  streak detection, and full crash-recovery replay
  - `VectorClock` (`Record<string, number>`) — distributed-ordering primitive that
    degenerates to a simple sequence number today and extends to multi-device sync
    without a schema change
  - `AuditEvent` — `entityId`, `entityType`, `actionType` denormalized at the top
    level for fast IndexedDB filtering; full `action` payload for replay
  - `StateSnapshot` — periodic full-state checkpoints that bound replay cost to
    O(recent events) instead of O(all events ever)
  - **Write-Ahead Log guarantee** — audit middleware runs *before* `next()` so a crash
    between audit write and persistence write is self-correcting via replay
  - **Log compaction** — triggered on startup (event count > 500) and on
    `visibilitychange:hidden`; writes a snapshot then trims superseded events
  - **`getActivitiesForEntity(storage, entityId, options?)`** — returns the ordered
    history of a single entity for activity feeds and streak detection
  - **`getRecentActivities(storage, options?)`** — cross-entity activity feed, newest
    first; filterable by `entityType`
  - `STATE_LOAD` and `VIEW_SET_ACTIVE` intentionally excluded from the audit log
    (hydration noise and ephemeral tab nav respectively)
- **Entity ID pre-generation** in `createTaskAction` — `crypto.randomUUID()` moved
  from the reducer into the action creator so the audit middleware can record the
  new entity's ID before the reducer ever runs; also makes `TASK_CREATE` replay
  deterministic (same ID every time)
- **IndexedDB schema v3** — `"events"` and `"snapshots"` object stores added;
  all existing v1/v2 data is untouched (additive migration, no data loss)
- **61 new unit tests** in `src/__tests__/audit.test.ts` covering `getDeviceId`,
  `nextClockTick`, `extractEntityId`, WAL ordering, skipped actions,
  `getActivitiesForEntity`, `getRecentActivities`, and `compactEventLog`

### Changed

- `src/actions.ts` — `createTaskAction` now includes `id: crypto.randomUUID()` so
  the ID is stable and auditable before the reducer runs
- `src/reducer.ts` — `TASK_CREATE` case reads `action.id` (falls back to
  `crypto.randomUUID()` for raw test objects for backward compat)
- `src/main.tsx` — audit middleware registered before persistence middleware;
  IDB version bumped 2 → 3; startup compaction check added

## [0.2.0] - 2026-03-28

### Added

- **Calendar view** (`#/calendar`) — read-only monthly grid showing todos by due date
  - Each todo appears as a compact priority-coloured chip in its due-date cell
  - Completed todos shown with strikethrough and reduced opacity
  - Month navigation (‹ ›) and "Today" jump button
  - Today's cell highlighted with accent colour
  - Overflow days from adjacent months shown at reduced opacity
  - Powered by the new generic `CalendarView` component from `@coding-adventures/ui-components`
- **App navigation bar** — "☰ List" and "📅 Calendar" buttons in the header
  - Active route highlighted with accent colour and `aria-current="page"`
  - `/`, `/new`, `/edit/:id` all map to "List" as the active nav item
- **`TodoCalendar` component** — wires `CalendarView` to the todo store with `useStore`
- **Calendar and chip styles** in `app.lattice` — priority-coloured chips matching the app's dark theme

## [0.1.0] - 2026-03-28

### Added

- Initial app scaffolding with Vite + React + TypeScript
- Offline-first IndexedDB persistence via `@coding-adventures/indexeddb`
- Flux state management via `@coding-adventures/store`
- Lattice CSS superset for styling (via `@coding-adventures/vite-plugin-lattice`)
- Full CRUD: create, read, update, delete todos
- Status lifecycle: todo → in-progress → done
- Priority levels: low, medium, high, urgent
- Free-form categories with auto-suggestion
- Due date tracking with overdue/due-today badges
- Search, filter (status/priority/category), and sort
- Clear completed todos
- Seed data for first-time visitors
- Dark theme with glassmorphism and micro-animations
- Electron wrapper for desktop app packaging
- Comprehensive unit tests (reducer, actions, types, persistence)
- Playwright e2e test setup
