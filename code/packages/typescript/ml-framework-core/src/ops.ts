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
  if (!shapesEqual(a.shape, b.shape)) {
    throw new RangeError(
      `shape mismatch for ${kind}: [${a.shape.join(", ")}] vs [${b.shape.join(", ")}]`,
    );
  }
  if (a.numel >= DISPATCH_THRESHOLD) {
    const envelope = binaryElementwiseEnvelope(kind, a, b);
    const out = runEnvelope(envelope, a.numel);
    return new Tensor(Array.from(out), { shape: a.shape.slice() });
  }
  const out = new Array(a.numel);
  for (let i = 0; i < a.numel; i++) out[i] = tsOp(a.data[i]!, b.data[i]!);
  return new Tensor(out, { shape: a.shape.slice() });
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
// Stub for not-yet-implemented backward — every Op below uses this.
// ===========================================================================
function notImplementedBackward(): never {
  throw new Error("backward not implemented; lands in PR #4");
}

// ===========================================================================
// The 15 differentiable ops.
// ===========================================================================

// ────────── Binary elementwise: Add / Sub / Mul / Div ──────────

export class AddOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    return binaryElementwise("Add", inputs[0] as Tensor, inputs[1] as Tensor, (x, y) => x + y);
  }
  backward(): (Tensor | null)[] { return notImplementedBackward(); }
}

export class SubOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    return binaryElementwise("Sub", inputs[0] as Tensor, inputs[1] as Tensor, (x, y) => x - y);
  }
  backward(): (Tensor | null)[] { return notImplementedBackward(); }
}

export class MulOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    return binaryElementwise("Mul", inputs[0] as Tensor, inputs[1] as Tensor, (x, y) => x * y);
  }
  backward(): (Tensor | null)[] { return notImplementedBackward(); }
}

export class DivOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    return binaryElementwise("Div", inputs[0] as Tensor, inputs[1] as Tensor, (x, y) => x / y);
  }
  backward(): (Tensor | null)[] { return notImplementedBackward(); }
}

// ────────── Unary elementwise: Neg / Abs / Tanh ──────────

export class NegOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    return unaryElementwise("Neg", inputs[0] as Tensor, (v) => -v);
  }
  backward(): (Tensor | null)[] { return notImplementedBackward(); }
}

export class AbsOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    return unaryElementwise("Abs", inputs[0] as Tensor, (v) => Math.abs(v));
  }
  backward(): (Tensor | null)[] { return notImplementedBackward(); }
}

export class TanhOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    return unaryElementwise("Tanh", inputs[0] as Tensor, (v) => Math.tanh(v));
  }
  backward(): (Tensor | null)[] { return notImplementedBackward(); }
}

// ────────── Pow: scalar exponent, pure TS for v0.3.0 ──────────

export class PowOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    const a = inputs[0] as Tensor;
    const exponent = inputs[1] as number;
    const out = new Array(a.numel);
    for (let i = 0; i < a.numel; i++) out[i] = Math.pow(a.data[i]!, exponent);
    return new Tensor(out, { shape: a.shape.slice() });
  }
  backward(): (Tensor | null)[] { return notImplementedBackward(); }
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

    if (a.numel >= DISPATCH_THRESHOLD || b.numel >= DISPATCH_THRESHOLD) {
      const envelope = matmulEnvelope(a, b);
      const out = runEnvelope(envelope, m * n);
      return new Tensor(Array.from(out), { shape: [m, n] });
    }

    // Pure-TS triple-loop matmul.  O(m*k*n).
    const out = new Array(m * n).fill(0);
    for (let i = 0; i < m; i++) {
      for (let j = 0; j < n; j++) {
        let acc = 0;
        for (let kk = 0; kk < k1; kk++) {
          acc += a.data[i * k1 + kk]! * b.data[kk * n + j]!;
        }
        out[i * n + j] = acc;
      }
    }
    return new Tensor(out, { shape: [m, n] });
  }
  backward(): (Tensor | null)[] { return notImplementedBackward(); }
}

// ────────── ReLU / Sigmoid / GELU / Softmax — pure TS for v0.3.0 ──────────

export class ReLUOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    const a = inputs[0] as Tensor;
    const out = new Array(a.numel);
    for (let i = 0; i < a.numel; i++) out[i] = a.data[i]! > 0 ? a.data[i]! : 0;
    return new Tensor(out, { shape: a.shape.slice() });
  }
  backward(): (Tensor | null)[] { return notImplementedBackward(); }
}

export class SigmoidOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    const a = inputs[0] as Tensor;
    const out = new Array(a.numel);
    for (let i = 0; i < a.numel; i++) out[i] = 1 / (1 + Math.exp(-a.data[i]!));
    return new Tensor(out, { shape: a.shape.slice() });
  }
  backward(): (Tensor | null)[] { return notImplementedBackward(); }
}

export class GELUOp extends Function {
  // GELU(x) = 0.5 * x * (1 + tanh(sqrt(2/π) * (x + 0.044715 * x³)))
  // tanh-approximation form matching PyTorch default + Python reference.
  static readonly SQRT_2_OVER_PI = Math.sqrt(2 / Math.PI);
  static readonly COEFF = 0.044715;

  forward(...inputs: unknown[]): Tensor {
    const a = inputs[0] as Tensor;
    const c = GELUOp.SQRT_2_OVER_PI;
    const k = GELUOp.COEFF;
    const out = new Array(a.numel);
    for (let i = 0; i < a.numel; i++) {
      const x = a.data[i]!;
      out[i] = 0.5 * x * (1 + Math.tanh(c * (x + k * x * x * x)));
    }
    return new Tensor(out, { shape: a.shape.slice() });
  }
  backward(): (Tensor | null)[] { return notImplementedBackward(); }
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
    return new Tensor(out, { shape: a.shape.slice() });
  }
  backward(): (Tensor | null)[] { return notImplementedBackward(); }
}

// ────────── Reductions: Sum / Mean (reduce-all) ──────────

export class SumOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    const a = inputs[0] as Tensor;
    if (a.numel >= DISPATCH_THRESHOLD) {
      const envelope = reduceAllEnvelope("ReduceSum", a);
      const out = runEnvelope(envelope, 1);
      return new Tensor(Array.from(out), { shape: [1] });
    }
    let sum = 0;
    for (let i = 0; i < a.numel; i++) sum += a.data[i]!;
    return new Tensor([sum], { shape: [1] });
  }
  backward(): (Tensor | null)[] { return notImplementedBackward(); }
}

export class MeanOp extends Function {
  forward(...inputs: unknown[]): Tensor {
    const a = inputs[0] as Tensor;
    if (a.numel >= DISPATCH_THRESHOLD) {
      const envelope = reduceAllEnvelope("ReduceMean", a);
      const out = runEnvelope(envelope, 1);
      return new Tensor(Array.from(out), { shape: [1] });
    }
    let sum = 0;
    for (let i = 0; i < a.numel; i++) sum += a.data[i]!;
    return new Tensor([sum / a.numel], { shape: [1] });
  }
  backward(): (Tensor | null)[] { return notImplementedBackward(); }
}
