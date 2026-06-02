# Changelog

## Unreleased

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
