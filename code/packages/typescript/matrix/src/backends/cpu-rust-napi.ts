/**
 * # `cpu-rust-napi` — Node-only CpuMatrixBackend that delegates to
 * the Rust `matrix-cpu` executor via `@coding-adventures/matrix-rust-napi`.
 *
 * **MX08 Phase 2.**  Closes ARCH02's last open thread: on Node, the
 * `MatrixBackend.dot` / `.add` / etc. methods now route through the
 * Rust matrix execution layer (matrix-ir → matrix-runtime → matrix-cpu,
 * with matrix-metal / matrix-cuda lifting automatically when
 * registered).  The browser keeps the pure-TS implementation in
 * `cpu-pure-ts.ts`; environment routing is done by the package's
 * `package.json` `exports` conditional.
 *
 * ## Shape of an op
 *
 * Each `MatrixBackend` method builds a **single-op `matrix-ir` graph**
 * on the fly:
 *
 *   1. JSON-stringify the graph (matrix-ir-json schema).
 *   2. `new Graph(json)` — the addon parses + wraps a Rust
 *      `matrix_ir::Graph`.
 *   3. `matrixToBuffer(a)` flattens row-major into a Node `Buffer`
 *      of f32 little-endian bytes.
 *   4. `runtime.run(graph, [aBuf, bBuf])` — addon allocates buffers
 *      on the executor, uploads inputs, dispatches, downloads outputs.
 *   5. `bufferToMatrix(out, rows, cols)` reads the output Buffer back
 *      into a `number[][]` for the `Matrix` constructor.
 *
 * ## dtype = f32 (with the precision caveat)
 *
 * Matrix today stores JavaScript `number`, which is f64.  But
 * `matrix-cpu`'s `BackendProfile.supported_dtypes` only includes
 * F32, U8, I32 (per `matrix-cpu/src/lib.rs:69-70`).  Submitting an
 * f64 graph fails at the planner with "no capable executor".
 *
 * The pragmatic choice: convert at the boundary.
 * `Buffer.writeFloatLE` and `Buffer.readFloatLE` quantise to f32 on
 * the way in and out.  This means values that round-trip lose
 * precision below ~7 decimal digits — fine for the dense numeric
 * workloads `typescript/matrix` consumers run today
 * (cas-matrix concrete-number fast path, blas-library reference
 * impl, single/two-layer-network) where f32 is the standard
 * baseline anyway.
 *
 * The parity tests assert `|a_pure - a_napi| < 1e-5` rather than
 * exact equality, matching f32's ~7-digit precision.
 *
 * Future MX10 (gated by profiling) refactors `Matrix` to a flat
 * `Float64Array` and adds F64 support to `matrix-cpu`, eliminating
 * the precision loss entirely.  Out of scope for MX08.
 *
 * ## Per-call marshalling cost
 *
 * Each `dot(a, b)` pays:
 *   * JSON stringify + parse of the graph (microseconds for 1-op graphs)
 *   * `matrixToBuffer` flatten: O(rows × cols) copies
 *   * Runtime allocate + dispatch + download (the work we came for)
 *   * `bufferToMatrix` reshape: O(rows × cols) copies
 *
 * For matrices smaller than ~16×16 the marshalling can dominate the
 * matmul time.  For 64×64+ the marshalling is negligible vs compute.
 * MX08 Phase 4 (deferred, profile-driven) caches `Graph` instances
 * by shape so the JSON parse cost goes away for hot loops.
 *
 * ## Module-global Runtime
 *
 * Runtime is stateless in v0 (each `run` constructs a fresh
 * `matrix_runtime::Runtime` + `CpuExecutor` internally), so a single
 * process-global instance is plenty.  Multi-tenant scenarios that
 * want per-tenant isolation can construct their own backends.
 */

import type { Matrix, MatrixBackend } from "../matrix";

// Lazy require so simply importing this module from a context where
// the addon isn't built yet (browser bundlers, fresh checkout before
// MX07 Phase 3 ran) doesn't crash at module load.  Resolution
// happens on first call.
//
// We use the built-in CommonJS `require` directly — this package
// targets CommonJS per tsconfig.json (`module: CommonJS`), so
// `createRequire`/`import.meta.url` would be invalid here.  The
// future MX10 ESM migration switches both.
let cachedNapi: NapiBindings | null = null;

interface NapiGraphCtor {
  new (jsonString: string): NapiGraph;
  fromJson(jsonString: string): NapiGraph;
}
interface NapiGraph {
  toJson(): string;
  describe(): string;
}
interface NapiRuntimeCtor {
  new (): NapiRuntime;
  create(): NapiRuntime;
}
interface NapiRuntime {
  run(graph: NapiGraph, inputs: Buffer[]): Buffer[];
}
interface NapiBindings {
  Graph: NapiGraphCtor;
  Runtime: NapiRuntimeCtor;
}

