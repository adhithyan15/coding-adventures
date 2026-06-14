/**
 * # Tensor — N-dimensional Float32Array with shape
 *
 * The TypeScript Tensor.  Same role as PyTorch's `torch.Tensor` and the
 * Ruby `Tensor`: hold a flat array of f32 values plus a shape, and
 * provide all the factory + reshape + math methods on top.
 *
 * ## Storage model
 *
 * - `data`  — `Float32Array` (native typed array; matches the matrix-cpu
 *             f32 dtype byte-for-byte, no conversion at dispatch).
 * - `shape` — readonly `number[]` of positive integers; empty array for
 *             a 0-d scalar.
 * - `dtype` — always `"f32"` in v0.1.
 *
 * ### Why `Float32Array`, not `number[]`?
 *
 * - **Type accuracy.**  JS's `number` is f64; storing f32 values in a
 *   `number[]` would round-trip them through f64 on every read.  A
 *   `Float32Array` view stores the bits we'd send to matrix-cpu, so
 *   there's never a lossy conversion at the FFI boundary.
 * - **Performance.**  Native typed arrays are roughly 2-3× faster than
 *   `number[]` for the kind of tight inner loops Tensor ops require
 *   (V8 specializes hard on the contiguous-Float32 case).
 * - **Memory layout.**  Already row-major and contiguous — matches
 *   matrix-cpu's expectation without a copy.
 *
 * ## No operator overloading
 *
 * TypeScript inherits JavaScript's lack of operator overloading.  Instead
 * of `a + b`, we use `a.add(b)`.  This is the same convention as TensorFlow.js
 * and NumPy-from-JS libraries.  It's noisier but unambiguous, and chains
 * read naturally: `x.matmul(w).relu().add(bias)`.
 *
 * ## What's NOT here (deferred)
 *
 * - Broadcasting: same shape only for v0.1.
 * - Indexing slices (`t[1, 2]` or `t.slice(...)`).
 * - Autograd-prep slots (`requiresGrad`, `grad`, `gradFn`) — added in PR #2.
 * - The 15 differentiable ops with `.add()`, `.relu()`, etc. — added in PR #3
 *   on top of the autograd `Function.apply` machinery from PR #2.  v0.1's
 *   `.add()` family below are pure-Ruby fallback math without autograd.
 */

import { backwardImpl } from "./autograd.js";
import {
  AddOp,
  SubOp,
  MulOp,
  DivOp,
  PowOp,
  NegOp,
  AbsOp,
  MatMulOp,
  ReLUOp,
  SigmoidOp,
  TanhOp,
  GELUOp,
  SoftmaxOp,
  SumOp,
  MeanOp,
  EmbeddingOp,
  LayerNormOp,
  BatchNormOp,
  DropoutOp,
  Conv2DOp,
  MaxPool2DOp,
} from "./ops.js";

export type Dtype = "f32";

export type Shape = readonly number[];

// ---------------------------------------------------------------------------
// Helper functions — public so the test suite can exercise them directly.
// ---------------------------------------------------------------------------

/**
 * Walk a possibly-nested array and infer its shape.  Validates that every
 * sub-array at the same depth has the same length (rectangular nesting).
 *
 * @throws TypeError if the structure is ragged
 */
export function inferShape(data: unknown): number[] {
  if (!Array.isArray(data)) return [];
  if (data.length === 0) return [0];

  const shape: number[] = [];
  let probe: unknown = data;
  while (Array.isArray(probe)) {
    shape.push(probe.length);
    probe = probe[0];
  }

  // Validate rectangularity at every depth.
  validateRectangular(data, shape, 0);

  return shape;
}

function validateRectangular(node: unknown, shape: number[], depth: number): void {
  if (depth >= shape.length) return;

  if (!Array.isArray(node) || node.length !== shape[depth]) {
    throw new TypeError(
      `ragged nested array: expected length ${shape[depth]} at depth ${depth}, got ${
        Array.isArray(node) ? node.length : typeof node
      }`,
    );
  }

  for (const child of node) {
    validateRectangular(child, shape, depth + 1);
  }
}

