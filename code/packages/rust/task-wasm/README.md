# task-wasm

The **browser / Electron boundary** for task-app: a thin linear-memory WebAssembly ABI
that surfaces the pure [`task-core`](../task-core) engine to JavaScript.

## No facade, on purpose

Per [`task-app-architecture.md`](../../../specs/task-app-architecture.md), the engine is
**pure computation** and each backend consumes it **natively**. So this ABI is *not* a
`dispatch`/`getProps`/`handleEvent` bus — it exposes the engine's own functions, **one
export per operation and query**. The web/React host keeps the engine object in React
state and re-renders on change: idiomatic React, with the React-isms confined to the
web backend where they belong.

## The wire protocol

The repo-standard linear-memory protocol:

- `alloc(len) -> ptr` / `dealloc(ptr, len)` — caller-managed buffers.
- Inputs are `(ptr, len)` UTF-8 JSON; outputs are a fresh `[u32 little-endian length]
  [JSON bytes]` block the caller reads and frees.
- Every result is a JSON envelope: `{ "ok": true }` / `{ "ok": true, "data": … }` on
  success, `{ "ok": false, "error": …, "code": … }` on a rejected operation. Nothing
  traps the boundary.

Exports mirror the engine: operations (`create_task`, `reparent`, `link_dependency`,
`set_duration`, `assign`, `set_field_value`, `answer_decision`, …) and queries
(`checklist`, `todos`, `kanban`, `gantt`, `flowchart`, `schedule`), plus lifecycle
(`reset`, `snapshot`, `load`).

## JavaScript accessor

[`js/task-engine.mjs`](js/task-engine.mjs) wraps the ABI:

```js
import { createTaskEngine } from "./js/task-engine.mjs";

const engine = createTaskEngine(wasmBytes);
engine.createTask({ id: "a", name: "Write spec" });     // → { ok: true }
engine.setDuration({ id: "a", duration: { workingMinutes: 480, elapsed: false } });
engine.checklist();                                     // → { ok: true, data: [...] }
engine.gantt(projectStartDays);                         // → { ok: true, data: { bars, projectFinish } }
const snap = engine.snapshot();  engine.load(snap);     // host-owned persistence
```

## Building & testing

```bash
cargo test -p task-wasm      # native unit tests of the ABI logic
./build-wasm.sh              # → pkg/task_engine.wasm
node js/smoke.mjs            # end-to-end JS ↔ wasm round-trip
```

The engine (including the formula CAS) compiles cleanly to `wasm32-unknown-unknown`.

## How it fits the stack

```
task-core          the pure engine (operations + queries + scheduler + formula)
  └ task-wasm      ← you are here: WASM ABI for web / Electron
  └ task-capi      C ABI sibling for native shells (SwiftUI / Compose / Qt)

Mosaic UI (authored once) is wired to this engine natively per backend.
```
