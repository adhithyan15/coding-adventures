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
import { getMode } from "./mode.js";

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

// ────────── MatMul — N-D batched ──────────
//
// Supported shapes (rank ≥ 2 on both sides):
//   * 2-D × 2-D                                           (M, K) @ (K, N) → (M, N)
//   * batched same rank                       (B…, M, K) @ (B…, K, N) → (B…, M, N)
//   * broadcast right operand                 (B…, M, K) @       (K, N) → (B…, M, N)
//   * broadcast left operand                        (M, K) @ (B…, K, N) → (B…, M, N)
//   * batch-dim broadcasting                  (B1, 1, M, K) @ (1, B2, K, N) → (B1, B2, M, N)
//
// Algorithm: split each input into a "batch portion" (all dims except
// the last two) and a "matrix portion" (the trailing two).  Broadcast
// the batch portions to a common shape via the existing broadcasting
// machinery, then loop a single 2-D matmul per batch index.  The
// matrix dims themselves are NEVER broadcast — that's not what batched
// matmul means.
//
// Backward uses the same per-slice 2-D formulas (dL/dA_slice = grad_slice @
// B_slice^T, dL/dB_slice = A_slice^T @ grad_slice), then unbroadcasts the
// batch dims back to each parent's original shape so gradients flow to
// shared/broadcast operands correctly.
//
// Rust dispatch is intentionally limited to the pure 2-D case for now —
// matrix-cpu's MatMul kernel is 2-D, and pumping a thousand small matmuls
// through the JSON/FFI roundtrip would be slower than the TS loop.  A
// future PR can lift batched matmul to Rust by adding a BatchMatMul op
// to matrix-ir, but for v1.2 the per-slice TS path is correct + fast
// enough at the parameter sizes ML training uses.