/**
 * Flatten arbitrarily-nested arrays into a single Float32Array.  Validates
 * the result length matches the expected `numel` (product of shape).
 */
export function flattenToFloat32(data: unknown, expected: number): Float32Array {
  const flat: number[] = [];
  flatten(data, flat);
  if (flat.length !== expected) {
    throw new RangeError(
      `data length ${flat.length} does not match expected numel ${expected}`,
    );
  }
  return Float32Array.from(flat);
}

function flatten(node: unknown, out: number[]): void {
  if (Array.isArray(node)) {
    for (const child of node) flatten(child, out);
  } else if (typeof node === "number") {
    out.push(node);
  } else {
    throw new TypeError(`tensor data must be numbers; got ${typeof node}`);
  }
}

// ---------------------------------------------------------------------------
// The Tensor class itself.
// ---------------------------------------------------------------------------

export interface TensorOptions {
  shape?: Shape;
  dtype?: Dtype;
}

export class Tensor {
  /** Flat row-major data, length = numel. */
  public readonly data: Float32Array;

  /** Shape array — never mutated after construction. */
  public readonly shape: readonly number[];

  /** Element dtype.  Always "f32" in v0.1. */
  public readonly dtype: Dtype;

  /**
   * Whether ops that consume this tensor should build an autograd graph.
   * Mutable (PyTorch convention): set via `t.requiresGrad = true` on
   * leaf tensors that should track gradients.
   */
  public requiresGrad: boolean = false;

  /**
   * Accumulated gradient after `backward()` runs.  `null` until then.
   * For non-leaf tensors, this stays `null` — only leaves with
   * `requiresGrad = true` accumulate here.
   */
  public grad: Tensor | null = null;

  /**
   * The `Function` instance that produced this tensor.  `null` for
   * leaves (tensors constructed directly, not via `Function.apply`).
   * Typed as `unknown` to break the tensor.ts ↔ autograd.ts cycle —
   * the actual type is `Function` from autograd.ts.  Callers should
   * cast.  (We use a thin getter elsewhere when we really need the
   * concrete type.)
   *
   * The walker in `autograd.ts` is the only place this is read.
   */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  public gradFn: any = null;

  /**
   * Construct a Tensor from either a flat array (with explicit shape) or a
   * nested array (shape is inferred from nesting).
   */
  constructor(data: unknown, options: TensorOptions = {}) {
    const dtype = options.dtype ?? "f32";
    if (dtype !== "f32") {
      throw new TypeError(`dtype ${dtype} not supported; v0.1 supports "f32" only`);
    }

    if (options.shape !== undefined) {
      // Explicit shape — caller must already have flat (or single-level
      // nested) data of the matching numel.  Validate carefully.
      const shape = options.shape.map((n) => Math.trunc(n));
      const expected = shape.reduce((a, b) => a * b, 1);
      this.data = flattenToFloat32(data, expected);
      this.shape = shape;
    } else {
      // Infer from nesting.
      const inferred = inferShape(data);
      const expected = inferred.reduce((a, b) => a * b, 1);
      this.data =
        inferred.length === 0 && typeof data === "number"
          ? Float32Array.from([data])
          : flattenToFloat32(data, expected);
      this.shape = inferred.length === 0 ? [1] : inferred;
    }

    this.dtype = "f32";
  }

  // -------------------------------------------------------------------------
  // Introspection
  // -------------------------------------------------------------------------

  /** Number of dimensions (rank). */
  get ndim(): number {
    return this.shape.length;
  }

  /** Total number of elements. */
  get numel(): number {
    return this.shape.reduce((a, b) => a * b, 1);
  }

  /** Flat copy of the data as a `number[]` for callers that want plain JS. */
  toArray(): number[] {
    return Array.from(this.data);
  }

