/**
 * SubsliceEngine -- Intel Xe hybrid SIMD execution engine.
 *
 * === What is a Subslice? ===
 *
 * Intel's GPU architecture uses a hierarchical organization that's different
 * from both NVIDIA's SIMT warps and AMD's SIMD wavefronts. The basic unit
 * is the "subslice" (also called "sub-slice" or "dual sub-slice" in newer
 * architectures).
 *
 * A subslice contains:
 * - Multiple Execution Units (EUs), typically 8
 * - Each EU runs multiple hardware threads, typically 7
 * - Each thread processes SIMD8 (8-wide vector) instructions
 *
 *     +------------------------------------------------------+
 *     |  Subslice                                             |
 *     |                                                       |
 *     |  +----------------------+  +----------------------+   |
 *     |  |  EU 0                |  |  EU 1                |   |
 *     |  |  +----------------+  |  |  +----------------+  |   |
 *     |  |  | Thread 0: SIMD8|  |  |  | Thread 0: SIMD8|  |   |
 *     |  |  | Thread 1: SIMD8|  |  |  | Thread 1: SIMD8|  |   |
 *     |  |  | ...            |  |  |  | ...            |  |   |
 *     |  |  | Thread 6: SIMD8|  |  |  | Thread 6: SIMD8|  |   |
 *     |  |  +----------------+  |  |  +----------------+  |   |
 *     |  |  Thread Arbiter      |  |  Thread Arbiter      |   |
 *     |  +----------------------+  +----------------------+   |
 *     |                                                       |
 *     |  ... (EU 2 through EU 7, same structure) ...          |
 *     |                                                       |
 *     |  Shared Local Memory (SLM): 64 KB                     |
 *     |  Instruction Cache                                    |
 *     |  Thread Dispatcher                                    |
 *     +------------------------------------------------------+
 *
 * === Why Multiple Threads Per EU? ===
 *
 * This is Intel's approach to latency hiding. When one thread is stalled
 * (waiting for memory, for example), the EU's thread arbiter switches to
 * another ready thread. This keeps the SIMD ALU busy even when individual
 * threads are blocked.
 *
 * === Total Parallelism ===
 *
 * One subslice: 8 EUs x 7 threads x 8 SIMD lanes = 448 operations per cycle.
 */

import {
  type FloatFormat,
  FP32,
} from "@coding-adventures/fp-arithmetic";

import {
  GPUCore,
  GenericISA,
  type Instruction,
  type InstructionSet,
} from "@coding-adventures/gpu-core";

import { type EngineTrace, ExecutionModel } from "./protocols.js";

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/**
 * Configuration for an Intel Xe-style SIMD subslice.
 *
 * Real-world reference values:
 *
 *     Architecture   | EUs/subslice | Threads/EU | SIMD Width | GRF
 *     ---------------+--------------+------------+------------+-----
 *     Intel Xe-LP    | 16           | 7          | 8          | 128
 *     Intel Xe-HPG   | 16           | 8          | 8/16       | 128
 *     Intel Xe-HPC   | 16           | 8          | 8/16/32    | 128
 *     Our default    | 8            | 7          | 8          | 128
 */
export interface SubsliceConfig {
  readonly numEus: number;
  readonly threadsPerEu: number;
  readonly simdWidth: number;
  readonly grfSize: number;
  readonly slmSize: number;
  readonly floatFormat: FloatFormat;
  readonly isa: InstructionSet;
}

/**
 * Create a SubsliceConfig with sensible defaults.
 */
export function makeSubsliceConfig(
  partial: Partial<SubsliceConfig> = {},
): SubsliceConfig {
  return {
    numEus: 8,
    threadsPerEu: 7,
    simdWidth: 8,
    grfSize: 128,
    slmSize: 65536,
    floatFormat: FP32,
    isa: new GenericISA(),
    ...partial,
  };
}

// ---------------------------------------------------------------------------
// Execution Unit -- manages multiple hardware threads
// ---------------------------------------------------------------------------

/**
 * One Execution Unit (EU) in the subslice.
 *
 * Each EU has multiple hardware threads and a thread arbiter that picks
 * one ready thread to execute per cycle. Each thread runs SIMD8
 * instructions, which we simulate with one GPUCore per SIMD lane.
 *
 * === Thread Arbitration ===
 *
 * The arbiter's job is to keep the SIMD ALU busy. On each cycle, it:
 * 1. Scans all threads to find which are "ready" (not stalled).
 * 2. Picks one ready thread (round-robin among ready threads).
 * 3. Issues that thread's next SIMD8 instruction.
 */
