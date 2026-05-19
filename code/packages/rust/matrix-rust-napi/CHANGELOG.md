# Changelog

All notable changes to `matrix-rust-napi` are documented here.  The
format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.2.0] — 2026-05-18

### Added — MX07 Phase 2: end-to-end execution on `matrix-cpu`

The napi addon can now actually **run** a `matrix_ir::Graph` on the
Rust CPU executor and return output tensor bytes.  Phase 1 was the
plumbing smoke test; Phase 2 connects the plumbing to a working
backend.

**New exported function**

```javascript
const result = m.runGraphOnCpu(JSON.stringify({
  graph: <matrix-ir-json schema>,
  inputs: ["<lowercase-hex>", "..."],
}));
const { outputs } = JSON.parse(result);
// outputs[i] is a hex string of the i-th output tensor's LE bytes.
```

**New pure-Rust helper**

`pub fn run_graph_on_cpu(graph: &Graph, inputs: &[Vec<u8>]) ->
Result<Vec<Vec<u8>>, String>` — the bare-bones planning + execution
glue.  Unit-tested without any Node toolchain.

**Pipeline**

```text
matrix_ir::Graph
   │ matrix_runtime::Runtime::plan()
   ▼
compute_ir::ComputeGraph (planner-assigned BufferIds)
   │ allocate one CpuExecutor buffer per planner-BufferId,
   │ remember the planner→real BufferId map
   ▼
ComputeGraph with real BufferIds (rewritten in place)
   │ upload constants → upload caller inputs → Dispatch → download outputs
   ▼
Vec<Vec<u8>>  (one byte vector per graph.outputs())
```

The helper pre-allocates every buffer up front and treats the
planner's `PlacedOp::Alloc` / `Free` lifetime annotations as
no-ops.  Per `compute-ir`'s spec, executors that manage their own
allocations may do exactly that — and `CpuExecutor` already does.
We trade a bit of peak memory for a far simpler glue layer that
doesn't need to thread the planner→real BufferId map into the
executor.  Phase 2b can revisit if memory pressure becomes real.

### New dependencies

```toml
matrix-runtime                     = { path = "../matrix-runtime" }
matrix-cpu                         = { path = "../matrix-cpu" }
compute-ir                         = { path = "../compute-ir" }
executor-protocol                  = { path = "../executor-protocol" }
coding-adventures-json-value       = { path = "../json-value" }
coding-adventures-json-serializer  = { path = "../json-serializer" }
```

All workspace path-only deps; no crates.io additions.

### Tests (16 total: 5 from Phase 1 + 11 new)

End-to-end execution:

* `add_two_vectors_executes_end_to_end` — element-wise add f32×3.
* `matmul_2x2_executes_end_to_end` — 2×2 MatMul (canonical example).
* `relu_layer_executes_end_to_end` — `max(0, x @ W + b)` with two
  constants (W identity + b bias) plus the zero comparison tensor.
  Proves constant upload + multi-op chaining.
* `rejects_wrong_input_count` — wrong input arity fails cleanly.
* `rejects_wrong_input_byte_length` — wrong input shape/dtype fails
  cleanly (catches the most common caller bug — wrong dtype on the
  Node side — with a precise error before any execution).

JSON envelope wrapping:

* `envelope_runs_add_end_to_end` — full envelope-shaped flow,
  including hex-encode/decode round-trip.
* `envelope_rejects_missing_graph` — schema validation.
* `envelope_rejects_non_array_inputs` — schema validation.

Hex codec (envelope payload):

* `hex_round_trips` — `bytes → hex → bytes` is identity.
* `hex_decoder_rejects_odd_length` — malformed input rejected.
* `hex_decoder_rejects_bad_chars` — non-hex characters rejected.

### Why the JSON envelope (and not real `Buffer[]`)?

Per MX07 §"Phases", real `Buffer[]` marshalling is **Phase 2b** —
it requires extending `node-bridge` with Buffer helpers
(`napi_create_buffer`, `napi_get_buffer_info`, finalizer plumbing).
For Phase 2 we keep the napi surface as one string-in, string-out
function (the same pattern as `graphRoundTripJson`), with hex
bytes inside the JSON envelope.  The wire cost is 2× the raw
bytes plus JSON overhead; perfectly fine for the proof-of-concept,
replaced in Phase 2b when the workspace's `node-bridge` grows
Buffer helpers.

The class-based `Graph` + `Runtime` API from MX07 §"The binding
surface" lands with Phase 2b too — once Buffer marshalling works
there's a natural place to wrap each handle.

### Out of scope (still deferred)

* `Graph` / `Runtime` JS classes — Phase 2b.
* Real `Buffer[]` I/O — Phase 2b.
* GPU executors (`matrix-metal`, `matrix-cuda`) — picked up
  transparently once the runtime planner has them registered;
  no napi changes needed.
* `package.json` + per-platform prebuilt-binary workflow — Phase 3.
* Async / `runAsync()` — v1+ once profiling shows it matters.

### Security

Security-reviewed by a senior-Rust-N-API security sub-agent.  One
MEDIUM finding (unbounded buffer allocation enabling DoS) and one
LOW finding (defensive planner-invariant check) were raised and
**fixed in this same PR**.  Final review notes:

* **`MAX_TOTAL_BUFFER_BYTES = 4 GiB` cap.**  Before any
  `AllocBuffer` call, `run_graph_on_cpu` sums
  `t.shape.byte_size(t.dtype)` across every placed tensor and
  rejects the graph if the total exceeds the cap.  Without this,
  a ~500-byte JSON envelope could declare a tensor like
  `shape=[1_000_000_000, 1_000_000_000], dtype=F32` — passes
  `Graph::validate()` (the validator only catches `u64` overflow,
  ~18 EB) — and flow `bytes` into `vec![0u8; bytes]` inside
  `matrix-cpu`'s `BufferStore`, triggering a process abort via
  `handle_alloc_error`.  The cap fires *before* any allocation.
  Tested by `rejects_graph_exceeding_total_buffer_cap` and
  `rejects_graph_with_oversized_output`.
* **Defensive `placed.inputs.get(i)`.**  The upload loop now uses
  `.get(i).ok_or(...)?` instead of `[i]` indexing, and an explicit
  `placed.inputs.len() == graph.inputs.len()` invariant check runs
  immediately after `plan(graph)`.  Belt-and-braces against any
  future planner regression — panics across the FFI boundary are UB.
* No `unsafe` outside the standard extern "C" napi shim layer.
* Hex decoder validates length parity AND character range; rejects
  malformed input with precise errors.
* Envelope schema validation catches every common shape mismatch
  (missing fields, wrong types) before any execution starts.
* Input arity + byte-length checks run *before* any executor calls;
  no half-completed dispatches on caller errors.
* No `unwrap()` / `expect()` reachable from adversarial input.
* No `Box::into_raw`, no `napi_wrap`, no finalizer plumbing — pure
  value semantics across the FFI boundary, so no leaks or
  use-after-free possible.

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