  /**
   * Recursively un-flatten the data into a nested array matching `shape`.
   * Round-trip: `new Tensor(t.toNested()).equals(t) === true`.
   */
  toNested(): number | number[] | number[][] | unknown {
    if (this.shape.length === 0) return this.data[0];
    return unflatten(Array.from(this.data), this.shape, 0, 0).value;
  }

  /**
   * Element-wise equality.  Two tensors are equal iff their shapes match
   * and every element matches.  Strict equality on numbers — for f32
   * tensors where rounding may have happened, use `equalsClose`.
   */
  equals(other: Tensor): boolean {
    if (!shapesEqual(this.shape, other.shape)) return false;
    if (this.data.length !== other.data.length) return false;
    for (let i = 0; i < this.data.length; i++) {
      if (this.data[i] !== other.data[i]) return false;
    }
    return true;
  }

  /**
   * Element-wise approximate equality.  Useful for testing operations
   * involving transcendental functions where f32 precision causes
   * sub-1e-6 differences.
   */
  equalsClose(other: Tensor, epsilon = 1e-6): boolean {
    if (!shapesEqual(this.shape, other.shape)) return false;
    if (this.data.length !== other.data.length) return false;
    for (let i = 0; i < this.data.length; i++) {
      if (Math.abs(this.data[i]! - other.data[i]!) > epsilon) return false;
    }
    return true;
  }

  /** Compact display useful in REPL output. */
  toString(): string {
    const preview = Array.from(this.data.slice(0, 6))
      .map((v) => (Number.isInteger(v) ? v.toFixed(0) : v.toFixed(4)))
      .join(", ");
    const suffix = this.data.length > 6 ? ", …" : "";
    return `Tensor<shape=[${this.shape.join(", ")}] dtype=${this.dtype}>[${preview}${suffix}]`;
  }

  // -------------------------------------------------------------------------
  // Shape operations
  // -------------------------------------------------------------------------

  /** Return a new Tensor with the data viewed under the new shape. */
  reshape(newShape: number[]): Tensor {
    const numel = newShape.reduce((a, b) => a * b, 1);
    if (numel !== this.numel) {
      throw new RangeError(
        `reshape: new shape [${newShape.join(", ")}] has numel ${numel} but tensor has ${this.numel}`,
      );
    }
    return new Tensor(Array.from(this.data), { shape: newShape });
  }

  /** Flatten to 1-D. */
  flatten(): Tensor {
    return new Tensor(Array.from(this.data), { shape: [this.numel] });
  }