export class MatMulOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    const a = inputs[0] as Tensor;
    const b = inputs[1] as Tensor;
    if (a.ndim < 2 || b.ndim < 2) {
      throw new RangeError(
        `matmul requires rank ≥ 2 on both inputs, got ndim ${a.ndim} and ${b.ndim}`,
      );
    }
    const aMat = a.shape.slice(-2) as [number, number];
    const bMat = b.shape.slice(-2) as [number, number];
    const [m, k1] = aMat;
    const [k2, n] = bMat;
    if (k1 !== k2) {
      throw new RangeError(
        `matmul inner-dim mismatch: [${a.shape.join(", ")}] @ [${b.shape.join(", ")}] ` +
          `(trailing dims must match: ${k1} vs ${k2})`,
      );
    }

    // Pure 2-D fast path — keeps the Rust dispatch and the v1.0 codepath
    // bit-identical when both operands are exactly 2-D.
    if (a.ndim === 2 && b.ndim === 2) {
      this.savedForBackward.a = a;
      this.savedForBackward.b = b;
      this.savedForBackward.batchShape = [] as number[];
      this.savedForBackward.aShape = a.shape.slice();
      this.savedForBackward.bShape = b.shape.slice();
      this.savedForBackward.batched = false;

      if (a.numel >= DISPATCH_THRESHOLD || b.numel >= DISPATCH_THRESHOLD) {
        const envelope = matmulEnvelope(a, b);
        const out = runEnvelope(envelope, m * n);
        return new Tensor(Array.from(out), { shape: [m, n] });
      }

      return new Tensor(MatMulOp._matmulNaive(a.data, b.data, m, k1, n), {
        shape: [m, n],
      });
    }

    // ── N-D batched path ──
    //
    // Broadcast the batch dims (everything except the trailing two) to
    // a common shape.  Then concatenate that with the per-side matrix
    // dims to form the broadcast shape used by the per-batch loop.
    const aBatch = a.shape.slice(0, -2);
    const bBatch = b.shape.slice(0, -2);
    const outBatch = broadcastShapes(aBatch, bBatch);
    const batchSize = outBatch.reduce((acc, d) => acc * d, 1);

    const aFullShape = [...outBatch, m, k1];
    const bFullShape = [...outBatch, k1, n];

    // Materialize the broadcasted batch portion.  Matching shapes are
    // a near-free copy in `broadcastDataTo` (its fast path also handles
    // the no-stretch case).
    const aBuf = broadcastDataTo(a.data, a.shape, aFullShape);
    const bBuf = broadcastDataTo(b.data, b.shape, bFullShape);

    const outShape = [...outBatch, m, n];
    const out = new Float32Array(batchSize * m * n);
    const aStride = m * k1;
    const bStride = k1 * n;
    const oStride = m * n;
    for (let batch = 0; batch < batchSize; batch++) {
      const aSlice = aBuf.subarray(batch * aStride, (batch + 1) * aStride);
      const bSlice = bBuf.subarray(batch * bStride, (batch + 1) * bStride);
      const cSlice = MatMulOp._matmulNaive(aSlice, bSlice, m, k1, n);
      for (let i = 0; i < oStride; i++) out[batch * oStride + i] = cSlice[i]!;
    }

    // Save broadcasted buffers — backward needs them at the broadcast
    // shape to compute per-slice grads, then unbroadcasts back to the
    // original parent shapes.
    this.savedForBackward.aBuf = aBuf;
    this.savedForBackward.bBuf = bBuf;
    this.savedForBackward.aShape = a.shape.slice();
    this.savedForBackward.bShape = b.shape.slice();
    this.savedForBackward.aFullShape = aFullShape;
    this.savedForBackward.bFullShape = bFullShape;
    this.savedForBackward.outBatch = outBatch;
    this.savedForBackward.m = m;
    this.savedForBackward.k = k1;
    this.savedForBackward.n = n;
    this.savedForBackward.batched = true;

    return new Tensor(Array.from(out), { shape: outShape });
  }

  // Backward for C = A @ B:
  //   dL/dA = grad @ B^T           (per batch slice)
  //   dL/dB = A^T @ grad           (per batch slice)
  //
  // 2-D: identical to v1.0.  N-D batched: loop over batch slices,
  // accumulate into broadcast-shaped grads, then unbroadcast back to
  // each parent's original shape (so a broadcast operand receives a
  // properly summed gradient).
  backward(grad: Tensor): (Tensor | null)[] {
    if (!this.savedForBackward.batched) {
      const a = this.savedForBackward.a as Tensor;
      const b = this.savedForBackward.b as Tensor;
      const [m, k] = a.shape as [number, number];
      const [, n] = b.shape as [number, number];

      const bT = MatMulOp._transpose2D(b.data, k, n);                  // (n, k)
      const gradAData = MatMulOp._matmulNaive(grad.data, bT, m, n, k); // (m, k)
      const aT = MatMulOp._transpose2D(a.data, m, k);                  // (k, m)
      const gradBData = MatMulOp._matmulNaive(aT, grad.data, k, m, n); // (k, n)

      return [
        new Tensor(gradAData, { shape: [m, k] }),
        new Tensor(gradBData, { shape: [k, n] }),
      ];
    }

    const aBuf = this.savedForBackward.aBuf as Float32Array;
    const bBuf = this.savedForBackward.bBuf as Float32Array;
    const aShape = this.savedForBackward.aShape as number[];
    const bShape = this.savedForBackward.bShape as number[];
    const aFullShape = this.savedForBackward.aFullShape as number[];
    const bFullShape = this.savedForBackward.bFullShape as number[];
    const outBatch = this.savedForBackward.outBatch as number[];
    const m = this.savedForBackward.m as number;
    const k = this.savedForBackward.k as number;
    const n = this.savedForBackward.n as number;
    const batchSize = outBatch.reduce((acc, d) => acc * d, 1);

    const gradABig = new Float32Array(batchSize * m * k);
    const gradBBig = new Float32Array(batchSize * k * n);
    const aStride = m * k;
    const bStride = k * n;
    const gStride = m * n;

    for (let batch = 0; batch < batchSize; batch++) {
      const aSlice = aBuf.subarray(batch * aStride, (batch + 1) * aStride);
      const bSlice = bBuf.subarray(batch * bStride, (batch + 1) * bStride);
      const gSlice = grad.data.subarray(batch * gStride, (batch + 1) * gStride);

      const bT = MatMulOp._transpose2D(bSlice, k, n);                 // (n, k)
      const gA = MatMulOp._matmulNaive(gSlice, bT, m, n, k);          // (m, k)
      for (let i = 0; i < aStride; i++) gradABig[batch * aStride + i] = gA[i]!;

      const aT = MatMulOp._transpose2D(aSlice, m, k);                 // (k, m)
      const gB = MatMulOp._matmulNaive(aT, gSlice, k, m, n);          // (k, n)
      for (let i = 0; i < bStride; i++) gradBBig[batch * bStride + i] = gB[i]!;
    }

    return [
      tensorFromBuf(unbroadcastDataTo(gradABig, aFullShape, aShape), aShape),
      tensorFromBuf(unbroadcastDataTo(gradBBig, bFullShape, bShape), bShape),
    ];
  }

  /** O(m*k*n) naive matmul on raw Float32Array data. */
  static _matmulNaive(
    aData: ArrayLike<number>,
    bData: ArrayLike<number>,
    m: number,
    k: number,
    n: number,
  ): number[] {
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

// ────────── Embedding — lookup table (Phase A.3) ──────────
//
// Standard NN embedding layer.  Given a learnable weight matrix of shape
// (vocab_size, embedding_dim) and an integer-valued indices tensor of
// arbitrary shape `S`, looks up each index's row and produces an output
// of shape `[...S, embedding_dim]`.
//
// ## Why indices are a Tensor (of integers) and not number[]
//
// Two reasons:
//   1. Consistency — everything else in the framework is a Tensor.  An
//      embedding lookup naturally chains: tokens → embedding → linear → ...
//   2. Shape: `indices.shape` IS the prefix of the output shape.  Carrying
//      shape on the Tensor saves a separate `indicesShape` argument.
//
// Indices values are still cast to int at lookup time (we use Math.trunc
// on each f32 cell).  This matches PyTorch's de-facto "you pass a Long
// tensor" convention but works within our f32-only storage model.
//
// ## Backward = scatter-add
//
// This is the MOST IMPORTANT correctness property of an embedding layer.
// When the same vocabulary index appears multiple times in `indices`
// (which is the common case — most sentences have repeated tokens), the
// gradient at that weight row is the SUM of grad slices from every
// occurrence.  A naive "set weight[idx, :] = grad slice" would drop all
// but the last occurrence's contribution.  We use += (accumulate) into
// the grad-weight buffer to avoid that bug.
//
// Indices receive no gradient (they're not differentiable; return null).
//
// ## What this unlocks
//
// Any NLP model that takes token IDs as input — i.e. essentially every
// transformer ever published.  v1.3 with Embedding is the last big op
// missing before we can start expressing attention layers (matmul + softmax
// + embedding all in place).

export class EmbeddingOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    const weight = inputs[0] as Tensor;
    const indices = inputs[1] as Tensor;

    if (weight.ndim !== 2) {
      throw new RangeError(
        `embedding weight must be 2-D (vocab_size, embedding_dim); got shape [${weight.shape.join(", ")}]`,
      );
    }
    const [vocabSize, embDim] = weight.shape as [number, number];

    // Materialize a Int32Array view of the indices so range-checks and
    // lookups are integer-clean.  We use Math.trunc to convert (round
    // toward zero), then validate range.
    const numIndices = indices.numel;
    const idx = new Int32Array(numIndices);
    for (let i = 0; i < numIndices; i++) {
      const v = Math.trunc(indices.data[i]!);
      if (v < 0 || v >= vocabSize) {
        throw new RangeError(
          `embedding index ${indices.data[i]} at position ${i} out of range ` +
            `[0, ${vocabSize}); embedding weight has ${vocabSize} rows`,
        );
      }
      idx[i] = v;
    }

    // Output shape: indices.shape ++ [embedding_dim].  Special case:
    // scalar indices (shape []) yields shape [embedding_dim].
    const outShape = [...indices.shape, embDim];
    const out = new Float32Array(numIndices * embDim);
    for (let i = 0; i < numIndices; i++) {
      const row = idx[i]!;
      const srcStart = row * embDim;
      const dstStart = i * embDim;
      for (let d = 0; d < embDim; d++) {
        out[dstStart + d] = weight.data[srcStart + d]!;
      }
    }

    this.savedForBackward.weightShape = weight.shape.slice();
    this.savedForBackward.indicesShape = indices.shape.slice();
    this.savedForBackward.idx = idx;
    this.savedForBackward.vocabSize = vocabSize;
    this.savedForBackward.embDim = embDim;

    return new Tensor(Array.from(out), { shape: outShape });
  }

  // Backward = scatter-add into a zeros-initialized grad-weight.
  //
  // grad shape: [...indices.shape, embedding_dim]
  // → flatten the leading dims to numIndices and treat as
  //   (numIndices, embedding_dim).
  // For each i in 0..numIndices:
  //   gradWeight[idx[i], :] += grad[i, :]
  //
  // The += (not =) is the scatter-add — repeated indices accumulate.
  //
  // Indices receive null (not differentiable).
  backward(grad: Tensor): (Tensor | null)[] {
    const idx = this.savedForBackward.idx as Int32Array;
    const vocabSize = this.savedForBackward.vocabSize as number;
    const embDim = this.savedForBackward.embDim as number;
    const weightShape = this.savedForBackward.weightShape as number[];
    const numIndices = idx.length;

    const gradWeight = new Float32Array(vocabSize * embDim); // zero-init
    for (let i = 0; i < numIndices; i++) {
      const row = idx[i]!;
      const srcStart = i * embDim;
      const dstStart = row * embDim;
      for (let d = 0; d < embDim; d++) {
        gradWeight[dstStart + d]! += grad.data[srcStart + d]!;
      }
    }

    return [
      new Tensor(Array.from(gradWeight), { shape: weightShape }),
      null,
    ];
  }
}

