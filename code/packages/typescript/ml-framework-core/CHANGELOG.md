# Changelog

## Unreleased

### Added — v1.2.0: N-D batched MatMul + higher-rank Transpose (Phase A.2)

PR #2 of 7 in **Phase A** — v1.0 had a 2-D-only `MatMul` and a
2-D-only `transpose`; ML code with batches (attention, batched matmul
in MLPs over sequences, etc.) couldn't express what it needed without
manually reshaping.  v1.2 fixes both.

#### What works now

```ts
// Batched matmul: (B, M, K) @ (B, K, N) → (B, M, N)
const a = Tensor.zeros(8, 3, 4);      // 8 batches of (3, 4)
const b = Tensor.zeros(8, 4, 5);      // 8 batches of (4, 5)
a.matmul(b).shape;                    // → [8, 3, 5]

// Broadcast right operand: (B, M, K) @ (K, N) → (B, M, N)
const w = Tensor.zeros(4, 5);         // shared weight matrix
a.matmul(w).shape;                    // → [8, 3, 5]; gradient on `w` sums across batch

// Multi-batch broadcasting: (B1, 1, M, K) @ (1, B2, K, N) → (B1, B2, M, N)
const x = Tensor.zeros(2, 1, 3, 4);
const y = Tensor.zeros(1, 5, 4, 6);
x.matmul(y).shape;                    // → [2, 5, 3, 6]

// N-D transpose with arbitrary perm
const t = Tensor.zeros(2, 3, 4, 5);
t.transpose(2, 0, 3, 1).shape;        // → [4, 2, 5, 3]
```

#### Implementation notes

- **`MatMulOp`** (`src/ops.ts`): the 2-D fast path is unchanged
  (identical bytes/numerical output as v1.0; the Rust dispatch path
  still triggers at `numel ≥ 10_000`).  For rank ≥ 3 on either input,
  the op splits each tensor into a "batch portion" (all dims except
  the trailing two) and a "matrix portion".  Batch portions broadcast
  via the existing `broadcastShapes`/`broadcastDataTo` helpers from
  Phase A.1; the matrix portion stays untouched (broadcasting matrix
  dims is not what batched matmul means).  Then a per-slice 2-D matmul
  loop reuses the existing `_matmulNaive` helper.
- **Backward** for batched matmul applies the same per-slice formulas
  (`dL/dA_slice = grad_slice @ B_slice^T`, `dL/dB_slice = A_slice^T @
  grad_slice`), then `unbroadcastDataTo` flows gradients back to each
  parent's original shape — so a shared/broadcast operand (e.g. a
  per-layer weight matrix used across a batch) receives a properly
  summed gradient.
- **`Tensor.transpose`** (`src/tensor.ts`): 2-D fast path preserved.
  For rank ≥ 3, generic strided index math: walk every output flat
  index, decompose into output coordinates, map to input coordinates
  via the inverse permutation, look up the source value.  O(numel).
  Pure TS — no Rust dispatch yet (matrix-cpu would need a generic
  Transpose op; deferred).
- Rust dispatch is intentionally limited to the original 2-D MatMul
  path for now.  A future PR can add a `BatchMatMul` op to matrix-ir
  and lift the per-slice loop into Rust SIMD — for now, the TS path
  is correct and fast at the parameter sizes ML training uses.

#### Tests

- 15 new vitest cases (`tests/batched-matmul.test.ts` + extended
  transpose coverage in `tests/tensor.test.ts`).
- Covers all five supported batched-matmul shape patterns, shape
  validation (rank < 2 rejected, inner-dim mismatch rejected,
  incompatible batch dims rejected), backward gradient correctness
  for the broadcast cases, plus N-D transpose forward + roundtrip.
- The existing v1.0 2-D matmul + 2-D transpose tests still pass
  bit-identically.
- **194 tests pass.**  Up from 179 in v1.1.

#### Why this matters for the bigger picture

Batched matmul + N-D transpose are the two ops you need before
implementing anything attention-shaped (Q, K, V tensors are
typically rank-3 or rank-4; computing `Q @ K^T` for attention
requires both batched matmul and a higher-rank transpose).  This PR
clears the runway for Phase A.3 (Embedding) and the eventual
transformer architectures.