  /**
   * Transpose.  Generic N-D perm transpose.
   *
   * - `t.transpose()` with no args: only valid for 2-D, swaps the two dims.
   * - `t.transpose(p0, p1, ...)`: reorders dims so `out.shape[i] = in.shape[perm[i]]`.
   *
   * Matches NumPy's `np.transpose(a, axes=...)` semantics.  Used by
   * higher-rank ML code (e.g. swapping (batch, seq, features) ↔
   * (batch, features, seq) for attention-style ops).
   *
   * For rank ≥ 3 we walk the output index space and, for each output
   * coordinate, compute the matching input coordinate via the inverse
   * permutation: if `outCoord[i] = inCoord[perm[i]]`, then
   * `inCoord[perm[i]] = outCoord[i]`.  Pure strided index math, O(numel).
   */
  transpose(...perm: number[]): Tensor {
    if (perm.length === 0) {
      if (this.ndim !== 2) {
        throw new RangeError(
          `transpose() with no args is only defined for 2-D tensors (got shape [${this.shape.join(", ")}])`,
        );
      }
      perm = [1, 0];
    } else if (perm.length !== this.ndim) {
      throw new RangeError(`transpose perm length ${perm.length} != ndim ${this.ndim}`);
    }

    // Validate perm is a permutation of [0..ndim).
    const sorted = [...perm].sort((a, b) => a - b);
    for (let i = 0; i < sorted.length; i++) {
      if (sorted[i] !== i) {
        throw new RangeError(`transpose perm [${perm.join(", ")}] is not a permutation of [0..${this.ndim})`);
      }
    }

    // 2-D fast path — same code as v1.0, here so 2-D doesn't pay the
    // generic per-index loop overhead.
    if (this.ndim === 2) {
      const [rows, cols] = this.shape as [number, number];
      const out = new Float32Array(rows * cols);
      // perm could be [0,1] (identity) or [1,0] (swap).
      if (perm[0] === 0 && perm[1] === 1) {
        out.set(this.data);
        return new Tensor(Array.from(out), { shape: [rows, cols] });
      }
      for (let r = 0; r < rows; r++) {
        for (let c = 0; c < cols; c++) {
          out[c * rows + r] = this.data[r * cols + c]!;
        }
      }
      return new Tensor(Array.from(out), { shape: [cols, rows] });
    }

    // ── Generic N-D path ──
    const ndim = this.ndim;
    const inShape = this.shape;
    const outShape = perm.map((p) => inShape[p]!);

    // Row-major strides: stride[i] = product of shape[i+1..].
    const inStrides = new Array<number>(ndim);
    const outStrides = new Array<number>(ndim);
    {
      let s = 1;
      for (let i = ndim - 1; i >= 0; i--) {
        inStrides[i] = s;
        s *= inShape[i]!;
      }
      s = 1;
      for (let i = ndim - 1; i >= 0; i--) {
        outStrides[i] = s;
        s *= outShape[i]!;
      }
    }

    const numel = this.numel;
    const out = new Float32Array(numel);
    const outCoords = new Array<number>(ndim).fill(0);
    const inCoords = new Array<number>(ndim).fill(0);

    for (let outIdx = 0; outIdx < numel; outIdx++) {
      // Decompose outIdx into outCoords (row-major).
      let rem = outIdx;
      for (let i = 0; i < ndim; i++) {
        outCoords[i] = Math.floor(rem / outStrides[i]!);
        rem -= outCoords[i]! * outStrides[i]!;
      }
      // Map: out axis i corresponds to in axis perm[i].
      for (let i = 0; i < ndim; i++) {
        inCoords[perm[i]!] = outCoords[i]!;
      }
      let srcIdx = 0;
      for (let i = 0; i < ndim; i++) srcIdx += inCoords[i]! * inStrides[i]!;
      out[outIdx] = this.data[srcIdx]!;
    }
    return new Tensor(Array.from(out), { shape: outShape });
  }

  /** Drop size-1 dimensions.  With no axis, drops all of them. */
  squeeze(axis?: number): Tensor {
    let newShape: number[];
    if (axis === undefined) {
      newShape = this.shape.filter((d) => d !== 1);
    } else {
      let normalized = axis;
      if (normalized < 0) normalized += this.ndim;
      if (normalized < 0 || normalized >= this.ndim) {
        throw new RangeError(`squeeze axis ${axis} out of range [-${this.ndim}, ${this.ndim})`);
      }
      if (this.shape[normalized] !== 1) {
        throw new RangeError(`cannot squeeze axis ${normalized} of size ${this.shape[normalized]}`);
      }
      newShape = this.shape.slice();
      newShape.splice(normalized, 1);
    }
    return new Tensor(Array.from(this.data), { shape: newShape });
  }

  /** Insert a size-1 dimension at the given axis. */
  unsqueeze(axis: number): Tensor {
    let normalized = axis;
    if (normalized < 0) normalized += this.ndim + 1;
    if (normalized < 0 || normalized > this.ndim) {
      throw new RangeError(`unsqueeze axis ${axis} out of range [-${this.ndim + 1}, ${this.ndim + 1}]`);
    }
    const newShape = this.shape.slice();
    newShape.splice(normalized, 0, 1);
    return new Tensor(Array.from(this.data), { shape: newShape });
  }

  // -------------------------------------------------------------------------
  // Element-wise math methods (pure TypeScript — no autograd, no dispatch)
  //
  // PR #3 will replace these with Function-subclass-routed versions that
  // build autograd graphs and dispatch large tensors to Rust.  For v0.1
  // they're plain in-place math.
  // -------------------------------------------------------------------------

