# journal-wasm

The JavaScript boundary for **Journal**: a linear-memory WebAssembly ABI over the
pure [`journal-core`](../journal-core) engine, used by the web build, Electron,
and the Mosaic `journal-app` (J3 of
[#14416](https://github.com/adhithyan15/coding-adventures/issues/14416)).

## Where this sits in the stack

```
journal-core        pure engine: model + commands + projections
  └ journal-wasm    ← you are here: JSON over wasm linear memory
      …consumed by the Mosaic journal-app (J3)
```

It follows the repo's `*-wasm` convention exactly as `task-wasm` does
(`alloc`/`dealloc`, `(ptr, len)` JSON in, `[u32 LE len][JSON]` out) and adds **no
model logic** — it parses, calls `journal-core`, and serialises. The contract is
[`code/specs/journal-wasm.md`](../../../specs/journal-wasm.md).

## Usage from JavaScript

```js
import { createJournalEngine } from "./js/journal-engine.mjs";

const engine = createJournalEngine(await (await fetch("journal_engine.wasm")).arrayBuffer());
engine.init({ journalId: crypto.randomUUID(), name: "Personal" });
engine.apply({ type: "createEntry", id: crypto.randomUUID(), journal: /* id */ "…",
               date: "2026-09-23", title: "First light", body: "Walked to the lighthouse." });

engine.timeline();              // { ok: true, data: [{ date, entries: [...] }] }
engine.search("lighthouse");    // ranked hits with one-line snippets
engine.onThisDay("2026-09-23"); // earlier years on this date

localStorage.setItem("journal", engine.snapshot());
engine.load(localStorage.getItem("journal")); // refused whole unless it validates
```

Every call returns `{ ok: true }`, `{ ok: true, data }`, or
`{ ok: false, error, code }` — branch on `code`. Returned text is plain text:
escape it before rendering as HTML.

## Building and testing

```sh
cargo test -p journal-wasm      # the ABI logic, natively
./build-wasm.sh                 # → pkg/journal_engine.wasm (wasm32-unknown-unknown)
node js/smoke.mjs               # drive the real wasm end to end
```
