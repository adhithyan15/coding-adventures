# MX07 — `matrix-rust-napi`: Node.js binding to the Rust matrix execution layer

## Why this spec exists

[ARCH02](./ARCH02-rust-native-execution-backbone.md) Phase 2 calls for
a Rust crate that exposes the matrix execution layer (`matrix-cpu`,
and transparently `matrix-metal` / `matrix-cuda` via `matrix-runtime`)
to Node.js TypeScript via `napi-rs`.  This spec pins down **how**.

Reading ARCH02 alone leaves several decisions open:

1. **Where does the crate live?**  Under `code/packages/rust/` like
   every other Rust crate, or under `code/packages/typescript/` next
   to the wrapper package?
2. **What is the binding surface?**  Pass `Graph` instances by handle,
   or by JSON wire format?  Expose individual ops, or only whole-graph
   execution?  Sync or async?
3. **How are `.node` binaries distributed?**  Build at consumer
   install time (requires the consumer to have a Rust toolchain), or
   prebuilt per-platform and uploaded to npm under the per-platform
   `@coding-adventures/matrix-rust-napi-<os>-<arch>` package family
   that napi-rs's tooling expects?
4. **How does CI prove the binding works?**  Just `cargo test`, or a
   round-trip test that builds the `.node` file and exercises it from
   Node.js?
5. **How does this not break the MX00 zero-dependency mandate?**
   `matrix-ir` / `matrix-runtime` / `matrix-cpu` must stay zero-dep;
   the napi crate is downstream and gets to pull in `napi`, `napi-derive`,
   `napi-build`, and whatever they transitively want.
6. **How does this interact with `typescript/matrix`'s existing
   `CpuMatrixBackend`?**

This spec answers each.

---

## Where the crate lives

```
code/packages/rust/matrix-rust-napi/
  Cargo.toml             # cdylib only (workspace convention)
  BUILD                  # cargo test invocation
  BUILD_windows
  CHANGELOG.md
  README.md
  package.json           # Phase 3: lists the per-platform .node packages
  src/
    lib.rs               # round_trip_json (Phase 1) + future
                         # Graph / Runtime wrappers + N-API exports
  required_capabilities.json
  __test__/
    smoke.test.mjs       # node --test (lands with Phase 4)
```

It lives under `code/packages/rust/` because it **is** a Rust crate
(`Cargo.toml`, `src/lib.rs`, `cargo test`).  The fact that its
build output ends up in a Node.js consumer is incidental — the same
way `font-parser-node` is a Rust crate that happens to be consumed by
a Node-side wrapper.

### Phase 2 deviation: JSON-envelope I/O before `Buffer[]`

ARCH02 and MX07's §"The binding surface" both describe the eventual
shape — `Runtime.run(graph: Graph, inputs: TypedArray[]):
TypedArray[]` — with `Graph` and `Runtime` as napi-wrapped classes
and inputs as `Buffer` / `TypedArray` views.

But the workspace's `node-bridge` crate doesn't yet expose Buffer
helpers (`napi_create_buffer`, `napi_get_buffer_info`).  Adding them
is straightforward but is its own change with its own review surface.

Rather than gate Phase 2's execution work on that extension, Phase 2
ships a **JSON-envelope-shaped** binding:

```javascript
const result = m.runGraphOnCpu(JSON.stringify({
  graph: { matrix_ir_version: 1, /* ... */ },
  inputs: ["<lowercase-hex>", "..."],  // byte payloads as hex strings
}));
const { outputs } = JSON.parse(result);
// outputs[i] is a hex string of the i-th output tensor's LE bytes.
```