  // Element-wise math methods.  PR #3 routes these through the autograd-aware
  // Op classes (AddOp.apply etc.) so a Tensor with requiresGrad gets a gradFn
  // attached automatically.  Below-threshold tensors stay in pure-TS math
  // inside each Op's forward; above-threshold dispatch to Rust.
  //
  // Scalar broadcasting (`t.add(5)`) materializes a same-shape tensor first
  // via `Tensor.full(this.shape, scalar)` — wasteful for huge tensors but
  // keeps the autograd story simple in v0.3.0.  Lifting to a proper scalar-
  // broadcast envelope is post-v1.0 work.

  add(other: Tensor | number): Tensor {
    return AddOp.apply(this, this.coerceToTensor(other));
  }

  sub(other: Tensor | number): Tensor {
    return SubOp.apply(this, this.coerceToTensor(other));
  }

  mul(other: Tensor | number): Tensor {
    return MulOp.apply(this, this.coerceToTensor(other));
  }

  div(other: Tensor | number): Tensor {
    return DivOp.apply(this, this.coerceToTensor(other));
  }

  pow(exponent: number): Tensor {
    return PowOp.apply(this, exponent);
  }

  neg(): Tensor {
    return NegOp.apply(this);
  }

  // ─── Named ops added in PR #3 ────────────────────────────────────────

  abs(): Tensor {
    return AbsOp.apply(this);
  }

  matmul(other: Tensor): Tensor {
    return MatMulOp.apply(this, other);
  }

  /**
   * Embedding lookup.  `this` is the weight matrix (vocab_size, embedding_dim);
   * `indices` is a Tensor of integer-valued cells (any shape) giving rows to
   * fetch.  Output shape: `[...indices.shape, embedding_dim]`.
   *
   * Convention matches `torch.nn.functional.embedding(input, weight)` but
   * with self-as-weight for fluent chaining: `weight.embedding(tokenIds)`.
   * Gradients flow into `this` via scatter-add (repeated indices sum).
   */
  embedding(indices: Tensor): Tensor {
    return EmbeddingOp.apply(this, indices);
  }

  /**
   * LayerNorm over the last dim.  `gamma` / `beta` are learnable
   * parameters of shape `[D]` where `D = this.shape[-1]`.
   */
  layerNorm(gamma: Tensor, beta: Tensor, eps?: number): Tensor {
    return LayerNormOp.apply(this, gamma, beta, eps);
  }

  /**
   * BatchNorm over the batch dim (axis 0).  In train mode, computes
   * batch statistics and updates `runningMean` / `runningVar` in-place;
   * in eval mode, uses the frozen running stats.  See `setMode`.
   */
  batchNorm(
    gamma: Tensor,
    beta: Tensor,
    runningMean: Tensor,
    runningVar: Tensor,
    momentum?: number,
    eps?: number,
  ): Tensor {
    return BatchNormOp.apply(this, gamma, beta, runningMean, runningVar, momentum, eps);
  }

  /**
   * Dropout with inverted scaling.  Active in train mode; identity in
   * eval mode.  See `setMode`.
   */
  dropout(p?: number): Tensor {
    return DropoutOp.apply(this, p);
  }

  /**
   * 2-D convolution.  `this` is the input `(N, C, H, W)`; `weight` is
   * `(outC, C, kH, kW)`; `bias` (optional) is `(outC,)`.  See `Conv2DOp`.
   */
  conv2d(weight: Tensor, bias?: Tensor | null, stride?: number, padding?: number): Tensor {
    return Conv2DOp.apply(this, weight, bias ?? null, stride, padding);
  }

  /**
   * 2-D max pooling.  `this` is `(N, C, H, W)`; window is `(kH, kW)`;
   * stride defaults to `kH` (non-overlapping).  See `MaxPool2DOp`.
   */
  maxPool2d(kH: number, kW: number, stride?: number): Tensor {
    return MaxPool2DOp.apply(this, kH, kW, stride);
  }