### Added — v1.1.0: NumPy-style broadcasting (Phase A.1)

PR #1 of 7 in **Phase A** — broadening the op vocabulary toward
realistic models.  v1.0 only accepted same-shape inputs to binary ops;
v1.1 brings NumPy/PyTorch-style broadcasting.

#### What works now

```ts
// Bias + (batch, features) — previously had to materialize bias as
// (batch, features) yourself; now broadcasts automatically.
const bias = new Tensor([0.1, 0.2, 0.3]);          // (3,)
const x = new Tensor([[1, 2, 3], [4, 5, 6]]);     // (2, 3)
const y = x.add(bias);                              // → (2, 3)

// Outer product: (3, 1) * (1, 4) → (3, 4)
const a = new Tensor([[1], [2], [3]]);
const b = new Tensor([[10, 20, 30, 40]]);
const c = a.mul(b);                                 // (3, 4)

// Gradient flow is correct: gradients are unbroadcast back to the
// ORIGINAL input shapes (stretched dims are summed out).
bias.requiresGrad = true;
y.backward();
console.log(bias.grad!.shape);                      // → [3], NOT [2, 3]
```

#### New module: `src/broadcasting.ts`

Three pure helpers (no Tensor allocation cost):

- `broadcastShapes(a, b)` — pure shape math; returns the broadcast
  shape or throws RangeError on incompatibility.  Follows NumPy rules:
  right-align, pad shorter with 1s on the left, each dim must be equal
  or one must be 1.
- `broadcastDataTo(data, fromShape, toShape)` — materializes a fresh
  Float32Array in `toShape` layout by stretching size-1 dims.  Used by
  binary ops to align inputs before elementwise math.
- `unbroadcastDataTo(data, fromShape, toShape)` — inverse for backward:
  sums the gradient along stretched dims.  Used by binary op backward.

#### Updated ops (Add / Sub / Mul / Div)

- `forward()` now calls `broadcastShapes` and broadcasts inputs to the
  common output shape via `broadcastDataTo` before doing elementwise
  math.  Same-shape fast path is preserved (no extra alloc when
  shapes already match).
- `backward()` saves the broadcast and original shapes; gradients are
  unbroadcast back to each parent's original shape using
  `unbroadcastDataTo`.

  Critical insight: when broadcasting stretches a size-1 dim to size N
  in the forward output, the corresponding backward must SUM that dim's
  gradient back into a single cell.  Concretely: `bias (3,)` added to
  `x (B, 3)` → `bias.grad` is the column sums of the (B, 3) gradient,
  shape `(3,)` — not the (B, 3) gradient itself.

#### New op: `BroadcastOp`

Explicit broadcasting as an autograd Function.  Most callers don't
need this — binary ops broadcast implicitly — but for code that wants
to express "promote this bias once" rather than relying on every
downstream op:

```ts
import { BroadcastOp } from "@coding-adventures/ml-framework-core";

const expanded = BroadcastOp.apply(bias, [batch, features]);
// expanded.shape === [batch, features]
// On backward, gradient sums back to bias's original shape.
```

#### Tests added (31 vitest cases, 179 total)

- `broadcastShapes` (8): identical shapes, scalar broadcast, (3,) +
  (2, 3), (5, 1, 3) + (2, 3), (1, 4) + (3, 4), outer-product layout,
  incompatibility detection, zero-sized dims
- `broadcastDataTo` (6): identity copy, row replication, column
  replication, outer-product materialization, incompatible target
- `unbroadcastDataTo` (5): identity, sum along axis 0, sum along axis
  1, row-sum to single column, 3-D leading-axis sum
- Binary ops with broadcasting (5): Add/Sub/Mul/Div with various
  broadcast patterns, incompatibility errors
- Binary op backward unbroadcasts (3): bias-gradient column-sum,
  outer-product backward gradients, sign correctness for Sub
- `BroadcastOp` (4): forward broadcast, backward gradient summing,
  explicit seed grad, chaining through binary ops

Existing 148 tests continue to pass — no regressions.

