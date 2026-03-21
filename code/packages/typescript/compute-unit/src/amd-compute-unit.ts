/**
 * AMDComputeUnit -- AMD Compute Unit (GCN/RDNA) simulator.
 *
 * === How AMD CUs Differ from NVIDIA SMs ===
 *
 * While NVIDIA and AMD GPUs look similar from the outside, their internal
 * organization is quite different:
 *
 *     NVIDIA SM:                          AMD CU (GCN):
 *     ---------                           --------------
 *     4 warp schedulers                   4 SIMD units (16-wide each)
 *     Each issues 1 warp (32 threads)     Each runs 1 wavefront (64 lanes)
 *     Total: 128 threads/cycle            Total: 64 lanes x 4 = 256 lanes/cycle
 *
 *     Register file: unified              Register file: per-SIMD VGPR
 *     Shared memory: explicit             LDS: explicit (similar to shared mem)
 *     Warp scheduling: hardware           Wavefront scheduling: hardware
 *     Scalar unit: per-thread             Scalar unit: SHARED by wavefront
 *
 * === The Scalar Unit -- AMD's Key Innovation ===
 *
 * The scalar unit executes operations that are the SAME across all lanes:
 * - Address computation (base_addr + offset)
 * - Loop counters (i++)
 * - Branch conditions (if i < N)
 * - Constants (pi, epsilon, etc.)
 *
 * Instead of doing this 64 times (once per lane), AMD does it ONCE in the
 * scalar unit and broadcasts the result. This saves power and register space.
 *
 * === Architecture Diagram ===
 *
 *     AMDComputeUnit (GCN-style)
 *     +---------------------------------------------------------------+
 *     |                                                               |
 *     |  Wavefront Scheduler                                          |
 *     |  +----------------------------------------------------------+ |
 *     |  | wf0: READY  wf1: STALLED  wf2: READY  wf3: READY ...    | |
 *     |  +----------------------------------------------------------+ |
 *     |                                                               |
 *     |  +------------------+ +------------------+                    |
 *     |  | SIMD Unit 0      | | SIMD Unit 1      |                    |
 *     |  | 16-wide ALU      | | 16-wide ALU      |                    |
 *     |  | VGPR: 256        | | VGPR: 256        |                    |
 *     |  +------------------+ +------------------+                    |
 *     |  +------------------+ +------------------+                    |
 *     |  | SIMD Unit 2      | | SIMD Unit 3      |                    |
 *     |  | 16-wide ALU      | | 16-wide ALU      |                    |
 *     |  +------------------+ +------------------+                    |
 *     |                                                               |
 *     |  +------------------+                                         |
 *     |  | Scalar Unit      |  <- executes once for all lanes         |
 *     |  | SGPR: 104        |  (address computation, flow control)    |
 *     |  +------------------+                                         |
 *     |                                                               |
 *     |  Shared Resources:                                            |
 *     |  +-----------------------------------------------------------+|
 *     |  | LDS (Local Data Share): 64 KB                              ||
 *     |  | L1 Vector Cache: 16 KB                                     ||
 *     |  | L1 Scalar Cache: 16 KB                                     ||
 *     |  +-----------------------------------------------------------+|
 *     +---------------------------------------------------------------+
 */

import { GenericISA, type InstructionSet } from "@coding-adventures/gpu-core";
import { FP32, type FloatFormat } from "@coding-adventures/fp-arithmetic";
import {
  WavefrontEngine,
  type WavefrontConfig,
  makeWavefrontConfig,
  type EngineTrace,
} from "@coding-adventures/parallel-execution-engine";

import {
  Architecture,
  type ComputeUnitTrace,
  makeComputeUnitTrace,
  SchedulingPolicy,
  SharedMemory,
  WarpState,
  type WorkItem,
} from "./protocols.js";
import { ResourceError } from "./streaming-multiprocessor.js";

// ---------------------------------------------------------------------------
// AMDCUConfig -- configuration for an AMD-style Compute Unit
// ---------------------------------------------------------------------------

