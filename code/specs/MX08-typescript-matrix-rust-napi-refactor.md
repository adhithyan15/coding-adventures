# MX08 — `typescript/matrix` Refactor: Delegate Native Execution to `matrix-rust-napi`

## Why this spec exists

[ARCH02](./ARCH02-rust-native-execution-backbone.md) §"The rule"
states that **all native execution lives in Rust**, and that the
browser is the single exception.  [MX07](./MX07-matrix-rust-napi.md)
landed the binding chain that makes that real:

* `matrix-ir`, `matrix-runtime`, `matrix-cpu` — the Rust matrix
  execution layer.
* `matrix-rust-napi` — the Node.js N-API addon (PRs #3518, #3527,
  #3539, #3546, #3551 across Phases 1–4).
* `@coding-adventures/matrix-rust-napi` — the typed TypeScript
  wrapper that re-exports the addon with proper class declarations
  + a vitest smoke suite.

What's still missing: **the existing `typescript/matrix` package
keeps using its own pure-TS `CpuMatrixBackend` even on Node**.
Six downstream packages already consume the
`MatrixBackend` / `Matrix` API from `typescript/matrix`
(`cas-matrix`, `neural-graph-vm`, `single-layer-network`,
`two-layer-network`, `blas-library`, `network-stack`), and they
get a triple-nested-loop pure-JS implementation regardless of
whether the host has the much faster Rust + matrix-cpu (and
eventually matrix-metal / matrix-cuda) path available.

MX08 closes that loop without breaking any consumer.

This is **a small implementation spec, not an architectural one**.
It pins down:

1. The conditional-exports pattern that routes the right
   `CpuMatrixBackend` per environment.
2. How the Node adapter wraps each `MatrixBackend` op as a
   single-op `matrix-ir` graph executed through the napi addon.
3. The migration plan that keeps the 6 downstream packages
   working unchanged.
4. What's deliberately out of scope (multi-op fusion,
   `setMatrixBackend` removal, breaking `Matrix`'s `number[][]`
   storage, …) so MX08 stays the size of one PR.

---

## The end state

```typescript
// What consumers see (no change from today):
import { Matrix, getMatrixBackend } from "@coding-adventures/matrix";

const a = new Matrix([[1, 2], [3, 4]]);
const b = new Matrix([[5, 6], [7, 8]]);

// On Node: routes through @coding-adventures/matrix-rust-napi →
//          matrix-rust-napi.node → matrix-runtime → matrix-cpu.
//          Future Metal / CUDA executors lift automatically.
// On browser: pure-TS implementation (unchanged).
const product = getMatrixBackend().dot(a, b);
```

No source-level change in any of the 6 downstream packages.  The
public surface (`MatrixBackend` interface, `Matrix` class,
`getMatrixBackend` / `setMatrixBackend` / `resetMatrixBackend`
accessors, `CpuMatrixBackend` class) stays identical — only the
*implementation* behind the Node `CpuMatrixBackend` changes.

---

## Package layout

```
code/packages/typescript/matrix/
  package.json       # adds Node/browser conditional exports
  src/
    matrix.ts        # the Matrix class + interface declarations (today's file, ~572 lines, unchanged)
    backends/
      cpu-pure-ts.ts        # the current CpuMatrixBackend impl, moved into a sibling module
      cpu-rust-napi.ts      # NEW: Node-side adapter wrapping @coding-adventures/matrix-rust-napi
    entry-node.ts    # NEW: re-exports Matrix + interfaces + Node-side CpuMatrixBackend default
    entry-browser.ts # NEW: re-exports Matrix + interfaces + pure-TS CpuMatrixBackend default
  tests/
    matrix.test.ts        # existing 50-test suite, runs on both entry points
    parity.test.ts        # NEW: feeds the same inputs into both backends, asserts numerical equivalence
```

The current `src/matrix.ts` already declares everything inline
(interface, `Matrix` class, `CpuMatrixBackend`, accessors); MX08
splits the **`CpuMatrixBackend` implementation** into a sibling
module so the entry-point files can pick the right one per
environment.  Nothing else in `matrix.ts` moves.

### `package.json` exports