export class ExecutionUnit {
  readonly euId: number;
  private readonly _config: SubsliceConfig;
  private _currentThread: number = 0;
  private _threads: GPUCore[][];
  private _threadActive: boolean[];
  private _program: Instruction[] = [];

  constructor(euId: number, config: SubsliceConfig) {
    this.euId = euId;
    this._config = config;

    // Each thread has `simdWidth` SIMD lanes, each backed by a GPUCore.
    this._threads = Array.from({ length: config.threadsPerEu }, () =>
      Array.from(
        { length: config.simdWidth },
        () =>
          new GPUCore({
            isa: config.isa,
            fmt: config.floatFormat,
            numRegisters: Math.min(config.grfSize, 256),
            memorySize: Math.floor(
              config.slmSize / Math.max(config.threadsPerEu, 1),
            ),
          }),
      ),
    );

    this._threadActive = new Array(config.threadsPerEu).fill(false);
  }

  /** Access to thread SIMD lanes. */
  get threads(): readonly (readonly GPUCore[])[] {
    return this._threads;
  }

  /** Load a program into all threads of this EU. */
  loadProgram(program: Instruction[]): void {
    this._program = [...program];
    for (let threadId = 0; threadId < this._config.threadsPerEu; threadId++) {
      for (const laneCore of this._threads[threadId]) {
        laneCore.loadProgram(this._program);
      }
      this._threadActive[threadId] = true;
    }
    this._currentThread = 0;
  }

  /** Set a register value for a specific lane of a specific thread. */
  setThreadLaneRegister(
    threadId: number,
    lane: number,
    reg: number,
    value: number,
  ): void {
    this._threads[threadId][lane].registers.writeFloat(reg, value);
  }

  /**
   * Execute one cycle using the thread arbiter.
   *
   * The arbiter selects one ready thread and executes its SIMD
   * instruction across all lanes.
   */
  step(): Record<number, string> {
    const traces: Record<number, string> = {};

    // Find a ready thread using round-robin
    const threadId = this._findReadyThread();
    if (threadId === null) {
      return traces;
    }

    // Execute SIMD instruction on all lanes of the selected thread
    const laneDescriptions: string[] = [];
    for (const laneCore of this._threads[threadId]) {
      if (!laneCore.halted) {
        try {
          const trace = laneCore.step();
          laneDescriptions.push(trace.description);
        } catch {
          laneDescriptions.push("(error)");
        }
      }
    }

    // Check if all lanes of this thread are halted
    if (this._threads[threadId].every((c) => c.halted)) {
      this._threadActive[threadId] = false;
    }

    if (laneDescriptions.length > 0) {
      traces[threadId] =
        `Thread ${threadId}: SIMD${this._config.simdWidth} -- ${laneDescriptions[0]}`;
    }

    return traces;
  }

  /**
   * Find the next ready thread using round-robin arbitration.
   */
  private _findReadyThread(): number | null {
    for (let offset = 0; offset < this._config.threadsPerEu; offset++) {
      const tid =
        (this._currentThread + offset) % this._config.threadsPerEu;
      if (
        this._threadActive[tid] &&
        this._threads[tid].some((c) => !c.halted)
      ) {
        this._currentThread = (tid + 1) % this._config.threadsPerEu;
        return tid;
      }
    }
    return null;
  }

  /** True if all threads on this EU are done. */
  get allHalted(): boolean {
    return !this._threadActive.some(Boolean);
  }

  /** Reset all threads on this EU. */
  reset(): void {
    for (let threadId = 0; threadId < this._config.threadsPerEu; threadId++) {
      for (const laneCore of this._threads[threadId]) {
        laneCore.reset();
        if (this._program.length > 0) {
          laneCore.loadProgram(this._program);
        }
      }
      this._threadActive[threadId] = this._program.length > 0;
    }
    this._currentThread = 0;
  }
}

// ---------------------------------------------------------------------------
// SubsliceEngine -- the hybrid SIMD execution engine
// ---------------------------------------------------------------------------

