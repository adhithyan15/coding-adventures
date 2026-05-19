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

Two exported functions and two classes:

```javascript
const m = require("./matrix_rust_napi.node");

// ── Phase 2b: recommended API — class-based, Buffer[] I/O ──
//
// Construct the Graph once, run it many times with raw Buffer I/O.
// No JSON re-parsing per call, no hex encoding overhead.
const graph = new m.Graph(jsonString);
//  or:  m.Graph.fromJson(jsonString)   (static-method sugar)

const summary = graph.describe();
// "Graph(tensors=4, ops=3, inputs=1, outputs=1, constants=2)"

const json = graph.toJson();
// re-serialise back to the matrix-ir-json wire format

const rt = new m.Runtime();
//  or:  m.Runtime.create()             (static-method sugar)

const outputs = rt.run(graph, [inputBuf1, inputBuf2]);
// outputs is an Array<Buffer> — one Buffer per graph.outputs() tensor,
// each containing the tensor's little-endian byte payload.

// ── Phase 1: JSON-string utility — validation / debugging ──
const roundTripped = m.graphRoundTripJson(jsonString);
// Useful as a JSON-schema validator: throws if the graph JSON is malformed.

// ── Phase 2: JSON-envelope alternative — CLI-friendly, no Buffers ──
//
// Kept as a Buffer-free path for environments where exchanging Node
// `Buffer` objects is awkward (CLI tools that just want to JSON-pipe
// graphs over stdin, language hosts that don't have a Buffer concept).
const result = m.runGraphOnCpu(JSON.stringify({
  graph: { matrix_ir_version: 1, /* ... */ },
  inputs: ["3f8000003f000000", /* hex-encoded f32 input */],
}));
const { outputs } = JSON.parse(result);
```

The class-based API and the JSON-envelope API are semantically
equivalent — they both flow through the same `run_graph_on_cpu`
pure-Rust helper, which does the full plan → allocate → upload →
dispatch → download pipeline through `matrix-runtime` and
`matrix-cpu`.

### Why the class-based API is preferred

* **No JSON re-parse per call.**  `new Graph(json)` pays the parse
  cost once; subsequent `rt.run(graph, ...)` calls reuse the parsed
  value.  Critical when running the same graph in a hot loop.
* **Raw bytes via `Buffer`.**  No hex encoding overhead.  Tensors
  flow through as the same little-endian bytes the Rust executors
  use natively.
* **Mirrors the eventual Graph/Runtime API surface across other
  language bindings** (`python-bridge`, `ruby-bridge`, etc. as the
  workspace adds them).

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
| 2 | `runGraphOnCpu(envelopeJson)` — full plan + allocate + dispatch + download pipeline on `matrix-cpu`. JSON-envelope I/O (hex-encoded bytes). |
| 2b | `Graph` + `Runtime` JS classes with `Buffer[]` I/O via the node-bridge Buffer helpers (added in PR #3529). |
| 3 (this PR) | `package.json` + GitHub Actions workflow that builds the `.node` artifact on `darwin-arm64` and `linux-x64-gnu` and confirms it loads. |
| 4 | TypeScript wrapper package with `node --test` smoke. |
| 5 (MX08) | `typescript/matrix` refactor — separately scoped. |

## Building

The addon ships a `package.json` with two convenience scripts:

```sh
cd code/packages/rust/matrix-rust-napi

npm run build    # cargo build --release + rename .dylib/.so/.dll -> .node
npm run smoke    # build + load + assert all 4 exports are present
```

Under the hood, `npm run build` runs `cargo build --release`
(which produces `code/packages/rust/target/release/libmatrix_rust_napi.{dylib,so,dll}`)
and copies it to `matrix_rust_napi.node` alongside `package.json`.
That's the file Node.js loads via `require("./matrix_rust_napi.node")`.

CI runs `npm run smoke` on `ubuntu-latest` (linux-x64-gnu) and
`macos-latest` (darwin-arm64) for every PR that touches the addon
or its workspace dependencies, ensuring the .node artifact
actually loads.  See
[`.github/workflows/matrix-rust-napi.yml`](../../../../.github/workflows/matrix-rust-napi.yml).

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