// ────────── LayerNorm / BatchNorm / Dropout (Phase A.4) ──────────
//
// The "normalize and regularize" trifecta every transformer uses.

/**
 * LayerNorm — normalize across the LAST dimension, then scale + shift.
 *
 * For each "row" of D values along the last axis:
 *   μ = mean(x), σ² = mean((x - μ)²),  x̂ = (x - μ) / √(σ² + ε)
 *   y = γ * x̂ + β
 *
 * γ (gamma) and β (beta) are learnable parameters of shape [D].
 * Standard PyTorch `nn.LayerNorm(D)` behavior.
 *
 * The variance is the biased (population) estimator — `mean(x²) - μ²` —
 * matching PyTorch's default.
 *
 * ## Backward (derivation)
 *
 * Let dy = grad w.r.t. y, and write dx̂_i = dy_i * γ_i.  Then for each
 * row of D values:
 *
 *   dL/dβ_i = Σ over batch of dy_i
 *   dL/dγ_i = Σ over batch of dy_i * x̂_i
 *   dL/dx_i = (1/(σ * D)) * (D * dx̂_i - Σ_j dx̂_j - x̂_i * Σ_j dx̂_j * x̂_j)
 *
 * Derived by chain rule through μ and σ.  Same formula as PyTorch and
 * every reference implementation.
 */
export class LayerNormOp extends Function {
  static readonly DEFAULT_EPS = 1e-5;

  forward(...inputs: unknown[]): Tensor {
    const x = inputs[0] as Tensor;
    const gamma = inputs[1] as Tensor;
    const beta = inputs[2] as Tensor;
    const eps = (inputs[3] as number | undefined) ?? LayerNormOp.DEFAULT_EPS;

    if (x.ndim < 1) {
      throw new RangeError("LayerNorm requires x with ndim ≥ 1");
    }
    const D = x.shape[x.shape.length - 1]!;
    if (gamma.ndim !== 1 || gamma.shape[0] !== D) {
      throw new RangeError(
        `LayerNorm gamma must be 1-D with shape [${D}]; got [${gamma.shape.join(", ")}]`,
      );
    }
    if (beta.ndim !== 1 || beta.shape[0] !== D) {
      throw new RangeError(
        `LayerNorm beta must be 1-D with shape [${D}]; got [${beta.shape.join(", ")}]`,
      );
    }

    const N = x.numel / D; // number of "rows" to normalize
    const out = new Float32Array(x.numel);
    const xhat = new Float32Array(x.numel);
    const mean = new Float32Array(N);
    const invStd = new Float32Array(N); // 1 / √(σ² + ε)

    for (let r = 0; r < N; r++) {
      const off = r * D;
      // mean
      let mu = 0;
      for (let i = 0; i < D; i++) mu += x.data[off + i]!;
      mu /= D;
      mean[r] = mu;
      // variance (biased)
      let sq = 0;
      for (let i = 0; i < D; i++) {
        const d = x.data[off + i]! - mu;
        sq += d * d;
      }
      const variance = sq / D;
      const inv = 1 / Math.sqrt(variance + eps);
      invStd[r] = inv;
      // normalize + affine
      for (let i = 0; i < D; i++) {
        const xh = (x.data[off + i]! - mu) * inv;
        xhat[off + i] = xh;
        out[off + i] = xh * gamma.data[i]! + beta.data[i]!;
      }
    }

    this.savedForBackward.xShape = x.shape.slice();
    this.savedForBackward.gammaData = new Float32Array(gamma.data);
    this.savedForBackward.D = D;
    this.savedForBackward.N = N;
    this.savedForBackward.xhat = xhat;
    this.savedForBackward.invStd = invStd;

    return new Tensor(Array.from(out), { shape: x.shape.slice() });
  }