```jsonc
{
  "name": "@coding-adventures/matrix",
  "type": "module",
  "main": "src/matrix.ts",
  "exports": {
    ".": {
      "node":    { "types": "./src/entry-node.ts",    "import": "./src/entry-node.ts" },
      "browser": { "types": "./src/entry-browser.ts", "import": "./src/entry-browser.ts" },
      "default": { "types": "./src/entry-browser.ts", "import": "./src/entry-browser.ts" }
    }
  },
  "dependencies": {
    "@coding-adventures/matrix-rust-napi": "file:../matrix-rust-napi"
  }
}
```

Why:

* The `node` branch always pulls the napi-backed adapter; the
  `browser` branch always pulls the pure-TS backend; `default`
  matches `browser` so non-bundler ESM environments (e.g.
  Deno, future runtimes) get the safe pure-TS path.
* Both entries re-export the same `Matrix`, `MatrixBackend`
  interface, accessor functions — only the `CpuMatrixBackend`
  *implementation* (the value assigned to
  `CPU_MATRIX_BACKEND` / installed as the default
  `activeMatrixBackend`) differs.
* `file:../matrix-rust-napi` is a path dep (workspace
  convention).  `npm install` resolves it to
  `code/packages/typescript/matrix-rust-napi/` which lazy-loads
  the `.node` addon.

This is the **first Node/browser conditional-exports package in
the workspace** — no precedent to mirror.  Conduit's
`files: [".node"]` pattern is the closest analogue but doesn't
do environment routing.

### Workspace convention call-out

The pattern lands a small lessons.md entry: "Node-only Rust
backends ride alongside browser-safe defaults via package.json
`exports` `node`/`browser` conditional branches; the per-env
entry point re-exports the same public symbols with different
implementations behind them."  This becomes the template for
any future package that wants the same hybrid story
(`paint-vm`, `dsp-fft`, `dsp-dct` — anywhere we have a fast
Rust path and a portable browser fallback).

---

## The Node adapter (`src/backends/cpu-rust-napi.ts`)

Each `MatrixBackend` op becomes a **single-op `matrix-ir` graph**
constructed on the fly:

```typescript
import { Graph, Runtime } from "@coding-adventures/matrix-rust-napi";
import type { MatrixBackend, Matrix } from "../matrix.js";

const runtime = Runtime.create();   // module-level; CPU-only in v0

export class CpuMatrixBackend implements MatrixBackend {
  readonly name = "cpu-rust-napi";

  dot(left: Matrix, right: Matrix): Matrix {
    return runMatMul(left, right);
  }
  add(left: Matrix, right: Matrix): Matrix {
    return runElementwise("Add", left, right);
  }
  // subtract, scale, transpose — same shape (different op kinds).
}

function runMatMul(a: Matrix, b: Matrix): Matrix {
  const graphJson = JSON.stringify({
    matrix_ir_version: 1,
    tensors: [
      { id: 0, dtype: "f64", shape: [a.rows, a.cols] },
      { id: 1, dtype: "f64", shape: [b.rows, b.cols] },
      { id: 2, dtype: "f64", shape: [a.rows, b.cols] },
    ],
    inputs:  [0, 1],
    outputs: [2],
    ops: [{ kind: "MatMul", a: 0, b: 1, output: 2 }],
    constants: [],
  });
  const [outBuf] = runtime.run(new Graph(graphJson), [
    matrixToBuffer(a),
    matrixToBuffer(b),
  ]);
  return bufferToMatrix(outBuf, a.rows, b.cols);
}

function matrixToBuffer(m: Matrix): Buffer {
  // Flatten row-major and write each cell as f64 LE.
  // (Matrix today stores number[][] — see "Cost of the per-op shim" below.)
}

function bufferToMatrix(buf: Buffer, rows: number, cols: number): Matrix {
  // Inverse of matrixToBuffer.
}
```

That's the entire adapter.  ~120 LOC.

### Dtype: f32 (with the precision caveat)

`Matrix` today stores JavaScript `number`, which is f64.  But
`matrix-cpu`'s `BackendProfile.supported_dtypes` only includes
F32, U8, I32 (per `matrix-cpu/src/lib.rs:69-70`).  Submitting an
f64 graph fails at the planner with "no capable executor".

So the adapter **quantises at the boundary** to f32:
`Buffer.writeFloatLE` on the way in, `Buffer.readFloatLE` on the
way out.  Values round to ~7 decimal digits at each crossing.

The parity tests then use a **combined absolute + relative
tolerance** rather than exact equality:
`|actual - expected| <= 1e-5 + 1e-5 * |expected|`.  Tracking
f32 precision: tighter for small values (where 1e-5 absolute
dominates), looser for large values (where 1e-5 relative
dominates, ~1e-2 for values around 1000).