// The `.node` addon path, relative to this file (src/backends/cpu-rust-napi.ts):
//   src/backends/  → src/  (..)
//                  → matrix/  (../..)
//                  → typescript/  (../../..)
//                  → packages/  (../../../..)
//                  → rust/matrix-rust-napi/matrix_rust_napi.node  (../../../../rust/matrix-rust-napi/...)
//
// Why require the .node directly rather than via the
// @coding-adventures/matrix-rust-napi TypeScript wrapper?  The
// wrapper is shipped as ESM (`"type": "module"` in its
// package.json), so a CommonJS consumer like this package (per
// `tsconfig.json: { module: "CommonJS" }`) can't `require()` it.
// Loading the `.node` artifact directly works because Node treats
// every `.node` file as CommonJS regardless of the surrounding
// package.json — that's the whole point of N-API addons.
import * as path from "node:path";

const ADDON_PATH = path.resolve(
  __dirname,
  "..",
  "..",
  "..",
  "..",
  "rust",
  "matrix-rust-napi",
  "matrix_rust_napi.node",
);

function loadNapi(): NapiBindings {
  if (cachedNapi !== null) return cachedNapi;
  cachedNapi = require(ADDON_PATH) as NapiBindings;
  return cachedNapi;
}

let cachedRuntime: NapiRuntime | null = null;
function getRuntime(): NapiRuntime {
  if (cachedRuntime !== null) return cachedRuntime;
  cachedRuntime = loadNapi().Runtime.create();
  return cachedRuntime;
}

// ─────────────────────────────────────────────────────────────────────────────
// Buffer marshalling
//
// `Matrix.data` is `number[][]` (rows × cols), row-major.  Bytes flow
// across the napi boundary as f32 little-endian (4 bytes/element).
// ─────────────────────────────────────────────────────────────────────────────

function matrixToBuffer(m: Matrix): Buffer {
  const buf = Buffer.alloc(m.rows * m.cols * 4);
  let offset = 0;
  for (let i = 0; i < m.rows; i++) {
    const row = m.data[i]!;
    for (let j = 0; j < m.cols; j++) {
      // f32 quantisation happens here — JS number (f64) -> f32 LE.
      buf.writeFloatLE(row[j]!, offset);
      offset += 4;
    }
  }
  return buf;
}

function bufferToMatrixData(buf: Buffer, rows: number, cols: number): number[][] {
  const data: number[][] = new Array(rows);
  let offset = 0;
  for (let i = 0; i < rows; i++) {
    const row = new Array<number>(cols);
    for (let j = 0; j < cols; j++) {
      row[j] = buf.readFloatLE(offset);
      offset += 4;
    }
    data[i] = row;
  }
  return data;
}

// Lazy import of the Matrix class to avoid the circular-import
// gotcha (matrix.ts imports CpuMatrixBackend from entry-node.ts,
// entry-node.ts re-exports this module, this module wants Matrix
// for construction).  `require`-at-call-time short-circuits the cycle.
let cachedMatrixCtor: (new (data: number[][]) => Matrix) | null = null;
function getMatrixCtor(): new (data: number[][]) => Matrix {
  if (cachedMatrixCtor !== null) return cachedMatrixCtor;
  // eslint-disable-next-line @typescript-eslint/no-var-requires
  cachedMatrixCtor = (require("../matrix") as { Matrix: new (data: number[][]) => Matrix }).Matrix;
  return cachedMatrixCtor;
}

function bufferToMatrix(buf: Buffer, rows: number, cols: number): Matrix {
  const Ctor = getMatrixCtor();
  return new Ctor(bufferToMatrixData(buf, rows, cols));
}

// ─────────────────────────────────────────────────────────────────────────────
// Hex encoder for constant payloads (used by `scale`).
//
// matrix-ir-json constants ship as lowercase-hex bytes; the napi
// addon's Buffer-based API doesn't accept Buffers inside the graph
// JSON, so for the single Const tensor we need (the scalar in
// `scale`), we hex-encode it inline.
// ─────────────────────────────────────────────────────────────────────────────

