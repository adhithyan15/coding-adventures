# Changelog

All notable changes to `matrix-rust-napi` are documented here.  The
format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] — 2026-05-18

### Added — MX07 Phase 1: crate skeleton + Graph JSON round-trip

First cut of the Node.js N-API addon for the Rust matrix execution
layer.  Establishes the build pipeline, the napi boundary, and the
JSON wire format as the interop surface.  Real execution
(`Runtime::run` on `matrix-cpu`) lands in Phase 2.

**Exported function**

```javascript
const m = require("./matrix_rust_napi.node");

const out = m.graphRoundTripJson(jsonString);
// jsonString -> matrix_ir::Graph -> jsonString
```

`graphRoundTripJson` decodes its input via `matrix-ir-json::decode`,
then re-encodes via `matrix-ir-json::encode`.  Malformed JSON throws
a `JS Error`.

**Pure-Rust core**

The work happens in `pub fn round_trip_json(input: &str) ->
Result<String, String>`, which is unit-testable without a Node
toolchain:

* `round_trip_preserves_graph_under_binary_wire_format` — a small
  ReLU layer round-trips and the result is byte-equal to the
  original under `matrix-ir`'s binary wire format.
* `round_trip_handles_multi_op_graph` — multi-op graph
  (`Add + Mul + Neg`) round-trips intact.
* `round_trip_rejects_garbage_json` — non-JSON input returns `Err`.
* `round_trip_rejects_unsupported_version` — schema-version
  mismatch returns `Err`.
* `round_trip_is_idempotent` — `round_trip_json(round_trip_json(x))
  == round_trip_json(x)`.

The N-API wrapper (`napi_graph_round_trip_json`) is one
un-marshal + one call + one marshal.  Errors thrown via
`node-bridge::throw_error` become JS exceptions.

### Why `node-bridge` instead of `napi-rs`

The workspace convention (established by `font-parser-node`) is to
depend on the sibling `node-bridge` crate — a zero-dep safe Rust
wrapper over the raw N-API `extern "C"` interface — rather than
pulling in `napi-rs` / `napi-sys` / `napi-derive`.  N-API is
ABI-stable; extern declarations work on every Node version that
supports the targeted N-API revision.

The MX07 spec originally cited `napi-rs`; it has been updated in
this same PR to reflect the workspace pattern.  See the spec's
"Open questions" and "Where the crate lives" sections.

### Dependencies

```toml
matrix-ir       = { path = "../matrix-ir" }       # zero-dep IR types
matrix-ir-json  = { path = "../matrix-ir-json" }  # JSON wire format
node-bridge     = { path = "../node-bridge" }     # raw N-API wrappers
```

No external crates from crates.io.  The upstream crates remain
MX00-zero-dep; this addon is the workspace's binding edge to Node.

### Crate type

`crate-type = ["cdylib"]` — single output that Node.js renames from
`libmatrix_rust_napi.{dylib,so,dll}` to `matrix_rust_napi.node` and
`require`s.  Matching `font-parser-node`'s shape.

### Out of scope (deferred to later phases)

* `Graph` / `Runtime` classes with handle-based API — Phase 2.
* Actual execution on `matrix-cpu` — Phase 2.
* `package.json` + per-platform prebuilt-binary workflow — Phase 3.
* TypeScript wrapper package — Phase 4.
* `typescript/matrix` refactor — Phase 5 (separately scoped as MX08).
* Async / `runAsync()` — v1+ once profiling shows it matters.
* Zero-copy output buffers — v1+.

### Security

* No `unsafe` outside the `extern "C"` shim layer.
* The N-API wrapper validates argument count and type before
  un-marshalling; mismatched shape throws a JS error rather than
  panicking.
* All decoder fallibility is mapped to `Result<_, String>` and then
  to a JS `Error`; no panics reachable from adversarial input.
* `required_capabilities.json` is empty: the addon opens no
  filesystem, network, process, or environment access.
