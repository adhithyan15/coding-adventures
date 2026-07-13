# Changelog

All notable changes to the `task-app` web program are documented here.

## [0.1.0] - Unreleased

### Changed

- **The web host is now a committed npm package** (`host/web/`) instead of files
  overlaid onto a generated Vite project. `scripts/build-web.{sh,ps1}` now emit only the
  generated `TaskApp.tsx` component (and copy the wasm runtime) *into* the package,
  rather than emitting a whole project and overlaying `main.tsx`. This makes the host —
  and the persistence added below — reliably buildable and testable. The host wiring
  moved from `host/web/main.tsx` to `host/web/src/main.tsx`. See `host/web/CHANGELOG.md`.

### Added

- **Local persistence for the web app** — the whole workspace is serialized via the
  engine's `snapshot()` and stored through the repo's pluggable `KVStorage` contract
  (`@coding-adventures/indexeddb`'s `IndexedDBStorage`, in-memory fallback), then
  restored on reload. Persistence is host-owned; the engine stays pure. (Phase 1 of
  `code/specs/task-app-super-app.md`.) Verified end-to-end: tasks + their schedule
  survive a page reload.

- **The task-app web UI** — a to-do app with automatic scheduling, authored in Mosaic
  and wired to the pure `task-core` engine through `task-wasm` (WebAssembly).
  - `src/TaskApp.mil` — interface: slots (`app-title`, `new-task-name`, `new-task-due`,
    `summary`, `task-rows`) and events (`onNewTaskNameChange`, `onNewTaskDueChange`,
    `onAddTask`, `onToggleTask`, `onDeleteTask`).
  - `src/TaskApp.mll` — layout: header, add-row (two `HostInput`s + `HostButton`), a
    summary line, and a `For` over `task-rows` with per-row toggle/delete buttons.
  - `src/TaskApp.light.msl` — light-theme styling.
  - `host/web/src/main.tsx` — the React entry: holds the engine in React state, maps the
    emitted `dispatch(event)` to engine operations (`createTask`/`setDuration`/
    `linkDependency`/`setDeadline`/`setCompleted`/`deleteTask`), and re-derives the
    slot props from engine queries (`todos`/`gantt`). **No dispatch/props facade** —
    idiomatic React, the engine stays pure.
- **Auto-scheduling** — new tasks are chained (finish-to-start) into a work queue, so
  the CPM engine sequences them across working days; the row shows each task's computed
  start→finish, the summary shows the projected project finish, and overdue tasks
  (finishing past their due date) are flagged.
- **Build tooling** — `scripts/build-web.{sh,ps1}` builds the engine to wasm, emits the
  `TaskApp.tsx` component via `mosaic-compile`, and copies it plus the wasm runtime into
  the `host/web` package. `cargo test` (`tests/package_compiles.rs`) verifies the Mosaic
  interface/layout/style compile and the manifest exports `TaskApp`; `npm test` in
  `host/web` verifies the persistence seam.

### Verified

- End-to-end in a browser: adding tasks auto-schedules them (e.g. three tasks land on
  consecutive working days Mon→Tue→Wed), completion toggles work, and the projected
  finish updates — all driven by the Rust engine over WASM, with no console errors.
