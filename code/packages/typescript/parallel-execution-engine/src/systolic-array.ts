/**
 * SystolicArray -- dataflow execution for matrix multiplication (Google TPU style).
 *
 * === What is a Systolic Array? ===
 *
 * The word "systolic" comes from the Greek "systole" (contraction), like a
 * heartbeat. In a systolic array, data pulses through a grid of processing
 * elements on each clock cycle, just like blood pulses through the body with
 * each heartbeat.
 *
 * A systolic array is radically different from GPU execution:
 *
 *     GPU (SIMT/SIMD):                   TPU (Systolic):
 *     +--------------------------+       +--------------------------+
 *     | Has instructions         |       | NO instructions          |
 *     | Has program counter      |       | NO program counter       |
 *     | Has branches             |       | NO branches              |
 *     | Complex control logic    |       | Dead-simple PEs          |
 *     | General-purpose          |       | Matrix multiply ONLY     |
 *     +--------------------------+       +--------------------------+
 *
 * Each PE in the array does exactly ONE thing on each clock cycle:
 *
 *     accumulator += input_from_left * local_weight
 *
 * Then it passes the input to the right neighbor and the accumulator down.
 * That's it. No instruction fetch, no decode, no branch prediction. Just
 * multiply, accumulate, and pass.
 *
 * === Why TPUs Use Systolic Arrays ===
 *
 * Neural network inference and training are dominated by matrix multiplication
 * (the GEMM operation). A systolic array is the most efficient hardware for
 * matrix multiply because:
 *
 *     1. No instruction overhead (no fetch, decode, branch)
 *     2. Maximum data reuse (each value is used N times as it flows through)
 *     3. Nearest-neighbor communication only (each PE talks to adjacent PEs)
 *     4. Regular, predictable data movement (no cache misses)
 *     5. Simple PE design -> high clock frequency, low power
 *
 * Google's TPU v1 has a 256x256 systolic array that performs 65,536 MAC
 * operations per clock cycle. At 700 MHz, that's ~46 TOPS (tera-ops/second).
 */

import {
  type FloatFormat,
  type FloatBits,
  FP32,
  floatToBits,
  bitsToFloat,
  fpFma,
} from "@coding-adventures/fp-arithmetic";

import {
  type DataflowInfo,
  type EngineTrace,
  ExecutionModel,
} from "./protocols.js";

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/**
 * Configuration for a systolic array engine.
 *
 * Real-world reference values:
 *
 *     Hardware    | Rows | Cols | Format | Accumulator
 *     ------------+------+------+--------+------------
 *     TPU v1      | 256  | 256  | INT8   | INT32
 *     TPU v2/v3   | 128  | 128  | BF16   | FP32
 *     TPU v4      | 128  | 128  | BF16   | FP32
 *     Our default | 4    | 4    | FP32   | FP32
 */
export interface SystolicConfig {
  readonly rows: number;
  readonly cols: number;
  readonly floatFormat: FloatFormat;
  readonly accumulatorFormat: FloatFormat;
}

/**
 * Create a SystolicConfig with sensible defaults.
 */
export function makeSystolicConfig(
  partial: Partial<SystolicConfig> = {},
): SystolicConfig {
  return {
    rows: 4,
    cols: 4,
    floatFormat: FP32,
    accumulatorFormat: FP32,
    ...partial,
  };
}

// ---------------------------------------------------------------------------
// SystolicPE -- one processing element in the grid
// ---------------------------------------------------------------------------

/**
 * One processing element in the systolic array.
 *
 * Each PE is extremely simple -- it's just a multiply-accumulate unit
 * with two data ports:
 *
 *     Input from left --> [  weight  ] --> Output to right
 *                         [  x + acc ]
 *                              |
 *                       Partial sum flows down
 *
 * On each clock cycle, a PE does:
 *     1. If there's an input: accumulator += input * weight
 *     2. Pass the input to the right neighbor
 *     3. (Partial sums flow down at the end of computation)
 */
export class SystolicPE {
  readonly row: number;
  readonly col: number;
  weight: FloatBits;
  accumulator: FloatBits;
  inputBuffer: FloatBits | null;

  constructor(
    row: number,
    col: number,
    weight: FloatBits,
    accumulator: FloatBits,
    inputBuffer: FloatBits | null = null,
  ) {
    this.row = row;
    this.col = col;
    this.weight = weight;
    this.accumulator = accumulator;
    this.inputBuffer = inputBuffer;
  }

  /**
   * Perform one MAC cycle.
   *
   * If there's an input waiting in the buffer:
   *     accumulator += input_buffer * weight
   * Returns the input (to be passed to the right neighbor), or null.
   */
  compute(): FloatBits | null {
    if (this.inputBuffer === null) {
      return null;
    }

    const inputVal = this.inputBuffer;
    this.inputBuffer = null;

    // MAC: accumulator = input * weight + accumulator
    // Using fpFma for fused multiply-add (more accurate than mul+add)
    this.accumulator = fpFma(inputVal, this.weight, this.accumulator);

    return inputVal; // Pass to right neighbor
  }
}

