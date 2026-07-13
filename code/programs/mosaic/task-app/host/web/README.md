# task-app-web

The **web host** for `task-app`: the Mosaic-authored `TaskApp` UI wired to the pure
`task-core` engine (over `task-wasm`/WebAssembly) with plain React state, and
**pluggable local persistence** through the repo's canonical storage layer.

This is a real npm package (not a throwaway emit): dependencies, tests, and build
config are committed here, and `scripts/build-web` only emits the generated
`TaskApp.tsx` component + the wasm runtime *into* it. That makes persistence — and
every later super-app phase — reliably buildable and testable.

## Architecture

```
TaskApp.mil / .mll / .msl                 (Mosaic: interface / layout / style)
        │  mosaic-compile --backend react  (emits ONE component file)
        ▼
   src/TaskApp.tsx                         (generated React component: { ...slotProps, dispatch })
        │  src/main.tsx wires it to…
        ▼
   createTaskEngine (task-wasm)  ──►  task-core (pure Rust engine) via WASM
        │  src/persistence.ts persists via…
        ▼
   @coding-adventures/storage  ──►  IndexedDBStorage  (in-memory fallback)
```

The engine is a **pure computation** — no I/O, no clock, no storage. Persistence is
**host-owned** and lives entirely in `src/persistence.ts`, behind the pluggable
`KVStorage` contract, so swapping IndexedDB for SQLite / a sync server / cloud later
is a one-line change here. See `code/specs/task-app-super-app.md` (§2.6, Phase 1).

## What it does

- Add a task (name + optional `YYYY-MM-DD` due date).
- Tasks are chained into a work queue and **auto-scheduled** by the CPM engine — each
  starts when the previous finishes, on working days, with a projected finish date.
- Tasks scheduled to finish after their due date are flagged **overdue**.
- Complete (✓) or delete a task.
- **Everything persists**: the whole workspace is saved to IndexedDB after each change
  and restored on reload (falls back to in-memory storage where IndexedDB is absent).

## Develop

```bash
# from code/programs/mosaic/task-app
bash scripts/build-web.sh          # build wasm, emit TaskApp.tsx + copy runtime here
cd host/web && npm install && npm run dev   # http://localhost:5173
```

## Test

```bash
npm install && npm test            # vitest — persistence round-trips (jsdom)
```

The persistence seam is unit-tested here; the live IndexedDB path and the full
add → schedule → reload → restore loop are verified end-to-end in a browser.
