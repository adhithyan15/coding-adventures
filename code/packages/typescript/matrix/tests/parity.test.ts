/**
 * # Parity tests — pure-TS CpuMatrixBackend vs napi-backed CpuMatrixBackend
 *
 * **MX08 Phase 2.**  Feeds the same `Matrix` inputs through both
 * backends and asserts numerical equivalence within f32 tolerance.
 *
 * Why f32 tolerance?  See `src/backends/cpu-rust-napi.ts`
 * §"dtype = f32 (with the precision caveat)".  Matrix stores JS
 * numbers (f64); `matrix-cpu` only supports F32 dtype today; the
 * napi adapter quantises at the boundary.  Future MX10 closes that
 * gap by adding F64 to matrix-cpu and refactoring `Matrix` to flat
 * `Float64Array` storage.  For MX08's scope, ~7-decimal-digit
 * agreement is the contract.
 *
 * Coverage: every method on the `MatrixBackend` interface
 * (add, subtract, scale, transpose, dot), plus a couple of larger
 * matrices to catch any shape-dependent bugs that 2×2 examples
 * would miss.
 *
 * These tests skip themselves on browsers / non-Node environments
 * by checking `typeof process !== "undefined"` — the napi adapter
 * `require()`s a `.node` binary which only exists in Node.
 */

import { CpuMatrixBackend as PureTsBackend } from "../src/backends/cpu-pure-ts";
import { Matrix } from "../src/matrix";

// The napi-backed backend lives behind the `node` conditional and
// requires the addon to be built.  The test file uses a try/catch
// around the import so a fresh checkout (no .node artifact) reports
// the missing prerequisite cleanly rather than erroring at collection.
let NapiBackendCtor: (new () => InstanceType<typeof PureTsBackend>) | null = null;
let napiLoadError: string | null = null;
try {
  // eslint-disable-next-line @typescript-eslint/no-var-requires
  const m = require("../src/backends/cpu-rust-napi") as {
    CpuMatrixBackend: new () => InstanceType<typeof PureTsBackend>;
  };
  NapiBackendCtor = m.CpuMatrixBackend;
} catch (e) {
  napiLoadError = (e as Error).message;
}

// f32 has ~7 significant digits of precision; matmul accumulates
// rounding error across the inner dimension, so we use a combined
// absolute + relative tolerance: `|diff| <= absTol + relTol * |expected|`.
// 1e-5 absolute + 1e-5 relative passes every test up to 16x16 matmul
// with values in [-50, 50] (worst-case product magnitudes around 1e4,
// giving combined tolerances around 0.1 in extreme cells).
const ABS_TOL = 1e-5;
const REL_TOL = 1e-5;

function expectMatricesClose(actual: Matrix, expected: Matrix, tag: string): void {
  expect(actual.rows).toBe(expected.rows);
  expect(actual.cols).toBe(expected.cols);
  for (let i = 0; i < expected.rows; i++) {
    for (let j = 0; j < expected.cols; j++) {
      const a = actual.data[i]![j]!;
      const e = expected.data[i]![j]!;
      const allowed = ABS_TOL + REL_TOL * Math.abs(e);
      if (Math.abs(a - e) > allowed) {
        throw new Error(
          `${tag}: mismatch at [${i},${j}] — pure-TS=${e}, napi=${a}, ` +
            `diff=${Math.abs(a - e)}, allowed=${allowed}`,
        );
      }
    }
  }
}

