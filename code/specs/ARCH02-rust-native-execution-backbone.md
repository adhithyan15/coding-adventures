# ARCH02 — Rust as the Native Execution Backbone

## Why this spec exists

The matrix execution layer (`matrix-ir`, `matrix-runtime`, `matrix-cpu`,
`matrix-metal`, `matrix-cuda`) is the canonical native execution
substrate for every workload in this repository: DSP (FFT, DCT, STFT,
wavelets), image (convolution per ARCH01, future image-FFT/DCT/wavelet
adapters), neural-network forward passes (NN01, pending), and any
future op family that needs portable GPU lift.

There are currently **two competing native execution stacks** in the
codebase:

1. The Rust matrix execution layer — designed for portability across
   CPU and GPU backends, used by every `dsp-*` crate that has a
   matrix-IR-lowered path.
2. The TypeScript `matrix` package + `neural-graph-vm`'s
   `matrix-plan.ts` + `webgpu-matrix-backend.ts` — a parallel native
   stack with its own IR (`NeuralMatrixPlan`), its own CPU backend
   (`TypeScriptMatrixBackend`), and its own GPU backend
   (`WebGpuMatrixBackend`).

Both stacks have CPU implementations. Both will need GPU backends.
Both will need to ship lowering passes for neural networks, image
processing, and signal processing. Without a written rule, every
future workload either re-implements its operator set twice (once for
each language's native stack) or arbitrarily picks one, leaving the
other behind. After three or four such decisions we have:

- Two convolution implementations (one in `dsp-conv`, one buried in
  `neural-graph-vm/matrix-plan.ts`) that drift on edge cases.
- Two GPU dispatch policies. Two op-coverage detection mechanisms.
  Two backend-selection algorithms.
- Test suites that have to be replicated. Bugs that get fixed in
  one stack and not the other.

This spec exists to **establish a single rule** so that the boundary
between Rust-native and browser-TS-native execution is explicit, and
so that future workloads have a clear answer to "where does this
implementation live?"

This is a small cross-cutting spec — not an implementation spec. It
constrains how new execution-layer work is structured and gives
existing TypeScript native execution code a migration path. It does
not change any existing code as a precondition.

---

## The rule

**Rust is the single source of truth for all native execution.**

- Every native CPU and native GPU code path lives in the Rust matrix
  execution layer (`matrix-cpu`, `matrix-metal`, `matrix-cuda`, future
  `matrix-rocm`, `matrix-vulkan`, `matrix-coreml`, etc.).
- Every non-Rust, non-browser language binding (Node.js TypeScript,
  Python, Ruby, Lua, Java, …) calls into the Rust matrix execution
  layer through the matching **workspace `*-bridge` crate** (see
  §"Bindings layer: workspace bridges, not ecosystem crates" below).
  The Rust side is the *only* place native code is implemented.
- The browser TypeScript runtime is **the single exception**. It
  cannot call host-native APIs directly (no FFI access to Metal /
  CUDA / system NPU), so the browser keeps its own TypeScript
  implementations of the matrix execution layer (`matrix-ir-ts`,
  `matrix-runtime-ts`) and its own browser-native backends
  (`matrix-webgpu-ts`, `matrix-cpu-ts`). The IR shape and the
  bytecode wire format are byte-for-byte identical to Rust's.

In one diagram:

```text
                                        Authoring surface
                                        (any language, any spec)
                                                  │
                                       matrix_ir::Graph (canonical shape)
                                                  │
                       ┌──────────────────────────┴──────────────────────────┐
                       │                                                     │
                       │ JSON serialisation                                   │ JSON serialisation
                       │ (universal wire format)                              │
                       ▼                                                     ▼
            ┌────────────────────┐                                ┌────────────────────┐
            │   Rust matrix      │                                │  matrix-ir-ts       │
            │   execution layer  │                                │  (browser only)     │
            │                    │                                │                     │
            │   matrix-runtime   │                                │  matrix-runtime-ts  │
            │       │            │                                │       │             │
            │   ┌───┴───┐        │                                │   ┌───┴───┐         │
            │   ▼       ▼        │                                │   ▼       ▼         │
            │ matrix-  matrix-   │                                │ matrix- matrix-     │
            │ cpu      metal     │                                │ webgpu- cpu-ts      │
            │          / cuda    │                                │ ts                  │
            └─────┬──────────────┘                                └─────────────────────┘
                  │                                                          ▲
                  │ FFI via workspace bridge crates                          │
                  │ (node-bridge, python-bridge, ruby-bridge,                │
                  │  lua-bridge, erl-nif-bridge, perl-bridge,                │
                  │  objc-bridge, ... — all zero-dep wrappers                │
                  │  over each language's C API)                             │
                  │                                                           │
        ┌─────────┼─────────────────────────────────────────────┐             │
        ▼         ▼         ▼         ▼          ▼              ▼             │
   Rust apps   Node.js   Python    Ruby      Lua /          Erlang /         │
   (direct)   (node-    (python-  (ruby-     lua-bridge     erl-nif-         │
              bridge)   bridge)   bridge)                    bridge          │
                                                                              │
                                                                Browser TS ───┘
                                                                (the one
                                                                exception)
```

Every native execution path on the left side terminates in the Rust
matrix execution layer. The single exception on the right side is the
browser, which cannot reach host-native APIs.

---

## Bindings layer: workspace bridges, not ecosystem crates

The first cut of this spec mentioned `napi-rs`, `pyo3`, `magnus`,
`mlua`, and JNI as the FFI mechanisms each language binding would
use.  Those are the *ecosystem-standard* binding libraries; the
workspace **does not use them**.  Instead, every language has a
dedicated workspace `*-bridge` crate that wraps the language's stable
C ABI directly with safe Rust:

| Target language | Workspace bridge crate | What it wraps |
|------------------|------------------------|---------------|
| Node.js          | `node-bridge`          | Node.js N-API |
| Python           | `python-bridge`        | CPython C API |
| Ruby             | `ruby-bridge`          | Ruby C API    |
| Lua 5.4          | `lua-bridge`           | Lua C API     |
| Erlang           | `erl-nif-bridge`       | `erl_nif`     |
| Perl             | `perl-bridge`          | Perl C API    |
| Objective-C      | `objc-bridge`          | Objective-C runtime + Metal / CoreGraphics / CoreText |

Common properties:

* **Zero external dependencies.**  No `bindgen`, no `cc`, no
  proc-macro crates.  Every bridge declares the C functions it uses
  with `extern "C"` blocks and pulls them at link time.  C ABIs of
  these runtimes are stable by design — that's exactly what they
  exist for — so the bridge crates work across every supported host
  version.
* **No macros.**  The bridges expose safe Rust functions
  (`str_to_js`, `str_from_js`, `vec_str_from_js`, …); binding crates
  call them directly without `#[napi]` / `#[pyfunction]` / etc.
  Stack traces stay shallow and `lldb` stepping works without proc-
  macro indirection.
* **One reviewer skillset.**  All bridges follow the same shape, so
  reviewing a new language binding is no harder than reviewing the
  previous one.

A future binding crate for a language not yet covered (Swift via
`@_cdecl`, R via `.Call`, Julia via `ccall`, etc.) gets a new
`<lang>-bridge` crate.  Ecosystem crates may be referenced for
inspiration but are not consumed as dependencies.

Deviating from this default — for example, if a binding needs an
async runtime an ecosystem crate provides out of the box — is
allowed but must be justified in the binding's own design doc
(typically a sibling MX## spec, as MX07 is for the Node.js binding).

---

## Current state (2026-05-17)

### Rust side — already follows the rule

| Crate                        | Version | What it does                                                       |
|------------------------------|---------|--------------------------------------------------------------------|
| `matrix-ir`                  | shipped | The graph types: `Graph`, `Tensor`, `Shape`, `DType`, `GraphBuilder`. |
| `matrix-runtime`             | shipped | Plans an IR graph onto a placed `ComputeGraph` with backend-aware op coverage. |
| `matrix-cpu`                 | shipped | The CPU executor. Always available. Reference behaviour.           |
| `matrix-metal`               | shipped | Metal GPU executor. macOS only. Lifts ops automatically.           |
| `matrix-cuda`                | shipped | CUDA GPU executor. Linux/Windows with NVIDIA. Lifts ops automatically. |
| `compute-ir`, `compute-runtime`, `executor-protocol`, `compute-unit` | shipped | Placement, dispatch protocol, profiling. |
| `dsp-fft`, `dsp-stft`, `dsp-wavelets`, future `dsp-conv` Phase 6 | shipped (varied versions) | Emit `matrix_ir::Graph`, run via `matrix-runtime`. Reference implementation of the pattern. |

This is correct and complete for native execution. Future native GPU
backends (ROCm, Vulkan, CoreML/MLCompute for system NPU) plug in as
new crates under the `matrix-*` family without changing the IR.

### TypeScript side — has parallel work that needs to migrate

| Package                                            | Status                  | Migration target                                              |
|----------------------------------------------------|-------------------------|---------------------------------------------------------------|
| `typescript/matrix`                                | Shipped, used by 13 packages | **Node:** replace `CpuMatrixBackend` with a napi binding to Rust `matrix-cpu`. **Browser:** keep as a TS-side `matrix-cpu-ts` fallback when WebGPU is unavailable. |
| `typescript/neural-graph-vm/matrix-plan.ts`        | Shipped                 | **Node:** replace `compileBytecodeToMatrixPlan` with a Rust binding that emits a `matrix_ir::Graph` and runs it via the napi runtime. **Browser:** keep but refactor the IR shape from `NeuralMatrixPlan` to the universal `matrix_ir::Graph` shape (a TS mirror of the Rust types). |
| `typescript/neural-graph-vm/webgpu-matrix-backend.ts` | Shipped (browser GPU)  | **Browser:** kept. Refactored to take the universal `matrix_ir::Graph` shape as input. Becomes `matrix-webgpu-ts`. |
| `typescript/blas-library`, `typescript/cas-matrix`, `typescript/lattice-ast-to-css`, `typescript/macsyma-runtime`, `typescript/paint-instructions`, `typescript/paint-vm-svg`, `typescript/single-layer-network`, `typescript/two-layer-network`, `typescript/neural-graph-vm` | Shipped, all depend on `typescript/matrix` | **Node:** no source change required — they get the napi-bound `Matrix` class transparently. **Browser:** no source change required — they get the TS-side implementation. The `MatrixBackend` interface remains the public abstraction. |
| Future Python / Ruby / Lua / etc. bindings         | Not yet built           | Each language gets a `matrix-*-FFI` package that binds to the Rust matrix execution layer via the language's standard native-extension mechanism. |

The migration is **internally invisible to consumers** of the
`typescript/matrix` package — the `MatrixBackend` interface stays, the
`Matrix` class stays, only the implementation behind it changes.

---

## How the universal wire format works

Every `matrix_ir::Graph` value has a canonical JSON serialisation.
Both the Rust side and the browser TS side implement:

```rust
// Rust
impl Graph {
    pub fn to_json(&self) -> String;
    pub fn from_json(s: &str) -> Result<Self, GraphError>;
}
```

```typescript
// matrix-ir-ts (browser only)
export class Graph {
    static fromJson(s: string): Graph;
    toJson(): string;
}
```

The schema:

- Top-level: `{ "tensors": [...], "ops": [...], "inputs": [...], "outputs": [...] }`
- Each tensor: `{ "id": <u32>, "dtype": "f32" | "f64" | "i32" | "u8", "shape": [<u32>, ...] }`
- Each op: `{ "kind": "Mul" | "Add" | ... , "inputs": [<TensorId>, ...], "output": <TensorId>, "attrs": { ... } }`
- `Const` ops include their bytes inline (`{ "kind": "Const", "output": <id>, "bytes_b64": "..." }`).

The schema is versioned (`"matrix_ir_version": "1.0"`) so future
additions don't silently break older deserialisers.

This wire format is the *interop boundary*. A graph built in Rust can
be serialised → POST'd to a browser → loaded into `matrix-ir-ts` →
executed on WebGPU. A graph built in browser TypeScript can be
serialised → sent to a Rust backend → executed on Metal or CUDA. The
output values match within f32 tolerance.

---

## Backend selection

Each runtime picks its best available backend at execution time.

### Rust `matrix-runtime` (already implemented, by design)

```
Linux/Windows with CUDA  → matrix-cuda (when op coverage matches)
macOS                    → matrix-metal (when op coverage matches)
fallback                 → matrix-cpu (always available)
```

Adding future native GPU backends is a workspace addition — no IR or
runtime changes:

```
macOS Apple Silicon with NPU → matrix-coreml (when op coverage matches)
Linux with ROCm              → matrix-rocm   (when op coverage matches)
Any with Vulkan compute      → matrix-vulkan (when op coverage matches)
```

### Browser `matrix-runtime-ts` (the one TypeScript exception)

```
navigator.gpu defined and adapter supports compute → matrix-webgpu-ts
fallback (Safari ≤ 17, Firefox without flag, etc.) → matrix-cpu-ts (pure TS, possibly WASM SIMD)
```

### Node.js TypeScript (binds into Rust)

```
import { CpuMatrixBackend } from "@coding-adventures/matrix";
// → napi shim → Rust matrix-cpu / matrix-metal / matrix-cuda
//   (picked by the Rust matrix-runtime via the same logic above)
```

No backend-selection code lives in the Node.js TypeScript layer. The
Rust runtime owns that policy.

### All other language bindings

Same as Node.js — the language-specific package (`matrix-py`,
`matrix-rb`, `matrix-lua`, …) is a thin native-extension shim into
the Rust matrix execution layer. The Rust runtime owns backend
selection.

---

## What this means for NN01

The NN01 spec ("Matrix Backend Interface and Bytecode Lowering")
currently describes lowering `neural-graph-vm` bytecode to a generic
`MatrixBackend`. Under this spec, NN01 lands as:

- **Rust impl** — `neural-graph-vm` (Rust) compiles bytecode to a
  `matrix_ir::Graph`. Execution goes through `matrix-runtime` →
  `matrix-cpu` / `matrix-metal` / `matrix-cuda`. This is the
  production path for every Rust app and every Rust-bound language
  binding (Node, Python, Ruby, Lua, …).
- **Browser impl** — `neural-graph-vm` (TypeScript, browser build)
  compiles bytecode to a `matrix_ir::Graph` (using the TS-mirrored
  types from `matrix-ir-ts`). Execution goes through
  `matrix-runtime-ts` → `matrix-webgpu-ts` or `matrix-cpu-ts`.

Both implementations of NN01 emit the *same IR shape*. The
serialisation round-trip property means a model trained in Rust on
CUDA can be exported, shipped as a static file, loaded in a browser,
and run on WebGPU — without any model-format conversion step.

---

## What this means for the existing TypeScript packages

### `typescript/matrix`

The public surface (`MatrixBackend` interface, `Matrix` class,
`CpuMatrixBackend` impl, `getMatrixBackend` / `setMatrixBackend`
accessors) stays exactly as it is. Consumer code does not change.

The `CpuMatrixBackend` implementation gets a build-target split:

- `dist/node/cpu-matrix-backend.ts` — calls into a Rust napi
  module (provisional name `@coding-adventures/matrix-rust-napi`)
  that exposes the Rust `matrix-cpu` (and transparently lifts to
  `matrix-metal` / `matrix-cuda` when the runtime's planner picks
  them).
- `dist/browser/cpu-matrix-backend.ts` — keeps the current pure-JS
  implementation as the browser CPU fallback for when WebGPU is
  unavailable.

The package.json `exports` field uses the conditional `node`/`browser`
keys to route the right implementation per environment.

### `typescript/neural-graph-vm`

- `matrix-plan.ts` — refactor in two stages:
  1. Refactor `NeuralMatrixPlan` to be a TypeScript mirror of
     `matrix_ir::Graph` (rename the type, keep the structure
     equivalent). This is a one-time invasive change; all the matrix-
     plan consumer code updates with it.
  2. Split into `node` and `browser` builds the same way as the
     `matrix` package.
- `webgpu-matrix-backend.ts` — refactor to consume the new
  IR shape. Extract into a standalone `matrix-webgpu-ts` package
  so it can be reused outside `neural-graph-vm`.

### All other dependents

`typescript/blas-library`, `typescript/cas-matrix`, etc. continue to
use the `Matrix` class via the `@coding-adventures/matrix` package
exactly as they do today. They get the Rust-bound implementation in
Node and the TS-side implementation in the browser, transparently.

---

## What this does NOT mean

To pre-empt overreach:

- **The CPU-scalar `ml-framework-*` packages and the
  `single-layer-network`, `two-layer-network`, `perceptron`,
  `activation-functions`, `loss-functions`, `gradient-descent`, etc.
  educational packages are not affected.** They are pedagogical
  reference implementations that demonstrate the math in pure
  scalar form (in their respective languages) without going through
  any matrix execution layer. They stay as they are.
- **WASM-on-the-browser is not a Rust-native path under this rule.**
  WASM running in a browser cannot call WebGPU directly; it would
  have to bounce out to JS. The browser TS implementation is
  authoritative for browser execution.
- **WASM-on-the-server is allowed but not encouraged.** A server-side
  WASM runtime *could* host the Rust matrix execution layer compiled
  to WASM. That works but offers no advantage over the native Rust
  build; it exists only for sandboxed-execution use cases.
- **Educational language ports of matrix multiplication, FFT, etc.
  are not affected.** The `typescript/matrix` package's existing
  educational implementations of matrix multiplication in TypeScript
  are kept as reference — the napi binding wraps them; it does not
  delete them. (The browser side ships them.)
- **This rule applies to native execution only.** Higher-level
  authoring surfaces (the Keras-style API in `ml-framework-keras`,
  the PyTorch-style API in `ml-framework-torch`, the
  graph-construction DSL in `neural-network`) stay in their
  respective languages. They merely route through the matrix layer
  to execute.

---

## Why this matters

When the architectural boundary is implicit:

- Native GPU acceleration ships first in whichever language's native
  stack the implementer happens to prefer. The other language
  catches up six months later with a different op set. Models trained
  on one stack don't transfer to the other.
- Backend additions (a new native GPU API, a new WebGPU feature)
  require two implementations. The work is duplicated. The bug
  surface is doubled. Cross-platform parity testing is impossible.
- A given workload — say, image convolution — could end up with
  three independent implementations (image-cpu-ts, image-cpu-rust,
  dsp-conv) that all drift from each other.

With this rule:

- **Native GPU work happens once, in Rust.** Every language gets it
  via FFI. Metal lands once and is available in Node.js, Python,
  Ruby, Lua, and direct-Rust apps simultaneously.
- **Browser WebGPU work happens once, in TypeScript.** Same IR shape
  as Rust, same wire format, so models cross over freely.
- **Future native GPU backends are crate additions.** ROCm support is
  a `matrix-rocm` crate, picked automatically by `matrix-runtime` when
  available.
- **The wire format makes models portable.** Export from Rust, load
  in browser, run on WebGPU. Train in browser, export, ship to a
  Rust serving stack on CUDA.

---

## Phase / status

| Phase | Lands                                                       | Status |
|-------|-------------------------------------------------------------|--------|
| 0     | This spec                                                   | **this PR** |
| 1     | Universal wire format — `Graph::to_json()` / `from_json()` in Rust; matching `matrix-ir-ts` package in TypeScript with `Graph.fromJson()` / `.toJson()`. Round-trip integration test: Rust builds a graph, serialises, TS loads + runs, results match within f32 tolerance. | pending |
| 2     | `matrix-rust-napi` — a Rust-side N-API crate that exposes `matrix-cpu` (and indirectly `matrix-metal` / `matrix-cuda` via the runtime planner) to Node.js via the workspace `node-bridge` crate. | pending |
| 3     | `typescript/matrix` refactor — conditional `node`/`browser` exports. The Node build calls into `matrix-rust-napi` via N-API; the browser build keeps the current JS-CPU implementation. All 13 downstream packages keep working without source changes. | pending |
| 4     | `matrix-webgpu-ts` package — extracted from `neural-graph-vm/webgpu-matrix-backend.ts`. Consumes the unified `matrix_ir::Graph` shape. | pending |
| 5     | `matrix-runtime-ts` package — TypeScript mirror of `matrix-runtime` for browser use. Backend selection: WebGPU when available, CPU-TS fallback. | pending |
| 6     | NN01 implementation — Rust `neural-graph-vm` bytecode → `matrix_ir::Graph` → `matrix-runtime`. (Parallel TS implementation for browser follows the same shape but is its own phase.) | pending |
| 7     | First non-Rust, non-Node FFI binding — Python via the workspace `python-bridge` crate. Demonstrates the FFI pattern. | pending |
| 8     | Subsequent FFI bindings — Ruby via `ruby-bridge`, Lua via `lua-bridge`, Erlang via `erl-nif-bridge`, Perl via `perl-bridge`, etc. as needed. Each is a thin shim; the pattern is established by Phase 7. | pending |

Phases 1–3 unblock everything else. Phase 6 is the original NN-on-
matrix-IR work that motivated this spec.

---

## Out of scope

This spec does not address:

- **Specific WebGPU shader implementations.** The
  `webgpu-matrix-backend.ts` code already has WGSL shaders for the
  ops it supports; extending coverage is Phase 5+ implementation
  work, not architectural.
- **The neural-graph-vm bytecode design.** Covered by NN00.
- **Specific FFI binding choices per language**. The workspace
  default is the matching `*-bridge` crate (see §"Bindings layer:
  workspace bridges, not ecosystem crates"); if a future binding
  ever needs to deviate (e.g. for an async runtime an ecosystem
  crate provides out of the box), that decision is made in the
  binding's own design doc, not here.
- **GPU memory management policy.** Already addressed by
  `compute-ir` / `compute-runtime` / `executor-protocol` / `matrix-runtime`.
- **Training infrastructure.** Forward pass and matrix-IR lowering
  is what this spec scopes. Backward pass, optimisers, distributed
  training — all subsequent specs.
- **Migration timeline.** The rule is in force from the merge of
  this spec; existing TypeScript native execution code keeps working
  until it migrates per the phase plan.

---

## Relationship to other architectural specs

- **ARCH01 (image ↔ DSP routing)** is orthogonal. ARCH01 says image
  ops with DSP counterparts route through `dsp-*`. ARCH02 says all
  native execution of `dsp-*` (and everything else) goes through
  the Rust matrix execution layer. Composition: a hypothetical
  `image-convolution::gaussian_blur` calls
  `dsp-conv::sep_conv2d` which calls
  `dsp_conv::build_sep_conv2d_graph` (future Phase 6 of `dsp-conv`)
  which executes via `matrix-runtime`. The browser equivalent goes
  through the TS mirror of the same chain. The two routing rules
  compose without conflict.
- **00-architecture.md** is the global architecture overview for
  the source-to-gates stack. ARCH02 is a domain-specific spec
  about native execution within that overall architecture, sitting
  near the top of the stack at the matrix-execution layer.
- **NN00 (neural graph + bytecode)** is the authoring + bytecode
  surface; orthogonal to where execution happens.
- **NN01 (matrix-backend lowering)** is the spec that gets
  implemented under this rule; ARCH02 pins down what
  "matrix-backend" means at the language level.