Two reasons this is acceptable for MX08's scope:

* **Downstream consumers already use f32-precision workloads** —
  cas-matrix's concrete-number fast path, blas-library, the
  single/two-layer network demos all live well within f32's
  ~7-digit window.
* **Future MX10 (gated by profiling) closes the gap** by adding
  F64 support to `matrix-cpu` and refactoring `Matrix` to
  flat-storage `Float64Array`.  At that point precision becomes
  bit-exact again.

### Why a single-op graph per call?

For the per-op `MatrixBackend` API there's no opportunity to
fuse multiple ops into one graph — each method returns a `Matrix`
that may go anywhere the caller likes before the next call.
Fusion belongs in `neural-graph-vm` (which already builds a full
`matrix-ir::Graph` for the whole network); MX08 is just the
adapter layer.

A future MX09 could add a thin builder API
(`MatrixBackend.batch(graphSpec)`) that lets sophisticated
callers submit a multi-op graph and pay the
marshalling + dispatch cost once.  Not in MX08.

---

## Cost of the per-op shim

Each `dot(a, b)` call pays:

1. `JSON.stringify(graph)` — for a 2x2 graph, microseconds.
2. `new Graph(json)` — addon-side JSON parse, microseconds.
3. `matrixToBuffer(a)` + `matrixToBuffer(b)` — flatten
   `number[][]` to a fresh `Buffer`.  **O(rows × cols)** copies.
4. `runtime.run(...)` — addon-side allocate + upload + dispatch
   + download.  **O(matmul time)** which is what we came here
   for.
5. `bufferToMatrix(out)` — copy the output `Buffer` back into a
   `number[][]`.  **O(rows × cols)**.

For matrices < ~16x16, the marshalling overhead probably
dominates the matmul.  For matrices ≥ ~64x64 it's negligible
versus the actual compute.

MX08's parity test asserts numerical equivalence and the speed
crossover point.  The expectation is "Node adapter is no slower
than pure-TS for 16x16 and clearly faster for 64x64+".  If
profiling shows otherwise on real downstream workloads, the
adapter caches `Graph` instances by shape — same shape every
call means the per-call JSON parse goes away.  v1 optimization,
not v0 scope.

### Bigger optimization: zero-copy Buffer views

`Matrix` is `number[][]` today.  Refactoring storage to a
single flat `Float64Array` (typed-array, backed by the same
`ArrayBuffer` we hand to `runtime.run`) would eliminate steps 3
and 5 entirely.  But:

* All 6 downstream packages access `.data[i][j]` directly via
  the public `get` / `set` methods, OR via the `data: number[][]`
  property in their tests.  Switching to a flat view would be a
  breaking change.
* The `Matrix` class has ~40 methods that assume `number[][]`
  storage.  Refactoring touches every one.