  backward(grad: Tensor): (Tensor | null)[] {
    const xShape = this.savedForBackward.xShape as number[];
    const gammaData = this.savedForBackward.gammaData as Float32Array;
    const D = this.savedForBackward.D as number;
    const N = this.savedForBackward.N as number;
    const xhat = this.savedForBackward.xhat as Float32Array;
    const invStd = this.savedForBackward.invStd as Float32Array;

    const dx = new Float32Array(N * D);
    const dGamma = new Float32Array(D); // accumulated across all rows
    const dBeta = new Float32Array(D);

    for (let r = 0; r < N; r++) {
      const off = r * D;
      const inv = invStd[r]!;
      // Compute dx̂_i = dy_i * γ_i  and the two row-sums needed for dx.
      let sumDxhat = 0;
      let sumDxhatXhat = 0;
      const dxhatRow = new Float32Array(D);
      for (let i = 0; i < D; i++) {
        const dyi = grad.data[off + i]!;
        const xhi = xhat[off + i]!;
        const dxh = dyi * gammaData[i]!;
        dxhatRow[i] = dxh;
        sumDxhat += dxh;
        sumDxhatXhat += dxh * xhi;
        // Accumulate gradient w.r.t. γ and β across all rows.
        dGamma[i]! += dyi * xhi;
        dBeta[i]! += dyi;
      }
      // dL/dx_i = (1/(σ*D)) * (D * dx̂_i - Σ dx̂ - x̂_i * Σ dx̂*x̂)
      const scale = inv / D;
      for (let i = 0; i < D; i++) {
        dx[off + i] = scale * (D * dxhatRow[i]! - sumDxhat - xhat[off + i]! * sumDxhatXhat);
      }
    }

    return [
      new Tensor(Array.from(dx), { shape: xShape }),
      new Tensor(Array.from(dGamma), { shape: [D] }),
      new Tensor(Array.from(dBeta), { shape: [D] }),
    ];
  }
}

/**
 * BatchNorm — normalize across the BATCH dimension (axis 0).
 *
 * For each column-position c (everything-except-axis-0):
 *   μ_c = mean over batch of x[:, c],   σ²_c = mean over batch of (x[:, c] - μ_c)²
 *   x̂[:, c] = (x[:, c] - μ_c) / √(σ²_c + ε)
 *   y[:, c] = γ_c * x̂[:, c] + β_c
 *
 * Train mode uses the current batch's μ/σ² AND updates the running
 * statistics in-place:
 *   runningMean := (1 - momentum) * runningMean + momentum * batchMean
 *   runningVar  := (1 - momentum) * runningVar  + momentum * batchVar
 *
 * Eval mode uses runningMean / runningVar instead, no update.
 *
 * γ, β, runningMean, runningVar all have shape [C] where C is the
 * product of all-dims-except-batch.  For typical (N, C) input C is
 * just the feature count.  In v1.4 we support general input but
 * the typical use is 2-D.
 *
 * Backward (train mode only) uses the same derivative form as
 * LayerNorm, just over the batch axis instead of the last axis.
 * Running stats are non-differentiable buffers → no gradient flows
 * through them.
 */
export class BatchNormOp extends Function {
  static readonly DEFAULT_EPS = 1e-5;
  static readonly DEFAULT_MOMENTUM = 0.1;

  forward(...inputs: unknown[]): Tensor {
    const x = inputs[0] as Tensor;
    const gamma = inputs[1] as Tensor;
    const beta = inputs[2] as Tensor;
    const runningMean = inputs[3] as Tensor;
    const runningVar = inputs[4] as Tensor;
    const momentum = (inputs[5] as number | undefined) ?? BatchNormOp.DEFAULT_MOMENTUM;
    const eps = (inputs[6] as number | undefined) ?? BatchNormOp.DEFAULT_EPS;

    if (x.ndim < 2) {
      throw new RangeError(`BatchNorm requires x with ndim ≥ 2 (batch + features); got ndim ${x.ndim}`);
    }
    const N = x.shape[0]!;
    const C = x.numel / N; // product of all-but-first dims
    if (gamma.numel !== C || beta.numel !== C || runningMean.numel !== C || runningVar.numel !== C) {
      throw new RangeError(
        `BatchNorm gamma/beta/runningMean/runningVar must all have ${C} cells ` +
          `(matching non-batch dims of x with shape [${x.shape.join(", ")}])`,
      );
    }

    const mode = getMode();
    const out = new Float32Array(x.numel);
    const xhat = new Float32Array(x.numel);
    const useMean = new Float32Array(C);
    const useVar = new Float32Array(C);

    if (mode === "train") {
      // Compute per-feature batch mean + variance.
      for (let c = 0; c < C; c++) {
        let s = 0;
        for (let n = 0; n < N; n++) s += x.data[n * C + c]!;
        useMean[c] = s / N;
      }
      for (let c = 0; c < C; c++) {
        let sq = 0;
        for (let n = 0; n < N; n++) {
          const d = x.data[n * C + c]! - useMean[c]!;
          sq += d * d;
        }
        useVar[c] = sq / N;
      }
      // Update running stats in-place (PyTorch convention; non-differentiable).
      for (let c = 0; c < C; c++) {
        runningMean.data[c] = (1 - momentum) * runningMean.data[c]! + momentum * useMean[c]!;
        runningVar.data[c] = (1 - momentum) * runningVar.data[c]! + momentum * useVar[c]!;
      }
    } else {
      // Eval mode — use frozen running stats.
      for (let c = 0; c < C; c++) {
        useMean[c] = runningMean.data[c]!;
        useVar[c] = runningVar.data[c]!;
      }
    }

    const invStd = new Float32Array(C);
    for (let c = 0; c < C; c++) invStd[c] = 1 / Math.sqrt(useVar[c]! + eps);

    for (let n = 0; n < N; n++) {
      for (let c = 0; c < C; c++) {
        const xh = (x.data[n * C + c]! - useMean[c]!) * invStd[c]!;
        xhat[n * C + c] = xh;
        out[n * C + c] = xh * gamma.data[c]! + beta.data[c]!;
      }
    }

    this.savedForBackward.xShape = x.shape.slice();
    this.savedForBackward.gammaData = new Float32Array(gamma.data);
    this.savedForBackward.xhat = xhat;
    this.savedForBackward.invStd = invStd;
    this.savedForBackward.N = N;
    this.savedForBackward.C = C;
    this.savedForBackward.mode = mode;

    return new Tensor(Array.from(out), { shape: x.shape.slice() });
  }

