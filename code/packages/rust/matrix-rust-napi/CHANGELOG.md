# Changelog

All notable changes to `matrix-rust-napi` are documented here.  The
format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [unreleased] - shared CPU graph helper

The Node-free graph planner/executor now lives in `matrix-cpu`; this N-API
binding re-exports and calls that shared helper while keeping host-specific
symbols at the outer edge.

## [0.5.0] — 2026-05-18

### Fixed — CRITICAL: class constructors stored as stale `napi_value` instead of persistent `napi_ref`

Surfaced during MX07 Phase 4 vitest end-to-end runs.  The class
constructors for `Graph` and `Runtime` were stored in `AtomicUsize`
as raw `napi_value` (a **local handle** valid only inside the
current handle scope).  When a later JS-triggered callback
(`Graph.fromJson(...)` or `Runtime.create()`) loaded the stored
value and passed it to `napi_new_instance`, the call returned
`napi_invalid_arg` (status 1) — because the local handle had
been invalidated when its scope ended.  Both static methods
silently returned `undefined`.

Fix.  Switched `GRAPH_CTOR` / `RUNTIME_CTOR` (now
`GRAPH_CTOR_REF` / `RUNTIME_CTOR_REF`) to store **`napi_ref`**, the
persistent equivalent:

* In `register()`, after `define_class`, wrap the class
  `napi_value` in a `napi_ref` via
  `napi_create_reference(env, class, 1, &mut ref)`.
* Each static-method callback calls a new `resolve_ctor` helper that
  reads the stored `napi_ref`, calls
  `napi_get_reference_value(env, ref, &mut value)` to get a
  scope-bound `napi_value`, and passes that to
  `napi_new_instance`.
* On any failure (null ref, `get_reference_value` error,
  `new_instance` error), throw a precise JS error instead of
  silently returning `undefined`.

This also tightens error reporting: the old static methods returned
`undefined` for any failure mode, which masked exactly the kind of
issue we hit.  The new ones throw with the failing N-API call name
and status code.

A `lessons.md` entry was added so the next napi addon doesn't
re-discover this.  `font-parser-node` has the same latent bug
(`FONT_FILE_CTOR` stores a raw `napi_value`), but no existing test
in that crate reaches `napi_new_instance` (every input rejects
earlier via `fp::load`), so the bug never fired.  Filed for a
follow-up fix there.

## [0.4.0] — 2026-05-18

### Added — MX07 Phase 3: build pipeline + GitHub Actions smoke

Adds the build and CI plumbing that produces the
`matrix_rust_napi.node` artifact on each supported platform and
asserts it loads cleanly into Node.

**New files**

* `package.json` — npm scripts:
  - `build` runs `cargo build --release` then renames the per-
    platform shared library to `matrix_rust_napi.node` (handles
    `.dylib` / `.so` / `.dll` extensions).
  - `smoke` runs `build` then `node -e "require('./matrix_rust_napi.node')"`,
    asserting that all four exports are present
    (`graphRoundTripJson`, `runGraphOnCpu`, `Graph` class,
    `Runtime` class).
