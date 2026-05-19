# matrix-rust-napi

Node.js N-API addon exposing the Rust matrix execution layer
(`matrix-ir`, `matrix-ir-json`, future `matrix-runtime` /
`matrix-cpu` / `matrix-metal` / `matrix-cuda`) to JavaScript and
TypeScript.

This is **Phase 1** of [MX07](../../../specs/MX07-matrix-rust-napi.md)
— the minimal crate skeleton that proves the build pipeline and
the JSON wire format survive the napi boundary.  Real execution
lands in Phase 2.

## What's here

Two exported functions:

```javascript
const m = require("./matrix_rust_napi.node");

// Phase 1 — JSON round-trip smoke (validates that the matrix-ir-json
// schema survives the napi boundary).
const roundTripped = m.graphRoundTripJson(jsonString);
// jsonString -> matrix_ir::Graph -> jsonString

// Phase 2 — actually execute the graph on the Rust CPU executor.
// Envelope shape:
//   { "graph": <matrix-ir-json schema>,
//     "inputs": [ "<lowercase-hex bytes>", ... ] }
// Returns:
//   { "outputs": [ "<lowercase-hex bytes>", ... ] }
const result = m.runGraphOnCpu(JSON.stringify({
  graph: { matrix_ir_version: 1, /* ... */ },
  inputs: ["3f8000003f000000", /* ... */],   // hex-encoded f32 inputs
}));
const { outputs } = JSON.parse(result);
// outputs[0] is a hex string of the first output tensor's bytes.
```

`runGraphOnCpu` runs the full plan + allocate + upload + dispatch +
download pipeline through `matrix-runtime` and `matrix-cpu`, with
the planner's BufferIds rewritten to the executor's real BufferIds
in place.

### Why hex-encoded inputs instead of `Buffer[]`?

Per MX07 §"Phases", real `Buffer[]` marshalling is Phase 2b — it
requires extending `node-bridge` with Buffer helpers (`napi_create_buffer`,
`napi_get_buffer_info`, finalizer plumbing).  For Phase 2 we keep
the napi surface as one string-in, string-out function (the same
pattern as `graphRoundTripJson`), with hex bytes inside the JSON
envelope.  The wire cost is 2× the raw bytes plus JSON overhead;
fine for the proof-of-concept, replaced in Phase 2b.

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
| 1 | `graphRoundTripJson(jsonString)` — round-trip smoke. |
| 2 (this PR) | `runGraphOnCpu(envelopeJson)` — full plan + allocate + dispatch + download pipeline on `matrix-cpu`. JSON-envelope I/O (hex-encoded bytes). |
| 2b | Replace JSON envelope with real `Buffer[]` marshalling (extend `node-bridge` with `napi_create_buffer` / `napi_get_buffer_info` helpers; bind `Graph` + `Runtime` as handle-based JS classes). |
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
