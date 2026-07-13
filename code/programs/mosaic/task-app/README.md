# task-app (web)

A to-do app with **automatic scheduling**, built entirely on the shared `task-core`
engine. Add tasks (with optional due dates), complete or delete them — and the engine
auto-schedules them into a working-day timeline via the Critical Path Method.

## Architecture

The UI is authored **once in Mosaic** and emitted to React; the React host wires it to
the pure Rust engine through WebAssembly, holding engine state in plain **React state** —
no `dispatch`/`getProps`/`handleEvent` facade. React-isms live only here, in the web
backend, where they belong (per `code/specs/task-app-architecture.md`).

```
TaskApp.mil / .mll / .msl        (Mosaic: interface / layout / style)
        │  mosaic-compile --backend react --emit-project
        ▼
   TaskApp.tsx                    (generated React component: { ...slotProps, dispatch })
        │  host/web/main.tsx wires it to…
        ▼
   createTaskEngine (task-wasm)  →  task-core (the pure Rust engine) via WASM
```

## What it does

- Add a task (name + optional `YYYY-MM-DD` due date).
- Tasks are chained into a work queue and **auto-scheduled** — each starts when the
  previous finishes, on working days (weekends skipped), with a projected finish date.
- Tasks scheduled to finish after their due date are flagged **overdue**.
- Click a row to complete it (✓); Delete to remove it.

Everything above runs on the pure Rust engine — the browser only holds UI state and
calls the engine's operations/queries.

## Build & run

```bash
scripts/build-web.sh                            # build wasm + emit React + wire → dist/react
cd dist/react && npm install && npm run dev     # http://localhost:5173
```

## Files

- `src/TaskApp.{mil,mll,light.msl}` — the Mosaic UI (interface, layout, style).
- `host/web/main.tsx` — the React entry that wires the emitted component to the engine.
- `mosaic-package.toml` — the package manifest.
- `tests/package_compiles.rs` — `cargo test` verifies the Mosaic sources compile.
- `scripts/build-web.{sh,ps1}` — assemble the runnable project into `dist/` (gitignored).