/**
 * Configuration for an AMD-style Compute Unit.
 *
 * Real-world CU configurations:
 *
 *     Parameter            | GCN (Vega)   | RDNA2 (RX 6000) | RDNA3
 *     ---------------------+--------------+------------------+------
 *     SIMD units           | 4            | 2 (per CU)       | 2
 *     Wave width           | 64           | 32 (native)      | 32
 *     Max wavefronts       | 40           | 32               | 32
 *     VGPRs per SIMD       | 256          | 256              | 256
 *     SGPRs                | 104          | 104              | 104
 *     LDS size             | 64 KB        | 128 KB           | 128 KB
 *     L1 vector cache      | 16 KB        | 128 KB           | 128 KB
 */
export interface AMDCUConfig {
  readonly numSimdUnits: number;
  readonly waveWidth: number;
  readonly maxWavefronts: number;
  readonly maxWorkGroups: number;
  readonly schedulingPolicy: SchedulingPolicy;
  readonly vgprPerSimd: number;
  readonly sgprCount: number;
  readonly ldsSize: number;
  readonly l1VectorCache: number;
  readonly l1ScalarCache: number;
  readonly l1InstructionCache: number;
  readonly floatFormat: FloatFormat;
  readonly isa: InstructionSet;
  readonly memoryLatencyCycles: number;
}

/** Create an AMDCUConfig with sensible defaults. */
export function makeAMDCUConfig(
  partial: Partial<AMDCUConfig> = {},
): AMDCUConfig {
  return {
    numSimdUnits: 4,
    waveWidth: 64,
    maxWavefronts: 40,
    maxWorkGroups: 16,
    schedulingPolicy: SchedulingPolicy.LRR,
    vgprPerSimd: 256,
    sgprCount: 104,
    ldsSize: 65536,
    l1VectorCache: 16384,
    l1ScalarCache: 16384,
    l1InstructionCache: 32768,
    floatFormat: FP32,
    isa: new GenericISA(),
    memoryLatencyCycles: 200,
    ...partial,
  };
}

// ---------------------------------------------------------------------------
// WavefrontSlot -- tracks one wavefront's state
// ---------------------------------------------------------------------------

/**
 * One wavefront in the AMD CU's scheduler.
 *
 * Similar to WarpSlot in the NVIDIA SM, but for AMD wavefronts.
 * Each slot tracks the wavefront's state and which SIMD unit
 * it's assigned to.
 */
export interface WavefrontSlot {
  readonly waveId: number;
  readonly workId: number;
  state: WarpState;
  readonly simdUnit: number;
  readonly engine: WavefrontEngine;
  stallCounter: number;
  age: number;
  readonly vgprsUsed: number;
}

// ---------------------------------------------------------------------------
// AMDComputeUnit -- the main CU simulator
// ---------------------------------------------------------------------------

/**
 * AMD Compute Unit (GCN/RDNA) simulator.
 *
 * Manages wavefronts across SIMD units, with scalar unit support,
 * LDS (Local Data Share), and wavefront scheduling.
 *
 * === Key Differences from StreamingMultiprocessor ===
 *
 * 1. **SIMD units instead of warp schedulers**: Each SIMD unit is a
 *    16-wide vector ALU. A 64-wide wavefront takes 4 cycles to execute
 *    on a 16-wide SIMD unit (the wavefront is "over-scheduled").
 *
 * 2. **Scalar unit**: Operations common to all lanes execute once on
 *    the scalar unit instead of per-lane.
 *
 * 3. **LDS instead of shared memory**: Functionally similar, but AMD's
 *    LDS has different banking (32 banks, 4 bytes/bank).
 *
 * 4. **LRR scheduling**: AMD typically uses Loose Round Robin instead
 *    of NVIDIA's GTO.
 */
export class AMDComputeUnit {
  private _config: AMDCUConfig;
  private _cycle: number = 0;
  private _lds: SharedMemory;
  private _ldsUsed: number = 0;
  private _wavefrontSlots: WavefrontSlot[] = [];
  private _nextWaveId: number = 0;
  private _vgprAllocated: number[];
  private _rrIndex: number = 0;

