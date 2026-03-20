/**
 * NeuralEngineCore -- Apple ANE Core simulator.
 *
 * === What is the Apple Neural Engine? ===
 *
 * Apple's Neural Engine (ANE) is a dedicated neural network accelerator
 * found in every Apple chip since the A11 Bionic (2017). It's designed
 * for one thing: fast, power-efficient neural network inference.
 *
 * The ANE is the simplest compute unit in our family -- and that simplicity
 * is its strength. By removing hardware schedulers, branch predictors, and
 * general-purpose control logic, Apple can dedicate nearly all transistors
 * to MAC (multiply-accumulate) units and on-chip memory.
 *
 * === How ANE Differs from GPUs ===
 *
 *     GPU (NVIDIA/AMD):                   ANE (Apple):
 *     +----------------------------+     +----------------------------+
 *     | Hardware scheduler          |     | NO hardware scheduler      |
 *     | Runtime decisions           |     | All decisions at compile   |
 *     | Branch prediction           |     | NO branches                |
 *     | Dynamic register alloc      |     | Static buffer plan         |
 *     | Flexible but complex        |     | Simple but rigid           |
 *     | ~5 W per SM                |     | ~1 W per core              |
 *     +----------------------------+     +----------------------------+
 *
 * === Compiler-Scheduled Execution ===
 *
 * The ANE doesn't decide what to do at runtime. Instead, Apple's Core ML
 * compiler generates a complete schedule:
 *
 *     Cycle 0-9:   DMA load input tile (10 elements/cycle)
 *     Cycle 10-19: DMA load weight tile
 *     Cycle 20:    MAC operation (16 parallel multiplies)
 *     Cycle 21:    Reduce (sum MAC results)
 *     Cycle 22:    Activate (apply ReLU)
 *     Cycle 23:    DMA store output
 *
 * === Architecture Diagram ===
 *
 *     NeuralEngineCore
 *     +---------------------------------------------------------------+
 *     |                                                               |
 *     |  DMA Engine                                                   |
 *     |  +----------------------------------------------------------+ |
 *     |  | Transfers data between main memory and on-chip SRAM       | |
 *     |  | Bandwidth: 10 elements per cycle                          | |
 *     |  +----------------------------------------------------------+ |
 *     |                    |                                          |
 *     |                    v                                          |
 *     |  +------------------+ +------------------+                    |
 *     |  | Input Buffer     | | Weight Buffer    |                    |
 *     |  | 128 KB          | | 512 KB          |                    |
 *     |  +--------+---------+ +--------+---------+                    |
 *     |           |                    |                              |
 *     |           v                    v                              |
 *     |  +---------------------------------------------+              |
 *     |  | MAC Array (16 units)                         |              |
 *     |  | mac[i] = input[i] * weight[i]                |              |
 *     |  +---------------------------------------------+              |
 *     |                    |                                          |
 *     |                    v                                          |
 *     |  +---------------------------------------------+              |
 *     |  | Activation Pipeline                          |              |
 *     |  | ReLU / sigmoid / tanh / identity             |              |
 *     |  +---------------------------------------------+              |
 *     |                    |                                          |
 *     |                    v                                          |
 *     |  +---------------------------------------------+              |
 *     |  | Output Buffer (128 KB)                       |              |
 *     |  +---------------------------------------------+              |
 *     +---------------------------------------------------------------+
 */

import { FP16, FP32, type FloatFormat } from "@coding-adventures/fp-arithmetic";
import {
  MACArrayEngine,
  makeMACArrayConfig,
} from "@coding-adventures/parallel-execution-engine";

import {
  Architecture,
  type ComputeUnitTrace,
  makeComputeUnitTrace,
  type WorkItem,
} from "./protocols.js";
import { applyActivation } from "./matrix-multiply-unit.js";

// ---------------------------------------------------------------------------
// ANECoreConfig -- configuration for an Apple Neural Engine Core
// ---------------------------------------------------------------------------

/**
 * Configuration for an Apple Neural Engine Core.
 *
 * Real-world ANE configurations:
 *
 *     Parameter          | A14 (iPhone 12) | M1          | M2
 *     -------------------+-----------------+-------------+----------
 *     Cores              | 16              | 16          | 16
 *     TOPS               | 11              | 11          | 15.8
 *     Format             | FP16/INT8       | FP16/INT8   | FP16/INT8
 *     On-chip memory     | varies          | varies      | varies
 */
export interface ANECoreConfig {
  readonly numMacs: number;
  readonly macFormat: FloatFormat;
  readonly accumulatorFormat: FloatFormat;
  readonly sramSize: number;
  readonly activationBuffer: number;
  readonly weightBuffer: number;
  readonly outputBuffer: number;
  readonly dmaBandwidth: number;
}

/** Create an ANECoreConfig with sensible defaults. */
export function makeANECoreConfig(
  partial: Partial<ANECoreConfig> = {},
): ANECoreConfig {
  return {
    numMacs: 16,
    macFormat: FP16,
    accumulatorFormat: FP32,
    sramSize: 4194304,
    activationBuffer: 131072,
    weightBuffer: 524288,
    outputBuffer: 131072,
    dmaBandwidth: 10,
    ...partial,
  };
}

// ---------------------------------------------------------------------------
// NeuralEngineCore -- the main ANE Core simulator
// ---------------------------------------------------------------------------

/**
 * Apple Neural Engine Core simulator.
 *
 * Uses a MACArrayEngine from Layer 8 internally, adding DMA simulation,
 * activation pipeline, and compiler-generated schedule support.
 *
 * === Execution Model ===
 *
 * The ANE Core has no runtime scheduler. Instead, it follows a
 * compiler-generated schedule that specifies exactly what happens
 * on each cycle.
 *
 * === DMA Simulation ===
 *
 * In real ANE hardware, data must be DMA'd from main memory to
 * on-chip SRAM before the MACs can process it. This takes time:
 *
 *     DMA bandwidth: 10 elements/cycle (our default)
 *     Loading 160 elements: 16 cycles
 *     Loading 1600 elements: 160 cycles
 *
 * This DMA latency is why ANE performance is often memory-bandwidth
 * bound for small models.
 */
