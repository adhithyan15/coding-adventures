/**
 * MatrixMultiplyUnit -- Google TPU MXU simulator.
 *
 * === What is an MXU? ===
 *
 * The Matrix Multiply Unit is the heart of Google's TPU (Tensor Processing
 * Unit). It's fundamentally different from GPU compute units -- there are NO
 * threads, NO warps, NO schedulers. Instead, it has:
 *
 * 1. **Systolic arrays** -- the main compute engine (from Layer 8)
 * 2. **Vector unit** -- for element-wise operations (activation functions)
 * 3. **Accumulators** -- for storing partial matrix results
 * 4. **Control sequencer** -- manages the tiling schedule
 *
 * === Why No Threads? ===
 *
 * Matrix multiplication is perfectly predictable. You know exactly which
 * values need to be multiplied together and in what order. There's no
 * branching, no data-dependent control flow, no need for a runtime scheduler.
 *
 * This predictability lets the compiler (XLA) generate a complete execution
 * plan at compile time -- which tiles to load, when to multiply, when to
 * drain results. The MXU hardware just follows this plan cycle by cycle.
 *
 *     GPU:  Complex hardware scheduler decides at runtime
 *     TPU:  Simple hardware follows compile-time plan
 *
 * === Architecture Diagram ===
 *
 *     MatrixMultiplyUnit (TPU v2-style)
 *     +---------------------------------------------------------------+
 *     |                                                               |
 *     |  Control Sequencer                                            |
 *     |  +----------------------------------------------------------+ |
 *     |  | Tile schedule: load A[0:128], matmul, load A[128:256]    | |
 *     |  +----------------------------------------------------------+ |
 *     |                                                               |
 *     |  +---------------------------------------------+              |
 *     |  | Systolic Array (128x128)                     |              |
 *     |  |   Weights pre-loaded into PEs                |              |
 *     |  |   Activations stream in from left            |              |
 *     |  |   Partial sums flow down to accumulators     |              |
 *     |  +---------------------------------------------+              |
 *     |                    |                                          |
 *     |                    v                                          |
 *     |  +---------------------------------------------+              |
 *     |  | Accumulators (128 x FP32)                    |              |
 *     |  +---------------------------------------------+              |
 *     |                    |                                          |
 *     |                    v                                          |
 *     |  +---------------------------------------------+              |
 *     |  | Vector Unit (128-wide)                       |              |
 *     |  | ReLU, sigmoid, add bias, normalize           |              |
 *     |  +---------------------------------------------+              |
 *     +---------------------------------------------------------------+
 */

import { BF16, FP32, type FloatFormat } from "@coding-adventures/fp-arithmetic";
import {
  SystolicArray,
  makeSystolicConfig,
} from "@coding-adventures/parallel-execution-engine";

import {
  Architecture,
  type ComputeUnitTrace,
  makeComputeUnitTrace,
  type WorkItem,
} from "./protocols.js";

// ---------------------------------------------------------------------------
// MXUConfig -- configuration for a TPU-style Matrix Multiply Unit
// ---------------------------------------------------------------------------

/**
 * Configuration for a TPU-style Matrix Multiply Unit.
 *
 * Real-world MXU configurations:
 *
 *     Parameter           | TPU v1       | TPU v2/v3    | TPU v4
 *     --------------------+--------------+--------------+----------
 *     Array size          | 256x256      | 128x128      | 128x128
 *     Input format        | INT8         | BF16         | BF16
 *     Accumulator format  | INT32        | FP32         | FP32
 *     Vector width        | 256          | 128          | 128
 *     HBM bandwidth       | 30 GB/s      | 900 GB/s     | 1200 GB/s
 */
export interface MXUConfig {
  readonly arrayRows: number;
  readonly arrayCols: number;
  readonly systolicFormat: FloatFormat;
  readonly accumulatorFormat: FloatFormat;
  readonly vectorWidth: number;
  readonly vectorFormat: FloatFormat;
  readonly accumulatorCount: number;
  readonly weightBufferSize: number;
  readonly activationBufferSize: number;
}

