# `@coding-adventures/ml-framework-core`

Idiomatic TypeScript Tensor + autograd on top of the Rust matrix-cpu
execution engine.  v0.1.0 ships the Tensor layer in pure TypeScript;
autograd, op dispatch through Rust, and an end-to-end MLP land in
PRs #2–#5.

## Install

```bash
npm install @coding-adventures/ml-framework-core
```

(Not yet published; in-repo path-resolution works today via the workspace.)

## Quick start (v0.1.0)

```ts
import { Tensor } from "@coding-adventures/ml-framework-core";

// Construction
const a = new Tensor([[1, 2, 3], [4, 5, 6]]);  // shape inferred → [2, 3]
const b = new Tensor([1, 2, 3], { shape: [3] });

// Factories
Tensor.zeros(2, 3);
Tensor.ones(3);
Tensor.full([2, 2], 7.5);
Tensor.eye(3);                  // 3×3 identity
Tensor.eye(2, 3);               // rectangular
Tensor.arange(5);               // 0, 1, 2, 3, 4
Tensor.arange(0, 10, 2);        // 0, 2, 4, 6, 8
Tensor.randn([3, 4], 42);       // standard-normal with seed
Tensor.fromArray([[1, 2], [3, 4]]);

// Shape ops
a.reshape([3, 2]);
a.transpose();                  // 2-D only in v0.1
a.flatten();
a.squeeze();
a.unsqueeze(0);

// Element-wise math (TypeScript has no operator overloading;
// chain methods instead — PyTorch-from-JS convention)
a.add(b)   a.sub(b)   a.mul(b)   a.div(b)
a.pow(2)   a.neg()
a.add(5)                        // scalar broadcasts

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

## What's intentionally NOT in v0.1.0

| Feature                | Lands in | Why deferred                              |
|------------------------|----------|-------------------------------------------|
| Autograd (`backward`)  | PR #2    | Needs its own focused PR                  |
| Rust dispatch          | PR #3    | Needs autograd graph first                |
| Broadcasting           | PR #3    | Couples Tensor to shape algebra           |
| Indexing/slicing       | post-#5  | 50+ lines of arg-shape handling           |
| Reductions (sum/mean)  | PR #3    | These become Function subclasses          |
| Higher-rank transpose  | post-#5  | Generic strided index math not needed yet |

## Storage model

- `data` is a `Float32Array`.
- `shape` is `readonly number[]`.
- `dtype` is `"f32"`.

Using `Float32Array` instead of `number[]`:

- **No lossy conversion at the dispatch boundary** — the bytes we'd send
  to matrix-cpu are already f32 in memory.  No round-trip through f64.
- **Roughly 2-3× faster** for tight inner loops (V8 specializes the
  contiguous-Float32 path).
- **Row-major contiguous layout** — matches matrix-cpu's expectation.

## Architecture (will be filled out by PRs #2–#5)

```
@coding-adventures/ml-framework-core   ← this package (v0.1.0)
       ↓ (PR #2 adds autograd)
       ↓ (PR #3 adds Rust dispatch above 10k cells)
@coding-adventures/matrix-rust-napi    ← TS wrapper (v0.4.0, exists)
       ↓
matrix-rust-napi (Rust cdylib)         ← N-API addon (v0.3.0, exists)
       ↓
node-bridge (Rust workspace crate)     ← zero-dep N-API wrapper (v0.1.0, exists)
       ↓
matrix-ir-json → matrix-ir → matrix-runtime → matrix-cpu
```

The bottom three layers already exist — this package is the top of the
TS stack we're building.

## Testing

```bash
npm install
npm test
```

The v0.1.0 suite runs ~50 vitest cases covering construction,
factories, shape ops, element-wise math, equality, and round-trip
properties.

## License

MIT.