* `.github/workflows/matrix-rust-napi.yml` — GitHub Actions
  workflow with a path filter on the addon and its workspace
  dependencies (`node-bridge`, `matrix-ir`, `matrix-ir-json`,
  `matrix-runtime`, `matrix-cpu`, `compute-ir`,
  `executor-protocol`).  Matrix on `ubuntu-latest`
  (linux-x64-gnu) and `macos-latest` (darwin-arm64 since
  GitHub's M-series migration in 2024).  Each runner:
  1. Installs Rust stable + Node.js 20.
  2. Caches the cargo registry / git / workspace target dir.
  3. Runs `cargo test -p matrix-rust-napi --release` (22 unit
     tests).
  4. Runs `npm run smoke` to build the addon and verify it loads.
  5. Reports the artifact size as a quick sanity smoke.
* `.gitignore` — excludes the built `matrix_rust_napi.node`
  (regenerated per build) and `node_modules/` (used by the
  future Phase 4 wrapper).

**What's deliberately not in this PR**

* **No publish step.**  Publishing the per-platform prebuilts to
  npm is its own coordination (gated by manual approval), tracked
  separately.  This PR proves the build works; the publish
  workflow is a follow-up that calls `npm publish` after the
  smoke passes on all platforms.
* **No Windows.**  MX07 §"Distribution model" defers Windows to
  the post-v0 follow-up.  The package.json's `cp ... *.dll`
  branch is wired up so Windows works the moment we add it to
  the workflow matrix — the workflow just doesn't run it yet.
* **No `node --test` end-to-end suite.**  That's MX07 Phase 4,
  which adds the TypeScript wrapper package and the smoke.test.mjs
  that drives the addon through real JS code (constructs a
  `Graph`, builds a `Buffer[]`, calls `runtime.run`, asserts on
  the output bytes).  Phase 3 only proves the addon *loads*.

### Verification

Tested locally on darwin-arm64:

```
$ npm run smoke
   Compiling matrix-rust-napi v0.1.0 (...)
    Finished `release` profile [optimized] target(s) in 18.04s
matrix-rust-napi addon loaded; exports: graphRoundTripJson, runGraphOnCpu, Graph, Runtime
```

The `.node` artifact is ~2.5 MiB on macOS arm64 — reasonable for
a release build pulling matrix-ir + matrix-runtime + matrix-cpu +
matrix-ir-json + node-bridge + the executor protocol.

### Security

No new untrusted-input surface in this PR (it's a build/CI
addition).  The workflow runs only `cargo`, `npm`, and `node`
on the addon's own code on GitHub-hosted runners; standard
isolation, no secrets used, no external network calls beyond
crate registry fetches (which are cached).

## [0.3.0] — 2026-05-18

### Added — MX07 Phase 2b: Graph + Runtime JS classes with `Buffer[]` I/O

Replaces the Phase 2 JSON-hex envelope with the recommended
class-based API described in [MX07 §"The binding surface"][mx07].
The JSON-envelope `runGraphOnCpu` function remains as the
CLI-friendly / Buffer-free alternative path; both APIs flow through
the same pure-Rust `run_graph_on_cpu` helper internally, so
behaviour is identical.

[mx07]: ../../../specs/MX07-matrix-rust-napi.md

**New JS classes**

```javascript
const m = require("./matrix_rust_napi.node");

// Construct once, reuse many times — no JSON re-parse per run().
const graph = new m.Graph(jsonString);
//  or:  m.Graph.fromJson(jsonString)

graph.toJson();           // re-serialise to wire format
graph.describe();         // "Graph(tensors=4, ops=3, ...)"

const rt = new m.Runtime();
//  or:  m.Runtime.create()

const outputs = rt.run(graph, [inputBuf1, inputBuf2]);
// outputs: Array<Buffer> — one Buffer per graph.outputs() tensor.
```

**What landed**

* `src/classes.rs` — Graph and Runtime class definitions wiring
  N-API class registration (`napi_define_class`, `napi_wrap` with
  finalizer, `napi_unwrap` on every instance method, static-method
  attachment via `set_named_property` on the class itself).
* Static `AtomicUsize` storage for the class constructors so static
  methods (`Graph.fromJson`, `Runtime.create`) can look them up via
  `napi_new_instance` — mirrors font-parser-node's Finding 3.1 fix
  for Worker-thread safety.
* Both constructors check `napi_wrap` status BEFORE calling
  `Box::into_raw` would leak — mirrors font-parser-node's Finding
  3.5 fix.

**Why both class API and JSON envelope?**

The class API is preferred for any consumer with a real Node.js
`Buffer`:

* No JSON re-parse per `run()` call.
* No hex-encoding overhead (2× the byte cost).
* Mirrors the eventual cross-language API shape.

The JSON envelope is kept because some consumers genuinely don't
want or have `Buffer`:

* CLI tools that just pipe graph JSON over stdin / stdout.
* Language hosts that haven't yet adopted the workspace's Buffer
  helpers.
* Debugging / golden-file fixtures where everything is text.

Removing it would force those callers to ship a Buffer codec.
Keeping it costs ~100 lines of code; both paths share the same
`run_graph_on_cpu` core, so they can never disagree.

### New dependencies

None — the existing `node-bridge` Buffer helpers (added in PR
#3529) cover everything the class API needs.

### Tests (18, unchanged)

The pure-Rust `run_graph_on_cpu` suite covers the execution path
behind both APIs.  JS-side end-to-end tests for the class API land
with **Phase 4** (`node --test` smoke).  Until then, the
build-succeeds + class-registration-doesn't-panic check on the
shared cdylib is the smoke test (font-parser-node uses the same
strategy).

### Security

The Phase 2b class wrappers were security-reviewed by a senior
Rust-N-API sub-agent.  One CRITICAL finding (Type Confusion between
`Graph` and `Runtime`) was raised and **fixed in this same PR**.
Final review notes:

* **CRITICAL fix — type-tag discriminator.**  `napi_unwrap` is
  type-agnostic by N-API design: it returns whichever raw pointer
  was stored by *any* previous `napi_wrap` in the env, regardless
  of which JS class the object belongs to.  Without a software
  type tag, a JS caller could do `rt.run(rt, [])` and have
  `unwrap_graph` return a `Box<WrappedRuntime>` pointer cast as
  `&Graph` — reading `graph.tensors.len()` then reads `(ptr, len,
  cap)` out of bounds.  Immediate UB; near-guaranteed crash or RCE.
  Fix: every napi_wrap'd payload in this crate (`WrappedGraph`,
  `WrappedRuntime`) starts with a 16-byte `tag: [u64; 2]` prefix
  with a class-specific constant.  `unwrap_graph` / `unwrap_runtime`
  validate the tag before dereferencing as the typed struct.  Both
  wrapped types share the `[u64; 2]` prefix layout so reading 16
  bytes from any pointer we stored is safe; cross-addon collision
  probability is ~2^-128.  The "right" long-term answer is N-API's
  `napi_type_tag_object` / `napi_check_object_type_tag` (v8+),
  deferred to a follow-up node-bridge PR.
* Static class-ctor storage uses `AtomicUsize` (Release/Acquire) —
  Worker-thread-safe.
* Both constructors check `napi_wrap` status before letting
  `Box::into_raw` leak — no leak on wrap failure.
* All instance methods validate `napi_unwrap` returned a non-null
  pointer AND that the type tag matches before dereferencing.
* `runtime.run` validates input count + reuses the
  `MAX_TOTAL_BUFFER_BYTES` cap from `run_graph_on_cpu` (Phase 2
  finding fix) — a malicious graph cannot DoS the host via huge
  tensors.
* All buffer transfers go through the node-bridge Buffer helpers
  with the copy-in / copy-out discipline (PR #3529) — no use-after-
  detach UB possible.

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