/** Create an MXUConfig with sensible defaults. */
export function makeMXUConfig(partial: Partial<MXUConfig> = {}): MXUConfig {
  return {
    arrayRows: 128,
    arrayCols: 128,
    systolicFormat: BF16,
    accumulatorFormat: FP32,
    vectorWidth: 128,
    vectorFormat: FP32,
    accumulatorCount: 128,
    weightBufferSize: 4194304,
    activationBufferSize: 2097152,
    ...partial,
  };
}

// ---------------------------------------------------------------------------
// MatrixMultiplyUnit -- the main MXU simulator
// ---------------------------------------------------------------------------

/**
 * Google TPU Matrix Multiply Unit simulator.
 *
 * Uses a systolic array from Layer 8 to perform matrix multiplication,
 * with tiling logic for matrices larger than the array, and a vector
 * unit for post-processing (activation functions, bias add).
 *
 * === Execution Model ===
 *
 * The MXU has no threads or schedulers. Instead, it processes **tiles**
 * of a larger matrix operation. The control sequencer manages:
 *
 * 1. Loading weight tiles into the systolic array
 * 2. Streaming activation tiles through the array
 * 3. Accumulating partial results
 * 4. Applying vector operations (activation functions)
 * 5. Storing output tiles
 */
export class MatrixMultiplyUnit {
  private _config: MXUConfig;
  private _cycle: number = 0;
  private _array: SystolicArray;
  private _accumulators: number[][] = [];
  private _result: number[][] = [];
  private _workItems: WorkItem[] = [];
  private _idle: boolean = true;
  private _currentResult: number[][] = [];

  constructor(config: MXUConfig) {
    this._config = config;
    this._array = new SystolicArray(
      makeSystolicConfig({
        rows: config.arrayRows,
        cols: config.arrayCols,
        floatFormat: FP32, // use FP32 internally for simulation
        accumulatorFormat: FP32,
      }),
    );
  }

  // --- Properties ---

  get name(): string {
    return "MXU";
  }

  get architecture(): Architecture {
    return Architecture.GOOGLE_MXU;
  }

  get idle(): boolean {
    return this._idle;
  }

  get config(): MXUConfig {
    return this._config;
  }

  get result(): readonly (readonly number[])[] {
    return this._currentResult;
  }

  get systolicArray(): SystolicArray {
    return this._array;
  }

  // --- Dispatch ---

  /**
   * Dispatch a matrix multiply operation.
   *
   * The WorkItem must provide inputData (activation matrix) and
   * weightData (weight matrix). The MXU will perform:
   *
   *     result = inputData x weightData
   */
  dispatch(work: WorkItem): void {
    this._workItems.push(work);
    this._idle = false;
  }

  // --- Execution ---

  /**
   * Advance one cycle of the MXU.
   *
   * If work is pending, performs the matmul using the systolic array.
   */
  step(clockEdge: { cycle: number }): ComputeUnitTrace {
    this._cycle += 1;

    if (this._idle || this._workItems.length === 0) {
      return this._makeIdleTrace();
    }

    // Process the first pending work item
    const work = this._workItems[0];

    if (work.inputData !== null && work.weightData !== null) {
      // Perform the full matmul using the systolic array's runMatmul
      this._currentResult = this._array.runMatmul(
        work.inputData as number[][],
        work.weightData as number[][],
      );
    } else {
      this._currentResult = [];
    }

    // Mark work as done
    this._workItems.shift();
    if (this._workItems.length === 0) {
      this._idle = true;
    }

    // Build trace
    const rows = this._currentResult.length;
    const cols =
      this._currentResult.length > 0 ? this._currentResult[0].length : 0;

    return makeComputeUnitTrace({
      cycle: this._cycle,
      unitName: this.name,
      architecture: this.architecture,
      schedulerAction: `matmul complete: ${rows}x${cols} result`,
      activeWarps: this._idle ? 0 : 1,
      totalWarps: 1,
      engineTraces: {},
      sharedMemoryUsed: 0,
      sharedMemoryTotal: this._config.weightBufferSize,
      registerFileUsed: this._config.accumulatorCount,
      registerFileTotal: this._config.accumulatorCount,
      occupancy: this._idle ? 0.0 : 1.0,
    });
  }