  backward(grad: Tensor): (Tensor | null)[] {
    const xShape = this.savedForBackward.xShape as number[];
    const gammaData = this.savedForBackward.gammaData as Float32Array;
    const xhat = this.savedForBackward.xhat as Float32Array;
    const invStd = this.savedForBackward.invStd as Float32Array;
    const N = this.savedForBackward.N as number;
    const C = this.savedForBackward.C as number;
    const mode = this.savedForBackward.mode as "train" | "eval";

    const dx = new Float32Array(N * C);
    const dGamma = new Float32Array(C);
    const dBeta = new Float32Array(C);

    if (mode === "eval") {
      // In eval, useMean/useVar were CONSTANTS (running stats are
      // non-differentiable buffers), so x̂ = (x - const) / const'.
      // dy/dx = γ / σ̂ per feature column.
      // dy/dγ = x̂  (accumulated across batch)
      // dy/dβ = 1  (accumulated across batch)
      for (let n = 0; n < N; n++) {
        for (let c = 0; c < C; c++) {
          const dy = grad.data[n * C + c]!;
          dx[n * C + c] = dy * gammaData[c]! * invStd[c]!;
          dGamma[c]! += dy * xhat[n * C + c]!;
          dBeta[c]! += dy;
        }
      }
    } else {
      // Train: same form as LayerNorm but over the batch axis (N) per
      // feature column (C).
      for (let c = 0; c < C; c++) {
        const inv = invStd[c]!;
        let sumDxhat = 0;
        let sumDxhatXhat = 0;
        const dxhatCol = new Float32Array(N);
        for (let n = 0; n < N; n++) {
          const dy = grad.data[n * C + c]!;
          const xh = xhat[n * C + c]!;
          const dxh = dy * gammaData[c]!;
          dxhatCol[n] = dxh;
          sumDxhat += dxh;
          sumDxhatXhat += dxh * xh;
          dGamma[c]! += dy * xh;
          dBeta[c]! += dy;
        }
        const scale = inv / N;
        for (let n = 0; n < N; n++) {
          dx[n * C + c] = scale * (N * dxhatCol[n]! - sumDxhat - xhat[n * C + c]! * sumDxhatXhat);
        }
      }
    }

    // Parents: x, gamma, beta, runningMean, runningVar.
    // Running stats receive no gradient.
    const gammaShape = (this.parents[1] as Tensor).shape.slice();
    const betaShape = (this.parents[2] as Tensor).shape.slice();
    return [
      new Tensor(Array.from(dx), { shape: xShape }),
      new Tensor(Array.from(dGamma), { shape: gammaShape }),
      new Tensor(Array.from(dBeta), { shape: betaShape }),
      null,
      null,
    ];
  }
}

/**
 * Dropout — randomly zero activations with probability `p` during
 * training; passthrough during eval.
 *
 * Inverted dropout: surviving cells are scaled by 1/(1-p) so the
 * expected magnitude stays constant.  This means inference doesn't
 * need any scaling — the same network with `setMode("eval")` just
 * lets activations through unchanged.
 *
 * ## Why Math.random() is fine
 *
 * Dropout is a regularization heuristic — the network learns to be
 * robust to which units are dropped, NOT to any specific random
 * sequence.  A cryptographically secure RNG would be slower with no
 * benefit to model quality.  We document this choice rather than
 * pull in a crypto dep.
 *
 * Determinism note: there's no seed control here.  Run-to-run training
 * is non-reproducible.  A future PR can add `setSeed()` if needed.
 */
export class DropoutOp extends Function {
  static readonly DEFAULT_P = 0.5;

  forward(...inputs: unknown[]): Tensor {
    const x = inputs[0] as Tensor;
    const p = (inputs[1] as number | undefined) ?? DropoutOp.DEFAULT_P;
    if (p < 0 || p >= 1) {
      throw new RangeError(`Dropout p must satisfy 0 ≤ p < 1, got ${p}`);
    }

    const mode = getMode();
    if (mode === "eval" || p === 0) {
      // Passthrough (still produces a new Tensor so identity differs;
      // backward becomes pure passthrough).
      this.savedForBackward.passthrough = true;
      this.savedForBackward.xShape = x.shape.slice();
      return new Tensor(Array.from(x.data), { shape: x.shape.slice() });
    }

    const scale = 1 / (1 - p);
    const mask = new Float32Array(x.numel); // 0 or `scale`
    const out = new Float32Array(x.numel);
    for (let i = 0; i < x.numel; i++) {
      // Math.random() returns [0, 1).  Keep with prob (1 - p).
      const keep = Math.random() >= p ? 1 : 0;
      const m = keep * scale;
      mask[i] = m;
      out[i] = x.data[i]! * m;
    }

    this.savedForBackward.passthrough = false;
    this.savedForBackward.mask = mask;
    this.savedForBackward.xShape = x.shape.slice();

    return new Tensor(Array.from(out), { shape: x.shape.slice() });
  }

  backward(grad: Tensor): (Tensor | null)[] {
    const xShape = this.savedForBackward.xShape as number[];
    if (this.savedForBackward.passthrough) {
      return [new Tensor(Array.from(grad.data), { shape: xShape })];
    }
    const mask = this.savedForBackward.mask as Float32Array;
    const out = new Float32Array(grad.numel);
    for (let i = 0; i < grad.numel; i++) {
      out[i] = grad.data[i]! * mask[i]!;
    }
    return [new Tensor(Array.from(out), { shape: xShape })];
  }
}