  constructor(config: AMDCUConfig) {
    this._config = config;
    this._lds = new SharedMemory(config.ldsSize);
    this._vgprAllocated = new Array(config.numSimdUnits).fill(0);
  }

  // --- Properties ---

  get name(): string {
    return "CU";
  }

  get architecture(): Architecture {
    return Architecture.AMD_CU;
  }

  get idle(): boolean {
    return (
      this._wavefrontSlots.length === 0 ||
      this._wavefrontSlots.every((w) => w.state === WarpState.COMPLETED)
    );
  }

  get occupancy(): number {
    if (this._config.maxWavefronts === 0) {
      return 0.0;
    }
    const active = this._wavefrontSlots.filter(
      (w) => w.state !== WarpState.COMPLETED,
    ).length;
    return active / this._config.maxWavefronts;
  }

  get config(): AMDCUConfig {
    return this._config;
  }

  get lds(): SharedMemory {
    return this._lds;
  }

  get wavefrontSlots(): readonly WavefrontSlot[] {
    return this._wavefrontSlots;
  }

  // --- Dispatch ---

  /**
   * Dispatch a work group to this CU.
   *
   * Decomposes the work group into wavefronts and assigns them to
   * SIMD units round-robin.
   *
   * @throws ResourceError if not enough resources.
   */
  dispatch(work: WorkItem): void {
    const numWaves = Math.ceil(
      work.threadCount / this._config.waveWidth,
    );

    const currentActive = this._wavefrontSlots.filter(
      (w) => w.state !== WarpState.COMPLETED,
    ).length;

    if (currentActive + numWaves > this._config.maxWavefronts) {
      throw new ResourceError(
        `Not enough wavefront slots: need ${numWaves}, ` +
          `available ${this._config.maxWavefronts - currentActive}`,
      );
    }

    const smemNeeded = work.sharedMemBytes;
    if (this._ldsUsed + smemNeeded > this._config.ldsSize) {
      throw new ResourceError(
        `Not enough LDS: need ${smemNeeded}, ` +
          `available ${this._config.ldsSize - this._ldsUsed}`,
      );
    }

    this._ldsUsed += smemNeeded;

    for (let waveIdx = 0; waveIdx < numWaves; waveIdx++) {
      const waveId = this._nextWaveId;
      this._nextWaveId += 1;

      const threadStart = waveIdx * this._config.waveWidth;
      const threadEnd = Math.min(
        threadStart + this._config.waveWidth,
        work.threadCount,
      );
      const actualLanes = threadEnd - threadStart;

      // Assign to a SIMD unit round-robin
      const simdUnit = waveIdx % this._config.numSimdUnits;

      // Create WavefrontEngine
      const engine = new WavefrontEngine(
        makeWavefrontConfig({
          waveWidth: actualLanes,
          numVgprs: Math.min(this._config.vgprPerSimd, 256),
          numSgprs: this._config.sgprCount,
          floatFormat: this._config.floatFormat,
          isa: this._config.isa,
        }),
      );

      if (work.program !== null) {
        engine.loadProgram(work.program);
      }

      // Set per-lane data
      for (let laneOffset = 0; laneOffset < actualLanes; laneOffset++) {
        const globalTid = threadStart + laneOffset;
        if (globalTid in work.perThreadData) {
          const regs = work.perThreadData[globalTid];
          for (const regStr of Object.keys(regs)) {
            const reg = Number(regStr);
            engine.setLaneRegister(laneOffset, reg, regs[reg]);
          }
        }
      }

      const slot: WavefrontSlot = {
        waveId,
        workId: work.workId,
        state: WarpState.READY,
        simdUnit,
        engine,
        stallCounter: 0,
        age: 0,
        vgprsUsed: Math.min(this._config.vgprPerSimd, 256),
      };
      this._wavefrontSlots.push(slot);
    }
  }