  /** Run until all work completes or maxCycles. */
  run(maxCycles: number = 100000): ComputeUnitTrace[] {
    const traces: ComputeUnitTrace[] = [];
    for (let cycleNum = 1; cycleNum <= maxCycles; cycleNum++) {
      const trace = this.step({ cycle: cycleNum });
      traces.push(trace);
      if (this.idle) {
        break;
      }
    }
    return traces;
  }

  /**
   * Convenience: run a complete matmul with optional activation.
   *
   * === Supported Activation Functions ===
   *
   *     none:    f(x) = x              (identity)
   *     relu:    f(x) = max(0, x)      (most popular)
   *     sigmoid: f(x) = 1/(1+e^-x)    (squashes to [0,1])
   *     tanh:    f(x) = tanh(x)        (squashes to [-1,1])
   */
  runMatmul(
    activations: number[][],
    weights: number[][],
    activationFn: string = "none",
  ): number[][] {
    // Use the systolic array to do the actual matmul
    let result = this._array.runMatmul(activations, weights);

    // Apply activation function via the vector unit
    if (activationFn !== "none") {
      result = applyActivation(result, activationFn);
    }

    this._currentResult = result;
    return result;
  }

  /** Reset all state. */
  reset(): void {
    this._array.reset();
    this._accumulators = [];
    this._currentResult = [];
    this._workItems = [];
    this._idle = true;
    this._cycle = 0;
  }

  // --- Private helpers ---

  private _makeIdleTrace(): ComputeUnitTrace {
    return makeComputeUnitTrace({
      cycle: this._cycle,
      unitName: this.name,
      architecture: this.architecture,
      schedulerAction: "idle",
      activeWarps: 0,
      totalWarps: 1,
      engineTraces: {},
      sharedMemoryUsed: 0,
      sharedMemoryTotal: this._config.weightBufferSize,
      registerFileUsed: 0,
      registerFileTotal: this._config.accumulatorCount,
      occupancy: 0.0,
    });
  }

  toString(): string {
    return (
      `MatrixMultiplyUnit(` +
      `${this._config.arrayRows}x${this._config.arrayCols}, ` +
      `idle=${this._idle})`
    );
  }
}

// ---------------------------------------------------------------------------
// Activation functions (shared between MXU and ANE)
// ---------------------------------------------------------------------------

/**
 * Apply an activation function element-wise to a matrix.
 *
 * This simulates the MXU's vector unit, which processes one row
 * at a time, applying the activation function to each element.
 *
 * === Supported Activation Functions ===
 *
 *     relu:    f(x) = max(0, x)       -- Rectified Linear Unit, the most
 *              common activation. Kills negative values, passes positives.
 *
 *     sigmoid: f(x) = 1 / (1 + e^-x) -- Squashes any value to [0, 1].
 *              Used for binary classification outputs.
 *
 *     tanh:    f(x) = tanh(x)         -- Squashes to [-1, 1].
 *              Like sigmoid but centered at zero.
 */
export function applyActivation(
  matrix: number[][],
  fnName: string,
): number[][] {
  const result: number[][] = [];
  for (const row of matrix) {
    const newRow: number[] = [];
    for (const val of row) {
      switch (fnName) {
        case "relu":
          newRow.push(Math.max(0.0, val));
          break;
        case "sigmoid": {
          const clamped = Math.max(-500.0, Math.min(500.0, val));
          newRow.push(1.0 / (1.0 + Math.exp(-clamped)));
          break;
        }
        case "tanh":
          newRow.push(Math.tanh(val));
          break;
        default:
          newRow.push(val);
          break;
      }
    }
    result.push(newRow);
  }
  return result;
}