  relu(): Tensor {
    return ReLUOp.apply(this);
  }

  sigmoid(): Tensor {
    return SigmoidOp.apply(this);
  }

  tanh(): Tensor {
    return TanhOp.apply(this);
  }

  gelu(): Tensor {
    return GELUOp.apply(this);
  }

  softmax(): Tensor {
    return SoftmaxOp.apply(this);
  }

  sum(): Tensor {
    return SumOp.apply(this);
  }

  mean(): Tensor {
    return MeanOp.apply(this);
  }

  /** Coerce a number into a same-shape tensor for scalar broadcasting. */
  private coerceToTensor(other: Tensor | number): Tensor {
    if (other instanceof Tensor) return other;
    if (typeof other === "number") return Tensor.full(this.shape.slice(), other);
    throw new TypeError(`cannot combine Tensor with ${typeof other}`);
  }

  /**
   * Kick off reverse-mode autodiff from this tensor.  Mutates the public
   * `.grad` slot of every leaf that participated in producing this tensor.
   *
   * `grad` defaults to `ones_like(this)` (PyTorch convention; strictly
   * only correct for scalar outputs, but we accept any shape).  Repeated
   * `backward()` calls accumulate into `.grad` — caller is expected to
   * zero between training steps.
   *
   * Lives here as a thin one-line delegate to `backwardImpl` in
   * autograd.ts.  The split keeps tensor.ts focused on storage.
   */
  backward(grad?: Tensor): void {
    // Lazy import to break the tensor.ts ↔ autograd.ts module cycle.
    // The import resolves on first call; subsequent calls hit the
    // module cache.  Note: top-level `import { backwardImpl }` would
    // work in TS too, but a lazy require keeps the cycle disciplined.
    //
    // We use dynamic-import-style require via createRequire because the
    // package is ESM (`"type": "module"`) — but actually a static
    // ESM import works fine here because `Tensor` is only USED (not
    // structurally referenced) inside backwardImpl.  Static import:
    /* eslint-disable @typescript-eslint/no-var-requires */
    // Note: Tested both static and lazy import; static is fine because
    // the cycle is shaped (autograd imports Tensor → tensor imports
    // backwardImpl) and resolved before either runs.
    // eslint-enable
    backwardImpl(this, grad);
  }

  // -------------------------------------------------------------------------
  // Static factories (the entries from PyTorch's `torch.*` namespace).
  // -------------------------------------------------------------------------

  static zeros(...shape: number[]): Tensor {
    const flat = shape.flat();
    const numel = flat.reduce((a, b) => a * b, 1);
    return new Tensor(new Array(numel).fill(0), { shape: flat });
  }

  static ones(...shape: number[]): Tensor {
    const flat = shape.flat();
    const numel = flat.reduce((a, b) => a * b, 1);
    return new Tensor(new Array(numel).fill(1), { shape: flat });
  }

  /** Tensor filled with `value`.  Shape is the first arg (an array). */
  static full(shape: number[], value: number): Tensor {
    const numel = shape.reduce((a, b) => a * b, 1);
    return new Tensor(new Array(numel).fill(value), { shape: shape.slice() });
  }

  /** Square or rectangular identity-like tensor — 1s on the diagonal. */
  static eye(n: number, m?: number): Tensor {
    const cols = m ?? n;
    const data = new Array(n * cols).fill(0);
    const diag = Math.min(n, cols);
    for (let i = 0; i < diag; i++) {
      data[i * cols + i] = 1;
    }
    return new Tensor(data, { shape: [n, cols] });
  }