Total: 179 tests, 0 failures.

#### What's next in Phase A

- **A.2** — N-D batched MatMul + Transpose for rank ≥ 3 (foundation
  for attention)
- **A.3** — `Embedding` op (lookup table, the first transformer op)
- **A.4** — `LayerNorm` + `BatchNorm` + `Dropout`
- **A.5** — `Conv2D` + `MaxPool2D` via im2col
- **A.6** — Adam optimizer + `Linear` / `Sequential` layer abstractions
- **A.7** — safetensors read/write (first Hugging Face interop)

After A.7: load a tiny transformer checkpoint from HF Hub, run
inference on it, fine-tune it, save the result — all in TypeScript.

### Added — v1.0.0: complete TypeScript ML framework on the Rust matrix-cpu engine

PR #5 of 5 (FINAL) in the JS/TS pilot.  v1.0.0 marks the completion of
the multi-PR plan: a PyTorch-shaped TypeScript ML framework with all 15
ops (forward + backward), autograd engine, end-to-end MLP training, and
a benchmark script for performance characterization.

#### What v1.0.0 means

The complete stack works:

```ts
import { Tensor } from "@coding-adventures/ml-framework-core";

// Build a model
let w1 = new Tensor([[0.5, -0.3]]); w1.requiresGrad = true;
let w2 = new Tensor([[0.4], [0.7]]); w2.requiresGrad = true;

// Train (loss drops 75%+ in 30 SGD steps on the test suite's data)
for (let step = 0; step < 30; step++) {
  const pred = x.matmul(w1).relu().matmul(w2);
  const loss = pred.sub(target).mul(pred.sub(target)).mean();
  loss.backward();
  w1 = sgdStep(w1, 0.01);
  w2 = sgdStep(w2, 0.01);
}
```

All math is pure TypeScript for tensors under 10k cells.  At 10k+ cells
the ops auto-dispatch through `@coding-adventures/matrix-rust-napi` to
the Rust matrix-cpu executor (SIMD-accelerated f32).  No Rust toolchain
is required to use the package at small/medium sizes.

#### New in v1.0.0

- **`scripts/benchmark.ts`** — performance characterization.  Runs
  forward + backward on a 2-layer MLP at batch sizes 100, 1000, 5000,
  10_000, 50_000 and prints a markdown table of median timings.
  Gracefully handles the case where matrix-rust-napi isn't built
  (skips Rust-only rows with a clear "Rust needed" label).  Run via
  `npm run benchmark`.

- **Package.json polish**:
  - Tightened description to highlight v1.0.0 scope (15 ops, autograd,
    auto-dispatch, end-to-end training verified)
  - Added `tsx` as a devDependency for the benchmark script
  - Added `"benchmark"` npm script

- **README polish**: Quick-start section with the end-to-end MLP
  example at the top; Benchmark section with example output table;
  Test coverage section; "Future work" table.

#### Layered architecture (now complete)

```
@coding-adventures/ml-framework-core (THIS PACKAGE, v1.0.0)
  Tensor + autograd + 15 differentiable ops (forward + backward)
  ↓ dispatch large tensors through ↓
@coding-adventures/matrix-rust-napi (v0.4.0)
  TypeScript wrapper for the Rust N-API addon
  ↓
matrix-rust-napi (Rust cdylib, v0.3.0)
  Exposes runGraphOnCpu via N-API
  ↓
node-bridge (Rust workspace crate, v0.1.0)
  Zero-dep N-API wrapper
  ↓
matrix-ir-json → matrix-ir → matrix-runtime → matrix-cpu
```

#### Test coverage (unchanged from v0.4.0 — all still passing)

- `tests/tensor.test.ts`             61 tests
- `tests/autograd.test.ts`           18 tests
- `tests/ops.test.ts`                67 tests
- `tests/end-to-end-training.test.ts` 2 tests
- Total: 148 tests, 0 failures

#### What's next (for the project, not the package)

The user will pick the next language pilot — Lua, Go, or Swift.  The
architecture pattern established by both the Ruby pilot (8 PRs) and
this JS/TS pilot (5 PRs because the bottom three layers already
existed) transfers directly.

