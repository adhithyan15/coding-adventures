/**
 * MACArrayEngine -- compiler-scheduled MAC array execution (NPU style).
 *
 * === What is a MAC Array? ===
 *
 * A MAC (Multiply-Accumulate) array is a bank of multiply-accumulate units
 * driven entirely by a schedule that the compiler generates at compile time.
 * There is NO hardware scheduler -- the compiler decides exactly which MAC
 * unit processes which data on which cycle.
 *
 * This is the execution model used by:
 * - Apple Neural Engine (ANE)
 * - Qualcomm Hexagon NPU
 * - Many custom AI accelerator ASICs
 *
 * === How It Differs from Other Models ===
 *
 *     GPU (SIMT/SIMD):                   NPU (Scheduled MAC):
 *     +--------------------------+       +--------------------------+
 *     | Hardware scheduler       |       | NO hardware scheduler    |
 *     | Runtime decisions        |       | All decisions at compile |
 *     | Branch prediction        |       | NO branches              |
 *     | Dynamic resource alloc   |       | Static resource plan     |
 *     | Flexible but complex     |       | Simple but rigid         |
 *     +--------------------------+       +--------------------------+
 *
 * === The Execution Pipeline ===
 *
 *     1. LOAD_INPUT:    Move data from external memory to input buffer
 *     2. LOAD_WEIGHTS:  Move weights from external memory to weight buffer
 *     3. MAC:           Multiply input[i] * weight[i] for all MACs in parallel
 *     4. REDUCE:        Sum the MAC results (adder tree)
 *     5. ACTIVATE:      Apply activation function (ReLU, sigmoid, tanh)
 *     6. STORE_OUTPUT:  Write result to output buffer
 *
 * === Why NPUs Are Power-Efficient ===
 *
 * By moving all scheduling to compile time, NPUs eliminate:
 * - Branch prediction hardware (saves transistors and power)
 * - Instruction cache (the "program" is a simple schedule table)
 * - Warp/wavefront scheduler (no runtime thread management)
 * - Speculation hardware (nothing is speculative)
 */

import {
  type FloatFormat,
  FP16,
  FP32,
} from "@coding-adventures/fp-arithmetic";

import { type EngineTrace, ExecutionModel } from "./protocols.js";

// ---------------------------------------------------------------------------
// Operations and activation functions
// ---------------------------------------------------------------------------

/**
 * Operations that can appear in a MAC array schedule.
 *
 * Each operation corresponds to one stage of the MAC pipeline.
 */
export enum MACOperation {
  LOAD_INPUT = "load_input",
  LOAD_WEIGHTS = "load_weights",
  MAC = "mac",
  REDUCE = "reduce",
  ACTIVATE = "activate",
  STORE_OUTPUT = "store_output",
}

/**
 * Hardware-supported activation functions.
 *
 * Neural networks use non-linear "activation functions" after each layer.
 * NPUs typically implement a few common ones in hardware for speed:
 *
 *     NONE:    f(x) = x              (identity / linear)
 *     RELU:    f(x) = max(0, x)      (most popular; simple, fast)
 *     SIGMOID: f(x) = 1/(1+e^-x)    (classic; squashes to [0,1])
 *     TANH:    f(x) = tanh(x)        (squashes to [-1,1])
 */
export enum ActivationFunction {
  NONE = "none",
  RELU = "relu",
  SIGMOID = "sigmoid",
  TANH = "tanh",
}

// ---------------------------------------------------------------------------
// Schedule entry
// ---------------------------------------------------------------------------

/**
 * One entry in the MAC array schedule.
 *
 * The compiler generates these at compile time. Each entry describes
 * exactly what happens on one cycle.
 */
export interface MACScheduleEntry {
  readonly cycle: number;
  readonly operation: MACOperation;
  readonly inputIndices: readonly number[];
  readonly weightIndices: readonly number[];
  readonly outputIndex: number;
  readonly activation: string;
}

/**
 * Create a MACScheduleEntry with sensible defaults.
 */
