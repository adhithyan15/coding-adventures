# macsyma-runtime-wasm

WebAssembly bindings for the Rust MACSYMA runtime.

The exported API is intentionally JSON-string based so browser and TypeScript
callers can consume a stable schema without depending on Rust enum layouts:

- `evalSource(source)` evaluates one source string with a fresh session.
- `new WasmMacsymaSession().eval(source)` keeps bindings and history across
  calls.
- `historyJson()` exposes the current history counters and last output.
- `resetHistory()` clears `%i`/`%o` history without rebuilding the session.

Build with `wasm-pack build --target web` or `wasm-pack build --target nodejs`.

For browser TypeScript callers, build this package with `wasm-pack --target web`
and load the generated module through
`@coding-adventures/macsyma-wasm-runtime`:

```ts
import { loadMacsymaWasmRuntime } from "@coding-adventures/macsyma-wasm-runtime";

const runtime = await loadMacsymaWasmRuntime(() => import("./pkg/macsyma_runtime_wasm.js"));
const result = runtime.eval("x : 5$\nx + 2;");
```
