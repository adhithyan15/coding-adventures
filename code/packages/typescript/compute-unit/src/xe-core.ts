/**
 * XeCore -- Intel Xe Core simulator.
 *
 * === What is an Xe Core? ===
 *
 * Intel's Xe Core is a hybrid: it combines SIMD execution units (like AMD)
 * with hardware threads (like NVIDIA), wrapped in a unique organizational
 * structure. It's the building block of Intel's Arc GPUs and Data Center
 * GPUs (Ponte Vecchio, Flex series).
 *
 * === Architecture ===
 *
 * An Xe Core contains:
 * - **Execution Units (EUs)**: 8-16 per Xe Core, each with its own ALU
 * - **Hardware threads**: 7 threads per EU for latency hiding
 * - **SIMD width**: SIMD8 (or SIMD16/32 on newer architectures)
 * - **SLM (Shared Local Memory)**: 64 KB, similar to NVIDIA's shared memory
 * - **Thread dispatcher**: distributes work to EU threads
 *
 *     XeCore
 *     +---------------------------------------------------------------+
 *     |  Thread Dispatcher                                            |
 *     |  +----------------------------------------------------------+ |
 *     |  | Dispatches work to available EU thread slots               | |
 *     |  +----------------------------------------------------------+ |
 *     |                                                               |
 *     |  +------------------+ +------------------+                    |
 *     |  | EU 0             | | EU 1             |                    |
 *     |  | Thread 0: SIMD8  | | Thread 0: SIMD8  |                    |
 *     |  | Thread 1: SIMD8  | | Thread 1: SIMD8  |                    |
 *     |  | ...              | | ...              |                    |
 *     |  | Thread 6: SIMD8  | | Thread 6: SIMD8  |                    |
 *     |  | Thread Arbiter   | | Thread Arbiter   |                    |
 *     |  +------------------+ +------------------+                    |
 *     |  ... (EU 2 through EU 15)                                     |
 *     |                                                               |
 *     |  Shared Local Memory (SLM): 64 KB                             |
 *     |  L1 Cache: 192 KB                                             |
 *     +---------------------------------------------------------------+
 *
 * === How Xe Differs from NVIDIA and AMD ===
 *
 *     NVIDIA SM:  4 schedulers, each manages many warps
 *     AMD CU:     4 SIMD units, each runs wavefronts
 *     Intel Xe:   8-16 EUs, each has 7 threads, each thread does SIMD8
 *
 * The key insight: Intel puts the thread-level parallelism INSIDE each EU
 * (7 threads per EU), while NVIDIA puts it across warps (64 warps per SM)
 * and AMD puts it across wavefronts (40 wavefronts per CU).
 *
 * Total parallelism:
 *     NVIDIA SM: 64 warps x 32 threads = 2048 threads
 *     AMD CU:    40 wavefronts x 64 lanes = 2560 lanes
 *     Intel Xe:  16 EUs x 7 threads x 8 SIMD = 896 lanes
 */

import { GenericISA, type InstructionSet } from "@coding-adventures/gpu-core";
import { FP32, type FloatFormat } from "@coding-adventures/fp-arithmetic";
import {
  SubsliceEngine,
  makeSubsliceConfig,
  type EngineTrace,
} from "@coding-adventures/parallel-execution-engine";

import {
  Architecture,
  type ComputeUnitTrace,
  makeComputeUnitTrace,
  SchedulingPolicy,
  SharedMemory,
  type WorkItem,
} from "./protocols.js";

// ---------------------------------------------------------------------------
// XeCoreConfig -- configuration for an Intel Xe Core
// ---------------------------------------------------------------------------

/**
 * Configuration for an Intel Xe Core.
 *
 * Real-world Xe Core configurations:
 *
 *     Parameter           | Xe-LP (iGPU) | Xe-HPG (Arc)  | Xe-HPC
 *     --------------------+--------------+---------------+----------
 *     EUs per Xe Core     | 16           | 16            | 16
 *     Threads per EU      | 7            | 8             | 8
 *     SIMD width          | 8            | 8 (or 16)     | 8/16/32
 *     GRF per EU          | 128          | 128           | 128
 *     SLM size            | 64 KB        | 64 KB         | 128 KB
 *     L1 cache            | 192 KB       | 192 KB        | 384 KB
 */
export interface XeCoreConfig {
  readonly numEus: number;
  readonly threadsPerEu: number;
  readonly simdWidth: number;
  readonly grfPerEu: number;
  readonly slmSize: number;
  readonly l1CacheSize: number;
  readonly instructionCacheSize: number;
  readonly schedulingPolicy: SchedulingPolicy;
  readonly floatFormat: FloatFormat;
  readonly isa: InstructionSet;
  readonly memoryLatencyCycles: number;
}

/** Create an XeCoreConfig with sensible defaults. */
export function makeXeCoreConfig(
  partial: Partial<XeCoreConfig> = {},
): XeCoreConfig {
  return {
    numEus: 16,
    threadsPerEu: 7,
    simdWidth: 8,
    grfPerEu: 128,
    slmSize: 65536,
    l1CacheSize: 196608,
    instructionCacheSize: 65536,
    schedulingPolicy: SchedulingPolicy.ROUND_ROBIN,
    floatFormat: FP32,
    isa: new GenericISA(),
    memoryLatencyCycles: 200,
    ...partial,
  };
}

