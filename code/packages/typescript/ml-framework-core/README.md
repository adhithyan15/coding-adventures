# `@coding-adventures/ml-framework-core`

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Node](https://img.shields.io/badge/Node-%3E%3D%2020-green)](https://nodejs.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-%3E%3D%205-blue)](https://www.typescriptlang.org/)
[![Tests](https://img.shields.io/badge/tests-148%20passing-brightgreen)]()

A small, PyTorch-shaped TypeScript ML library.  All 15 differentiable
ops, full autograd, end-to-end MLP training — all in idiomatic
TypeScript.  Tensors under 10k cells stay in pure TS; tensors above
auto-dispatch through `@coding-adventures/matrix-rust-napi` to the Rust
`matrix-cpu` executor for SIMD-accelerated f32 math.

## Quick start

```ts
import { Tensor } from "@coding-adventures/ml-framework-core";

// A 2-layer MLP, no bias: 1 input → 2 hidden ReLU → 1 output
let w1 = new Tensor([[0.5, -0.3]]); w1.requiresGrad = true;
let w2 = new Tensor([[0.4], [0.7]]); w2.requiresGrad = true;

// Synthetic data: regress y = 2x + 3 over 4 samples
const x = new Tensor([[0], [1], [2], [3]]);
const target = new Tensor([[3], [5], [7], [9]]);

function sgdStep(p: Tensor, lr: number): Tensor {
  const newData = p.toArray().map((v, i) => v - lr * p.grad!.toArray()[i]!);
  const np = new Tensor(newData, { shape: p.shape.slice() });
  np.requiresGrad = true;
  return np;
}

for (let step = 0; step < 30; step++) {
  const pred = x.matmul(w1).relu().matmul(w2);
  const diff = pred.sub(target);
  const loss = diff.mul(diff).mean();
  loss.backward();
  w1 = sgdStep(w1, 0.01);
  w2 = sgdStep(w2, 0.01);
}

// Loss drops 75%+ from initial in 30 SGD steps.
```

That snippet runs in pure TypeScript today (no native addon required).
The test suite exercises this exact training loop in
`tests/end-to-end-training.test.ts`.

## What's in the box

### `Tensor`

```ts
// Construction
new Tensor(nestedOrFlat, { shape?, dtype? })

// Factories
Tensor.zeros(2, 3);   Tensor.ones(3);    Tensor.full([2, 2], 7.5);
Tensor.eye(3);        Tensor.eye(2, 3);   // rectangular
Tensor.arange(5);     Tensor.arange(0, 10, 2);   Tensor.arange(5, 0, -1);
Tensor.randn([3, 4], 42);                  // standard-normal with seed
Tensor.fromArray([[1, 2], [3, 4]]);
Tensor.onesLike(t);   Tensor.zerosLike(t);

// Shape ops
t.reshape([3, 2]);   t.transpose();   t.flatten();
t.squeeze();         t.unsqueeze(0);

// Element-wise math methods (TypeScript has no operator overloading)
a.add(b)   a.sub(b)   a.mul(b)   a.div(b)
a.pow(2)   a.neg()
a.add(5)                                   // scalar broadcasts

// Named ops
a.matmul(b)
a.relu()   a.sigmoid()   a.tanh()   a.gelu()   a.softmax()
a.sum()    a.mean()      a.abs()

// Autograd
x.requiresGrad = true;
y.backward(grad?);

// Introspection
a.shape;        // → [2, 3]
a.dtype;        // → "f32"
a.ndim;         // → 2
a.numel;        // → 6
a.toArray();    // flat number[]
a.toNested();   // nested array matching shape
a.equals(b);    // structural element-wise equality
a.equalsClose(b, 1e-6);
```

### `Function` (autograd)

```ts
// Define a new differentiable op:
import { Function, Tensor } from "@coding-adventures/ml-framework-core";

class MyOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    const x = inputs[0] as Tensor;
    this.savedForBackward.x = x;
    // ... return a Tensor ...
  }

  backward(grad: Tensor): (Tensor | null)[] {
    const x = this.savedForBackward.x as Tensor;
    // ... return [Tensor or null per parent] ...
  }
}

// Use it:
const y = MyOp.apply(x);
y.backward();
// x.grad now populated.
```

## Op dispatch matrix

| Op       | Rust dispatch (≥10k cells)? | Notes                            |
|----------|-----------------------------|----------------------------------|
| Add/Sub  | yes                         | Elementwise                      |
| Mul/Div  | yes                         | Elementwise                      |
| Neg/Abs  | yes                         | Elementwise                      |
| Tanh     | yes                         | Elementwise activation           |
| MatMul   | yes (2-D only)              | (m,k) @ (k,n) → (m,n)            |
| Sum/Mean | yes (reduce-all)            | Output shape `[1]`               |
| Pow      | pure TS                     | Scalar exponent                  |
| ReLU     | pure TS                     | Would use Max + zero-tensor const|
| Sigmoid  | pure TS                     | 4-op multi-graph                 |
| GELU     | pure TS                     | Large multi-op graph             |
| Softmax  | pure TS                     | Multi-op; numerically stable     |

## Installation

```bash
npm install @coding-adventures/ml-framework-core
```

The package itself is pure TypeScript.  To enable Rust dispatch above
the 10k-cell threshold, also install
`@coding-adventures/matrix-rust-napi` (ships an N-API addon built from
the workspace Rust crate).

## Benchmark

```bash
cd code/packages/typescript/ml-framework-core
npm install
npm run benchmark
```

Example output (Apple M-series, Node 20, pure-TS fallback only):

```
| batch  | forward (ms) | backward (ms) | total (ms) | dispatch       |
|--------|--------------|---------------|------------|----------------|
|    100 |         0.11 |          0.23 |       0.33 | TS (no Rust)   |
|   1000 |         0.32 |          0.47 |       0.79 | TS (no Rust)   |
|   5000 |    (skipped) |     (skipped) |  (skipped) | Rust needed    |
|  10000 |    (skipped) |     (skipped) |  (skipped) | Rust needed    |
|  50000 |    (skipped) |     (skipped) |  (skipped) | Rust needed    |
```

Build the matrix-rust-napi native addon (`cd ../matrix-rust-napi && npm
run build`) to see the Rust-dispatch numbers.

## Architecture

```
┌──────────────────────────────────────────────────────────────────────┐
│  @coding-adventures/ml-framework-core (THIS PACKAGE, v1.0.0)         │
│    Tensor + autograd + 15 differentiable ops (forward + backward)    │
│    ↓ dispatch large tensors through ↓                                │
│  @coding-adventures/matrix-rust-napi (v0.4.0, exists)                │
│    TypeScript wrapper for the Rust N-API addon                       │
│    ↓                                                                  │
│  matrix-rust-napi (Rust cdylib, v0.3.0, exists)                      │
│    Exposes runGraphOnCpu via N-API                                   │
│    ↓                                                                  │
│  node-bridge (Rust workspace crate, v0.1.0, exists)                  │
│    Zero-dep N-API wrapper                                            │
│    ↓                                                                  │
│  matrix-ir-json → matrix-ir → matrix-runtime → matrix-cpu            │
└──────────────────────────────────────────────────────────────────────┘
```

This stack — `node-bridge` → `matrix-rust-napi` → `@coding-adventures/matrix-rust-napi`
→ `@coding-adventures/ml-framework-core` — is the **JS/TS pilot** for a
multi-language plan.  Sister stack: `c-bridge` →
`matrix_rust_ruby_native` → `matrix_rust_ruby` →
`coding_adventures_ml_framework_core` (Ruby).

## Test coverage

148 vitest cases, 0 failures across 4 files:

```bash
npm test                   # runs all four
# Or individually:
npx vitest run tests/tensor.test.ts                # 61 tests
npx vitest run tests/autograd.test.ts              # 18 tests
npx vitest run tests/ops.test.ts                   # 67 tests
npx vitest run tests/end-to-end-training.test.ts   #  2 tests
```

## Storage model

- `data` is a `Float32Array`.
- `shape` is `readonly number[]`.
- `dtype` is `"f32"`.

Using `Float32Array` instead of `number[]`:

- **No lossy conversion at the dispatch boundary** — the bytes we'd
  send to matrix-cpu are already f32 in memory.  No round-trip through
  f64.
- **Roughly 2-3× faster** for tight inner loops (V8 specializes the
  contiguous-Float32 path).
- **Row-major contiguous layout** — matches matrix-cpu's expectation.

## Future work

| Feature                | Status                                |
|------------------------|---------------------------------------|
| Broadcasting           | Deferred — couples Tensor to shape algebra |
| Indexing/slicing       | Deferred — 50+ lines of arg-shape handling |
| Higher-rank transpose  | Deferred — generic strided index math not needed yet |
| Rust dispatch for Pow/ReLU/Sigmoid/GELU/Softmax | Deferred — multi-op graphs / constants would double PR size |
| Backward dispatch through Rust | Deferred — same envelope-shape investment as forward |

## License

MIT.