### Added — v0.4.0: backward dispatch + end-to-end MLP training test

PR #4 of 5 in the JS/TS pilot.  Adds `backward(outputGrad)` to all 15
Function subclasses from v0.3.0, plus an end-to-end test that trains a
2-layer MLP and asserts loss decreases.  The full PyTorch-shaped
forward+backward+SGD loop now works in pure TypeScript.

#### Backward formulas (mirroring Ruby PR #7 + Python `autograd.py`)

| Op       | Saves in forward    | Backward formula                                  |
|----------|---------------------|---------------------------------------------------|
| Add      | (nothing)           | `[g, g]`                                          |
| Sub      | (nothing)           | `[g, -g]`                                         |
| Mul      | inputs a, b         | `[g * b, g * a]`                                  |
| Div      | inputs a, b         | `[g / b, -g * a / b²]`                            |
| Neg      | (nothing)           | `[-g]`                                            |
| Abs      | input a             | `[g * sign(a)]`  (sign(0) = 0)                    |
| Pow      | input a, scalar e   | `[g * e * a^(e-1)]`  (e gets no grad)             |
| MatMul   | inputs A, B         | `[g @ B^T, A^T @ g]`                              |
| ReLU     | input a             | `[g * (a > 0 ? 1 : 0)]`                           |
| Sigmoid  | output y            | `[g * y * (1 - y)]`                               |
| Tanh     | output y            | `[g * (1 - y²)]`                                  |
| GELU     | input a             | `[g * (0.5*(1+tanh_v) + 0.5*x*sech²*d_inner)]`    |
| Softmax  | output y            | `[y * (g - Σ(g*y))]` per-row over last axis       |
| Sum      | input shape         | `[broadcast(g[0], shape)]`                        |
| Mean     | input shape, numel  | `[broadcast(g[0] / numel, shape)]`                |

