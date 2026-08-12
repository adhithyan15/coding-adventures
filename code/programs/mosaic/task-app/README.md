# task-app

A to-do app with **automatic scheduling**, built entirely on the shared `task-core`
engine. Add tasks (with optional due dates), complete or delete them — and the engine
auto-schedules them into a working-day timeline via the Critical Path Method.

## Architecture

The UI is authored **once in Mosaic**. The web host wires emitted React to
`task-core` through `task-wasm`, retaining idiomatic React state. Generated native
hosts load `task-mosaic-app`, a standard-ABI adapter that owns portable presentation
state and calls the same typed `task-core` operations and projections. The adapter is
not a second task engine: domain validation, scheduling, and task/project invariants
remain in `task-core`.

```text
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

The native path is:

```text
TaskApp.mil / .mll / .msl
        │  mosaic-compile --backend <native> --profile native-complete
        ▼
generated native UI + standard host binding
        │  bundled mosaic-app C ABI
        ▼
task-mosaic-app (MIL slots/events + presentation state) → task-core
```

Qt, Flutter, Compose Desktop, and SwiftUI on macOS are gated on this concrete
engine. CI requires zero degradations, builds the generated native project,
verifies the bundled library byte-for-byte, and launches the installed app without
an injected runtime path. The ABI conformance fixture remains a separate gate, so
a passing TaskApp launch cannot mask a regression in the standard host binding.
The generated SwiftUI sources also compile for the iOS 16 deployment target; that
gate is source portability rather than a claim that a macOS dylib can run on iOS.

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
- `../../../packages/rust/task-mosaic-app/` — native standard-ABI application adapter.
- `mosaic-package.toml` — the package manifest.
- `tests/package_compiles.rs` — `cargo test` verifies the Mosaic sources compile.
- `scripts/build-web.{sh,ps1}` — build wasm + emit the component into `host/web`.