// ---------------------------------------------------------------------------
// SystolicArray -- the dataflow execution engine
// ---------------------------------------------------------------------------

/**
 * Systolic dataflow execution engine (Google TPU style).
 *
 * An NxN grid of processing elements. Data flows through the array --
 * activations left-to-right, partial sums accumulate in each PE.
 * No instruction stream. Just data in, results out.
 *
 * === Data Flow Pattern ===
 *
 *     Inputs feed from the left edge:
 *
 *     a[0] --> PE(0,0) --> PE(0,1) --> PE(0,2) --> PE(0,3)
 *     a[1] --> PE(1,0) --> PE(1,1) --> PE(1,2) --> PE(1,3)
 *     a[2] --> PE(2,0) --> PE(2,1) --> PE(2,2) --> PE(2,3)
 *     a[3] --> PE(3,0) --> PE(3,1) --> PE(3,2) --> PE(3,3)
 *
 *     Each PE accumulates: acc += input * weight
 *     After all inputs flow through, drain accumulators as the result.
 */
export class SystolicArray {
  private readonly _config: SystolicConfig;
  private _cycle: number = 0;
  private _halted: boolean = false;
  private _grid: SystolicPE[][];
  private _inputQueues: FloatBits[][];
  private _totalInputsFed: number = 0;

  constructor(config: SystolicConfig) {
    this._config = config;

    // Create the NxN grid of PEs
    this._grid = Array.from({ length: config.rows }, (_, r) =>
      Array.from(
        { length: config.cols },
        (_, c) =>
          new SystolicPE(
            r,
            c,
            floatToBits(0.0, config.floatFormat),
            floatToBits(0.0, config.accumulatorFormat),
          ),
      ),
    );

    // Input queues: one per row
    this._inputQueues = Array.from({ length: config.rows }, () => []);
  }

  // --- Properties ---

  get name(): string {
    return "SystolicArray";
  }

  get width(): number {
    return this._config.rows * this._config.cols;
  }

  get executionModel(): ExecutionModel {
    return ExecutionModel.SYSTOLIC;
  }

  get halted(): boolean {
    return this._halted;
  }

  get config(): SystolicConfig {
    return this._config;
  }

  get grid(): readonly (readonly SystolicPE[])[] {
    return this._grid;
  }

  // --- Weight loading ---

  /**
   * Pre-load the weight matrix into the PE array.
   * weights[row][col] goes to PE(row, col).
   */
  loadWeights(weights: number[][]): void {
    for (let r = 0; r < Math.min(weights.length, this._config.rows); r++) {
      for (
        let c = 0;
        c < Math.min(weights[r].length, this._config.cols);
        c++
      ) {
        this._grid[r][c].weight = floatToBits(
          weights[r][c],
          this._config.floatFormat,
        );
      }
    }
  }

  // --- Input feeding ---

  /**
   * Feed one activation value into the left edge of the specified row.
   */
  feedInput(row: number, value: number): void {
    if (row < 0 || row >= this._config.rows) {
      throw new RangeError(
        `Row ${row} out of range [0, ${this._config.rows})`,
      );
    }
    this._inputQueues[row].push(
      floatToBits(value, this._config.floatFormat),
    );
    this._totalInputsFed += 1;
  }

  /**
   * Feed a full column vector to all rows.
   */
  feedInputVector(values: number[]): void {
    for (let rowIdx = 0; rowIdx < values.length; rowIdx++) {
      const fb = floatToBits(values[rowIdx], this._config.floatFormat);
      this._inputQueues[rowIdx].push(fb);
      this._totalInputsFed += 1;
    }
  }

  // --- Execution ---

