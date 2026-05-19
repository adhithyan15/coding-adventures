/**
 * # `cpu-pure-ts` — the original pure-TypeScript `CpuMatrixBackend`
 *
 * **MX08 Phase 1.**  Moved verbatim from `src/matrix.ts:51-73`.  Zero
 * behaviour change.  This sibling module exists so MX08 Phase 2 can
 * land a Node-only sibling (`cpu-rust-napi.ts`) that delegates to
 * the `@coding-adventures/matrix-rust-napi` addon, while the browser
 * keeps the pure-TS implementation via the conditional-exports
 * pattern in `package.json`.
 *
 * The class implementation here is **identical** to what previously
 * lived inline in `matrix.ts`:
 *
 * * `name = "cpu"` for backward-compatible identification (consumer
 *   tests assert on this string).
 * * Each method delegates straight to the corresponding `Matrix`
 *   instance method (`left.add(right)`, etc.) — no fusion, no
 *   buffer marshalling, no GPU lift.  Pure triple-loop arithmetic
 *   on the `number[][]` storage that `Matrix` uses today.
 *
 * Re-exported from `matrix.ts` so existing imports
 * (`import { CpuMatrixBackend } from "@coding-adventures/matrix"`)
 * continue to resolve without source change.
 */

import type { Matrix, MatrixBackend } from "../matrix";

/**
 * Pure-TypeScript `CpuMatrixBackend`.  Identical to the
 * implementation that previously lived inline in `matrix.ts:51-73`;
 * the move is purely organisational so MX08 Phase 2 can introduce a
 * Node-only sibling alongside.
 */
export class CpuMatrixBackend implements MatrixBackend {
  readonly name = "cpu";

  add(left: Matrix, right: Matrix): Matrix {
    return left.add(right);
  }

  subtract(left: Matrix, right: Matrix): Matrix {
    return left.subtract(right);
  }

  scale(matrix: Matrix, scalar: number): Matrix {
    return matrix.scale(scalar);
  }

  transpose(matrix: Matrix): Matrix {
    return matrix.transpose();
  }

  dot(left: Matrix, right: Matrix): Matrix {
    return left.dot(right);
  }
}