// ────────── Conv2D / MaxPool2D via im2col (Phase A.5) ──────────
//
// The classic im2col formulation: re-express a 2-D convolution as a
// big matrix multiply.  Forward unrolls each receptive-field patch
// into a row of a (N*outH*outW, C*kH*kW) matrix, then matmul with
// the weight tensor reshaped to (C*kH*kW, outC) yields the conv
// output in matrix form, which we reshape back to (N, outC, outH, outW).
//
// Backward is two matmuls (dL/dW and dL/dX in matrix form) plus a
// col2im — the inverse of im2col that ACCUMULATES (since multiple
// output patches share input cells when the receptive fields overlap
// or when stride < kernel).

/** Output spatial dim formula: floor((in + 2*pad - kernel)/stride) + 1. */
function conv2dOutDim(inDim: number, kernel: number, stride: number, padding: number): number {
  return Math.floor((inDim + 2 * padding - kernel) / stride) + 1;
}

/**
 * im2col: unfold (N, C, H, W) into (N*outH*outW, C*kH*kW).
 *
 * Each output row corresponds to one (n, oh, ow) position; columns
 * are the cells of the receptive field in C-major, then ki, then kj
 * order.  Cells outside the padded input become 0 (zero-padding
 * convention; matches PyTorch's default for nn.Conv2d).
 */
function im2col(
  x: Float32Array,
  N: number, C: number, H: number, W: number,
  kH: number, kW: number,
  sH: number, sW: number,
  pH: number, pW: number,
  outH: number, outW: number,
): Float32Array {
  const cols = C * kH * kW;
  const rows = N * outH * outW;
  const out = new Float32Array(rows * cols);
  for (let n = 0; n < N; n++) {
    for (let oh = 0; oh < outH; oh++) {
      for (let ow = 0; ow < outW; ow++) {
        const rowIdx = (n * outH + oh) * outW + ow;
        const rowOff = rowIdx * cols;
        for (let c = 0; c < C; c++) {
          for (let ki = 0; ki < kH; ki++) {
            const ih = oh * sH - pH + ki;
            for (let kj = 0; kj < kW; kj++) {
              const iw = ow * sW - pW + kj;
              const colIdx = (c * kH + ki) * kW + kj;
              if (ih >= 0 && ih < H && iw >= 0 && iw < W) {
                out[rowOff + colIdx] = x[((n * C + c) * H + ih) * W + iw]!;
              }
              // else: 0 (Float32Array is zero-init)
            }
          }
        }
      }
    }
  }
  return out;
}

/**
 * col2im: scatter-accumulate inverse of im2col.
 *
 * Given a (N*outH*outW, C*kH*kW) matrix, write each cell back to its
 * source position in (N, C, H, W).  Multiple output patches that
 * touched the same input cell accumulate via += — this is what makes
 * the conv backward gradient flow correct.  Padded positions are
 * silently dropped (no input cell exists there to receive a grad).
 */
function col2im(
  cols: Float32Array,
  N: number, C: number, H: number, W: number,
  kH: number, kW: number,
  sH: number, sW: number,
  pH: number, pW: number,
  outH: number, outW: number,
): Float32Array {
  const nCols = C * kH * kW;
  const out = new Float32Array(N * C * H * W);
  for (let n = 0; n < N; n++) {
    for (let oh = 0; oh < outH; oh++) {
      for (let ow = 0; ow < outW; ow++) {
        const rowIdx = (n * outH + oh) * outW + ow;
        const rowOff = rowIdx * nCols;
        for (let c = 0; c < C; c++) {
          for (let ki = 0; ki < kH; ki++) {
            const ih = oh * sH - pH + ki;
            for (let kj = 0; kj < kW; kj++) {
              const iw = ow * sW - pW + kj;
              if (ih >= 0 && ih < H && iw >= 0 && iw < W) {
                const colIdx = (c * kH + ki) * kW + kj;
                out[((n * C + c) * H + ih) * W + iw]! += cols[rowOff + colIdx]!;
              }
            }
          }
        }
      }
    }
  }
  return out;
}

/** Naive (m × k) @ (k × n) on raw buffers — used internally by Conv2D. */
function matmulBuf(
  a: ArrayLike<number>, b: ArrayLike<number>,
  m: number, k: number, n: number,
): Float32Array {
  const out = new Float32Array(m * n);
  for (let i = 0; i < m; i++) {
    for (let j = 0; j < n; j++) {
      let acc = 0;
      for (let kk = 0; kk < k; kk++) acc += a[i * k + kk]! * b[kk * n + j]!;
      out[i * n + j] = acc;
    }
  }
  return out;
}

/** Transpose a flat row-major (rows × cols) matrix.  Used internally. */
function transposeBuf(data: ArrayLike<number>, rows: number, cols: number): Float32Array {
  const out = new Float32Array(rows * cols);
  for (let r = 0; r < rows; r++) {
    for (let c = 0; c < cols; c++) out[c * rows + r] = data[r * cols + c]!;
  }
  return out;
}

/**
 * Conv2D — 2-D convolution via im2col + matmul.
 *
 * Input  x:      (N, C, H, W)
 * Weight w:      (outC, C, kH, kW)
 * Bias b:        (outC,) or null
 * Output:        (N, outC, outH, outW)
 *
 * Forward steps:
 *   1. X = im2col(x)            → (N*outH*outW, C*kH*kW)
 *   2. Wm = w reshaped flat     → (outC, C*kH*kW)
 *   3. Y_NHWC_flat = X @ Wm.T   → (N*outH*outW, outC)
 *   4. Reshape to (N, outH, outW, outC), permute (0, 3, 1, 2) →
 *      (N, outC, outH, outW)
 *   5. Add bias broadcast along axis 1 if present.
 *
 * Backward steps (with grad of shape (N, outC, outH, outW)):
 *   1. Permute grad (0, 2, 3, 1) → (N, outH, outW, outC); flatten leading
 *      → grad_flat of shape (N*outH*outW, outC).
 *   2. dL/dWm = X.T @ grad_flat         → (C*kH*kW, outC)
 *      dL/dw  = (dL/dWm).T reshaped     → (outC, C, kH, kW)
 *   3. dL/dX = grad_flat @ Wm           → (N*outH*outW, C*kH*kW)
 *      dL/dx = col2im(dL/dX)            → (N, C, H, W)
 *   4. dL/db = sum grad over (N, outH, outW) axes → (outC,)
 *
 * Stride and padding default to 1 and 0 respectively (PyTorch default).
 */