All backward implementations are pure TypeScript for v0.4.0.  Routing
through Rust (mirroring v0.3.0's forward dispatch) would require new
envelope shapes per backward op — deferred to a follow-up.  The pure-TS
versions are correct and fast enough for parameter-shaped tensors.

#### MatMul backward uses internal static helpers

`MatMulOp._matmulNaive` and `_transpose2D` are pure functions that
operate on raw `ArrayLike<number>` data.  Backward uses them directly
(not `MatMulOp.apply(...)`) so the backward computation stays a leaf
math operation — no extra autograd subgraph.

#### End-to-end MLP test

New file `tests/end-to-end-training.test.ts` runs a real training loop:

```ts
let w1 = new Tensor([[0.5, -0.3]]); w1.requiresGrad = true;
let w2 = new Tensor([[0.4], [0.7]]); w2.requiresGrad = true;

for (let step = 0; step < 30; step++) {
  const pred = x.matmul(w1).relu().matmul(w2);
  const diff = pred.sub(target);
  const loss = diff.mul(diff).mean();
  loss.backward();
  w1 = sgdStep(w1, 0.01);
  w2 = sgdStep(w2, 0.01);
}
expect(finalLoss).toBeLessThan(initialLoss * 0.25);  // 75%+ drop
```

Synthetic dataset (4 samples, regress `y = 2x + 3`) — converges
substantially in 30 SGD steps.  Mostly-monotonic check allows up to
⌊steps/3⌋ noisy up-steps (SGD is noisy).

Companion test: 1-layer linear regression converges `w` from 0.5 to
≈2.0 in 20 steps.

#### Tests added (19 new vitest cases)

- `BackwardCorrectness` (17 tests in tests/ops.test.ts):
  - One test per op for analytical cases: Add, Sub, Mul, Div, Neg,
    Abs, Pow, MatMul, ReLU, Sigmoid, Tanh, Softmax (uniform → zero),
    Sum, Mean
  - Numerical-gradient checking via finite differences for the
    transcendentals (GELU at x=1, Softmax at x=[1,2,3] with seed
    grad [1,0,0])
  - Chained-op backward: x → Mul → Add → Sum → backward; assert x.grad
    propagates through the chain
- `EndToEndTraining` (2 tests in new file): MLP loss drops 75%+; 1-layer
  linear regression converges

All existing tests still pass — no regressions.

Total: 148 tests, 0 failures.

#### What's next

- PR #5: benchmark script + v1.0.0 polish.  Bumps to v1.0.0 —
  complete TS ML framework on top of the Rust matrix-cpu engine via
  matrix-rust-napi.

### Added — v0.3.0: forward op dispatch (15 Function subclasses)

PR #3 of 5 in the JS/TS pilot.  Adds the 15 differentiable operations on
top of v0.2's autograd engine.  Every op is a `Function` subclass with a
`forward` method; the autograd graph builds automatically when any input
has `requiresGrad`.  Backward implementations land in PR #4.

#### The 15 ops

| Op       | Class      | Rust dispatch?     | matrix-ir kind                   |
|----------|------------|--------------------|----------------------------------|
| Add      | `AddOp`    | yes (≥ 10k cells)  | `Add (lhs/rhs/output)`           |
| Sub      | `SubOp`    | yes                | `Sub`                            |
| Mul      | `MulOp`    | yes                | `Mul`                            |
| Div      | `DivOp`    | yes                | `Div`                            |
| Neg      | `NegOp`    | yes                | `Neg (input/output)`             |
| Abs      | `AbsOp`    | yes                | `Abs`                            |
| Tanh     | `TanhOp`   | yes                | `Tanh`                           |
| MatMul   | `MatMulOp` | yes (2-D only)     | `MatMul (a/b/output)`            |
| Sum      | `SumOp`    | yes (reduce-all)   | `ReduceSum (axes/keep_dims)`     |
| Mean     | `MeanOp`   | yes                | `ReduceMean`                     |
| Pow      | `PowOp`    | pure TS            | —  (Rust Pow takes 2 tensors)    |
| ReLU     | `ReLUOp`   | pure TS            | —  (Max + zero-tensor constant)  |
| Sigmoid  | `SigmoidOp`| pure TS            | —  (4-op multi-graph)            |
| GELU     | `GELUOp`   | pure TS            | —  (large multi-op graph)        |
| Softmax  | `SoftmaxOp`| pure TS            | —  (multi-op graph)              |

The "pure TS" five have working Rust paths in the Python reference;
left in pure TS for v0.3.0 to keep this PR focused (matches Ruby's
PR #6).  Follow-ups can lift them individually.

#### Dispatch threshold

`DISPATCH_THRESHOLD = 10_000` cells.  Below it, every op uses pure
TypeScript — fast at small sizes, avoids the JSON-build + hex-encode
+ FFI overhead.  Above it, the eligible ops dispatch into the Rust
executor via `@coding-adventures/matrix-rust-napi.runGraphOnCpu`.

#### Lazy require for matrix-rust-napi

`@coding-adventures/matrix-rust-napi` is `require`d LAZILY inside
`runEnvelope` — only when we actually need to dispatch to Rust.  This
keeps small-tensor / pure-TS workflows runnable even when the
`matrix_rust_napi.node` addon isn't built (e.g. CI machines without a
Rust toolchain, or fresh checkouts).  We use `createRequire(import.meta.url)`
since this package is ESM but the addon is loaded as CJS.

#### Hex packing helpers

```ts
packF32Hex([1.0, 2.5])           // → "0000803f00002040"
unpackF32Hex("0000803f", 1)      // → Float32Array([1.0])
```

Uses Node's `Buffer.from(...).toString("hex")` for encode and
`Buffer.from(hex, "hex")` view-over-Float32Array for decode.  Matches
the Python reference's `struct.pack("<{n}f", ...)` byte-for-byte so
envelopes built here are bit-compatible with Python and Ruby.

#### Tensor extensions

```ts
// Element-wise math — now routes through Op classes (autograd-aware)
a.add(b)   a.sub(b)   a.mul(b)   a.div(b)   a.pow(2)   a.neg()

// Scalar broadcasting (small-tensor only for v0.3.0)
a.add(5)

// Named op methods
a.matmul(b)
a.relu()  a.sigmoid()  a.tanh()  a.gelu()  a.softmax()
a.sum()   a.mean()
a.abs()
```

The previous v0.2.0 inline `add`/`sub`/`mul`/`div`/`pow`/`neg` methods
are REPLACED by versions that go through `<Op>.apply(...)`.  Numeric
results are unchanged (Tensor `equals` is value-based), but each call
now builds a graph node if any input has `requiresGrad`.

#### Architecture choices

- **Function.apply always, threshold in forward**: every op routes
  through `Function.apply` (autograd graph builds uniformly); each
  `forward` chooses Rust vs. TS based on numel.  Same shape as
  Ruby's PR #6.
- **Shared `binaryElementwise` / `unaryElementwise` helpers**: each
  op subclass is ~5 lines.
- **Envelope shapes mirror Python `_rust_backend.py` byte-for-byte**:
  same `kind` strings, same tensor/op/inputs/outputs JSON layout.
  Cross-language wire compatibility is the contract.

#### Tests added (49 vitest cases, all passing)

- `HexHelpers` (5): pack/unpack round-trip + known-value spot checks
  + wrong-length rejection
- `ForwardSmall` (22): every op's small-tensor (pure-TS) path —
  Add/Sub/Mul/Div/Neg/Abs/Pow/MatMul/ReLU/Sigmoid/Tanh/GELU/Softmax/Sum/Mean
  with numerical correctness assertions + edge cases (sigmoid at 0,
  tanh approaches 1, softmax sums to 1, softmax numerical stability
  with 1000-magnitude inputs, GELU at 1 ≈ 0.8413, matmul argument
  validation) + backward-not-implemented stub
- `AutogradWiring` (5): every op produces a tensor with the right
  `gradFn` class when input has `requiresGrad`
- `TensorMethods` (14): each `t.<op>()` dispatches correctly,
  including scalar broadcasting, unsupported operand rejection
- `DispatchPathBranching` (2): threshold constant + small tensor
  doesn't trigger matrix-rust-napi lazy require

Existing tests:
- `tests/tensor.test.ts` — 61/61 pass (no regressions)
- `tests/autograd.test.ts` — 18/18 pass (no regressions)

Total: 128 tests, all passing.

#### What's next

- PR #4: backward dispatch — implement `backward(outputGrad)` on each
  of the 15 Function subclasses using the analytical gradient formulas
  from the Ruby pilot's PR #7 (same as Python autograd.py).  End-to-end
  test: train a 2-layer MLP on a tiny synthetic dataset for 30 SGD
  steps; assert loss drops 91%.
- PR #5: benchmark script + v1.0.0 polish.

### Added — v0.2.0: autograd engine (Function.apply + Tensor#backward)

PR #2 of 5 in the JS/TS pilot.  Adds reverse-mode automatic
differentiation on top of v0.1's Tensor class.  All math still pure
TypeScript; PR #3 layers on the 15 concrete Function subclasses that
route large ops through Rust via `@coding-adventures/matrix-rust-napi`.

#### New module: `src/autograd.ts`

```ts
import { Function, Identity, Tensor } from "@coding-adventures/ml-framework-core";

// Base class for every differentiable op:
abstract class Function {
  parents: Tensor[];                          // input Tensors (filtered)
  savedForBackward: Record<string, unknown>;  // subclass scratch
  static apply<T extends Function>(...inputs: unknown[]): Tensor;
  abstract forward(...inputs: unknown[]): Tensor;
  abstract backward(outputGrad: Tensor): (Tensor | null)[];
}

// Built-in subclass for testing the machinery:
class Identity extends Function { ... }

// Tensor gets four new things (added in tensor.ts):
class Tensor {
  requiresGrad: boolean;       // default false; user-mutable
  grad: Tensor | null;         // populated by backward()
  gradFn: Function | null;     // set by Function.apply()
  backward(grad?: Tensor): void;
  static onesLike(t): Tensor;
  static zerosLike(t): Tensor;
}
```

#### How it works (same algorithm as Ruby pilot + Python reference)

`Function.apply(...inputs)`:
  1. Instantiate the Function (so it can hold state for backward).
  2. Filter `inputs` to Tensors and stash as `parents`.  Non-Tensor args
     (e.g. Pow's scalar exponent) flow through to `forward` but don't
     appear in the autograd graph.
  3. Call `forward(...inputs)`.
  4. If any Tensor input has `requiresGrad`, mark the output the same
     way and set `output.gradFn = fn` so backwardImpl can find us.

`Tensor#backward(grad?)`:
  1. Default `grad` to `onesLike(this)`.
  2. DFS post-order to build the topological list upstream via gradFn.
  3. Walk topo list REVERSE: for each non-leaf, call gradFn.backward,
     distribute per-input grads into a `Map<Tensor, Tensor>` keyed on
     object identity (JS Maps use reference equality for object keys —
     perfect for our shared-parent case).
  4. For each leaf with requiresGrad, accumulate the gradient into
     `.grad`.  Supports repeated backward() without zero_grad.

O(V + E) where V is operations, E is tensor edges.

#### Architecture choices

- **Free `backwardImpl` function, not Tensor method**: keeps tensor.ts
  focused on storage; the autograd walker lives in autograd.ts.
  `Tensor#backward(grad?)` is a 1-line delegate.
- **`Map<Tensor, Tensor>` for gradMap**: JS Map uses reference equality
  for object keys.  Same Tensor reaching via multiple paths (e.g.
  `Add(x, x)`) collapses to one Map entry — gradients accumulate
  correctly.
- **`Set<Tensor>` for visited**: same reference-equality property.
- **`gradFn` typed `any`**: would otherwise create a tensor.ts ↔
  autograd.ts circular type dependency.  TypeScript handles cycles at
  runtime fine, but the type system needs help.  Documented in code.
- **Static `Function.apply` with `this: new () => T` constraint**:
  preserves the concrete subclass type so `MyOp.apply(...)` returns
  `Tensor` without losing `gradFn`'s `MyOp` type.

#### Tests added (18 vitest cases, all passing)

- `Function.apply — wiring` (5): requiresGrad propagation, no gradFn
  when no input requires grad, apply returns NEW Tensor, abstract
  method errors propagate
- `Tensor#backward — sanity` (3): non-grad tensor throws, mismatched
  grad shape throws, returns void
- `Tensor#backward — end-to-end` (6): identity backward writes ones,
  explicit seed, backward twice accumulates, chain of identities,
  shared parent accumulates, leaf without requiresGrad is skipped
- `Function — introspection` (2): toString shows class + parent count,
  default state on fresh instance
- `Tensor — onesLike / zerosLike` (2)

Existing 61 Tensor tests continue to pass — no regressions.

Total: 79 tests, all passing.

#### What's next

- PR #3: `ops.ts` — 15 Function subclasses (Add/Sub/Mul/Div/Neg/Abs/
  Pow/MatMul/ReLU/Sigmoid/Tanh/GELU/Softmax/Sum/Mean).  Each `forward`
  builds a matrix-ir-json envelope matching the Python `_rust_backend.py`
  shapes and dispatches large tensors through
  `@coding-adventures/matrix-rust-napi.runGraphOnCpu`.
- PR #4: backward dispatch + end-to-end MLP training test.
- PR #5: benchmark + v1.0.0 polish.

### Added — v0.1.0: pure-TypeScript Tensor class

First release.  Ships the bottom layer of the TypeScript ML framework
stack: a PyTorch-shaped `Tensor` class implemented entirely in
TypeScript.  No Rust calls, no native addons.  Layered design — PRs
#2–#5 will add the autograd engine, forward + backward op dispatch,
end-to-end MLP test, and benchmark + RubyGems-style publishing polish.

#### Public API

```ts
import { Tensor } from "@coding-adventures/ml-framework-core";
```

- **Construction**: `new Tensor(data, { shape?, dtype? })`
- **Factories**: `zeros`, `ones`, `full`, `eye` (square or rectangular),
  `arange` (1/2/3-arg, supports negative step, rejects NaN/Infinity),
  `randn` (deterministic with `seed:`, via Box-Muller + seeded LCG),
  `fromArray`
- **Shape ops**: `reshape`, `transpose` (2-D only in v0.1), `flatten`,
  `squeeze` (all-1 or specific axis), `unsqueeze` (negative axes OK)
- **Element-wise math methods**: `add`, `sub`, `mul`, `div`, `pow`, `neg`
  — tensor⊗tensor (same shape) and tensor⊗scalar.  TypeScript has no
  operator overloading; chain methods (`x.add(y).mul(2)`) instead.
- **Conversions**: `toArray` (flat `number[]`), `toNested`
  (shape-respecting nested array)
- **Introspection**: `shape`, `dtype`, `ndim`, `numel`, `equals`,
  `equalsClose`, `toString`

#### Architecture choices

- **`Float32Array` storage** instead of `number[]`:
  - Matches matrix-cpu's f32 dtype byte-for-byte — no lossy round-trip
    at the future FFI dispatch boundary
  - ~2-3× faster for tight inner loops (V8 specializes Float32Array
    aggressively)
  - Row-major contiguous layout matches matrix-cpu's expectation
- **No operator overloading**: TypeScript inherits JavaScript's lack
  of it.  Method-chaining (`x.add(y).relu()`) is unambiguous and reads
  naturally; matches TensorFlow.js convention.
- **Same-shape only for binary ops**: no NumPy-style broadcasting in
  v0.1.  Adding it now would couple Tensor to the shape-broadcasting
  algorithm; pulled in when ops dispatch lands.
- **Box-Muller `randn`**: textbook two-line algorithm; avoids pulling
  in a distribution library.  Seeded variant uses a tiny LCG with
  Numerical Recipes constants (not crypto-secure; documented).
- **Arange rejects non-finite bounds**: `Infinity` would loop forever
  building the result array; `NaN` would silently return empty.  Both
  raise `RangeError`.

#### File layout

```
ml-framework-core/
├── package.json                        (ESM, TS, vitest)
├── tsconfig.json                       (target ES2022, module ESNext)
├── vitest.config.ts                    (80% coverage thresholds)
├── src/
│   ├── index.ts                        (entry point, re-exports)
│   ├── version.ts                      ("0.1.0")
│   └── tensor.ts                       (the Tensor class + helpers)
├── tests/
│   └── tensor.test.ts                  (~50 vitest cases, 7 describe blocks)
├── BUILD / BUILD_windows               (npm install + npm test)
├── required_capabilities.json          (empty — no FFI/net/fs)
├── README.md
└── CHANGELOG.md
```

#### Test coverage (~50 vitest cases)

- **Construction** (8): flat/nested/scalar/deep-nested/ragged-rejection/
  explicit-shape validation/dtype rejection/non-number elements
- **Factories** (16): every factory + arange edge cases (zero step,
  negative step, non-finite bounds) + randn determinism + Box-Muller
  mean sanity check on 1000 samples
- **Shape ops** (12): reshape (mismatch rejection, round-trip),
  flatten, transpose (default, perm, double-identity, invalid perm,
  higher-rank not-implemented), squeeze (default, axis, negative,
  non-unit rejection), unsqueeze (positive, negative, round-trip)
- **Element-wise math** (8): every method + scalar broadcast + shape
  mismatch + type rejection
- **Equality + toString** (4): shape-aware equals, equalsClose with
  epsilon, toString formatting, toString truncation
- **Round-trip** (3): toNested 2-D and 3-D, reshape preserves toArray
- **Helpers** (5): inferShape edge cases, flattenToFloat32 validation
- **Version** (1): VERSION constant well-formed

#### What's next

- PR #2: `autograd.ts` — `Function` base class, `apply`, topological
  sort, `Tensor#backward`.  Mirrors the Ruby pilot's PR #5 structure.
- PR #3: `ops.ts` — 15 Function subclasses (Add, Sub, Mul, Div, Neg,
  Abs, Pow, MatMul, ReLU, Sigmoid, Tanh, GELU, Softmax, Sum, Mean).
  Each dispatches large tensors through `@coding-adventures/matrix-rust-napi`'s
  `runGraphOnCpu`.
- PR #4: backward dispatch + end-to-end MLP training test.
- PR #5: benchmark script + v1.0.0 polish.