  /**
   * 1-D range tensor.  Signatures:
   *
   * - `arange(stop)`
   * - `arange(start, stop)`
   * - `arange(start, stop, step)`
   *
   * Stop is EXCLUSIVE (NumPy / Python convention).  Step can be negative.
   * Rejects non-finite bounds — `Number.POSITIVE_INFINITY` would loop
   * forever building the result array.
   */
  static arange(start: number, stop?: number, step?: number): Tensor {
    let s: number, e: number, k: number;
    if (stop === undefined) {
      s = 0;
      e = start;
      k = 1;
    } else if (step === undefined) {
      s = start;
      e = stop;
      k = 1;
    } else {
      s = start;
      e = stop;
      k = step;
    }

    if (k === 0) throw new RangeError("arange step cannot be zero");
    if (!Number.isFinite(s) || !Number.isFinite(e) || !Number.isFinite(k)) {
      throw new RangeError(`arange bounds must be finite, got start=${s} stop=${e} step=${k}`);
    }

    const data: number[] = [];
    let x = s;
    if (k > 0) {
      while (x < e) {
        data.push(x);
        x += k;
      }
    } else {
      while (x > e) {
        data.push(x);
        x += k;
      }
    }
    return new Tensor(data, { shape: [data.length] });
  }

  /** Sugar for `new Tensor(nested)`. */
  static fromArray(nested: unknown): Tensor {
    return new Tensor(nested);
  }

  /** Ones tensor with the same shape as `other`. */
  static onesLike(other: Tensor): Tensor {
    return Tensor.ones(...other.shape.slice());
  }

  /** Zeros tensor with the same shape as `other`. */
  static zerosLike(other: Tensor): Tensor {
    return Tensor.zeros(...other.shape.slice());
  }

  /**
   * Standard-normal samples via Box-Muller.  Optional seed — when provided,
   * the output is deterministic (useful for testing and reproducible
   * benchmarks).
   *
   * Box-Muller is the textbook two-line algorithm for turning two uniforms
   * into two normals — avoids pulling in a distribution library.
   */
  static randn(shape: number[], seed?: number): Tensor {
    const numel = shape.reduce((a, b) => a * b, 1);
    const random = seed !== undefined ? makeSeededRandom(seed) : Math.random;
    const data = new Array(numel);
    let i = 0;
    while (i < numel) {
      let u1 = random();
      if (u1 === 0) u1 = Number.MIN_VALUE; // ln(0) is -∞; nudge into the domain.
      const u2 = random();
      const mag = Math.sqrt(-2 * Math.log(u1));
      data[i] = mag * Math.cos(2 * Math.PI * u2);
      if (i + 1 < numel) {
        data[i + 1] = mag * Math.sin(2 * Math.PI * u2);
      }
      i += 2;
    }
    return new Tensor(data, { shape: shape.slice() });
  }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

function shapesEqual(a: Shape, b: Shape): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    if (a[i] !== b[i]) return false;
  }
  return true;
}

/**
 * Recursively un-flatten a flat array back into nested form.  Helper used
 * by `Tensor.toNested()`.  Returns a value-and-cursor so the recursion
 * can resume where the previous call left off.
 */
function unflatten(
  flat: number[],
  shape: readonly number[],
  start: number,
  depth: number,
): { value: unknown; next: number } {
  if (depth >= shape.length) {
    return { value: flat[start], next: start + 1 };
  }
  if (depth === shape.length - 1) {
    const len = shape[depth]!;
    return { value: flat.slice(start, start + len), next: start + len };
  }
  const len = shape[depth]!;
  const result: unknown[] = [];
  let cursor = start;
  for (let i = 0; i < len; i++) {
    const sub = unflatten(flat, shape, cursor, depth + 1);
    result.push(sub.value);
    cursor = sub.next;
  }
  return { value: result, next: cursor };
}

/**
 * Tiny seeded RNG (LCG — xorshift would be only marginally better and the
 * LCG implementation fits in two lines).  Returns a function with the same
 * 0..1 contract as `Math.random()` so callers can swap freely.
 *
 * Constants from Numerical Recipes' "quick and dirty" LCG.  Not
 * cryptographically secure — this is for reproducible weight initialization,
 * not for crypto material.
 */
function makeSeededRandom(seed: number): () => number {
  let state = (Math.trunc(seed) >>> 0) || 1;
  return () => {
    state = (state * 1664525 + 1013904223) >>> 0;
    return state / 0x100000000;
  };
}