export class Conv2DOp extends Function {
  static readonly DEFAULT_STRIDE = 1;
  static readonly DEFAULT_PADDING = 0;

  forward(...inputs: unknown[]): Tensor {
    const x = inputs[0] as Tensor;
    const w = inputs[1] as Tensor;
    const bRaw = inputs[2] as Tensor | null | undefined;
    const stride = (inputs[3] as number | undefined) ?? Conv2DOp.DEFAULT_STRIDE;
    const padding = (inputs[4] as number | undefined) ?? Conv2DOp.DEFAULT_PADDING;

    if (x.ndim !== 4) {
      throw new RangeError(`Conv2D x must be (N, C, H, W); got shape [${x.shape.join(", ")}]`);
    }
    if (w.ndim !== 4) {
      throw new RangeError(`Conv2D weight must be (outC, inC, kH, kW); got [${w.shape.join(", ")}]`);
    }
    const [N, C, H, W] = x.shape as [number, number, number, number];
    const [outC, wC, kH, kW] = w.shape as [number, number, number, number];
    if (wC !== C) {
      throw new RangeError(`Conv2D in-channels mismatch: weight says ${wC}, x has ${C}`);
    }
    if (stride < 1) throw new RangeError(`Conv2D stride must be ≥ 1, got ${stride}`);
    if (padding < 0) throw new RangeError(`Conv2D padding must be ≥ 0, got ${padding}`);

    const outH = conv2dOutDim(H, kH, stride, padding);
    const outW = conv2dOutDim(W, kW, stride, padding);
    if (outH < 1 || outW < 1) {
      throw new RangeError(
        `Conv2D output spatial dim would be < 1 (outH=${outH}, outW=${outW}); ` +
          `kernel ${kH}x${kW} too large for input ${H}x${W} with stride ${stride}, padding ${padding}`,
      );
    }

    const b = bRaw ?? null;
    if (b !== null) {
      if (b.ndim !== 1 || b.shape[0] !== outC) {
        throw new RangeError(
          `Conv2D bias must be 1-D with shape [${outC}]; got [${b.shape.join(", ")}]`,
        );
      }
    }

    // 1. im2col
    const X = im2col(x.data, N, C, H, W, kH, kW, stride, stride, padding, padding, outH, outW);
    const cols = C * kH * kW;

    // 2. Weight reshape (outC, C*kH*kW) — same memory layout, just a view shape.
    //    We need Wm.T = (C*kH*kW, outC) for the matmul X @ Wm.T.
    const WmT = transposeBuf(w.data, outC, cols);

    // 3. Y_flat = X @ Wm.T : shape (N*outH*outW, outC)
    const rows = N * outH * outW;
    const yFlat = matmulBuf(X, WmT, rows, cols, outC);

    // 4. Reshape (N, outH, outW, outC) → permute (0, 3, 1, 2) → (N, outC, outH, outW)
    const outData = new Float32Array(N * outC * outH * outW);
    for (let n = 0; n < N; n++) {
      for (let oc = 0; oc < outC; oc++) {
        for (let oh = 0; oh < outH; oh++) {
          for (let ow = 0; ow < outW; ow++) {
            const srcIdx = ((n * outH + oh) * outW + ow) * outC + oc;
            const dstIdx = ((n * outC + oc) * outH + oh) * outW + ow;
            outData[dstIdx] = yFlat[srcIdx]!;
          }
        }
      }
    }

    // 5. Add bias broadcast along axis 1.
    if (b !== null) {
      for (let n = 0; n < N; n++) {
        for (let oc = 0; oc < outC; oc++) {
          const bv = b.data[oc]!;
          const baseIdx = (n * outC + oc) * outH * outW;
          for (let i = 0; i < outH * outW; i++) outData[baseIdx + i]! += bv;
        }
      }
    }

    this.savedForBackward.X = X;
    this.savedForBackward.wData = new Float32Array(w.data);
    this.savedForBackward.xShape = x.shape.slice();
    this.savedForBackward.wShape = w.shape.slice();
    this.savedForBackward.hasBias = b !== null;
    this.savedForBackward.N = N; this.savedForBackward.C = C;
    this.savedForBackward.H = H; this.savedForBackward.W = W;
    this.savedForBackward.outC = outC; this.savedForBackward.outH = outH; this.savedForBackward.outW = outW;
    this.savedForBackward.kH = kH; this.savedForBackward.kW = kW;
    this.savedForBackward.stride = stride; this.savedForBackward.padding = padding;

    return new Tensor(Array.from(outData), { shape: [N, outC, outH, outW] });
  }

