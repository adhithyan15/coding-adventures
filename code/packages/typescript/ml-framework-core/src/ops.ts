/**
 * # ops.ts — the 15 differentiable operations
 *
 * This is where Tensors get math.  Every op is a `Function` subclass
 * (defined in autograd.ts) with a `forward` method that:
 *
 *   - If the input is below `DISPATCH_THRESHOLD` (10k cells), uses pure
 *     TypeScript — fast enough at that size, avoids JSON+hex+FFI overhead.
 *   - If above the threshold AND the op has a Rust path, builds a
 *     matrix-ir-json envelope and dispatches through
 *     `@coding-adventures/matrix-rust-napi.runGraphOnCpu` to the
 *     Rust `matrix-cpu` executor.
 *
 * The envelope shapes MIRROR the Python reference at
 * `code/packages/python/ml-framework-core/src/ml_framework_core/_rust_backend.py`
 * byte-for-byte.  Same wire format the Ruby pilot uses.  If you change
 * anything here, also update Python and Ruby — the JSON is the
 * cross-language contract.
 *
 * ## backward() implementations come in PR #4
 *
 * This PR ships forward-only.  Each subclass below has a forward but
 * `backward` throws `Error("not implemented; PR #4 will add it")`.  PR #4
 * fills them in using the analytical gradient formulas from the Ruby
 * pilot's PR #7 (which mirror Python autograd.py).
 *
 * ## Op coverage in v0.3.0
 *
 *   | Op       | Rust dispatch?    | matrix-ir-json kind        |
 *   |----------|-------------------|----------------------------|
 *   | Add      | yes (Rust)        | Add (lhs/rhs/output)       |
 *   | Sub      | yes               | Sub                        |
 *   | Mul      | yes               | Mul                        |
 *   | Div      | yes               | Div                        |
 *   | Neg      | yes               | Neg (input/output)         |
 *   | Abs      | yes               | Abs                        |
 *   | Tanh     | yes               | Tanh                       |
 *   | MatMul   | yes (2-D only)    | MatMul (a/b/output)        |
 *   | Sum      | yes (reduce-all)  | ReduceSum (axes/keep_dims) |
 *   | Mean     | yes               | ReduceMean                 |
 *   | Pow      | pure TS           | — (Rust takes 2 tensors)   |
 *   | ReLU     | pure TS           | — (would use Max + const)  |
 *   | Sigmoid  | pure TS           | — (multi-op graph)         |
 *   | GELU     | pure TS           | — (large multi-op graph)   |
 *   | Softmax  | pure TS           | — (multi-op graph)         |
 *
 * The "pure TS" five have working Rust paths in the Python reference
 * (and the Ruby pilot's roadmap); deferred from v0.3.0 to keep this
 * PR focused.  A follow-up can lift them.
 */

import { Tensor } from "./tensor.js";
import { Function } from "./autograd.js";
import {
  broadcastShapes,
  broadcastDataTo,
  unbroadcastDataTo,
} from "./broadcasting.js";

// ===========================================================================
// Module-level helpers: hex packing, threshold, envelope dispatcher.
// ===========================================================================

/**
 * Tensors with `numel` ≥ this dispatch to Rust; smaller use pure TypeScript.
 * 10_000 cells is the rough break-even from the Python benchmark — below
 * it, the JSON + hex + FFI overhead dominates; above it, matrix-cpu's
 * f32 SIMD wins big.  Same constant the Ruby pilot uses.
 */
export const DISPATCH_THRESHOLD = 10_000;

/**
 * Pack an array of numbers into a hex string of little-endian f32 bytes.
 *
 * The hex format matches what matrix-cpu expects on the wire — same shape
 * the Python and Ruby reference implementations produce, so envelopes
 * built here are bit-compatible across languages.
 */
export function packF32Hex(arr: ArrayLike<number>): string {
  const buf = new Float32Array(arr.length);
  for (let i = 0; i < arr.length; i++) buf[i] = arr[i]!;
  return Buffer.from(buf.buffer, buf.byteOffset, buf.byteLength).toString("hex");
}

/**
 * Inverse of `packF32Hex`.  Returns a Float32Array view of length `numel`.
 *
 * We pass `numel` explicitly because Buffer.from might allocate a larger
 * buffer than the hex implies on some Node versions; being explicit is
 * defensive.
 */
