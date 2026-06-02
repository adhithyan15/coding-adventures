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
   * Transpose.  2-D only in v0.1 (higher-rank perm transpose needs generic
   * strided index math; deferred to a later PR when there's actual demand).
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

    if (this.ndim !== 2) {
      throw new Error(`transpose on rank-${this.ndim} tensors not yet implemented (v0.1: 2-D only)`);
    }

    const [rows, cols] = this.shape as [number, number];
    const out = new Float32Array(rows * cols);
    for (let r = 0; r < rows; r++) {
      for (let c = 0; c < cols; c++) {
        out[c * rows + r] = this.data[r * cols + c]!;
      }
    }
    return new Tensor(Array.from(out), { shape: [cols, rows] });
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

  add(other: Tensor | number): Tensor {
    return this.binaryOp(other, (a, b) => a + b);
  }

  sub(other: Tensor | number): Tensor {
    return this.binaryOp(other, (a, b) => a - b);
  }

  mul(other: Tensor | number): Tensor {
    return this.binaryOp(other, (a, b) => a * b);
  }

  div(other: Tensor | number): Tensor {
    return this.binaryOp(other, (a, b) => a / b);
  }

  pow(exponent: number): Tensor {
    const out = new Float32Array(this.numel);
    for (let i = 0; i < this.numel; i++) {
      out[i] = Math.pow(this.data[i]!, exponent);
    }
    return new Tensor(Array.from(out), { shape: this.shape.slice() });
  }

  neg(): Tensor {
    const out = new Float32Array(this.numel);
    for (let i = 0; i < this.numel; i++) {
      out[i] = -this.data[i]!;
    }
    return new Tensor(Array.from(out), { shape: this.shape.slice() });
  }

  private binaryOp(other: Tensor | number, fn: (a: number, b: number) => number): Tensor {
    if (other instanceof Tensor) {
      if (!shapesEqual(this.shape, other.shape)) {
        throw new RangeError(
          `shape mismatch: [${this.shape.join(", ")}] vs [${other.shape.join(", ")}] (broadcasting not in v0.1)`,
        );
      }
      const out = new Float32Array(this.numel);
      for (let i = 0; i < this.numel; i++) {
        out[i] = fn(this.data[i]!, other.data[i]!);
      }
      return new Tensor(Array.from(out), { shape: this.shape.slice() });
    } else if (typeof other === "number") {
      const out = new Float32Array(this.numel);
      for (let i = 0; i < this.numel; i++) {
        out[i] = fn(this.data[i]!, other);
      }
      return new Tensor(Array.from(out), { shape: this.shape.slice() });
    } else {
      throw new TypeError(`cannot combine Tensor with ${typeof other}`);
    }
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