MX08 ships the marshalling adapter as-is, with parity tests.
A future MX10 (likely much later, gated by actual profiling
showing this matters for someone's workload) refactors `Matrix`
to flat-storage and updates all 6 downstream packages
mechanically.

---

## Backward compatibility

The public surface stays identical:

| Symbol | Status under MX08 |
|--------|------------------|
| `Matrix` class + all instance methods | unchanged |
| `MatrixBackend` interface | unchanged |
| `CpuMatrixBackend` class | unchanged name; **Node import gets the napi-backed impl**, browser import gets the pure-TS impl |
| `CPU_MATRIX_BACKEND` constant | unchanged signature; the instance is environment-specific |
| `getMatrixBackend()` / `setMatrixBackend()` / `resetMatrixBackend()` | unchanged |
| `data: number[][]` property on `Matrix` | unchanged (see "Bigger optimization" above) |

None of the 6 downstream packages need source changes.  They
import from `@coding-adventures/matrix` and get whichever
implementation the resolver picks for their environment.

The `tests/matrix.test.ts` suite — including the
"backend swap install/call tracking" tests — passes against
both entry points unchanged.  A new `parity.test.ts` runs the
same op suite through both backends and asserts numerical
equivalence within f64 tolerance.

---

## Migration plan

| Phase | Lands | Status |
|-------|-------|--------|
| 0     | This spec.                                                                                                                                                  | **this PR** |
| 1     | Split `CpuMatrixBackend` from `src/matrix.ts` into `src/backends/cpu-pure-ts.ts` with zero behaviour change.  Re-export from `matrix.ts` to keep public API stable.  All existing tests pass unchanged. | shipped (#3562) |
| 2     | Add `src/backends/cpu-rust-napi.ts` + `src/entry-node.ts` + `src/entry-browser.ts` + the `exports` conditional in `package.json`.  Add `parity.test.ts` proving numerical equivalence within f32 precision tolerance.  The adapter `require()`s the `matrix_rust_napi.node` artifact directly from the Rust crate (not via the ESM-only `@coding-adventures/matrix-rust-napi` TS wrapper, which CommonJS consumers can't load).  Update spec to record the f32 quantisation. | **this PR** |
| 3     | Per-downstream-package verification.  For each of cas-matrix, neural-graph-vm, single-layer-network, two-layer-network, blas-library, network-stack: run `npx vitest run` in the package; assert nothing breaks.  No source change needed (that's the whole point).  Land as 6 individual one-line CHANGELOG bumps. | pending |
| 4     | (Optional, profile-driven.)  Optimize the per-op shim — cache `Graph` instances by shape, switch to per-call JSON cache.                                                                                                                       | pending |

Phases 1, 2, 3 are each their own PR.  Phase 4 is "if and when
profiling shows it matters" — not on the critical path.

---

## What's deliberately not in this PR (or the implementation PRs)

* **Replace `Matrix`'s `number[][]` storage with a flat
  `Float64Array`.**  Mentioned above; would be a breaking
  change for 6 downstream packages.  Deferred to MX10 with
  profiling justification.
* **Remove `setMatrixBackend` / `resetMatrixBackend`.**  Some
  downstream tests rely on swapping in spy backends.  Keep the
  accessor.
* **Add a `Matrix.dtype` field for f32 / i32 / u8 graphs.**
  Out of scope for MX08; covered by a future MX11 that adds
  dtype tracking everywhere `Matrix` flows.
* **Move `cas-matrix`'s "concrete numeric entries" fast path
  to delegate through the new Node `CpuMatrixBackend`.**  It
  already delegates through the `MatrixBackend` interface, so
  it gets the speedup for free.  No change in `cas-matrix`.
* **`matrix-metal` / `matrix-cuda` enablement.**  The napi
  runtime's planner picks these up automatically when registered
  — no change in `typescript/matrix` is needed for the lift.
  MX07's Phase 2 added them to the Rust `Cargo.toml` already.
* **Browser WebGPU path.**  Out of scope; lives in the
  future `matrix-webgpu-ts` per ARCH02 Phase 4.
* **Async / `runAsync()`.**  Each `MatrixBackend` method is
  synchronous today.  Keeping that shape for source-compat
  reasons.  A future `MatrixBackend.runAsync(...)` extension
  is its own spec.

---

## Open questions

* **Does the test runner resolve the `node` conditional in
  vitest?**  Vitest uses Node's resolver by default — should
  pick `node`.  But the `tests/matrix.test.ts` suite needs to
  not assume which backend it's running against (the install
  / swap-in tests already factor this correctly; the
  arithmetic tests check numerical equivalence, so they don't
  care).  Validated by Phase 2's `parity.test.ts`.
* **How does the browser build skip the napi dep?**  Bundlers
  (Vite, Webpack, Rollup) honour the `"browser"` conditional
  and won't try to resolve `@coding-adventures/matrix-rust-napi`
  on a browser build.  For Phase 3 we'll add a smoke that runs
  `tsc --noEmit` against the browser entry point to confirm
  there's no leak.
* **Should the runtime instance be process-global or per
  `CpuMatrixBackend` instance?**  v0 ships
  module-global (one `Runtime.create()` per process is plenty;
  `Runtime` is stateless in MX07 v0).  v1+ may revisit if
  multi-tenant scenarios show contention.

---

## Relationship to other specs

* **ARCH02 §"Phases"** — MX08 is Phase 3 of that roadmap
  ("`typescript/matrix` refactor — conditional `node`/`browser`
  exports").
* **MX07** — this spec depends on it; in particular on Phase 4's
  `@coding-adventures/matrix-rust-napi` TypeScript wrapper.
* **MX09 (future)** — multi-op batch API for hot loops that
  want to amortise the per-op marshalling cost.
* **MX10 (future, gated by profiling)** — flat `Float64Array`
  storage refactor for `Matrix`.
* **MX11 (future)** — dtype tracking on `Matrix` for non-f64
  graphs.