export function unpackF32Hex(hex: string, numel: number): Float32Array {
  const buf = Buffer.from(hex, "hex");
  if (buf.byteLength !== numel * 4) {
    throw new Error(
      `unpackF32Hex: expected ${numel * 4} bytes (${numel} f32 cells), got ${buf.byteLength}`,
    );
  }
  // Float32Array view over the Buffer's underlying ArrayBuffer.  Copy
  // into a fresh Float32Array so the result owns its own memory and is
  // independent of Buffer lifecycle (Buffer.from(hex,"hex") returns a
  // tightly-allocated Buffer, but being defensive about ownership is cheap).
  const out = new Float32Array(numel);
  out.set(new Float32Array(buf.buffer, buf.byteOffset, numel));
  return out;
}

// -----------------------------------------------------------------------------
// Lazy require of the matrix-rust-napi addon.
//
// Pure-TS workflows don't need the .node addon, so we defer loading until
// the first envelope dispatch.  Once cached, all subsequent dispatches hit
// the module cache.  If the addon isn't built/installed, the first call
// throws a clear LoadError — earlier calls just stay in the pure-TS path.
// -----------------------------------------------------------------------------

interface RustNapi {
  runGraphOnCpu(envelopeJson: string): string;
}

let _rustNapiCache: RustNapi | null = null;
let _rustNapiLoadAttempted = false;

function loadRustNapi(): RustNapi {
  if (_rustNapiCache) return _rustNapiCache;
  if (_rustNapiLoadAttempted) {
    throw new Error(
      "@coding-adventures/matrix-rust-napi failed to load on a prior call; not retrying",
    );
  }
  _rustNapiLoadAttempted = true;
  // Use createRequire so this ESM module can require() the CJS package.
  // The matrix-rust-napi package itself goes through createRequire to
  // load the .node file (same pattern).
  // eslint-disable-next-line @typescript-eslint/no-var-requires
  const { createRequire } = require("node:module") as typeof import("node:module");
  const requireFn = createRequire(import.meta.url);
  // eslint-disable-next-line @typescript-eslint/no-var-requires
  const napi = requireFn("@coding-adventures/matrix-rust-napi") as RustNapi;
  _rustNapiCache = napi;
  return napi;
}

/**
 * Drive the Rust executor: JSON-stringify, call into the addon, parse
 * the JSON response, decode `outputs[0]` back to a Float32Array.
 *
 * @param envelope matrix-ir-json envelope as a plain object
 * @param outputNumel expected f32 cell count of the single output
 */
export function runEnvelope(envelope: unknown, outputNumel: number): Float32Array {
  const envJson = JSON.stringify(envelope);
  const resultJson = loadRustNapi().runGraphOnCpu(envJson);
  const result = JSON.parse(resultJson) as { outputs: string[] };
  const outHex = result.outputs?.[0];
  if (typeof outHex !== "string") {
    throw new Error(`matrix_cpu response missing outputs[0]: ${resultJson.slice(0, 200)}`);
  }
  return unpackF32Hex(outHex, outputNumel);
}

// -----------------------------------------------------------------------------
// Envelope builders — one per shape that matrix-cpu accepts.
// -----------------------------------------------------------------------------

function binaryElementwiseEnvelope(kind: string, a: Tensor, b: Tensor): object {
  const shape = a.shape.slice();
  return {
    graph: {
      matrix_ir_version: 1,
      tensors: [
        { id: 0, dtype: "f32", shape },
        { id: 1, dtype: "f32", shape },
        { id: 2, dtype: "f32", shape },
      ],
      inputs: [0, 1],
      outputs: [2],
      ops: [{ kind, lhs: 0, rhs: 1, output: 2 }],
      constants: [],
    },
    inputs: [packF32Hex(a.data), packF32Hex(b.data)],
  };
}

function unaryElementwiseEnvelope(kind: string, a: Tensor): object {
  const shape = a.shape.slice();
  return {
    graph: {
      matrix_ir_version: 1,
      tensors: [
        { id: 0, dtype: "f32", shape },
        { id: 1, dtype: "f32", shape },
      ],
      inputs: [0],
      outputs: [1],
      ops: [{ kind, input: 0, output: 1 }],
      constants: [],
    },
    inputs: [packF32Hex(a.data)],
  };
}

