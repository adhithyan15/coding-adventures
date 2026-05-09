# MACSYMA WASM Browser Runtime

## Goal

Provide a browser-consumable WASM path for the Rust MACSYMA implementation
without replacing the pure TypeScript runtime. The browser should be able to
choose either:

- `@coding-adventures/macsyma-runtime` for a pure JavaScript implementation.
- `macsyma-runtime-wasm` plus `@coding-adventures/macsyma-wasm-runtime` for a
  Rust/WASM implementation.

Both pathways must expose session-oriented evaluation, history counters, visible
output filtering, and stable structured IR payloads.

## Existing Rust Layers

`code/packages/rust/macsyma-wasm` owns the JSON facade over the typed Rust
runtime:

- `MacsymaWasmSession::eval_json(source)` evaluates against persistent bindings
  and `%i`/`%o` history.
- `MacsymaWasmSession::history_json()` exposes counters and last output.
- `eval_source_json(source)` evaluates in a fresh one-shot session.

`code/packages/wasm/macsyma-runtime` owns the `wasm-bindgen` boundary:

- `new WasmMacsymaSession()`
- `session.eval(source)`
- `session.historyJson()`
- `session.resetHistory()`
- `evalSource(source)`

## Browser TypeScript Contract

`@coding-adventures/macsyma-wasm-runtime` is the browser-side bridge over the
generated `wasm-pack --target web` module. It intentionally does not check in
generated `pkg/` output or a `.wasm` binary.

The bridge accepts a caller-supplied module loader:

```ts
const runtime = await loadMacsymaWasmRuntime(() => import("./pkg/macsyma_runtime_wasm.js"));
```

The bridge must:

- Run the generated module's default initializer when present.
- Create a stateful `WasmMacsymaSession`.
- Parse Rust snake_case JSON into camelCase TypeScript objects.
- Preserve error payloads as data instead of throwing for normal compile
  failures.
- Throw only when the WASM bridge returns malformed JSON or an invalid schema.

## Validation

The WASM package `BUILD` must prove both native wrapper tests and the actual
`wasm32-unknown-unknown` release build:

```sh
rustup target add wasm32-unknown-unknown
rustup run stable cargo test -- --nocapture
rustup run stable cargo build --target wasm32-unknown-unknown --release
```

The TypeScript bridge `BUILD` must install from lockfile, typecheck, and run
coverage tests.

## Next Phase

Once this bridge package lands, a browser demo can reuse the existing
`macsyma-browser-repl` UI with a runtime selector:

- Pure TypeScript session.
- Rust/WASM session loaded from a locally generated `pkg/` bundle.

That UI should keep generated WASM output out of git unless the repo introduces
an explicit artifact policy for committed browser WASM bundles.
