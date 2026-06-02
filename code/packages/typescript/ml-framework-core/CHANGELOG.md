# Changelog

## Unreleased

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