function matmulEnvelope(a: Tensor, b: Tensor): object {
  const [m, k] = a.shape as [number, number];
  const [, n] = b.shape as [number, number];
  return {
    graph: {
      matrix_ir_version: 1,
      tensors: [
        { id: 0, dtype: "f32", shape: [m, k] },
        { id: 1, dtype: "f32", shape: [k, n] },
        { id: 2, dtype: "f32", shape: [m, n] },
      ],
      inputs: [0, 1],
      outputs: [2],
      ops: [{ kind: "MatMul", a: 0, b: 1, output: 2 }],
      constants: [],
    },
    inputs: [packF32Hex(a.data), packF32Hex(b.data)],
  };
}

function reduceAllEnvelope(kind: string, a: Tensor): object {
  const shape = a.shape.slice();
  const axes = shape.map((_, i) => i);
  return {
    graph: {
      matrix_ir_version: 1,
      tensors: [
        { id: 0, dtype: "f32", shape },
        { id: 1, dtype: "f32", shape: [] },
      ],
      inputs: [0],
      outputs: [1],
      ops: [{ kind, input: 0, axes, keep_dims: false, output: 1 }],
      constants: [],
    },
    inputs: [packF32Hex(a.data)],
  };
}

// -----------------------------------------------------------------------------
// Shared helpers for the binary/unary forward implementations.
// -----------------------------------------------------------------------------

function shapesEqual(a: readonly number[], b: readonly number[]): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false;
  return true;
}

function binaryElementwise(
  kind: string,
  a: Tensor,
  b: Tensor,
  tsOp: (x: number, y: number) => number,
): Tensor {
  // Broadcast first.  If shapes already match this is a near-free copy.
  // For shape-mismatched inputs we materialize broadcasted views in
  // pure TS; the dispatched Rust path then sees same-shape inputs.
  //
  // Materializing the broadcast (vs. a strided view) costs memory but
  // keeps the matrix-cpu wire format unchanged — matrix-cpu doesn't
  // understand broadcasted views.
  const outShape = broadcastShapes(a.shape, b.shape);
  const aB =
    a.shape.length === outShape.length && shapesEqual(a.shape, outShape)
      ? a
      : new Tensor(Array.from(broadcastDataTo(a.data, a.shape, outShape)), {
          shape: outShape.slice(),
        });
  const bB =
    b.shape.length === outShape.length && shapesEqual(b.shape, outShape)
      ? b
      : new Tensor(Array.from(broadcastDataTo(b.data, b.shape, outShape)), {
          shape: outShape.slice(),
        });

  const numel = aB.numel;
  if (numel >= DISPATCH_THRESHOLD) {
    const envelope = binaryElementwiseEnvelope(kind, aB, bB);
    const out = runEnvelope(envelope, numel);
    return new Tensor(Array.from(out), { shape: outShape.slice() });
  }
  const out = new Array(numel);
  for (let i = 0; i < numel; i++) out[i] = tsOp(aB.data[i]!, bB.data[i]!);
  return new Tensor(out, { shape: outShape.slice() });
}

function unaryElementwise(
  kind: string,
  a: Tensor,
  tsOp: (v: number) => number,
): Tensor {
  if (a.numel >= DISPATCH_THRESHOLD) {
    const envelope = unaryElementwiseEnvelope(kind, a);
    const out = runEnvelope(envelope, a.numel);
    return new Tensor(Array.from(out), { shape: a.shape.slice() });
  }
  const out = new Array(a.numel);
  for (let i = 0; i < a.numel; i++) out[i] = tsOp(a.data[i]!);
  return new Tensor(out, { shape: a.shape.slice() });
}

// ===========================================================================
// BroadcastOp — explicit broadcasting as a first-class autograd op
// ===========================================================================
//
// Most callers don't need this — the binary ops broadcast internally.
// But for ML code that wants to express "promote this (out_dim,) bias
// vector to (batch, out_dim) once" rather than relying on implicit
// broadcasting in every downstream op, `BroadcastOp.apply(t, newShape)`
// makes that intent explicit.
//
// forward: broadcast `data` from `t.shape` to `newShape`.
// backward: sum the incoming gradient back from `newShape` to `t.shape`
//           — that's `unbroadcastDataTo`.