  // --- Execution ---

  /**
   * One cycle: schedule wavefronts, execute on SIMD units.
   *
   * The AMD CU scheduler uses LRR (Loose Round Robin) by default:
   * rotate through wavefronts, skip any that are stalled.
   */
  step(clockEdge: { cycle: number }): ComputeUnitTrace {
    this._cycle += 1;

    // Tick stall counters
    for (const slot of this._wavefrontSlots) {
      if (slot.stallCounter > 0) {
        slot.stallCounter -= 1;
        if (
          slot.stallCounter === 0 &&
          slot.state === WarpState.STALLED_MEMORY
        ) {
          slot.state = WarpState.READY;
        }
      }
      if (
        slot.state !== WarpState.COMPLETED &&
        slot.state !== WarpState.RUNNING
      ) {
        slot.age += 1;
      }
    }

    // Schedule: pick up to numSimdUnits wavefronts (one per SIMD unit)
    const engineTraces: Record<number, EngineTrace> = {};
    const schedulerActions: string[] = [];

    for (let simdId = 0; simdId < this._config.numSimdUnits; simdId++) {
      const ready = this._wavefrontSlots.filter(
        (w) => w.state === WarpState.READY && w.simdUnit === simdId,
      );
      if (ready.length === 0) {
        continue;
      }

      // LRR: pick oldest ready wavefront (approximation of LRR)
      let picked = ready[0];
      for (let i = 1; i < ready.length; i++) {
        if (ready[i].age > picked.age) {
          picked = ready[i];
        }
      }
      picked.state = WarpState.RUNNING;

      const trace = picked.engine.step(clockEdge);
      engineTraces[picked.waveId] = trace;

      schedulerActions.push(
        `SIMD${simdId}: issued wave ${picked.waveId}`,
      );
      picked.age = 0;

      // Update state after execution
      if (picked.engine.halted) {
        picked.state = WarpState.COMPLETED;
      } else if (this._isMemoryInstruction(trace)) {
        picked.state = WarpState.STALLED_MEMORY;
        picked.stallCounter = this._config.memoryLatencyCycles;
      } else {
        picked.state = WarpState.READY;
      }
    }

    if (schedulerActions.length === 0) {
      schedulerActions.push("all wavefronts stalled or completed");
    }

    const activeWaves = this._wavefrontSlots.filter(
      (w) => w.state !== WarpState.COMPLETED,
    ).length;
    const totalVgprs =
      this._config.vgprPerSimd * this._config.numSimdUnits;

    return makeComputeUnitTrace({
      cycle: this._cycle,
      unitName: this.name,
      architecture: this.architecture,
      schedulerAction: schedulerActions.join("; "),
      activeWarps: activeWaves,
      totalWarps: this._config.maxWavefronts,
      engineTraces,
      sharedMemoryUsed: this._ldsUsed,
      sharedMemoryTotal: this._config.ldsSize,
      registerFileUsed: this._vgprAllocated.reduce((a, b) => a + b, 0),
      registerFileTotal: totalVgprs,
      occupancy:
        this._config.maxWavefronts > 0
          ? activeWaves / this._config.maxWavefronts
          : 0.0,
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
    this._wavefrontSlots = [];
    this._lds.reset();
    this._ldsUsed = 0;
    this._vgprAllocated = new Array(this._config.numSimdUnits).fill(0);
    this._nextWaveId = 0;
    this._rrIndex = 0;
    this._cycle = 0;
  }

  // --- Private helpers ---

  private _isMemoryInstruction(trace: EngineTrace): boolean {
    const desc = trace.description.toUpperCase();
    return desc.includes("LOAD") || desc.includes("STORE");
  }

  toString(): string {
    const active = this._wavefrontSlots.filter(
      (w) => w.state !== WarpState.COMPLETED,
    ).length;
    return (
      `AMDComputeUnit(waves=${active}/${this._config.maxWavefronts}, ` +
      `occupancy=${(this.occupancy * 100).toFixed(1)}%)`
    );
  }
}
