# Changelog

All notable changes to the `task-app` web program are documented here.

## [0.1.0] - Unreleased

### Added

- **The task-app web UI** — a to-do app with automatic scheduling, authored in Mosaic
  and wired to the pure `task-core` engine through `task-wasm` (WebAssembly).
  - `src/TaskApp.mil` — interface: slots (`app-title`, `new-task-name`, `new-task-due`,
    `summary`, `task-rows`) and events (`onNewTaskNameChange`, `onNewTaskDueChange`,
    `onAddTask`, `onToggleTask`, `onDeleteTask`).
  - `src/TaskApp.mll` — layout: header, add-row (two `HostInput`s + `HostButton`), a
    summary line, and a `For` over `task-rows` with per-row toggle/delete buttons.
  - `src/TaskApp.light.msl` — light-theme styling.
  - `host/web/main.tsx` — the React entry: holds the engine in React state, maps the
    emitted `dispatch(event)` to engine operations (`createTask`/`setDuration`/
    `linkDependency`/`setDeadline`/`setCompleted`/`deleteTask`), and re-derives the
    slot props from engine queries (`todos`/`gantt`). **No dispatch/props facade** —
    idiomatic React, the engine stays pure.
- **Auto-scheduling** — new tasks are chained (finish-to-start) into a work queue, so
  the CPM engine sequences them across working days; the row shows each task's computed
  start→finish, the summary shows the projected project finish, and overdue tasks
  (finishing past their due date) are flagged.
- **Build tooling** — `scripts/build-web.{sh,ps1}` builds the engine to wasm, emits the
  React project via `mosaic-compile`, and overlays the host wiring + wasm runtime into
  `dist/react` (gitignored). `cargo test` (`tests/package_compiles.rs`) verifies the
  Mosaic interface/layout/style compile and the manifest exports `TaskApp`.

### Verified

- End-to-end in a browser: adding tasks auto-schedules them (e.g. three tasks land on
  consecutive working days Mon→Tue→Wed), completion toggles work, and the projected
  finish updates — all driven by the Rust engine over WASM, with no console errors.