export class BroadcastOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    const t = inputs[0] as Tensor;
    const newShape = inputs[1] as number[];
    this.savedForBackward.inputShape = t.shape.slice();
    this.savedForBackward.outShape = newShape.slice();
    const out = broadcastDataTo(t.data, t.shape, newShape);
    return new Tensor(Array.from(out), { shape: newShape.slice() });
  }
  backward(grad: Tensor): (Tensor | null)[] {
    const inputShape = this.savedForBackward.inputShape as number[];
    // Only one Tensor parent (newShape is a number[], filtered out by
    // Function.apply since it isn't a Tensor); return a 1-element Array.
    return [
      tensorFromBuf(
        unbroadcastDataTo(grad.data, grad.shape, inputShape),
        inputShape,
      ),
    ];
  }
}

// Re-export the pure helpers so power users can call them directly.
export { broadcastShapes, broadcastDataTo, unbroadcastDataTo };

// ===========================================================================
// The 15 differentiable ops — forward + backward.
//
// Backward formulas mirror Python autograd.py / Ruby PR #7 exactly.  All
// done in pure TS for v0.4.0 (the formulas are mostly element-wise muls
// + reductions; routing through Rust would require new envelope shapes
// per backward op — deferred to a follow-up).  Pure-TS is correct and
// fast enough for the parameter-shaped tensors that typify training.
// ===========================================================================

// ────────── Binary elementwise: Add / Sub / Mul / Div ──────────

// ─── Broadcast-aware helpers for backward ────────────────────────────────
//
// With broadcasting, the forward output has the broadcast shape `C`
// while parent inputs have shapes `A` and `B` (where A, B broadcast to C).
// Backward receives `grad` of shape `C` and must return gradients of
// shapes `A` and `B` — which means summing along the axes that got
// stretched by broadcasting.  `unbroadcastDataTo` handles exactly that.

/** Wrap a Float32Array into a Tensor with the given shape. */
function tensorFromBuf(buf: Float32Array, shape: readonly number[]): Tensor {
  return new Tensor(Array.from(buf), { shape: shape.slice() });
}

export class AddOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    const a = inputs[0] as Tensor;
    const b = inputs[1] as Tensor;
    // Save parent shapes so backward can unbroadcast back to them.
    this.savedForBackward.aShape = a.shape.slice();
    this.savedForBackward.bShape = b.shape.slice();
    return binaryElementwise("Add", a, b, (x, y) => x + y);
  }
  // d/dx (x + y) = 1, d/dy (x + y) = 1.  Gradient passes through, then
  // unbroadcast to each parent's original shape.
  backward(grad: Tensor): (Tensor | null)[] {
    const aShape = this.savedForBackward.aShape as number[];
    const bShape = this.savedForBackward.bShape as number[];
    return [
      tensorFromBuf(unbroadcastDataTo(grad.data, grad.shape, aShape), aShape),
      tensorFromBuf(unbroadcastDataTo(grad.data, grad.shape, bShape), bShape),
    ];
  }
}

export class SubOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    const a = inputs[0] as Tensor;
    const b = inputs[1] as Tensor;
    this.savedForBackward.aShape = a.shape.slice();
    this.savedForBackward.bShape = b.shape.slice();
    return binaryElementwise("Sub", a, b, (x, y) => x - y);
  }
  // d/dx (x - y) = 1, d/dy (x - y) = -1.  Unbroadcast after applying sign.
  backward(grad: Tensor): (Tensor | null)[] {
    const aShape = this.savedForBackward.aShape as number[];
    const bShape = this.savedForBackward.bShape as number[];
    // Negate first (on the broadcast shape), then unbroadcast.  Order
    // doesn't matter for negation but conceptually mirrors the chain rule.
    const negGrad = new Float32Array(grad.data.length);
    for (let i = 0; i < grad.data.length; i++) negGrad[i] = -grad.data[i]!;
    return [
      tensorFromBuf(unbroadcastDataTo(grad.data, grad.shape, aShape), aShape),
      tensorFromBuf(unbroadcastDataTo(negGrad, grad.shape, bShape), bShape),
    ];
  }
}