// Skip the whole suite on environments where the addon isn't
// available (e.g. browser test harness, fresh checkout pre-Phase 3
// build).
if (NapiBackendCtor === null) {
  // eslint-disable-next-line jest/no-disabled-tests
  describe.skip(
    `MX08 Phase 2 — backend parity (SKIPPED: napi addon unavailable — ${
      napiLoadError ?? "unknown reason"
    })`,
    () => {
      test("placeholder", () => {
        /* no-op */
      });
    },
  );
} else {
  const NapiBackend = NapiBackendCtor;

  describe("MX08 Phase 2 — backend parity (pure-TS vs napi)", () => {
    const pure = new PureTsBackend();
    const napi = new NapiBackend();

  describe("element-wise add", () => {
    test("2x3 floats", () => {
      const a = new Matrix([
        [1, 2, 3],
        [4, 5, 6],
      ]);
      const b = new Matrix([
        [10, 20, 30],
        [40, 50, 60],
      ]);
      expectMatricesClose(napi.add(a, b), pure.add(a, b), "add 2x3");
    });

    test("8x8 random floats", () => {
      const rand = () => Math.random() * 100 - 50;
      const a = new Matrix(
        Array.from({ length: 8 }, () => Array.from({ length: 8 }, rand)),
      );
      const b = new Matrix(
        Array.from({ length: 8 }, () => Array.from({ length: 8 }, rand)),
      );
      expectMatricesClose(napi.add(a, b), pure.add(a, b), "add 8x8");
    });
  });

  describe("element-wise subtract", () => {
    test("2x3 floats", () => {
      const a = new Matrix([
        [10, 20, 30],
        [40, 50, 60],
      ]);
      const b = new Matrix([
        [1, 2, 3],
        [4, 5, 6],
      ]);
      expectMatricesClose(napi.subtract(a, b), pure.subtract(a, b), "subtract 2x3");
    });
  });

  describe("scale", () => {
    test("3x2 scaled by 2.5", () => {
      const m = new Matrix([
        [1, 2],
        [3, 4],
        [5, 6],
      ]);
      expectMatricesClose(napi.scale(m, 2.5), pure.scale(m, 2.5), "scale by 2.5");
    });

    test("scale by negative", () => {
      const m = new Matrix([
        [1, -1],
        [-2, 2],
      ]);
      expectMatricesClose(napi.scale(m, -3), pure.scale(m, -3), "scale by -3");
    });
  });

  describe("transpose", () => {
    test("square 2x2", () => {
      const m = new Matrix([
        [1, 2],
        [3, 4],
      ]);
      expectMatricesClose(napi.transpose(m), pure.transpose(m), "transpose square");
    });

    test("non-square 2x3 -> 3x2", () => {
      const m = new Matrix([
        [1, 2, 3],
        [4, 5, 6],
      ]);
      const out = napi.transpose(m);
      expect(out.rows).toBe(3);
      expect(out.cols).toBe(2);
      expectMatricesClose(out, pure.transpose(m), "transpose 2x3");
    });
  });

  describe("dot (MatMul)", () => {
    test("2x2 × 2x2 textbook result", () => {
      const a = new Matrix([
        [1, 2],
        [3, 4],
      ]);
      const b = new Matrix([
        [5, 6],
        [7, 8],
      ]);
      // [[1,2],[3,4]] × [[5,6],[7,8]] = [[19,22],[43,50]]
      const expected = new Matrix([
        [19, 22],
        [43, 50],
      ]);
      expectMatricesClose(napi.dot(a, b), expected, "dot vs literal");
      // and matches pure-TS too:
      expectMatricesClose(napi.dot(a, b), pure.dot(a, b), "dot parity");
    });

    test("non-square 2x3 × 3x4 → 2x4", () => {
      const a = new Matrix([
        [1, 2, 3],
        [4, 5, 6],
      ]);
      const b = new Matrix([
        [1, 0, 0, 1],
        [0, 1, 0, 1],
        [0, 0, 1, 1],
      ]);
      const out = napi.dot(a, b);
      expect(out.rows).toBe(2);
      expect(out.cols).toBe(4);
      expectMatricesClose(out, pure.dot(a, b), "dot non-square");
    });

    test("16x16 random matmul", () => {
      const rand = () => Math.random() * 10 - 5;
      const a = new Matrix(
        Array.from({ length: 16 }, () => Array.from({ length: 16 }, rand)),
      );
      const b = new Matrix(
        Array.from({ length: 16 }, () => Array.from({ length: 16 }, rand)),
      );
      expectMatricesClose(napi.dot(a, b), pure.dot(a, b), "dot 16x16");
    });
  });

  describe("backend identity", () => {
      test("name = 'cpu' on both backends (downstream consumers test this)", () => {
        expect(pure.name).toBe("cpu");
        expect(napi.name).toBe("cpu");
      });
    });
  });
}