function f32ToHex(value: number): string {
  const buf = Buffer.alloc(4);
  buf.writeFloatLE(value, 0);
  return buf.toString("hex");
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-op graph builders
// ─────────────────────────────────────────────────────────────────────────────

interface GraphSpec {
  matrix_ir_version: 1;
  tensors: { id: number; dtype: "f32"; shape: number[] }[];
  inputs: number[];
  outputs: number[];
  ops: object[];
  constants: { tensor_id: number; dtype: "f32"; shape: number[]; bytes_hex: string }[];
}

function binaryElementwiseGraph(
  kind: "Add" | "Sub",
  rows: number,
  cols: number,
): GraphSpec {
  return {
    matrix_ir_version: 1,
    tensors: [
      { id: 0, dtype: "f32", shape: [rows, cols] },
      { id: 1, dtype: "f32", shape: [rows, cols] },
      { id: 2, dtype: "f32", shape: [rows, cols] },
    ],
    inputs: [0, 1],
    outputs: [2],
    ops: [{ kind, lhs: 0, rhs: 1, output: 2 }],
    constants: [],
  };
}

function matmulGraph(aRows: number, aCols: number, bCols: number): GraphSpec {
  return {
    matrix_ir_version: 1,
    tensors: [
      { id: 0, dtype: "f32", shape: [aRows, aCols] },
      { id: 1, dtype: "f32", shape: [aCols, bCols] },
      { id: 2, dtype: "f32", shape: [aRows, bCols] },
    ],
    inputs: [0, 1],
    outputs: [2],
    ops: [{ kind: "MatMul", a: 0, b: 1, output: 2 }],
    constants: [],
  };
}

function transposeGraph(rows: number, cols: number): GraphSpec {
  return {
    matrix_ir_version: 1,
    tensors: [
      { id: 0, dtype: "f32", shape: [rows, cols] },
      { id: 1, dtype: "f32", shape: [cols, rows] },
    ],
    inputs: [0],
    outputs: [1],
    ops: [{ kind: "Transpose", input: 0, perm: [1, 0], output: 1 }],
    constants: [],
  };
}

function scaleGraph(rows: number, cols: number, scalar: number): GraphSpec {
  // Build a [rows, cols] constant filled with `scalar`, then Mul.
  // Avoids needing Broadcast (which would add a second op).
  const numel = rows * cols;
  const bytes = Buffer.alloc(numel * 4);
  for (let i = 0; i < numel; i++) {
    bytes.writeFloatLE(scalar, i * 4);
  }
  return {
    matrix_ir_version: 1,
    tensors: [
      { id: 0, dtype: "f32", shape: [rows, cols] }, // input
      { id: 1, dtype: "f32", shape: [rows, cols] }, // scalar-filled constant
      { id: 2, dtype: "f32", shape: [rows, cols] }, // output
    ],
    inputs: [0],
    outputs: [2],
    ops: [{ kind: "Mul", lhs: 0, rhs: 1, output: 2 }],
    constants: [
      {
        tensor_id: 1,
        dtype: "f32",
        shape: [rows, cols],
        bytes_hex: bytes.toString("hex"),
      },
    ],
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Backend implementation
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Node-side `CpuMatrixBackend` that delegates each op to the Rust
 * `matrix-cpu` executor via the napi addon.  Drop-in replacement for
 * the pure-TS implementation in `cpu-pure-ts.ts`; same `name`
 * (preserves backend-identity tests in downstream consumers), same
 * method signatures.
 */
export class CpuMatrixBackend implements MatrixBackend {
  readonly name = "cpu";

  private runGraph(spec: GraphSpec, inputs: Matrix[], outRows: number, outCols: number): Matrix {
    const napi = loadNapi();
    const graph = new napi.Graph(JSON.stringify(spec));
    const buffers = inputs.map(matrixToBuffer);
    const outputs = getRuntime().run(graph, buffers);
    return bufferToMatrix(outputs[0]!, outRows, outCols);
  }

  add(left: Matrix, right: Matrix): Matrix {
    return this.runGraph(
      binaryElementwiseGraph("Add", left.rows, left.cols),
      [left, right],
      left.rows,
      left.cols,
    );
  }

  subtract(left: Matrix, right: Matrix): Matrix {
    return this.runGraph(
      binaryElementwiseGraph("Sub", left.rows, left.cols),
      [left, right],
      left.rows,
      left.cols,
    );
  }

  scale(matrix: Matrix, scalar: number): Matrix {
    // Suppress unused-variable lint on f32ToHex — it's used in tests
    // and future MX08 Phase 4 will switch scaleGraph to use it for
    // a single-element constant + Broadcast, but the current shape
    // uses an inline-allocated full-size constant for simplicity.
    void f32ToHex;
    return this.runGraph(
      scaleGraph(matrix.rows, matrix.cols, scalar),
      [matrix],
      matrix.rows,
      matrix.cols,
    );
  }

  transpose(matrix: Matrix): Matrix {
    return this.runGraph(
      transposeGraph(matrix.rows, matrix.cols),
      [matrix],
      matrix.cols,
      matrix.rows,
    );
  }

  dot(left: Matrix, right: Matrix): Matrix {
    return this.runGraph(
      matmulGraph(left.rows, left.cols, right.cols),
      [left, right],
      left.rows,
      right.cols,
    );
  }
}