export class MulOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    const a = inputs[0] as Tensor;
    const b = inputs[1] as Tensor;
    // Save the BROADCASTED versions of a and b so backward's chain-rule
    // multiply works on the same shape as `grad` (which has the
    // broadcast output shape).  We can't naively save the originals
    // because their shapes may differ from grad.shape.
    const outShape = broadcastShapes(a.shape, b.shape);
    const aB = broadcastDataTo(a.data, a.shape, outShape);
    const bB = broadcastDataTo(b.data, b.shape, outShape);
    this.savedForBackward.aB = aB;
    this.savedForBackward.bB = bB;
    this.savedForBackward.aShape = a.shape.slice();
    this.savedForBackward.bShape = b.shape.slice();
    this.savedForBackward.outShape = outShape;
    return binaryElementwise("Mul", a, b, (x, y) => x * y);
  }
  // d/dx (x*y) = y, d/dy (x*y) = x.  Element-wise on the broadcast shape,
  // then unbroadcast to each parent's original shape.
  backward(grad: Tensor): (Tensor | null)[] {
    const aB = this.savedForBackward.aB as Float32Array;
    const bB = this.savedForBackward.bB as Float32Array;
    const aShape = this.savedForBackward.aShape as number[];
    const bShape = this.savedForBackward.bShape as number[];
    const outShape = this.savedForBackward.outShape as number[];
    const numel = aB.length;
    const gradABig = new Float32Array(numel);
    const gradBBig = new Float32Array(numel);
    for (let i = 0; i < numel; i++) {
      gradABig[i] = grad.data[i]! * bB[i]!;
      gradBBig[i] = grad.data[i]! * aB[i]!;
    }
    return [
      tensorFromBuf(unbroadcastDataTo(gradABig, outShape, aShape), aShape),
      tensorFromBuf(unbroadcastDataTo(gradBBig, outShape, bShape), bShape),
    ];
  }
}

export class DivOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    const a = inputs[0] as Tensor;
    const b = inputs[1] as Tensor;
    const outShape = broadcastShapes(a.shape, b.shape);
    const aB = broadcastDataTo(a.data, a.shape, outShape);
    const bB = broadcastDataTo(b.data, b.shape, outShape);
    this.savedForBackward.aB = aB;
    this.savedForBackward.bB = bB;
    this.savedForBackward.aShape = a.shape.slice();
    this.savedForBackward.bShape = b.shape.slice();
    this.savedForBackward.outShape = outShape;
    return binaryElementwise("Div", a, b, (x, y) => x / y);
  }
  // d/dx (x/y) = 1/y, d/dy (x/y) = -x/y².  Quotient rule on broadcast
  // shape, then unbroadcast.
  backward(grad: Tensor): (Tensor | null)[] {
    const aB = this.savedForBackward.aB as Float32Array;
    const bB = this.savedForBackward.bB as Float32Array;
    const aShape = this.savedForBackward.aShape as number[];
    const bShape = this.savedForBackward.bShape as number[];
    const outShape = this.savedForBackward.outShape as number[];
    const numel = aB.length;
    const gradABig = new Float32Array(numel);
    const gradBBig = new Float32Array(numel);
    for (let i = 0; i < numel; i++) {
      const bv = bB[i]!;
      gradABig[i] = grad.data[i]! / bv;
      gradBBig[i] = -grad.data[i]! * aB[i]! / (bv * bv);
    }
    return [
      tensorFromBuf(unbroadcastDataTo(gradABig, outShape, aShape), aShape),
      tensorFromBuf(unbroadcastDataTo(gradBBig, outShape, bShape), bShape),
    ];
  }
}

// ────────── Unary elementwise: Neg / Abs / Tanh ──────────

export class NegOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    return unaryElementwise("Neg", inputs[0] as Tensor, (v) => -v);
  }
  // d/dx (-x) = -1.
  backward(grad: Tensor): (Tensor | null)[] {
    return [new Tensor(Array.from(grad.data).map((v) => -v), { shape: grad.shape.slice() })];
  }
}

export class AbsOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    const a = inputs[0] as Tensor;
    this.savedForBackward.a = a;
    return unaryElementwise("Abs", a, (v) => Math.abs(v));
  }
  // d/dx |x| = sign(x).  Convention: sign(0) = 0 (PyTorch).
  backward(grad: Tensor): (Tensor | null)[] {
    const a = this.savedForBackward.a as Tensor;
    const out = new Array(a.numel);
    for (let i = 0; i < a.numel; i++) {
      const av = a.data[i]!;
      out[i] = av > 0 ? grad.data[i]! : av < 0 ? -grad.data[i]! : 0;
    }
    return [new Tensor(out, { shape: a.shape.slice() })];
  }
}

