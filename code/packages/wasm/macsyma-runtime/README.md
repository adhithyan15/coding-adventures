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