  step(clockEdge: { cycle: number }): EngineTrace {
    this._cycle += 1;

    let activeCount = 0;
    const peStates: string[][] = [];

    // Phase 1: Move data rightward through the array.
    // Process from right to left to avoid data collision.
    for (let r = 0; r < this._config.rows; r++) {
      for (let c = this._config.cols - 1; c >= 0; c--) {
        const pe = this._grid[r][c];
        const output = pe.compute();

        if (output !== null) {
          activeCount += 1;
          // Pass input to right neighbor (if exists)
          if (c + 1 < this._config.cols) {
            this._grid[r][c + 1].inputBuffer = output;
          }
        }
      }

      // Build state strings (left to right for display)
      const rowStates: string[] = [];
      for (let c = 0; c < this._config.cols; c++) {
        const pe = this._grid[r][c];
        const accVal = bitsToFloat(pe.accumulator);
        const hasInput = pe.inputBuffer !== null;
        let state = `acc=${accVal.toPrecision(4)}`;
        if (hasInput) {
          const inVal = bitsToFloat(pe.inputBuffer!);
          state += `, in=${inVal.toPrecision(4)}`;
        }
        rowStates.push(state);
      }
      peStates.push(rowStates);
    }

    // Phase 2: Feed new inputs from queues into column 0
    for (let r = 0; r < this._config.rows; r++) {
      if (this._inputQueues[r].length > 0) {
        const val = this._inputQueues[r].shift()!;
        this._grid[r][0].inputBuffer = val;
      }
    }

    // Check if computation is complete
    const total = this._config.rows * this._config.cols;
    const anyInputRemaining = this._inputQueues.some((q) => q.length > 0);
    let anyInputInFlight = false;
    for (let r = 0; r < this._config.rows; r++) {
      for (let c = 0; c < this._config.cols; c++) {
        if (this._grid[r][c].inputBuffer !== null) {
          anyInputInFlight = true;
          break;
        }
      }
      if (anyInputInFlight) break;
    }

    if (!anyInputRemaining && !anyInputInFlight) {
      this._halted = true;
    }

    const utilization = total > 0 ? activeCount / total : 0.0;

    // Build unit traces and active mask
    const unitTraces: Record<number, string> = {};
    const activeMask: boolean[] = [];
    for (let r = 0; r < this._config.rows; r++) {
      for (let c = 0; c < this._config.cols; c++) {
        const flatId = r * this._config.cols + c;
        unitTraces[flatId] = peStates[r][c];
        activeMask.push(
          this._grid[r][c].inputBuffer !== null || flatId < activeCount,
        );
      }
    }

    return {
      cycle: this._cycle,
      engineName: this.name,
      executionModel: this.executionModel,
      description: `Systolic step -- ${activeCount}/${total} PEs active`,
      unitTraces,
      activeMask,
      activeCount,
      totalCount: total,
      utilization,
      dataflowInfo: { peStates, dataPositions: {} },
    };
  }

  /**
   * Convenience: run a complete matrix multiplication C = A x W.
   *
   * For C = A x W where A is MxK and W is KxN:
   *     C[i][j] = sum_k( A[i][k] * W[k][j] )
   */
  runMatmul(activations: number[][], weights: number[][]): number[][] {
    const numOutputRows = activations.length;
    const innerDim = activations.length > 0 ? activations[0].length : 0;
    const numOutputCols = weights.length > 0 ? weights[0].length : 0;

    // Load weights
    this.reset();
    this.loadWeights(weights);

    const result: number[][] = [];

    // Compute one output row at a time
    for (let i = 0; i < numOutputRows; i++) {
      // Reset accumulators (but keep weights)
      const zeroAcc = floatToBits(0.0, this._config.accumulatorFormat);
      for (let r = 0; r < this._config.rows; r++) {
        for (let c = 0; c < this._config.cols; c++) {
          this._grid[r][c].accumulator = zeroAcc;
          this._grid[r][c].inputBuffer = null;
        }
      }
      this._inputQueues = Array.from({ length: this._config.rows }, () => []);
      this._halted = false;

      // Feed A[i][k] into row k with staggered timing
      const feedSchedule: Map<number, [number, number][]> = new Map();
      for (let k = 0; k < innerDim; k++) {
        const cycle = k;
        if (!feedSchedule.has(cycle)) {
          feedSchedule.set(cycle, []);
        }
        feedSchedule.get(cycle)!.push([k, activations[i][k]]);
      }

      // Run until all data has flowed through
      const totalSteps = innerDim + this._config.cols + 1;
      for (let stepNum = 0; stepNum < totalSteps; stepNum++) {
        const entries = feedSchedule.get(stepNum);
        if (entries) {
          for (const [row, val] of entries) {
            this.feedInput(row, val);
          }
        }
        this.step({ cycle: stepNum + 1 });
      }

      // Drain: sum accumulators vertically for each column j
      const rowResult: number[] = [];
      for (let j = 0; j < numOutputCols; j++) {
        let colSum = 0.0;
        for (let k = 0; k < Math.min(innerDim, this._config.rows); k++) {
          colSum += bitsToFloat(this._grid[k][j].accumulator);
        }
        rowResult.push(colSum);
      }
      result.push(rowResult);
    }

    return result;
  }

  /**
   * Read the accumulated results from all PEs.
   */
  drainOutputs(): number[][] {
    const result: number[][] = [];
    for (let r = 0; r < this._config.rows; r++) {
      const row: number[] = [];
      for (let c = 0; c < this._config.cols; c++) {
        row.push(bitsToFloat(this._grid[r][c].accumulator));
      }
      result.push(row);
    }
    return result;
  }

  reset(): void {
    const zeroAcc = floatToBits(0.0, this._config.accumulatorFormat);
    for (let r = 0; r < this._config.rows; r++) {
      for (let c = 0; c < this._config.cols; c++) {
        this._grid[r][c].accumulator = zeroAcc;
        this._grid[r][c].inputBuffer = null;
      }
    }
    this._inputQueues = Array.from({ length: this._config.rows }, () => []);
    this._cycle = 0;
    this._halted = false;
    this._totalInputsFed = 0;
  }

  toString(): string {
    return (
      `SystolicArray(${this._config.rows}x${this._config.cols}, ` +
      `cycle=${this._cycle}, halted=${this._halted})`
    );
  }
}