export class TanhOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    // Save the OUTPUT (not the input) — tanh backward is 1 - tanh²(x)
    // which is cheaper to compute as 1 - y² from the already-computed
    // forward output.
    const out = unaryElementwise("Tanh", inputs[0] as Tensor, (v) => Math.tanh(v));
    this.savedForBackward.output = out;
    return out;
  }
  // d/dx tanh(x) = 1 - tanh²(x) = 1 - y².
  backward(grad: Tensor): (Tensor | null)[] {
    const y = this.savedForBackward.output as Tensor;
    const out = new Array(y.numel);
    for (let i = 0; i < y.numel; i++) {
      const yv = y.data[i]!;
      out[i] = grad.data[i]! * (1 - yv * yv);
    }
    return [new Tensor(out, { shape: y.shape.slice() })];
  }
}

// ────────── Pow: scalar exponent ──────────

export class PowOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    const a = inputs[0] as Tensor;
    const exponent = inputs[1] as number;
    this.savedForBackward.a = a;
    this.savedForBackward.exponent = exponent;
    const out = new Array(a.numel);
    for (let i = 0; i < a.numel; i++) out[i] = Math.pow(a.data[i]!, exponent);
    return new Tensor(out, { shape: a.shape.slice() });
  }
  // d/dx x^e = e * x^(e-1).  Only one Tensor parent (exponent is a Numeric,
  // filtered out by Function.apply); return a 1-element Array to match.
  backward(grad: Tensor): (Tensor | null)[] {
    const a = this.savedForBackward.a as Tensor;
    const e = this.savedForBackward.exponent as number;
    const out = new Array(a.numel);
    for (let i = 0; i < a.numel; i++) {
      out[i] = grad.data[i]! * e * Math.pow(a.data[i]!, e - 1);
    }
    return [new Tensor(out, { shape: a.shape.slice() })];
  }
}

// ────────── MatMul (2-D only) ──────────

export class MatMulOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    const a = inputs[0] as Tensor;
    const b = inputs[1] as Tensor;
    if (a.ndim !== 2 || b.ndim !== 2) {
      throw new RangeError(`matmul requires 2-D tensors, got ndim ${a.ndim} and ${b.ndim}`);
    }
    const [m, k1] = a.shape as [number, number];
    const [k2, n] = b.shape as [number, number];
    if (k1 !== k2) {
      throw new RangeError(`matmul shape mismatch: [${a.shape.join(", ")}] @ [${b.shape.join(", ")}]`);
    }

    this.savedForBackward.a = a;
    this.savedForBackward.b = b;

    if (a.numel >= DISPATCH_THRESHOLD || b.numel >= DISPATCH_THRESHOLD) {
      const envelope = matmulEnvelope(a, b);
      const out = runEnvelope(envelope, m * n);
      return new Tensor(Array.from(out), { shape: [m, n] });
    }

    return new Tensor(MatMulOp._matmulNaive(a.data, b.data, m, k1, n), { shape: [m, n] });
  }

  // Backward for C = A @ B (2-D):
  //   dL/dA = grad @ B^T
  //   dL/dB = A^T @ grad
  // Use internal helpers (not MatMulOp.apply) so backward stays a leaf
  // math operation — no extra autograd subgraph.
  backward(grad: Tensor): (Tensor | null)[] {
    const a = this.savedForBackward.a as Tensor;
    const b = this.savedForBackward.b as Tensor;
    const [m, k] = a.shape as [number, number];
    const [, n] = b.shape as [number, number];

    const bT = MatMulOp._transpose2D(b.data, k, n);             // (n, k)
    const gradAData = MatMulOp._matmulNaive(grad.data, bT, m, n, k);  // (m, k)

    const aT = MatMulOp._transpose2D(a.data, m, k);             // (k, m)
    const gradBData = MatMulOp._matmulNaive(aT, grad.data, k, m, n);  // (k, n)

    return [
      new Tensor(gradAData, { shape: [m, k] }),
      new Tensor(gradBData, { shape: [k, n] }),
    ];
  }

  /** O(m*k*n) naive matmul on raw Float32Array data. */
  static _matmulNaive(aData: ArrayLike<number>, bData: ArrayLike<number>, m: number, k: number, n: number): number[] {
    const out = new Array(m * n).fill(0);
    for (let i = 0; i < m; i++) {
      for (let j = 0; j < n; j++) {
        let acc = 0;
        for (let kk = 0; kk < k; kk++) {
          acc += aData[i * k + kk]! * bData[kk * n + j]!;
        }
        out[i * n + j] = acc;
      }
    }
    return out;
  }

  /** Transpose a flat (rows × cols) row-major matrix.  Returns a new Array. */
  static _transpose2D(data: ArrayLike<number>, rows: number, cols: number): number[] {
    const out = new Array(rows * cols);
    for (let r = 0; r < rows; r++) {
      for (let c = 0; c < cols; c++) {
        out[c * rows + r] = data[r * cols + c]!;
      }
    }
    return out;
  }
}