This keeps the napi surface as **one string-in, string-out function**
(identical pattern to Phase 1's `graphRoundTripJson`), proves the
end-to-end execution path works through the napi boundary, and
defers the Buffer marshalling work to **Phase 2b** (which also
introduces the `Graph` and `Runtime` JS classes — the natural
place to wrap each handle is once we have Buffer support to
exchange tensor data).

Cost of the JSON envelope: 2× the raw bytes plus JSON overhead per
call.  Acceptable for the proof-of-concept; Phase 2b removes it.

### N-API binding: `node-bridge`, not `napi-rs`

The workspace convention — established by `font-parser-node` — is
to use the sibling **`node-bridge`** crate: a zero-dependency safe
Rust wrapper over the raw N-API `extern "C"` interface.  No
`napi-rs`, no `napi-sys`, no `napi-derive`, no `napi-build`, no
build-time header requirements.  N-API is ABI-stable by design;
extern declarations work on every Node version that supports the
targeted N-API revision.

This is the **default for every Node binding in the workspace**.
An earlier draft of this spec specified `napi` + `napi-derive` +
`napi-build`; that draft has been superseded.  Reasons to prefer
`node-bridge`:

* Zero crates.io dependencies (matches the workspace's broader
  minimum-dependency ethos).
* Same pattern as the existing `font-parser-node` addon — one
  reviewer skillset, one debugging surface.
* No proc-macro at the binding boundary — easier to read, easier
  to step through under `lldb`.

If a future addon hits a real ceiling that only `napi-rs` can
break through (very complex async patterns, heavy class wrapping,
etc.), the option remains open — but each addon makes that call
on its own merits, and the default is `node-bridge`.

Companion TypeScript wrapper:

```
code/packages/typescript/matrix-rust-napi/
  package.json           # depends on prebuilt platform packages
  src/index.ts           # thin re-export with TypeScript types
  README.md
  CHANGELOG.md
  BUILD                  # npm ci && tsc && node --test
```

Companion exists so the `@coding-adventures/matrix` package has
exactly *one* import target (`@coding-adventures/matrix-rust-napi`)
under its Node conditional export, instead of having to do
napi-rs-platform-package selection itself.

---

## The binding surface

The napi crate exposes **three** classes plus one factory function.
Nothing more in v0; coverage grows by adding methods, not classes.

```typescript
// What Node sees (TypeScript declaration generated by napi-rs):

export interface DType { kind: "f32" | "f64" | "i32" | "i64" | "u8" | "u32" }

export class Graph {
  /** Build a Graph from a JSON wire-format string
   *  (see matrix-ir-json crate, MX01 §"JSON wire format"). */
  static fromJson(json: string): Graph

  /** Serialize to JSON wire format. */
  toJson(): string

  /** Compact debug summary: "Graph(tensors=4, ops=3, inputs=[0], outputs=[2])". */
  describe(): string
}

export class Runtime {
  /** Construct a runtime with the default planner (CPU always; promote
   *  to matrix-metal / matrix-cuda when available and op-coverage matches). */
  static create(): Runtime

  /** Plan + execute a graph with the given inputs.  Returns the output
   *  tensors in declaration order.
   *
   *  inputs[i] feeds graph.inputs()[i].  Each input is a TypedArray
   *  view; dtype + shape are read from the graph.
   *
   *  Returns: outputs[j] for each graph.outputs()[j], as a fresh
   *  TypedArray copy (the caller owns it). */
  run(graph: Graph, inputs: TypedArray[]): TypedArray[]
}

export class BackendInfo {
  /** "cpu" always; "cuda" / "metal" when planner picked them. */
  readonly kind: string
  readonly version: string
}

/** Information about which backends the binary was compiled with. */
export function availableBackends(): BackendInfo[]
```

**Why JSON, not handle-based, for `Graph::fromJson`?**

The user already constructs a `matrix_ir::Graph` *somewhere* —
either in Rust, or in TypeScript via a future `matrix-ir-ts` mirror.
Both can emit JSON via the canonical wire format.  Accepting JSON at
the FFI boundary is:

* trivially debuggable (paste the JSON into a `.json` file, diff it);
* immune to handle-lifetime bugs across the FFI boundary;
* the same wire format that ships graphs to browsers, so a single
  test fixture validates both paths;
* free of binary versioning concerns — only the JSON schema versions
  (per MX01).

The cost is one JSON parse per graph submission.  For workloads
where that matters (very small graphs, very high call rate), v1+
adds a `Graph.fromBinary(Buffer)` constructor that takes the
`matrix-ir` binary wire format.  But v0 is JSON only.

**Why fresh TypedArray copies on output?**

Returning a view into Rust-owned memory across the napi boundary
opens lifetime questions.  Copying is unambiguous and at typical
network/disk I/O scales is invisible.  Optimised zero-copy output
(via `Buffer.from(napi.External(...))`) is a v1+ optimisation gated
by profiling.

**Why no per-op API like `cpuExecutor.matmul(a, b)`?**

Per ARCH02, the runtime owns backend selection (CPU vs Metal vs
CUDA).  Exposing per-op entry points would force Node to re-implement
planning logic, defeating the architecture.  All execution goes
through a `Graph`.

For the **`typescript/matrix`** package's `Matrix.multiply(a, b)` API
to use this binding, it constructs a 2-op `Graph` (`MatMul`,
`Output`) on the fly.  The graph is essentially a recipe; the
runtime is the cook.

---

## Distribution model

We follow **napi-rs's standard prebuilt-binary model**.  The Rust
crate publishes one shape, the wrapper consumes prebuilt binaries
keyed by `os` + `arch`:

```
@coding-adventures/matrix-rust-napi                 (TS wrapper, all platforms)
├── @coding-adventures/matrix-rust-napi-linux-x64-gnu      (.node binary)
├── @coding-adventures/matrix-rust-napi-linux-arm64-gnu    (.node binary)
├── @coding-adventures/matrix-rust-napi-darwin-x64         (.node binary)
├── @coding-adventures/matrix-rust-napi-darwin-arm64       (.node binary)
└── @coding-adventures/matrix-rust-napi-win32-x64-msvc     (.node binary)
```

The wrapper's `optionalDependencies` lists every platform package;
npm installs only the one matching the host's `os`+`arch`.  At
runtime, `index.ts` `require()`s the matching package and re-exports.

Consumers see one package; they don't think about the architecture
matrix.

**v0 ships only `darwin-arm64` and `linux-x64-gnu`** — the two
platforms where CI runners exist and where the author can debug
locally.  Other platforms are added as needed.  The wrapper raises a
clear `Error("unsupported platform: <os>-<arch>; supported: ...")` on
load when no matching prebuilt is installed, rather than failing
opaquely deep inside `node-gyp`.

Prebuilt binaries are produced by a GitHub Actions matrix job that
runs `napi build --release --target <triple>` and uploads each
artifact to its per-platform npm package.  v0 lands the workflow
file but does not enable a publish step — npm publish gating is a
separate PR.

---

## How CI proves this works

Three layers, in order of expense:

1. **`cargo test -p matrix-rust-napi`** — pure-Rust tests.  Build the
   napi `.node` artifact via `napi build` invoked from `build.rs`
   would require Node at test time, which is wrong.  Instead, the
   crate's pure-Rust tests cover:
   - Graph JSON round-trip (already covered by `matrix-ir-json`, but
     exercise our re-export here too).
   - Runtime construction + execution on simple graphs *without going
     through napi* (a feature flag `internal-testing` exposes an
     internal `run_graph(graph, inputs)` function the tests use).
   - Error mapping (`matrix_runtime::Error` → `napi::Error` with the
     right `napi::Status` code).
2. **`napi build` smoke** — a CI step that runs `npx napi build
   --release` and asserts the binary file exists and is loadable
   (`node -e "require('./index.node')"`).  Catches build-config rot
   (missing `cdylib` crate-type, bad `napi-build` setup, etc.).
3. **`node --test __test__/smoke.test.mjs`** — actual Node binding
   exercise.  Builds a tiny 2x2 MatMul graph in TypeScript, sends
   JSON to the binding, runs it, asserts the output bytes.  This is
   the only end-to-end test; it runs after the napi build step.

Step 1 runs on every PR via the existing `cargo build --workspace`
gate.  Steps 2-3 run only when matrix-rust-napi files change (path
filter on the new workflow).

---

## MX00 compatibility

`matrix-ir`, `matrix-runtime`, `matrix-cpu`, `matrix-metal`,
`matrix-cuda`, `matrix-profile`, `compute-ir`, `compute-runtime`, and
`executor-protocol` are **all bound by the MX00 zero-dependency
mandate** (CI-enforced for the leaf crates).  None of them gains
a dependency from this work.

`matrix-rust-napi` is **not** bound by MX00.  It is explicitly an
FFI/binding crate at the edge of the workspace — its job is to
glue the zero-dep core to external runtimes.

But because it depends on `node-bridge` rather than `napi-rs`, it
still has **zero crates.io dependencies**.  Its `Cargo.toml` reads:

```toml
[dependencies]
matrix-ir                          = { path = "../matrix-ir" }
matrix-ir-json                     = { path = "../matrix-ir-json" }
node-bridge                        = { path = "../node-bridge" }

# Phase 2 adds:
# matrix-runtime  = { path = "../matrix-runtime" }
# matrix-cpu      = { path = "../matrix-cpu" }
# compute-ir      = { path = "../compute-ir" }
# executor-protocol = { path = "../executor-protocol" }
#
# Phase 2+ platform-conditional GPU deps:
# [target.'cfg(target_os = "macos")'.dependencies]
# matrix-metal = { path = "../matrix-metal" }
# [target.'cfg(any(target_os = "linux", target_os = "windows"))'.dependencies]
# matrix-cuda  = { path = "../matrix-cuda" }

[lib]
crate-type = ["cdylib"]
name = "matrix_rust_napi"
```

The single `cdylib` target produces
`libmatrix_rust_napi.{dylib,so,dll}`, which Phase 3's
`package.json` build script renames to `matrix_rust_napi.node` for
Node to `require()`.

**Why `cdylib`-only and not `cdylib + rlib`?**  `font-parser-node`
uses cdylib-only; an empty cargo test (zero `#[test]` functions in
the lib) still compiles and "runs" successfully because the test
harness builds the lib as cdylib + a tiny test binary that
references it.  We confirmed locally: a cdylib-only crate with
`#[cfg(test)] mod tests` *does* compile and run tests under
`cargo test`.  Adding an `rlib` target would double the build time
and produce an extra artifact for no benefit.

---

## Interaction with `typescript/matrix`

This is the consumer that motivates the work.  After **Phase 3**
(separately scoped, separately PR'd) the package layout is:

```
typescript/matrix/
  package.json   # exports = { ".": { node: "./dist/node/index.js",
                 #                    browser: "./dist/browser/index.js" } }
  src/
    matrix.ts                     # public API (unchanged)
    node/cpu-matrix-backend.ts    # NEW: calls into @coding-adventures/matrix-rust-napi
    browser/cpu-matrix-backend.ts # current pure-TS implementation
```

The Node implementation of `CpuMatrixBackend.multiply(a, b)` becomes
roughly:

```typescript
import { Graph, Runtime } from "@coding-adventures/matrix-rust-napi"

export class CpuMatrixBackend implements MatrixBackend {
  private rt = Runtime.create()
  multiply(a: Matrix, b: Matrix): Matrix {
    const graph = Graph.fromJson(buildMatMulGraphJson(a.shape, b.shape, a.dtype))
    const [out] = this.rt.run(graph, [a.data, b.data])
    return new Matrix(out, [a.rows, b.cols])
  }
  // …
}
```

All 13 downstream packages keep working without source changes
(per ARCH02 §"current state").

That refactor lives in **MX08** (separately scoped).  This spec
(MX07) covers only the napi crate + wrapper.

---

## Phases

Each phase is a separately-PR'd, independently-reviewable change.
The earlier phases unblock the later ones; nothing is held until the
end.

| Phase | Lands | Status |
|-------|-------|--------|
| 0 | This spec. | shipped (#3508) |
| 1 | Rust crate skeleton — `Cargo.toml` (`cdylib`), `src/lib.rs` exporting `graphRoundTripJson` (one function, JSON in → `matrix-ir-json::decode` → `matrix-ir-json::encode` → JSON out), 5 unit tests on the pure-Rust core. No Node side yet. | shipped (#3518) |
| 2 | `runGraphOnCpu(envelopeJson)` — end-to-end execution on `matrix-cpu` via `matrix-runtime`'s planner. JSON envelope with hex-encoded byte payloads (one string-in, string-out function — same napi pattern as Phase 1). Pure-Rust `run_graph_on_cpu(graph, inputs)` helper unit-tested with end-to-end Add, MatMul, and ReLU layer. | shipped (#3527) |
| 2b | `Graph` + `Runtime` JS classes with `Buffer[]` I/O via the node-bridge Buffer helpers (`napi_create_buffer` / `napi_get_buffer_info` / finalizer added in PR #3529). Constructors store class handles in `AtomicUsize` for Worker-thread safety; instance methods unwrap via `napi_unwrap` + 128-bit type-tag check. JSON-envelope `runGraphOnCpu` kept as CLI-friendly alternative path. | shipped (#3539) |
| 3 | `package.json` (npm `build` + `smoke` scripts) + `.github/workflows/matrix-rust-napi.yml` (path-filtered Actions workflow). Builds `matrix_rust_napi.node` on `ubuntu-latest` (linux-x64-gnu) and `macos-latest` (darwin-arm64); confirms it loads and all four exports (`graphRoundTripJson`, `runGraphOnCpu`, `Graph`, `Runtime`) are present.  No publish step yet — that's a separately-coordinated follow-up. | **this PR** |
| 2 | `Runtime::create()` + `Runtime::run(graph, inputs)` on the CPU executor.  Internal-testing feature flag for in-Rust round-trip tests of `graph_in → run → outputs_out`. | pending |
| 3 | `package.json` + `napi build` workflow file.  CI step that builds the `.node` artifact on `darwin-arm64` and `linux-x64-gnu` and confirms it loads.  No publish step. | pending |
| 4 | TypeScript wrapper package `typescript/matrix-rust-napi/` — re-exports the napi binding with typed declarations.  `__test__/smoke.test.mjs` round-trips a `MatMul` graph through the binding. | pending |
| 5 | (Separately scoped, MX08.) Refactor `typescript/matrix` to use this binding under the Node conditional export. | future |

Phase 0 (this PR) does not ship code — only this spec.  Per CLAUDE.md:
**specs first, implementation after**.

---

## Non-goals

To pre-empt overreach:

* **GPU execution is not added in MX07.**  The runtime planner
  already lifts to Metal / CUDA when available; the napi crate
  inherits that for free.  But the GPU executors themselves are
  out of scope for this spec.
* **TypeScript code generation from the Rust napi types is not
  scoped here.**  napi-rs already emits a `.d.ts` automatically;
  the wrapper package re-exports it.  No additional tooling is
  introduced.
* **Browser support is not scoped here.**  ARCH02 explicitly
  exempts the browser; browser execution goes through
  `matrix-ir-ts` + `matrix-webgpu-ts` + `matrix-cpu-ts` (ARCH02
  Phases 4-5).
* **Hot-reload / dynamic dispatch is not scoped.**  The binding
  loads once at `require()` and lives for the process lifetime.
* **Memory pooling across calls is not scoped.**  Each `run()` is
  independent.  Buffer reuse is a profiling-driven optimisation
  for v1+.
* **Async execution is not scoped.**  `Runtime::run` is blocking.
  GPU dispatch is async internally (kernels are queued to a
  command buffer), but `run()` waits for completion before
  returning.  A future `runAsync()` returns a `Promise` and is
  added when profiling shows the napi blocking overhead matters.

---

## Open questions

These are deferred until the implementation PRs hit them:

* **N-API revision target** — `font-parser-node` targets N-API v4
  (Node 10.16+ / 12+).  `matrix-rust-napi` defaults to the same
  unless a needed function requires a newer revision.  Decided per
  Phase as new N-API calls are added.
* **MSRV** — matches the workspace; revisited only if `node-bridge`
  or a workspace upstream crate forces a bump.
* **Per-platform packaging strategy** — Phase 3 question.  The
  workspace pattern (font-parser-node's `package.json` runs
  `cargo build --release` then `cp` the `.dylib` to `.node`) works
  for a single platform; cross-platform distribution needs a
  publish workflow.  Likely a GitHub Actions matrix builds each
  triple's `.node`, the wrapper consumes them as
  `optionalDependencies`.  Decided when Phase 3 lands.
