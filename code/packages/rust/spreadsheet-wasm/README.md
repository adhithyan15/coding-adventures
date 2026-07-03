# spreadsheet-wasm

The **`extern "C"` + linear-memory ABI** that lets a JavaScript host drive the
spreadsheet engine after it is compiled to `wasm32-unknown-unknown`. It is the
boundary, not the logic — it owns string marshalling across WASM linear memory
and nothing else. All behaviour lives below it:

```text
  JS host  ──(ptr,len strings)──▶  spreadsheet-wasm        ← this crate (extern "C")
                                        │
                                   spreadsheet-core-wasm    ← JSON facade
                                        │
                                   spreadsheet-core         ← cells, graph, recalc
```

This is the repo's **zero-dependency** WASM convention: **no `wasm-bindgen`**,
no wasm-pack, no third-party FFI framework. Just hand-written
`#[no_mangle] extern "C"` exports and a tiny memory protocol the JS loader
mirrors.

## Memory protocol

Linear memory is a flat byte array shared with JS; strings cross it as
`(ptr, len)`:

- **JS → WASM:** JS calls `alloc(len)`, writes `len` UTF-8 bytes there, passes
  `(ptr, len)`, and frees with `dealloc(ptr, len)` after the call.
- **WASM → JS:** each string export returns a pointer to a
  `[len: u32 LE][utf8 bytes]` buffer; JS reads the length, then the bytes, then
  frees the whole thing with `dealloc(ptr, 4 + len)`.

Every allocation uses an explicit `Layout` with `align = 1`, and `dealloc`
rebuilds the *same* layout from the `len` JS passes back — so allocation and
free can never disagree on size (the classic source of unsoundness in
hand-rolled WASM ABIs). The unsafe blocks are confined to this marshalling and
documented inline.

## Exports

`reset()`, `set_cell(a1, raw)`, `get_value(a1)`, `get_raw(a1)`, `get_values()`
— one per [`SpreadsheetSession`](../spreadsheet-core-wasm) method. A single
global session lives in thread-local storage (WASM is single-threaded);
`reset` starts a fresh sheet.

## Building & using

```bash
# Compile to pkg/spreadsheet_engine.wasm (one-time: rustup target add wasm32-unknown-unknown)
bash build-wasm.sh

# Prove it computes, end-to-end, in Node:
node js/smoke.mjs
```

The JS loader [`js/spreadsheet-engine-wasm.mjs`](js/spreadsheet-engine-wasm.mjs)
instantiates the `.wasm` and presents the **same API as the TypeScript
engine**, so a host can swap one for the other unchanged:

```js
import { createEngine } from "./js/spreadsheet-engine-wasm.mjs";
const engine = createEngine(wasmBytes);     // bytes from fetch() or embedded base64
const wb = engine.createSpreadsheet();
wb.setCell("B6", "=SUM(B1:B5)");
wb.getValue("B6");   // { kind: "number", value: 46 }
wb.getRaw("B6");     // "=SUM(B1:B5)"
```

It is dependency-free and runs in both Node and the browser; the caller
supplies the raw `.wasm` bytes, so it works from `file://` (embedded bytes) or
a server (fetched buffer). The committed `pkg/spreadsheet_engine.wasm` is the
artifact the VisiCalc demos consume.

## Tests

- `cargo test -p spreadsheet-wasm` — host-target tests that drive the
  `(ptr, len)` protocol by hand (so the marshalling and alloc/dealloc pairing
  are exercised without a WASM toolchain).
- `node js/smoke.mjs` — loads the real `.wasm` and checks it computes the same
  results as the Rust and TypeScript engines.