// ────────── ReLU / Sigmoid / GELU / Softmax — pure TS ──────────

export class ReLUOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    const a = inputs[0] as Tensor;
    this.savedForBackward.a = a;
    const out = new Array(a.numel);
    for (let i = 0; i < a.numel; i++) out[i] = a.data[i]! > 0 ? a.data[i]! : 0;
    return new Tensor(out, { shape: a.shape.slice() });
  }
  // d/dx ReLU(x) = 1 if x > 0 else 0.  x == 0 → 0 (PyTorch).
  backward(grad: Tensor): (Tensor | null)[] {
    const a = this.savedForBackward.a as Tensor;
    const out = new Array(a.numel);
    for (let i = 0; i < a.numel; i++) {
      out[i] = a.data[i]! > 0 ? grad.data[i]! : 0;
    }
    return [new Tensor(out, { shape: a.shape.slice() })];
  }
}

export class SigmoidOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    const a = inputs[0] as Tensor;
    const out = new Array(a.numel);
    for (let i = 0; i < a.numel; i++) out[i] = 1 / (1 + Math.exp(-a.data[i]!));
    const result = new Tensor(out, { shape: a.shape.slice() });
    // Save the OUTPUT y — sigmoid backward is y * (1 - y).
    this.savedForBackward.output = result;
    return result;
  }
  // d/dx σ(x) = σ(x) * (1 - σ(x)) = y * (1 - y).
  backward(grad: Tensor): (Tensor | null)[] {
    const y = this.savedForBackward.output as Tensor;
    const out = new Array(y.numel);
    for (let i = 0; i < y.numel; i++) {
      const yv = y.data[i]!;
      out[i] = grad.data[i]! * yv * (1 - yv);
    }
    return [new Tensor(out, { shape: y.shape.slice() })];
  }
}

export class GELUOp extends Function {
  // GELU(x) = 0.5 * x * (1 + tanh(sqrt(2/π) * (x + 0.044715 * x³)))
  // tanh-approximation form matching PyTorch default + Python reference.
  static readonly SQRT_2_OVER_PI = Math.sqrt(2 / Math.PI);
  static readonly COEFF = 0.044715;

  forward(...inputs: unknown[]): Tensor {
    const a = inputs[0] as Tensor;
    this.savedForBackward.a = a;
    const c = GELUOp.SQRT_2_OVER_PI;
    const k = GELUOp.COEFF;
    const out = new Array(a.numel);
    for (let i = 0; i < a.numel; i++) {
      const x = a.data[i]!;
      out[i] = 0.5 * x * (1 + Math.tanh(c * (x + k * x * x * x)));
    }
    return new Tensor(out, { shape: a.shape.slice() });
  }
  // GELU backward (tanh-approximation form):
  //   inner   = √(2/π) * (x + 0.044715 * x³)
  //   tanh_v  = tanh(inner)
  //   sech²   = 1 - tanh_v²
  //   d_inner = √(2/π) * (1 + 3 * 0.044715 * x²)
  //   dy/dx   = 0.5 * (1 + tanh_v) + 0.5 * x * sech² * d_inner
  backward(grad: Tensor): (Tensor | null)[] {
    const a = this.savedForBackward.a as Tensor;
    const c = GELUOp.SQRT_2_OVER_PI;
    const k = GELUOp.COEFF;
    const out = new Array(a.numel);
    for (let i = 0; i < a.numel; i++) {
      const x = a.data[i]!;
      const inner = c * (x + k * x * x * x);
      const tanhV = Math.tanh(inner);
      const sech2 = 1 - tanhV * tanhV;
      const dInner = c * (1 + 3 * k * x * x);
      out[i] = grad.data[i]! * (0.5 * (1 + tanhV) + 0.5 * x * sech2 * dInner);
    }
    return [new Tensor(out, { shape: a.shape.slice() })];
  }
}