  backward(grad: Tensor): (Tensor | null)[] {
    const X = this.savedForBackward.X as Float32Array;
    const wData = this.savedForBackward.wData as Float32Array;
    const xShape = this.savedForBackward.xShape as number[];
    const wShape = this.savedForBackward.wShape as number[];
    const hasBias = this.savedForBackward.hasBias as boolean;
    const N = this.savedForBackward.N as number;
    const C = this.savedForBackward.C as number;
    const H = this.savedForBackward.H as number;
    const W = this.savedForBackward.W as number;
    const outC = this.savedForBackward.outC as number;
    const outH = this.savedForBackward.outH as number;
    const outW = this.savedForBackward.outW as number;
    const kH = this.savedForBackward.kH as number;
    const kW = this.savedForBackward.kW as number;
    const stride = this.savedForBackward.stride as number;
    const padding = this.savedForBackward.padding as number;
    const cols = C * kH * kW;
    const rows = N * outH * outW;

    // 1. Permute grad (N, outC, outH, outW) → (N, outH, outW, outC) flat.
    const gradFlat = new Float32Array(rows * outC);
    for (let n = 0; n < N; n++) {
      for (let oc = 0; oc < outC; oc++) {
        for (let oh = 0; oh < outH; oh++) {
          for (let ow = 0; ow < outW; ow++) {
            const srcIdx = ((n * outC + oc) * outH + oh) * outW + ow;
            const dstIdx = ((n * outH + oh) * outW + ow) * outC + oc;
            gradFlat[dstIdx] = grad.data[srcIdx]!;
          }
        }
      }
    }

    // 2. dL/dWm = X.T @ gradFlat : shape (C*kH*kW, outC)
    //    Then dL/dW = (dL/dWm).T reshaped to (outC, C, kH, kW)
    const Xt = transposeBuf(X, rows, cols);
    const dWm = matmulBuf(Xt, gradFlat, cols, rows, outC); // (cols, outC)
    const dW = transposeBuf(dWm, cols, outC); // (outC, cols) — same flat as (outC, C, kH, kW)

    // 3. dL/dX = gradFlat @ Wm : shape (N*outH*outW, C*kH*kW)
    //    Wm has shape (outC, C*kH*kW) which is just wData reshaped.
    const dX = matmulBuf(gradFlat, wData, rows, outC, cols);
    const dxData = col2im(dX, N, C, H, W, kH, kW, stride, stride, padding, padding, outH, outW);

    // 4. dL/db = sum grad over (N, outH, outW) axes → (outC,)
    let dB: Tensor | null = null;
    if (hasBias) {
      const dBData = new Float32Array(outC);
      for (let n = 0; n < N; n++) {
        for (let oc = 0; oc < outC; oc++) {
          let s = 0;
          for (let oh = 0; oh < outH; oh++) {
            for (let ow = 0; ow < outW; ow++) {
              s += grad.data[((n * outC + oc) * outH + oh) * outW + ow]!;
            }
          }
          dBData[oc]! += s;
        }
      }
      dB = new Tensor(Array.from(dBData), { shape: [outC] });
    }

    const grads: (Tensor | null)[] = [
      new Tensor(Array.from(dxData), { shape: xShape }),
      new Tensor(Array.from(dW), { shape: wShape }),
    ];
    if (hasBias) grads.push(dB);
    return grads;
  }
}

/**
 * MaxPool2D — sliding-window max over (kH, kW) with the given stride.
 *
 * Input  x:  (N, C, H, W)
 * Output:    (N, C, outH, outW) where outH/outW use the conv2dOutDim
 *            formula with padding=0.
 *
 * Forward saves the flat input index of the argmax for each output
 * cell (one int per output cell).  Backward routes the upstream
 * gradient back to those exact positions; everything else in dx is
 * zero.  Repeated argmax indices (possible with overlapping windows
 * if stride < kernel) ACCUMULATE via +=.
 *
 * No padding in v1.5 — it's rare for max-pool anyway.  Default
 * stride equals kH (so non-overlapping windows by default — the
 * standard "downsample by k" use).
 */
export class MaxPool2DOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    const x = inputs[0] as Tensor;
    const kH = inputs[1] as number;
    const kW = inputs[2] as number;
    const strideArg = inputs[3] as number | undefined;
    const sH = strideArg ?? kH;
    const sW = strideArg ?? kW;

    if (x.ndim !== 4) {
      throw new RangeError(`MaxPool2D x must be (N, C, H, W); got shape [${x.shape.join(", ")}]`);
    }
    const [N, C, H, W] = x.shape as [number, number, number, number];
    if (kH < 1 || kW < 1) throw new RangeError(`MaxPool2D kernel must be ≥ 1`);
    if (sH < 1 || sW < 1) throw new RangeError(`MaxPool2D stride must be ≥ 1`);

    const outH = conv2dOutDim(H, kH, sH, 0);
    const outW = conv2dOutDim(W, kW, sW, 0);
    if (outH < 1 || outW < 1) {
      throw new RangeError(`MaxPool2D kernel ${kH}x${kW} too large for input ${H}x${W}`);
    }

    const out = new Float32Array(N * C * outH * outW);
    const argmax = new Int32Array(N * C * outH * outW); // flat index into x

    for (let n = 0; n < N; n++) {
      for (let c = 0; c < C; c++) {
        for (let oh = 0; oh < outH; oh++) {
          for (let ow = 0; ow < outW; ow++) {
            let best = -Infinity;
            let bestIdx = 0;
            for (let ki = 0; ki < kH; ki++) {
              const ih = oh * sH + ki;
              for (let kj = 0; kj < kW; kj++) {
                const iw = ow * sW + kj;
                const flat = ((n * C + c) * H + ih) * W + iw;
                const v = x.data[flat]!;
                if (v > best) {
                  best = v;
                  bestIdx = flat;
                }
              }
            }
            const outIdx = ((n * C + c) * outH + oh) * outW + ow;
            out[outIdx] = best;
            argmax[outIdx] = bestIdx;
          }
        }
      }
    }

    this.savedForBackward.argmax = argmax;
    this.savedForBackward.xShape = x.shape.slice();
    this.savedForBackward.xNumel = x.numel;

    return new Tensor(Array.from(out), { shape: [N, C, outH, outW] });
  }

  backward(grad: Tensor): (Tensor | null)[] {
    const argmax = this.savedForBackward.argmax as Int32Array;
    const xShape = this.savedForBackward.xShape as number[];
    const xNumel = this.savedForBackward.xNumel as number;
    const dx = new Float32Array(xNumel);
    for (let i = 0; i < argmax.length; i++) {
      // += so overlapping windows that elected the same input cell as
      // argmax accumulate (rare but correct).
      dx[argmax[i]!]! += grad.data[i]!;
    }
    return [new Tensor(Array.from(dx), { shape: xShape })];
  }
}