/**
 * Intel Xe-style subslice execution engine.
 *
 * Manages multiple EUs, each with multiple hardware threads, each
 * processing SIMD vectors. The thread arbiter in each EU selects
 * one ready thread per cycle.
 *
 * === Parallelism Hierarchy ===
 *
 *     Subslice (this engine)
 *     +-- EU 0
 *     |   +-- Thread 0: SIMD8 [lane0, lane1, ..., lane7]
 *     |   +-- Thread 1: SIMD8 [lane0, lane1, ..., lane7]
 *     |   +-- ... (threadsPerEu threads)
 *     +-- EU 1
 *     |   +-- Thread 0: SIMD8
 *     |   +-- ...
 *     +-- ... (numEus EUs)
 *
 * Total parallelism = numEus * threadsPerEu * simdWidth
 */
export class SubsliceEngine {
  private readonly _config: SubsliceConfig;
  private _cycle: number = 0;
  private _program: Instruction[] = [];
  private _eus: ExecutionUnit[];
  private _allHalted: boolean = false;

  constructor(config: SubsliceConfig) {
    this._config = config;
    this._eus = Array.from(
      { length: config.numEus },
      (_, i) => new ExecutionUnit(i, config),
    );
  }

  // --- Properties ---

  get name(): string {
    return "SubsliceEngine";
  }

  /** Total SIMD parallelism across all EUs and threads. */
  get width(): number {
    return (
      this._config.numEus *
      this._config.threadsPerEu *
      this._config.simdWidth
    );
  }

  get executionModel(): ExecutionModel {
    return ExecutionModel.SIMD;
  }

  get halted(): boolean {
    return this._allHalted;
  }

  get config(): SubsliceConfig {
    return this._config;
  }

  /** Access to the execution units. */
  get eus(): readonly ExecutionUnit[] {
    return this._eus;
  }

  // --- Program loading ---

  loadProgram(program: Instruction[]): void {
    this._program = [...program];
    for (const eu of this._eus) {
      eu.loadProgram(program);
    }
    this._allHalted = false;
    this._cycle = 0;
  }

  /** Set a register for a specific lane of a specific thread on a specific EU. */
  setEuThreadLaneRegister(
    euId: number,
    threadId: number,
    lane: number,
    reg: number,
    value: number,
  ): void {
    this._eus[euId].setThreadLaneRegister(threadId, lane, reg, value);
  }

  // --- Execution ---

  step(clockEdge: { cycle: number }): EngineTrace {
    this._cycle += 1;

    if (this._allHalted) {
      return this._makeHaltedTrace();
    }

    // Each EU steps independently
    const allTraces: Record<number, string> = {};
    let activeCount = 0;

    for (const eu of this._eus) {
      if (!eu.allHalted) {
        const euTraces = eu.step();
        for (const [threadId, desc] of Object.entries(euTraces)) {
          const flatId =
            eu.euId * this._config.threadsPerEu + Number(threadId);
          allTraces[flatId] = `EU${eu.euId}/${desc}`;
          activeCount += this._config.simdWidth;
        }
      }
    }

    // Check if all EUs are done
    if (this._eus.every((eu) => eu.allHalted)) {
      this._allHalted = true;
    }

    const total = this.width;

    // Build active mask
    const activeMask = Array.from({ length: total }, () => false);
    for (let i = 0; i < Math.min(activeCount, total); i++) {
      activeMask[i] = true;
    }

    return {
      cycle: this._cycle,
      engineName: this.name,
      executionModel: this.executionModel,
      description:
        `Subslice step -- ${activeCount}/${total} lanes active ` +
        `across ${this._config.numEus} EUs`,
      unitTraces: allTraces,
      activeMask,
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
      if (this._allHalted) {
        return traces;
      }
    }
    if (!this._allHalted) {
      throw new Error(
        `SubsliceEngine: max_cycles (${maxCycles}) reached`,
      );
    }
    return traces;
  }

  reset(): void {
    for (const eu of this._eus) {
      eu.reset();
    }
    this._allHalted = false;
    this._cycle = 0;
  }

  private _makeHaltedTrace(): EngineTrace {
    const total = this.width;
    return {
      cycle: this._cycle,
      engineName: this.name,
      executionModel: this.executionModel,
      description: "All EUs halted",
      unitTraces: {},
      activeMask: Array.from({ length: total }, () => false),
      activeCount: 0,
      totalCount: total,
      utilization: 0.0,
    };
  }

  toString(): string {
    const activeEus = this._eus.filter((eu) => !eu.allHalted).length;
    return (
      `SubsliceEngine(eus=${this._config.numEus}, ` +
      `active_eus=${activeEus}, halted=${this._allHalted})`
    );
  }
}