export class SoftmaxOp extends Function {
  // Softmax over the LAST axis.  Numerically stable: subtract row max before
  // exp() so values never overflow exp's range.
  forward(...inputs: unknown[]): Tensor {
    const a = inputs[0] as Tensor;
    const shape = a.shape;
    const lastAxisSize = shape.length === 0 ? 1 : shape[shape.length - 1]!;
    const outer = a.numel / lastAxisSize;

    const out = new Array(a.numel);
    for (let o = 0; o < outer; o++) {
      const rowStart = o * lastAxisSize;
      let rowMax = -Infinity;
      for (let k = 0; k < lastAxisSize; k++) {
        const v = a.data[rowStart + k]!;
        if (v > rowMax) rowMax = v;
      }
      let sum = 0;
      const tmp = new Array(lastAxisSize);
      for (let k = 0; k < lastAxisSize; k++) {
        const e = Math.exp(a.data[rowStart + k]! - rowMax);
        tmp[k] = e;
        sum += e;
      }
      for (let k = 0; k < lastAxisSize; k++) {
        out[rowStart + k] = tmp[k] / sum;
      }
    }
    const result = new Tensor(out, { shape: a.shape.slice() });
    this.savedForBackward.output = result;
    this.savedForBackward.lastAxisSize = lastAxisSize;
    return result;
  }

  // Softmax backward (per-row over the last axis):
  //   dL/dx_i = y_i * (g_i - Σ_j (g_j * y_j))
  // The dot product is per-row; each row is independent.
  backward(grad: Tensor): (Tensor | null)[] {
    const y = this.savedForBackward.output as Tensor;
    const lastAxisSize = this.savedForBackward.lastAxisSize as number;
    const numel = y.numel;
    const outer = numel / lastAxisSize;
    const out = new Array(numel);
    for (let o = 0; o < outer; o++) {
      const rowStart = o * lastAxisSize;
      let dot = 0;
      for (let k = 0; k < lastAxisSize; k++) {
        dot += grad.data[rowStart + k]! * y.data[rowStart + k]!;
      }
      for (let k = 0; k < lastAxisSize; k++) {
        const idx = rowStart + k;
        out[idx] = y.data[idx]! * (grad.data[idx]! - dot);
      }
    }
    return [new Tensor(out, { shape: y.shape.slice() })];
  }
}

// ────────── Reductions: Sum / Mean (reduce-all) ──────────

export class SumOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    const a = inputs[0] as Tensor;
    this.savedForBackward.inputShape = a.shape.slice();
    if (a.numel >= DISPATCH_THRESHOLD) {
      const envelope = reduceAllEnvelope("ReduceSum", a);
      const out = runEnvelope(envelope, 1);
      return new Tensor(Array.from(out), { shape: [1] });
    }
    let sum = 0;
    for (let i = 0; i < a.numel; i++) sum += a.data[i]!;
    return new Tensor([sum], { shape: [1] });
  }
  // d/dx_i (Σ x_j) = 1.  Broadcast the scalar gradient (shape [1]) to
  // a full tensor of the input shape.
  backward(grad: Tensor): (Tensor | null)[] {
    const inputShape = this.savedForBackward.inputShape as number[];
    const g = grad.data[0]!;
    const numel = inputShape.length === 0 ? 1 : inputShape.reduce((a, b) => a * b, 1);
    return [new Tensor(new Array(numel).fill(g), { shape: inputShape.slice() })];
  }
}

export class MeanOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    const a = inputs[0] as Tensor;
    this.savedForBackward.inputShape = a.shape.slice();
    this.savedForBackward.numel = a.numel;
    if (a.numel >= DISPATCH_THRESHOLD) {
      const envelope = reduceAllEnvelope("ReduceMean", a);
      const out = runEnvelope(envelope, 1);
      return new Tensor(Array.from(out), { shape: [1] });
    }
    let sum = 0;
    for (let i = 0; i < a.numel; i++) sum += a.data[i]!;
    return new Tensor([sum / a.numel], { shape: [1] });
  }
  // d/dx_i ((1/N) Σ x_j) = 1/N.  Broadcast g/N to the input shape.
  backward(grad: Tensor): (Tensor | null)[] {
    const inputShape = this.savedForBackward.inputShape as number[];
    const n = this.savedForBackward.numel as number;
    const g = grad.data[0]! / n;
    const numel = inputShape.length === 0 ? 1 : inputShape.reduce((a, b) => a * b, 1);
    return [new Tensor(new Array(numel).fill(g), { shape: inputShape.slice() })];
  }
}
