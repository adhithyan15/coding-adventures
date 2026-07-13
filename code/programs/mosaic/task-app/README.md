# task-app (web)

A to-do app with **automatic scheduling**, built entirely on the shared `task-core`
engine. Add tasks (with optional due dates), complete or delete them — and the engine
auto-schedules them into a working-day timeline via the Critical Path Method.

## Architecture

The UI is authored **once in Mosaic** and emitted to React; the React host (a real,
committed npm package under `host/web/`) wires it to the pure Rust engine through
WebAssembly, holding engine state in plain **React state** — no
`dispatch`/`getProps`/`handleEvent` facade. React-isms (and host-owned persistence)
live only here, in the web backend, where they belong (per
`code/specs/task-app-architecture.md`).

```
TaskApp.mil / .mll / .msl        (Mosaic: interface / layout / style)
        │  mosaic-compile --backend react   (emits ONE component into host/web/src)
        ▼
   host/web/src/TaskApp.tsx       (generated React component: { ...slotProps, dispatch })
        │  host/web/src/main.tsx wires it to…
        ▼
   createTaskEngine (task-wasm)  →  task-core (the pure Rust engine) via WASM
        │  host/web/src/persistence.ts saves/restores via…
        ▼
   @coding-adventures/storage    →  IndexedDB (in-memory fallback)
```

## What it does

- Add a task (name + optional `YYYY-MM-DD` due date).
- Tasks are chained into a work queue and **auto-scheduled** — each starts when the
  previous finishes, on working days (weekends skipped), with a projected finish date.
- Tasks scheduled to finish after their due date are flagged **overdue**.
- Click a row to complete it (✓); Delete to remove it.
- **Everything persists** — the whole workspace is saved to IndexedDB after each change
  and restored on reload (see `host/web/`).

Everything above runs on the pure Rust engine — the browser only holds UI state,
persists snapshots, and calls the engine's operations/queries.

## Build & run

```bash
scripts/build-web.sh                            # build wasm + emit TaskApp.tsx into host/web
cd host/web && npm install && npm run dev       # http://localhost:5173
```

## Files

- `src/TaskApp.{mil,mll,light.msl}` — the Mosaic UI (interface, layout, style).
- `host/web/` — the committed web-host npm package (`src/main.tsx`, `src/persistence.ts`,
  its own `package.json`/tests); see `host/web/README.md`.
- `mosaic-package.toml` — the package manifest.
- `tests/package_compiles.rs` — `cargo test` verifies the Mosaic sources compile.
- `scripts/build-web.{sh,ps1}` — build wasm + emit the component into `host/web`.
