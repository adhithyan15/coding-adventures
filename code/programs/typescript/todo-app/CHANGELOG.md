# Changelog

All notable changes to this package will be documented in this file.

## [0.7.0] - 2026-03-28

### Added

- **Graph-based Projects** (`src/graph.ts`, `src/types.ts`) — introduces Projects as
  first-class entities backed by a Directed Acyclic Graph (DAG). Every project and task
  is a node; "contains" edges link projects to their tasks and subprojects.
  - `GraphEdge` type with v7 (time-sortable) UUIDs for edge IDs
  - `buildGraph(projects, tasks, edges)` — reconstructs in-memory `LabeledDirectedGraph`
    from the flat persisted edge list
  - `wouldCreateCycle(graph, fromId, toId)` — cycle detection via `transitiveClosure`
  - `getProjectTaskIds`, `getSubprojectIds`, `getTaskProjectIds` — DAG query helpers
  - `newEdgeId()` — v7 UUID factory using `@coding-adventures/uuid`
- **`Project` type** (`src/types.ts`) — `{ id, name, isBuiltIn, createdAt, updatedAt }`
- **Three new action constants** (`src/actions.ts`):
  - `PROJECT_UPSERT` / `projectUpsertAction(project)`
  - `EDGE_ADD` / `edgeAddAction(edge)` — cycle-safe; reducer silently rejects cycles
  - `EDGE_REMOVE` / `edgeRemoveAction(edgeId)`
- **`createTaskAction` gains `projectId` parameter** (default `"default"`) — the reducer
  creates the task AND its "contains" edge atomically in a single dispatch
- **`AppState` expanded** — new `projects: Project[]` and `edges: GraphEdge[]` fields
- **IDB schema v4** (`src/main.tsx`) — two new stores:
  - `"projects"` (keyPath: `"id"`) — Project entity storage
  - `"edges"` (keyPath: `"id"`, indexes: `fromId`, `toId`) — edge storage
- **v3 → v4 data migration** — on first launch after upgrade: seeds the Default project,
  creates "contains" edges from `"default"` to every existing task
- **Default project seeding** (`src/seed.ts`) — `PROJECT_ID_DEFAULT = "default"`,
  `seedDefaultProject(store)` — dispatches `PROJECT_UPSERT` for the built-in Default project
- **Comprehensive tests** — `src/__tests__/graph.test.ts` (29 tests), plus new suites in
  `reducer.test.ts`, `persistence.test.ts`, and `actions.test.ts`
- **Vitest Web Crypto polyfill** (`src/__tests__/setup.ts`) — ensures
  `globalThis.crypto.getRandomValues` is always available across test workers;
  fixes tests that stub `crypto` for deterministic UUIDs while also needing `v7()`

### Changed

- `TASK_CREATE` in reducer now atomically creates both the task and a "contains" edge
- `TASK_DELETE` in reducer now cascade-removes all edges where `fromId` or `toId`
  matches the deleted task
- `TASK_DELETE` in persistence middleware now cascade-deletes edges from IndexedDB
- `TASK_CREATE` in persistence middleware now persists the auto-generated edge
- `STATE_LOAD` action and `stateLoadAction()` creator now accept `projects` and `edges`
- All `vi.stubGlobal("crypto", ...)` calls in `reducer.test.ts` updated to include
  `getRandomValues` so `newEdgeId()` / `v7()` continue to work during crypto mocks
- `vitest.config.ts` updated: `setupFiles`, `src/graph.ts` added to coverage includes

### Dependencies

- Added `@coding-adventures/directed-graph` (file: `../../../packages/typescript/directed-graph`)
- Added `@coding-adventures/uuid` (file: `../../../packages/typescript/uuid`)
- `BUILD` updated: installs transitive `file:` deps in leaf-to-root order:
  `sha1 → md5 → uuid → directed-graph → todo-app`

## [0.6.0] - 2026-03-28

### Added

- **i18n string catalog** (`src/strings.en.json` + `src/strings.ts`) — every user-visible string
  in the app is now keyed in a flat JSON catalog. Swapping to `strings.fr.json` translates the
  entire app. The `t(key, params?)` accessor is fully type-safe via `NestedKeys<T>` — invalid
  keys are TypeScript errors, not runtime surprises. Interpolation uses `{key}` syntax.

### Changed

- **Component renames** — all `Todo`-prefixed components renamed to `Task` to match the internal
  action/type rename done in Views Engine V1:
  - `TodoEditor` → `TaskEditor` (`src/components/TaskEditor.tsx`)
  - `TodoCard` → `TaskCard` (`src/components/TaskCard.tsx`)
  - `TodoList` → `TaskList` (`src/components/TaskList.tsx`)
  - `TodoCalendar` → `TaskCalendar` (`src/components/TaskCalendar.tsx`)
- All components updated to use `t()` for every user-visible string (labels, placeholders,
  button text, error messages, filter options, history descriptions).
- App title changed from "Todo" to "Tasks" (matches the Task API rename).
- `ViewRenderer.tsx` updated to import `TaskList` (was `TodoList`).
- `ViewRenderer.test.tsx` mock updated from `TodoList.js` to `TaskList.js`.

## [0.5.0] - 2026-03-28

### Added

- **Task activity history panel** — when editing a task, a vertical timeline appears below the
  form showing every action taken on that task, oldest to newest: creation, field updates (with
  the specific changed field names listed), and status changes. Reads from the audit log via
  `getActivitiesForEntity`. Uses relative timestamps ("3m ago", "2h ago") for recent events
  and absolute date/time for older ones.
- **Storage singleton** (`src/storage.ts`) — `initStorage` / `getStorage` pattern lets components
  query IndexedDB directly (e.g., the history panel) without prop-drilling or React Context overhead.
  `initStorage` is called in `main.tsx` before React mounts so `getStorage()` is always ready.

### Fixed

- **Default due date = today** — new tasks now pre-fill the due date field with today's date in
  local timezone. Previously the field was blank, which caused tasks to not appear in the "Today"
  view unless the user remembered to set the date manually. The date is computed in local time
  (not UTC), so users in negative UTC offsets no longer see yesterday's date pre-filled at night.

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