export function makeMACScheduleEntry(
  partial: Partial<MACScheduleEntry> & {
    cycle: number;
    operation: MACOperation;
  },
): MACScheduleEntry {
  return {
    inputIndices: [],
    weightIndices: [],
    outputIndex: 0,
    activation: "none",
    ...partial,
  };
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/**
 * Configuration for a scheduled MAC array engine.
 *
 * Real-world reference values:
 *
 *     Hardware          | MACs | Input Buf | Weight Buf | Format
 *     ------------------+------+-----------+------------+-------
 *     Apple ANE (M1)    | 16K  | varies    | varies     | FP16/INT8
 *     Qualcomm Hexagon  | 2K   | varies    | varies     | INT8
 *     Our default       | 8    | 1024      | 4096       | FP16
 */
export interface MACArrayConfig {
  readonly numMacs: number;
  readonly inputBufferSize: number;
  readonly weightBufferSize: number;
  readonly outputBufferSize: number;
  readonly floatFormat: FloatFormat;
  readonly accumulatorFormat: FloatFormat;
  readonly hasActivationUnit: boolean;
}

/**
 * Create a MACArrayConfig with sensible defaults.
 */
export function makeMACArrayConfig(
  partial: Partial<MACArrayConfig> = {},
): MACArrayConfig {
  return {
    numMacs: 8,
    inputBufferSize: 1024,
    weightBufferSize: 4096,
    outputBufferSize: 1024,
    floatFormat: FP16,
    accumulatorFormat: FP32,
    hasActivationUnit: true,
    ...partial,
  };
}

// ---------------------------------------------------------------------------
// MACArrayEngine -- the scheduled execution engine
// ---------------------------------------------------------------------------

/**
 * Compiler-scheduled MAC array execution engine (NPU style).
 *
 * No hardware scheduler. The compiler generates a static schedule that
 * says exactly what each MAC does on each cycle.
 *
 * === Usage Pattern ===
 *
 *     1. Create engine with config.
 *     2. Load inputs and weights into the buffers.
 *     3. Load a compiler-generated schedule.
 *     4. Step or run -- the engine follows the schedule exactly.
 *     5. Read results from the output buffer.
 */
export class MACArrayEngine {
  private readonly _config: MACArrayConfig;
  private _cycle: number = 0;
  private _inputBuffer: number[];
  private _weightBuffer: number[];
  private _outputBuffer: number[];
  private _macAccumulators: number[];
  private _schedule: MACScheduleEntry[] = [];
  private _halted: boolean = false;

  constructor(config: MACArrayConfig) {
    this._config = config;
    this._inputBuffer = new Array(config.inputBufferSize).fill(0.0);
    this._weightBuffer = new Array(config.weightBufferSize).fill(0.0);
    this._outputBuffer = new Array(config.outputBufferSize).fill(0.0);
    this._macAccumulators = new Array(config.numMacs).fill(0.0);
  }

  // --- Properties ---

  get name(): string {
    return "MACArrayEngine";
  }

  get width(): number {
    return this._config.numMacs;
  }

  get executionModel(): ExecutionModel {
    return ExecutionModel.SCHEDULED_MAC;
  }

  get halted(): boolean {
    return this._halted;
  }

  get config(): MACArrayConfig {
    return this._config;
  }

  // --- Data loading ---

  loadInputs(data: number[]): void {
    for (let i = 0; i < data.length && i < this._config.inputBufferSize; i++) {
      this._inputBuffer[i] = data[i];
    }
  }

  loadWeights(data: number[]): void {
    for (
      let i = 0;
      i < data.length && i < this._config.weightBufferSize;
      i++
    ) {
      this._weightBuffer[i] = data[i];
    }
  }

  loadSchedule(schedule: MACScheduleEntry[]): void {
    this._schedule = [...schedule];
    this._halted = false;
  }

  // --- Execution ---

  step(clockEdge: { cycle: number }): EngineTrace {
    this._cycle += 1;

    if (this._halted) {
      return this._makeIdleTrace("Schedule complete");
    }

    // Find schedule entries for this cycle
    const entries = this._schedule.filter((e) => e.cycle === this._cycle);

    if (entries.length === 0) {
      // Check if we've passed all schedule entries
      const maxCycle = this._schedule.reduce(
        (max, e) => Math.max(max, e.cycle),
        0,
      );
      if (this._cycle > maxCycle) {
        this._halted = true;
        return this._makeIdleTrace("Schedule complete");
      }
      return this._makeIdleTrace("No operation this cycle");
    }

    // Execute all entries for this cycle
    const unitTraces: Record<number, string> = {};
    let activeCount = 0;
    const descriptions: string[] = [];

    for (const entry of entries) {
      switch (entry.operation) {
        case MACOperation.LOAD_INPUT: {
          descriptions.push(`LOAD_INPUT indices=[${entry.inputIndices}]`);
          activeCount = entry.inputIndices.length;
          break;
        }
        case MACOperation.LOAD_WEIGHTS: {
          descriptions.push(`LOAD_WEIGHTS indices=[${entry.weightIndices}]`);
          activeCount = entry.weightIndices.length;
          break;
        }
        case MACOperation.MAC: {
          const [desc, traces] = this._execMac(entry);
          descriptions.push(desc);
          Object.assign(unitTraces, traces);
          activeCount = Object.keys(traces).length;
          break;
        }
        case MACOperation.REDUCE: {
          descriptions.push(this._execReduce(entry));
          activeCount = 1;
          break;
        }
        case MACOperation.ACTIVATE: {
          descriptions.push(this._execActivate(entry));
          activeCount = 1;
          break;
        }
        case MACOperation.STORE_OUTPUT: {
          descriptions.push(this._execStore(entry));
          activeCount = 1;
          break;
        }
      }
    }

    const total = this._config.numMacs;
    const description = descriptions.join("; ");

    return {
      cycle: this._cycle,
      engineName: this.name,
      executionModel: this.executionModel,
      description: `${description} -- ${activeCount}/${total} MACs active`,
      unitTraces,
      activeMask: Array.from({ length: total }, (_, i) => i < activeCount),
      activeCount,
      totalCount: total,
      utilization: total > 0 ? activeCount / total : 0.0,
    };
  }

  run(maxCycles: number = 10000): EngineTrace[] {
    const traces: EngineTrace[] = [];
    for (let cycleNum = 1; cycleNum <= maxCycles; cycleNum++) {
      const trace = this.step({ cycle: cycleNum });
      traces.push(trace);
      if (this._halted) {
        return traces;
      }
    }
    if (!this._halted) {
      throw new Error(
        `MACArrayEngine: max_cycles (${maxCycles}) reached`,
      );
    }
    return traces;
  }

  /** Read results from the output buffer. */
  readOutputs(): number[] {
    return [...this._outputBuffer];
  }

  reset(): void {
    this._inputBuffer = new Array(this._config.inputBufferSize).fill(0.0);
    this._weightBuffer = new Array(this._config.weightBufferSize).fill(0.0);
    this._outputBuffer = new Array(this._config.outputBufferSize).fill(0.0);
    this._macAccumulators = new Array(this._config.numMacs).fill(0.0);
    this._halted = false;
    this._cycle = 0;
  }

  // --- Operation implementations ---

  private _execMac(
    entry: MACScheduleEntry,
  ): [string, Record<number, string>] {
    const unitTraces: Record<number, string> = {};
    const numOps = Math.min(
      entry.inputIndices.length,
      entry.weightIndices.length,
      this._config.numMacs,
    );

    for (let macId = 0; macId < numOps; macId++) {
      const inIdx = entry.inputIndices[macId];
      const wtIdx = entry.weightIndices[macId];
      const inVal = this._inputBuffer[inIdx];
      const wtVal = this._weightBuffer[wtIdx];
      const result = inVal * wtVal;
      this._macAccumulators[macId] = result;
      unitTraces[macId] =
        `MAC: ${inVal.toPrecision(4)} * ${wtVal.toPrecision(4)} = ${result.toPrecision(4)}`;
    }

    return [`MAC ${numOps} operations`, unitTraces];
  }

  private _execReduce(entry: MACScheduleEntry): string {
    const total = this._macAccumulators.reduce((sum, val) => sum + val, 0);
    const outIdx = entry.outputIndex;
    if (outIdx < this._config.outputBufferSize) {
      this._outputBuffer[outIdx] = total;
    }
    return `REDUCE sum=${total.toPrecision(4)} -> output[${outIdx}]`;
  }

  private _execActivate(entry: MACScheduleEntry): string {
    if (!this._config.hasActivationUnit) {
      return "ACTIVATE skipped (no hardware activation unit)";
    }

    const outIdx = entry.outputIndex;
    if (outIdx >= this._config.outputBufferSize) {
      return `ACTIVATE error: index ${outIdx} out of range`;
    }

    const val = this._outputBuffer[outIdx];
    let result: number;

    switch (entry.activation) {
      case ActivationFunction.NONE:
        result = val;
        break;
      case ActivationFunction.RELU:
        result = Math.max(0.0, val);
        break;
      case ActivationFunction.SIGMOID: {
        // Sigmoid: 1 / (1 + e^-x), clamp to avoid overflow
        const clamped = Math.max(-500.0, Math.min(500.0, val));
        result = 1.0 / (1.0 + Math.exp(-clamped));
        break;
      }
      case ActivationFunction.TANH:
        result = Math.tanh(val);
        break;
      default:
        result = val;
    }

    this._outputBuffer[outIdx] = result;
    return `ACTIVATE ${entry.activation}(${val.toPrecision(4)}) = ${result.toPrecision(4)}`;
  }

  private _execStore(entry: MACScheduleEntry): string {
    const outIdx = entry.outputIndex;
    const val =
      outIdx < this._config.outputBufferSize
        ? this._outputBuffer[outIdx]
        : 0.0;
    return `STORE_OUTPUT output[${outIdx}] = ${val.toPrecision(4)}`;
  }

  private _makeIdleTrace(description: string): EngineTrace {
    return {
      cycle: this._cycle,
      engineName: this.name,
      executionModel: this.executionModel,
      description,
      unitTraces: {},
      activeMask: Array.from(
        { length: this._config.numMacs },
        () => false,
      ),
      activeCount: 0,
      totalCount: this._config.numMacs,
      utilization: 0.0,
    };
  }

  toString(): string {
    return (
      `MACArrayEngine(num_macs=${this._config.numMacs}, ` +
      `cycle=${this._cycle}, halted=${this._halted})`
    );
  }
}