// ---------------------------------------------------------------------------
// XeCore -- the main Intel Xe Core simulator
// ---------------------------------------------------------------------------

/**
 * Intel Xe Core simulator.
 *
 * Manages Execution Units (EUs) with hardware threads, SLM, and a
 * thread dispatcher that distributes work across EU threads.
 *
 * === How Work Distribution Works ===
 *
 * When a work group is dispatched to an Xe Core:
 * 1. The thread dispatcher calculates how many EU threads are needed
 * 2. Each thread gets a portion of the work (SIMD8 of the total)
 * 3. The EU's thread arbiter round-robins among active threads
 * 4. SLM is shared among all threads in the work group
 *
 * === Latency Hiding in Xe ===
 *
 * With 7 threads per EU, when one thread stalls on a memory access,
 * the EU arbiter switches to another ready thread on the NEXT cycle
 * (zero-penalty switching, just like NVIDIA warp switching). The
 * difference is granularity: Intel hides latency at the EU level
 * with 7 threads, while NVIDIA hides it at the SM level with 64 warps.
 */
export class XeCore {
  private _config: XeCoreConfig;
  private _cycle: number = 0;
  private _slm: SharedMemory;
  private _engine: SubsliceEngine;
  private _idleFlag: boolean = true;
  private _workItems: WorkItem[] = [];

  constructor(config: XeCoreConfig) {
    this._config = config;
    this._slm = new SharedMemory(config.slmSize);
    this._engine = new SubsliceEngine(
      makeSubsliceConfig({
        numEus: config.numEus,
        threadsPerEu: config.threadsPerEu,
        simdWidth: config.simdWidth,
        grfSize: config.grfPerEu,
        slmSize: config.slmSize,
        floatFormat: config.floatFormat,
        isa: config.isa,
      }),
    );
  }

  // --- Properties ---

  get name(): string {
    return "XeCore";
  }

  get architecture(): Architecture {
    return Architecture.INTEL_XE_CORE;
  }

  get idle(): boolean {
    if (this._workItems.length === 0 && this._idleFlag) {
      return true;
    }
    return this._idleFlag && this._engine.halted;
  }

  get config(): XeCoreConfig {
    return this._config;
  }

  get slm(): SharedMemory {
    return this._slm;
  }

  get engine(): SubsliceEngine {
    return this._engine;
  }

  // --- Dispatch ---

  /**
   * Dispatch a work group to this Xe Core.
   *
   * Loads the program into the SubsliceEngine and sets per-thread
   * register values.
   */
  dispatch(work: WorkItem): void {
    this._workItems.push(work);
    this._idleFlag = false;

    if (work.program !== null) {
      this._engine.loadProgram(work.program);
    }

    // Set per-thread data across EUs
    for (const globalTidStr of Object.keys(work.perThreadData)) {
      const globalTid = Number(globalTidStr);
      const regs = work.perThreadData[globalTid];

      // Map global thread ID to (eu, thread, lane)
      const totalLanes = this._config.simdWidth;
      const threadTotal = totalLanes * this._config.threadsPerEu;
      const euId = Math.floor(globalTid / threadTotal);
      const remainder = globalTid % threadTotal;
      const threadId = Math.floor(remainder / totalLanes);
      const lane = remainder % totalLanes;

      if (euId < this._config.numEus) {
        for (const regStr of Object.keys(regs)) {
          const reg = Number(regStr);
          this._engine.setEuThreadLaneRegister(
            euId,
            threadId,
            lane,
            reg,
            regs[reg],
          );
        }
      }
    }
  }

  // --- Execution ---

  /**
   * Advance one cycle.
   *
   * Delegates to the SubsliceEngine which manages EU thread arbitration.
   */
  step(clockEdge: { cycle: number }): ComputeUnitTrace {
    this._cycle += 1;

    const engineTrace = this._engine.step(clockEdge);

    if (this._engine.halted) {
      this._idleFlag = true;
    }

    const active = engineTrace.activeCount;

    return makeComputeUnitTrace({
      cycle: this._cycle,
      unitName: this.name,
      architecture: this.architecture,
      schedulerAction: engineTrace.description,
      activeWarps: active > 0 ? 1 : 0,
      totalWarps: 1,
      engineTraces: { 0: engineTrace },
      sharedMemoryUsed: 0,
      sharedMemoryTotal: this._config.slmSize,
      registerFileUsed: this._config.grfPerEu * this._config.numEus,
      registerFileTotal: this._config.grfPerEu * this._config.numEus,
      occupancy: active > 0 ? 1.0 : 0.0,
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

  /** Reset all state. */
  reset(): void {
    this._engine.reset();
    this._slm.reset();
    this._workItems = [];
    this._idleFlag = true;
    this._cycle = 0;
  }

  toString(): string {
    return (
      `XeCore(eus=${this._config.numEus}, ` +
      `threads_per_eu=${this._config.threadsPerEu}, ` +
      `idle=${this.idle})`
    );
  }
}
