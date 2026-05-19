# matrix-rust-napi

Node.js N-API addon exposing the Rust matrix execution layer
(`matrix-ir`, `matrix-ir-json`, future `matrix-runtime` /
`matrix-cpu` / `matrix-metal` / `matrix-cuda`) to JavaScript and
TypeScript.

This is **Phase 1** of [MX07](../../../specs/MX07-matrix-rust-napi.md)
— the minimal crate skeleton that proves the build pipeline and
the JSON wire format survive the napi boundary.  Real execution
lands in Phase 2.

## What's here (v0.1)

One exported function:

```javascript
const m = require("./matrix_rust_napi.node");

const roundTripped = m.graphRoundTripJson(jsonString);
// jsonString -> matrix_ir::Graph -> jsonString
```

`graphRoundTripJson` parses its input via the `matrix-ir-json`
crate, re-encodes the resulting `matrix_ir::Graph`, and returns the
canonical JSON form.  Malformed input throws a JS error.

That's deliberately tiny.  It proves three things — see the source
of `lib.rs` for the full rationale — and gives Phase 2 a known-good
build pipeline to extend.

## Why N-API instead of napi-rs?

The workspace convention (established by `font-parser-node`) is to
use **`node-bridge`**, a sibling crate that wraps the raw N-API
`extern "C"` interface in safe Rust.  Zero dependencies on
`napi-rs`, `napi-sys`, or `napi-derive`.  N-API is ABI-stable by
design (that is its entire purpose), so the extern declarations
just work on every Node version that supports the targeted N-API
revision.

This matches the broader workspace ethos: minimise the dependency
graph, depend on what's necessary, write the rest by hand.

Note: the MX07 spec originally mentioned napi-rs.  When the
implementation started, the workspace pattern won out; MX07 has
been updated in this same PR to record the divergence.

## What lands when

| Phase | Lands |
|-------|-------|
| 1 (this PR) | `graphRoundTripJson(jsonString)` — round-trip smoke. |
| 2 | `Graph` + `Runtime` classes; `runtime.run(graph, inputs)` on CPU. |
| 3 | `package.json` + workflow that builds the `.node` artifact on `darwin-arm64` and `linux-x64-gnu` and confirms it loads. |
| 4 | TypeScript wrapper package with `node --test` smoke. |
| 5 (MX08) | `typescript/matrix` refactor — separately scoped. |

## Building

```sh
cargo build -p matrix-rust-napi --release
```

The output `target/release/libmatrix_rust_napi.{dylib,so,dll}` is
what Node.js renames to `matrix_rust_napi.node` and loads via
`require()`.  Phase 3 adds the build script that does the rename
and the per-platform packaging.

## Testing

```sh
cargo test -p matrix-rust-napi
```

Tests cover the pure-Rust `round_trip_json` helper — no Node
toolchain required.  End-to-end Node-side tests land with Phase 4.

## Layout

```
matrix-rust-napi/
  Cargo.toml             # cdylib only; depends on matrix-ir,
                         # matrix-ir-json, node-bridge
  src/
    lib.rs               # round_trip_json (pure) + N-API wrapper
                         # + #[no_mangle] napi_register_module_v1
  BUILD                  # cargo test invocation
  BUILD_windows
  README.md              # this file
  CHANGELOG.md
  required_capabilities.json  # capabilities: []
```

## License

MIT