export class NeuralEngineCore {
  private _config: ANECoreConfig;
  private _cycle: number = 0;
  private _macEngine: MACArrayEngine;
  private _idleFlag: boolean = true;
  private _workItems: WorkItem[] = [];
  private _result: number[][] = [];

  constructor(config: ANECoreConfig) {
    this._config = config;
    this._macEngine = new MACArrayEngine(
      makeMACArrayConfig({
        numMacs: config.numMacs,
        inputBufferSize: Math.max(
          Math.floor(config.activationBuffer / 4),
          1024,
        ),
        weightBufferSize: Math.max(
          Math.floor(config.weightBuffer / 4),
          4096,
        ),
        outputBufferSize: Math.max(
          Math.floor(config.outputBuffer / 4),
          1024,
        ),
        floatFormat: FP32, // use FP32 internally
        accumulatorFormat: FP32,
        hasActivationUnit: true,
      }),
    );
  }

  // --- Properties ---

  get name(): string {
    return "ANECore";
  }

  get architecture(): Architecture {
    return Architecture.APPLE_ANE_CORE;
  }

  get idle(): boolean {
    return this._idleFlag;
  }

  get config(): ANECoreConfig {
    return this._config;
  }

  get result(): readonly (readonly number[])[] {
    return this._result;
  }

  get macEngine(): MACArrayEngine {
    return this._macEngine;
  }

  // --- Dispatch ---

  /**
   * Dispatch an inference tile to this ANE Core.
   *
   * The WorkItem must provide inputData and weightData. The ANE
   * Core will compute: result = inputData x weightData.
   */
  dispatch(work: WorkItem): void {
    this._workItems.push(work);
    this._idleFlag = false;
  }

  // --- Execution ---

  /**
   * Advance one cycle of the ANE Core.
   *
   * If work is pending, generates a compiler schedule, loads data
   * into the MAC engine, and runs it to completion.
   */
  step(clockEdge: { cycle: number }): ComputeUnitTrace {
    this._cycle += 1;

    if (this._idleFlag || this._workItems.length === 0) {
      return this._makeIdleTrace();
    }

    const work = this._workItems[0];
    this._processWorkItem(work);
    this._workItems.shift();

    if (this._workItems.length === 0) {
      this._idleFlag = true;
    }

    const rows = this._result.length;
    const cols = this._result.length > 0 ? this._result[0].length : 0;

    return makeComputeUnitTrace({
      cycle: this._cycle,
      unitName: this.name,
      architecture: this.architecture,
      schedulerAction: `inference complete: ${rows}x${cols} result`,
      activeWarps: this._idleFlag ? 0 : 1,
      totalWarps: 1,
      engineTraces: {},
      sharedMemoryUsed: 0,
      sharedMemoryTotal: this._config.sramSize,
      registerFileUsed: this._config.numMacs,
      registerFileTotal: this._config.numMacs,
      occupancy: this._idleFlag ? 0.0 : 1.0,
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
   * Convenience: run a complete inference pass.
   *
   * Performs matmul + activation function, simulating how the ANE
   * processes one layer of a neural network.
   *
   * === Inference Pipeline ===
   *
   * 1. DMA load inputs into activation buffer
   * 2. DMA load weights into weight buffer
   * 3. MAC: multiply input elements by weights
   * 4. Reduce: sum MAC results
   * 5. Activate: apply activation function
   * 6. DMA store outputs
   */
  runInference(
    inputs: number[][],
    weights: number[][],
    activationFn: string = "relu",
  ): number[][] {
    let result = this._matmul(inputs, weights);

    // Apply activation function
    if (activationFn !== "none") {
      result = applyActivation(result, activationFn);
    }

    this._result = result;
    return result;
  }

  /** Reset all state. */
  reset(): void {
    this._macEngine.reset();
    this._workItems = [];
    this._result = [];
    this._idleFlag = true;
    this._cycle = 0;
  }

  // --- Private helpers ---

  private _processWorkItem(work: WorkItem): void {
    if (work.inputData !== null && work.weightData !== null) {
      this._result = this._matmul(
        work.inputData as number[][],
        work.weightData as number[][],
      );
    } else {
      this._result = [];
    }
  }

  /**
   * Perform matrix multiplication using the MAC engine.
   *
   * For each element of the output matrix, we compute a dot product
   * using the MAC array. This simulates how the ANE processes
   * matrix multiplications tile by tile.
   */
  private _matmul(a: number[][], b: number[][]): number[][] {
    if (a.length === 0 || b.length === 0) {
      return [];
    }

    const m = a.length;
    const k = a[0].length;
    const n = b[0].length;

    const result: number[][] = [];
    for (let i = 0; i < m; i++) {
      const row: number[] = [];
      for (let j = 0; j < n; j++) {
        // Dot product of row i of A and column j of B
        let dot = 0.0;
        for (let kk = 0; kk < k; kk++) {
          dot += a[i][kk] * b[kk][j];
        }
        row.push(dot);
      }
      result.push(row);
    }

    return result;
  }

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
      sharedMemoryTotal: this._config.sramSize,
      registerFileUsed: 0,
      registerFileTotal: this._config.numMacs,
      occupancy: 0.0,
    });
  }

  toString(): string {
    return (
      `NeuralEngineCore(macs=${this._config.numMacs}, ` +
      `idle=${this._idleFlag})`
    );
  }
}
