/**
 * Elementwise binary operators — mirrors `array_runtime::ops::BinOp`
 * (`code/packages/rust/array-runtime/src/ops.rs`) plus `Pow`, which the
 * SIR22 spec's `ElementwiseOpKind` includes in its "original cut" but
 * `array-runtime`'s own `BinOp` does not yet — this runtime implements the
 * full SIR-level operator set, not just the subset Rust's array-runtime
 * crate happens to have ported to its GPU-dispatch pipeline so far.
 *
 * Comparisons follow the same APL-style boolean convention Rust's `BinOp`
 * uses: `1` for true, `0` for false (never a native `boolean`), since the
 * result must stay a plain array element like every other value here.
 */
import { ndarray, isScalar, type NDArray } from "./ndarray.js";

export type ElementwiseOpKind =
  | "Add"
  | "Sub"
  | "Mul"
  | "Div"
  | "Pow"
  | "Max"
  | "Min"
  | "Eq"
  | "Ne"
  | "Lt"
  | "Le"
  | "Ge"
  | "Gt";

function applyOp(op: ElementwiseOpKind, a: number, b: number): number {
  const b2f = (cond: boolean): number => (cond ? 1 : 0);
  switch (op) {
    case "Add":
      return a + b;
    case "Sub":
      return a - b;
    case "Mul":
      return a * b;
    case "Div":
      return a / b;
    case "Pow":
      return Math.pow(a, b);
    case "Max":
      return Math.max(a, b);
    case "Min":
      return Math.min(a, b);
    case "Eq":
      return b2f(a === b);
    case "Ne":
      return b2f(a !== b);
    case "Lt":
      return b2f(a < b);
    case "Le":
      return b2f(a <= b);
    case "Ge":
      return b2f(a >= b);
    case "Gt":
      return b2f(a > b);
    default:
      // Same "crosses a JS runtime boundary TypeScript can't enforce"
      // reasoning as `resolvePositions` in `indexing.ts`: an unrecognised
      // `op` must fail loudly here, not fall through to `undefined` —
      // which `Float64Array.from`/direct assignment would otherwise
      // silently coerce to `NaN`, corrupting data instead of erroring.
      throw new Error(`applyOp: unrecognised ElementwiseOpKind ${JSON.stringify(op)}`);
  }
}

function sameShape(a: readonly number[], b: readonly number[]): boolean {
  return a.length === b.length && a.every((d, i) => d === b[i]);
}

/**
 * Elementwise binary op with scalar broadcasting — mirrors
 * `array_runtime::ops::elementwise` exactly, including its branch order and
 * "result takes the non-scalar operand's shape" rule. Either operand may be
 * a scalar; otherwise the shapes must match exactly (full NumPy/MATLAB
 * broadcasting is out of scope here, same as the Rust reference).
 */
export function elementwise(op: ElementwiseOpKind, a: NDArray, b: NDArray): NDArray {
  const { data: ad } = a;
  const { data: bd } = b;
  let data: Float64Array;
  if (isScalar(a)) {
    data = Float64Array.from(bd, (y) => applyOp(op, ad[0], y));
  } else if (isScalar(b)) {
    data = Float64Array.from(ad, (x) => applyOp(op, x, bd[0]));
  } else {
    if (!sameShape(a.shape, b.shape)) {
      throw new Error(
        `elementwise: non-conformable arrays: ${JSON.stringify(a.shape)} vs ${JSON.stringify(b.shape)}`,
      );
    }
    data = new Float64Array(ad.length);
    for (let i = 0; i < data.length; i++) {
      data[i] = applyOp(op, ad[i], bd[i]);
    }
  }
  // Result takes the non-scalar operand's shape (or the scalar's if both are).
  const shape = isScalar(a) ? b.shape : a.shape;
  return ndarray(shape, data);
}
