/**
 * StreamingMultiprocessor -- NVIDIA SM simulator.
 *
 * === What is a Streaming Multiprocessor? ===
 *
 * The SM is the heart of NVIDIA's GPU architecture. Every NVIDIA GPU -- from
 * the GeForce in your laptop to the H100 in a data center -- is built from
 * SMs. Each SM is a self-contained compute unit that can independently
 * execute work without coordination with other SMs.
 *
 * An SM contains:
 * - **Warp schedulers** (4 on modern GPUs) that pick ready warps to execute
 * - **WarpEngines** (one per scheduler) that execute 32-thread warps
 * - **Register file** (256 KB, 65536 registers) partitioned among warps
 * - **Shared memory** (up to 228 KB) for inter-thread communication
 * - **L1 cache** (often shares capacity with shared memory)
 *
 * === The Key Innovation: Latency Hiding ===
 *
 * CPUs hide latency with deep pipelines, out-of-order execution, and branch
 * prediction -- complex hardware that's expensive in transistors and power.
 *
 * GPUs take the opposite approach: have MANY warps, and when one stalls,
 * switch to another. A single SM can have 48-64 warps resident. When warp 0
 * stalls on a memory access (~400 cycles), the scheduler instantly switches
 * to warp 1. By the time it has cycled through enough warps, warp 0's data
 * has arrived.
 *
 *     CPU strategy:  Make one thread FAST (deep pipeline, speculation, OoO)
 *     GPU strategy:  Have MANY threads, switch instantly to hide latency
 *
 * This is why GPUs have massive register files (256 KB vs 1-2 KB for a CPU
 * core) -- each of those 64 warps needs its own registers, and switching
 * between warps must be FREE (zero-cycle context switch).
 *
 * === Architecture Diagram ===
 *
 *     StreamingMultiprocessor
 *     +---------------------------------------------------------------+
 *     |                                                               |
 *     |  Warp Scheduler 0        Warp Scheduler 1                     |
 *     |  +------------------+   +------------------+                  |
 *     |  | w0: READY        |   | w1: STALLED      |                  |
 *     |  | w4: READY        |   | w5: READY        |                  |
 *     |  | w8: COMPLETED    |   | w9: RUNNING      |                  |
 *     |  +--------+---------+   +--------+---------+                  |
 *     |           |                      |                            |
 *     |           v                      v                            |
 *     |  +------------------+   +------------------+                  |
 *     |  | WarpEngine 0     |   | WarpEngine 1     |                  |
 *     |  | (32 threads)     |   | (32 threads)     |                  |
 *     |  +------------------+   +------------------+                  |
 *     |                                                               |
 *     |  Shared Resources:                                            |
 *     |  +-----------------------------------------------------------+|
 *     |  | Register File: 256 KB (65,536 x 32-bit registers)         ||
 *     |  | Shared Memory: 96 KB (configurable split with L1 cache)   ||
 *     |  +-----------------------------------------------------------+|
 *     +---------------------------------------------------------------+
 */

import { GenericISA, type Instruction, type InstructionSet } from "@coding-adventures/gpu-core";
import { FP32, type FloatFormat } from "@coding-adventures/fp-arithmetic";
import {
  WarpEngine,
  type WarpConfig,
  makeWarpConfig,
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

// ---------------------------------------------------------------------------
// SMConfig -- all tunable parameters for an NVIDIA-style SM
// ---------------------------------------------------------------------------

/**
 * Configuration for an NVIDIA-style Streaming Multiprocessor.
 *
 * Real-world SM configurations (for reference):
 *
 *     Parameter             | Volta (V100) | Ampere (A100) | Hopper (H100)
 *     ----------------------+--------------+---------------+--------------
 *     Warp schedulers       | 4            | 4             | 4
 *     Max warps per SM      | 64           | 64            | 64
 *     Max threads per SM    | 2048         | 2048          | 2048
 *     CUDA cores (FP32)     | 64           | 64            | 128
 *     Register file         | 256 KB       | 256 KB        | 256 KB
 *     Shared memory         | 96 KB        | 164 KB        | 228 KB
 *     L1 cache              | combined w/ shared mem
 *
 * Our default configuration models a Volta-class SM with reduced sizes
 * for faster simulation.
 */
export interface SMConfig {
  readonly numSchedulers: number;
  readonly warpWidth: number;
  readonly maxWarps: number;
  readonly maxThreads: number;
  readonly maxBlocks: number;
  readonly schedulingPolicy: SchedulingPolicy;
  readonly registerFileSize: number;
  readonly maxRegistersPerThread: number;
  readonly sharedMemorySize: number;
  readonly l1CacheSize: number;
  readonly instructionCacheSize: number;
  readonly floatFormat: FloatFormat;
  readonly isa: InstructionSet;
  readonly memoryLatencyCycles: number;
  readonly barrierEnabled: boolean;
}

/** Create an SMConfig with sensible defaults. */
export function makeSMConfig(partial: Partial<SMConfig> = {}): SMConfig {
  return {
    numSchedulers: 4,
    warpWidth: 32,
    maxWarps: 48,
    maxThreads: 1536,
    maxBlocks: 16,
    schedulingPolicy: SchedulingPolicy.GTO,
    registerFileSize: 65536,
    maxRegistersPerThread: 255,
    sharedMemorySize: 98304,
    l1CacheSize: 32768,
    instructionCacheSize: 131072,
    floatFormat: FP32,
    isa: new GenericISA(),
    memoryLatencyCycles: 200,
    barrierEnabled: true,
    ...partial,
  };
}

// ---------------------------------------------------------------------------
// WarpSlot -- tracks one warp's state in the scheduler
// ---------------------------------------------------------------------------

/**
 * One slot in the warp scheduler's table.
 *
 * Each WarpSlot tracks the state of one warp -- whether it's ready to
 * execute, stalled waiting for memory, completed, etc. The scheduler
 * scans these slots to find ready warps.
 *
 * === Warp Lifecycle ===
 *
 *     1. dispatch() creates a WarpSlot in READY state
 *     2. Scheduler picks it -> RUNNING
 *     3. After execution:
 *        - If LOAD/STORE: transition to STALLED_MEMORY for N cycles
 *        - If HALT: transition to COMPLETED
 *        - Otherwise: back to READY
 *     4. After stall countdown expires: back to READY
 */
export interface WarpSlot {
  readonly warpId: number;
  readonly workId: number;
  state: WarpState;
  readonly engine: WarpEngine;
  stallCounter: number;
  age: number;
  readonly registersUsed: number;
}

// ---------------------------------------------------------------------------
// WarpScheduler -- picks which warp to issue each cycle
// ---------------------------------------------------------------------------

/**
 * Warp scheduler that implements multiple scheduling policies.
 *
 * === How Warp Scheduling Works ===
 *
 * On each clock cycle, the scheduler:
 * 1. Scans all warp slots assigned to it
 * 2. Decrements stall counters for stalled warps
 * 3. Transitions warps whose stalls have resolved to READY
 * 4. Picks one READY warp according to the scheduling policy
 * 5. Returns that warp for execution
 *
 * === Scheduling Policies ===
 *
 * ROUND_ROBIN:
 *     Simply rotates through warps: 0, 1, 2, ..., wrap around.
 *     Skips non-READY warps. Fair but doesn't optimize for locality.
 *
 * GTO (Greedy-Then-Oldest):
 *     Keeps issuing from the same warp until it stalls, then picks
 *     the oldest ready warp. This improves cache locality because
 *     the same warp's instructions tend to access nearby memory.
 */
export class WarpScheduler {
  readonly schedulerId: number;
  readonly policy: SchedulingPolicy;
  private _warps: WarpSlot[] = [];
  private _rrIndex: number = 0;
  private _lastIssued: number | null = null;

  constructor(schedulerId: number, policy: SchedulingPolicy) {
    this.schedulerId = schedulerId;
    this.policy = policy;
  }

  /** The warp slots managed by this scheduler. */
  get warps(): readonly WarpSlot[] {
    return this._warps;
  }

  /** Add a warp to this scheduler's management. */
  addWarp(slot: WarpSlot): void {
    this._warps.push(slot);
  }

  /**
   * Decrement stall counters and transition stalled warps to READY.
   *
   * Called once per cycle before scheduling. Any warp whose stall
   * counter reaches 0 transitions back to READY.
   */
  tickStalls(): void {
    for (const warp of this._warps) {
      if (warp.stallCounter > 0) {
        warp.stallCounter -= 1;
        if (
          warp.stallCounter === 0 &&
          (warp.state === WarpState.STALLED_MEMORY ||
            warp.state === WarpState.STALLED_DEPENDENCY)
        ) {
          warp.state = WarpState.READY;
        }
      }

      // Age all non-completed warps (for OLDEST_FIRST / GTO)
      if (
        warp.state !== WarpState.COMPLETED &&
        warp.state !== WarpState.RUNNING
      ) {
        warp.age += 1;
      }
    }
  }

  /**
   * Select a ready warp according to the scheduling policy.
   *
   * @returns The selected WarpSlot, or null if no warps are ready.
   */
  pickWarp(): WarpSlot | null {
    const ready = this._warps.filter((w) => w.state === WarpState.READY);
    if (ready.length === 0) {
      return null;
    }

    switch (this.policy) {
      case SchedulingPolicy.ROUND_ROBIN:
        return this._pickRoundRobin(ready);
      case SchedulingPolicy.GTO:
        return this._pickGto(ready);
      case SchedulingPolicy.LRR:
        return this._pickLrr(ready);
      case SchedulingPolicy.OLDEST_FIRST:
        return this._pickOldestFirst(ready);
      case SchedulingPolicy.GREEDY:
        return this._pickOldestFirst(ready);
      default:
        return ready[0];
    }
  }

  /** Round-robin: rotate through warps in order. */
  private _pickRoundRobin(ready: WarpSlot[]): WarpSlot {
    const allIds = this._warps.map((w) => w.warpId);
    for (let i = 0; i < allIds.length; i++) {
      const idx = (this._rrIndex + i) % allIds.length;
      const targetId = allIds[idx];
      for (const w of ready) {
        if (w.warpId === targetId) {
          this._rrIndex = (idx + 1) % allIds.length;
          return w;
        }
      }
    }
    // Fallback
    return ready[0];
  }

  /** GTO: keep issuing same warp until it stalls, then oldest. */
  private _pickGto(ready: WarpSlot[]): WarpSlot {
    // Try to continue with the last issued warp
    if (this._lastIssued !== null) {
      for (const w of ready) {
        if (w.warpId === this._lastIssued) {
          return w;
        }
      }
    }
    // Last issued warp is not ready -- pick the oldest ready warp
    return this._pickOldestFirst(ready);
  }

  /** LRR (Loose Round Robin): round-robin but skip stalled warps. */
  private _pickLrr(ready: WarpSlot[]): WarpSlot {
    return this._pickRoundRobin(ready);
  }

  /** Oldest first: pick the warp that has been waiting longest. */
  private _pickOldestFirst(ready: WarpSlot[]): WarpSlot {
    let oldest = ready[0];
    for (let i = 1; i < ready.length; i++) {
      if (ready[i].age > oldest.age) {
        oldest = ready[i];
      }
    }
    return oldest;
  }

  /** Record that a warp was just issued (for GTO policy). */
  markIssued(warpId: number): void {
    this._lastIssued = warpId;
    for (const w of this._warps) {
      if (w.warpId === warpId) {
        w.age = 0;
        break;
      }
    }
  }

  /** Clear all warps from this scheduler. */
  reset(): void {
    this._warps = [];
    this._rrIndex = 0;
    this._lastIssued = null;
  }
}

// ---------------------------------------------------------------------------
// ResourceError -- raised when dispatch fails due to resource limits
// ---------------------------------------------------------------------------

/**
 * Raised when a compute unit cannot accommodate a work item.
 *
 * This happens when the SM doesn't have enough registers, shared memory,
 * or warp slots to fit the requested thread block. In real CUDA, this
 * would manifest as a launch failure or reduced occupancy.
 */
export class ResourceError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ResourceError";
  }
}

// ---------------------------------------------------------------------------
// StreamingMultiprocessor -- the main SM simulator
// ---------------------------------------------------------------------------

/**
 * NVIDIA Streaming Multiprocessor simulator.
 *
 * Manages multiple warps executing thread blocks, with a configurable
 * warp scheduler, shared memory, and register file partitioning.
 *
 * === Usage Pattern ===
 *
 *     1. Create SM with config
 *     2. Dispatch one or more WorkItems (thread blocks)
 *     3. Call step() or run() to simulate execution
 *     4. Read traces to understand what happened
 *
 * === How dispatch() Works ===
 *
 * When a thread block is dispatched to the SM:
 *
 *     1. Check resources: enough registers? shared memory? warp slots?
 *     2. Decompose the block into warps (every 32 threads = 1 warp)
 *     3. Allocate registers for each warp
 *     4. Reserve shared memory for the block
 *     5. Create WarpEngine instances for each warp
 *     6. Add warp slots to the schedulers (round-robin distribution)
 *
 * === How step() Works ===
 *
 * On each clock cycle:
 *
 *     1. Tick stall counters (memory latency countdown)
 *     2. Each scheduler picks one ready warp (using scheduling policy)
 *     3. Execute picked warps on their WarpEngines
 *     4. Check for memory instructions -> stall the warp
 *     5. Check for HALT -> mark warp as completed
 *     6. Build and return a ComputeUnitTrace
 */
export class StreamingMultiprocessor {
  private _config: SMConfig;
  private _cycle: number = 0;
  private _sharedMemory: SharedMemory;
  private _sharedMemoryUsed: number = 0;
  private _registersAllocated: number = 0;
  private _schedulers: WarpScheduler[];
  private _allWarpSlots: WarpSlot[] = [];
  private _nextWarpId: number = 0;
  private _activeBlocks: number[] = [];

  constructor(config: SMConfig) {
    this._config = config;
    this._sharedMemory = new SharedMemory(config.sharedMemorySize);
    this._schedulers = [];
    for (let i = 0; i < config.numSchedulers; i++) {
      this._schedulers.push(new WarpScheduler(i, config.schedulingPolicy));
    }
  }

  // --- Properties ---

  /** Compute unit name. */
  get name(): string {
    return "SM";
  }

  /** This is an NVIDIA SM. */
  get architecture(): Architecture {
    return Architecture.NVIDIA_SM;
  }

  /** True if no active warps remain. */
  get idle(): boolean {
    return (
      this._allWarpSlots.length === 0 ||
      this._allWarpSlots.every((w) => w.state === WarpState.COMPLETED)
    );
  }

  /**
   * Current occupancy: active (non-completed) warps / max warps.
   *
   * Occupancy is the key performance metric for GPU kernels. Low
   * occupancy means the SM can't hide memory latency because there
   * aren't enough warps to switch between when one stalls.
   */
  get occupancy(): number {
    if (this._config.maxWarps === 0) {
      return 0.0;
    }
    const active = this._allWarpSlots.filter(
      (w) => w.state !== WarpState.COMPLETED,
    ).length;
    return active / this._config.maxWarps;
  }

  /** The SM configuration. */
  get config(): SMConfig {
    return this._config;
  }

  /** Access to the shared memory instance. */
  get sharedMemory(): SharedMemory {
    return this._sharedMemory;
  }

  /** All warp slots (for inspection). */
  get warpSlots(): readonly WarpSlot[] {
    return this._allWarpSlots;
  }

  // --- Occupancy calculation ---

  /**
   * Calculate theoretical occupancy for a kernel launch configuration.
   *
   * This is the STATIC occupancy calculation -- how full the SM could
   * theoretically be, given the resource requirements of a kernel.
   *
   * === How Occupancy is Limited ===
   *
   * Occupancy is limited by the tightest constraint among:
   *
   * 1. Register pressure:
   *    Each warp needs registersPerThread * 32 registers.
   *    Total warps = registerFileSize / regsPerWarp.
   *
   * 2. Shared memory:
   *    Each block needs sharedMemPerBlock bytes.
   *    Max blocks = sharedMemorySize / sharedMemPerBlock.
   *    Max warps = maxBlocks * warpsPerBlock.
   *
   * 3. Hardware limit:
   *    The SM simply can't hold more than maxWarps warps.
   */
  computeOccupancy(
    registersPerThread: number,
    sharedMemPerBlock: number,
    threadsPerBlock: number,
  ): number {
    const warpW = this._config.warpWidth;
    const warpsPerBlock = Math.ceil(threadsPerBlock / warpW);

    // Limit 1: register file
    const regsPerWarp = registersPerThread * this._config.warpWidth;
    const maxWarpsByRegs =
      regsPerWarp > 0
        ? Math.floor(this._config.registerFileSize / regsPerWarp)
        : this._config.maxWarps;

    // Limit 2: shared memory
    let maxWarpsBySmem: number;
    if (sharedMemPerBlock > 0) {
      const maxBlocksBySmem = Math.floor(
        this._config.sharedMemorySize / sharedMemPerBlock,
      );
      maxWarpsBySmem = maxBlocksBySmem * warpsPerBlock;
    } else {
      maxWarpsBySmem = this._config.maxWarps;
    }

    // Limit 3: hardware limit
    const maxWarpsByHw = this._config.maxWarps;

    // Actual occupancy is limited by the tightest constraint
    const activeWarps = Math.min(maxWarpsByRegs, maxWarpsBySmem, maxWarpsByHw);
    return Math.min(activeWarps / this._config.maxWarps, 1.0);
  }

  // --- Dispatch ---

  /**
   * Dispatch a thread block to this SM.
   *
   * Decomposes the thread block into warps, allocates registers and
   * shared memory, creates WarpEngine instances, and adds warp slots
   * to the schedulers.
   *
   * @throws ResourceError if not enough resources for this work item.
   */
  dispatch(work: WorkItem): void {
    const numWarps = Math.ceil(
      work.threadCount / this._config.warpWidth,
    );
    const regsNeeded =
      work.registersPerThread * this._config.warpWidth * numWarps;
    const smemNeeded = work.sharedMemBytes;

    // Check resource availability
    const currentActive = this._allWarpSlots.filter(
      (w) => w.state !== WarpState.COMPLETED,
    ).length;

    if (currentActive + numWarps > this._config.maxWarps) {
      throw new ResourceError(
        `Not enough warp slots: need ${numWarps}, ` +
          `available ${this._config.maxWarps - currentActive}`,
      );
    }

    if (
      this._registersAllocated + regsNeeded >
      this._config.registerFileSize
    ) {
      const availRegs =
        this._config.registerFileSize - this._registersAllocated;
      throw new ResourceError(
        `Not enough registers: need ${regsNeeded}, ` +
          `available ${availRegs}`,
      );
    }

    if (
      this._sharedMemoryUsed + smemNeeded >
      this._config.sharedMemorySize
    ) {
      const availSmem =
        this._config.sharedMemorySize - this._sharedMemoryUsed;
      throw new ResourceError(
        `Not enough shared memory: need ${smemNeeded}, ` +
          `available ${availSmem}`,
      );
    }

    // Allocate resources
    this._registersAllocated += regsNeeded;
    this._sharedMemoryUsed += smemNeeded;
    this._activeBlocks.push(work.workId);

    // Create warps and distribute across schedulers
    for (let warpIdx = 0; warpIdx < numWarps; warpIdx++) {
      const warpId = this._nextWarpId;
      this._nextWarpId += 1;

      // Determine thread range for this warp
      const threadStart = warpIdx * this._config.warpWidth;
      const threadEnd = Math.min(
        threadStart + this._config.warpWidth,
        work.threadCount,
      );
      const actualThreads = threadEnd - threadStart;

      // Create a WarpEngine for this warp
      const engine = new WarpEngine(
        makeWarpConfig({
          warpWidth: actualThreads,
          numRegisters: work.registersPerThread,
          floatFormat: this._config.floatFormat,
          isa: this._config.isa,
        }),
      );

      // Load program if provided
      if (work.program !== null) {
        engine.loadProgram(work.program);
      }

      // Set per-thread data if provided
      for (let tOffset = 0; tOffset < actualThreads; tOffset++) {
        const globalTid = threadStart + tOffset;
        if (globalTid in work.perThreadData) {
          const regs = work.perThreadData[globalTid];
          for (const regStr of Object.keys(regs)) {
            const reg = Number(regStr);
            engine.setThreadRegister(tOffset, reg, regs[reg]);
          }
        }
      }

      // Create the warp slot
      const slot: WarpSlot = {
        warpId,
        workId: work.workId,
        state: WarpState.READY,
        engine,
        stallCounter: 0,
        age: 0,
        registersUsed: work.registersPerThread * actualThreads,
      };
      this._allWarpSlots.push(slot);

      // Distribute to schedulers round-robin
      const schedIdx = warpIdx % this._config.numSchedulers;
      this._schedulers[schedIdx].addWarp(slot);
    }
  }

  // --- Execution ---

  /**
   * One cycle: schedulers pick warps, engines execute, stalls update.
   *
   * === Step-by-Step ===
   *
   * 1. Tick stall counters on all schedulers. Warps whose memory
   *    latency countdown has expired transition back to READY.
   *
   * 2. Each scheduler independently picks one ready warp using the
   *    configured scheduling policy (GTO, ROUND_ROBIN, etc.).
   *
   * 3. Execute the picked warps on their WarpEngines. Each engine
   *    advances one instruction across all 32 threads.
   *
   * 4. Check execution results:
   *    - If the instruction was LOAD or STORE, stall the warp for
   *      memoryLatencyCycles.
   *    - If the instruction was HALT, mark the warp as COMPLETED.
   *    - Otherwise, transition back to READY.
   *
   * 5. Build a ComputeUnitTrace capturing the full state of the SM.
   */
  step(clockEdge: { cycle: number }): ComputeUnitTrace {
    this._cycle += 1;

    // Phase 1: Tick stall counters
    for (const sched of this._schedulers) {
      sched.tickStalls();
    }

    // Phase 2: Each scheduler picks a warp and executes it
    const engineTraces: Record<number, EngineTrace> = {};
    const schedulerActions: string[] = [];

    for (const sched of this._schedulers) {
      const picked = sched.pickWarp();
      if (picked === null) {
        schedulerActions.push(
          `S${sched.schedulerId}: no ready warp`,
        );
        continue;
      }

      // Mark as running
      picked.state = WarpState.RUNNING;

      // Execute one cycle on the warp's engine
      const trace = picked.engine.step(clockEdge);
      engineTraces[picked.warpId] = trace;

      // Record the scheduling decision
      sched.markIssued(picked.warpId);
      schedulerActions.push(
        `S${sched.schedulerId}: issued warp ${picked.warpId}`,
      );

      // Phase 3: Check execution results and update warp state
      if (picked.engine.halted) {
        picked.state = WarpState.COMPLETED;
      } else if (this._isMemoryInstruction(trace)) {
        // Stall for memory latency
        picked.state = WarpState.STALLED_MEMORY;
        picked.stallCounter = this._config.memoryLatencyCycles;
      } else {
        picked.state = WarpState.READY;
      }
    }

    // Build the trace
    const activeWarps = this._allWarpSlots.filter(
      (w) => w.state !== WarpState.COMPLETED,
    ).length;
    const totalWarps = this._config.maxWarps;

    return makeComputeUnitTrace({
      cycle: this._cycle,
      unitName: this.name,
      architecture: this.architecture,
      schedulerAction: schedulerActions.join("; "),
      activeWarps,
      totalWarps,
      engineTraces,
      sharedMemoryUsed: this._sharedMemoryUsed,
      sharedMemoryTotal: this._config.sharedMemorySize,
      registerFileUsed: this._registersAllocated,
      registerFileTotal: this._config.registerFileSize,
      occupancy: totalWarps > 0 ? activeWarps / totalWarps : 0.0,
    });
  }

  /**
   * Run until all work completes or maxCycles.
   *
   * Creates clock edges internally to drive execution.
   */
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

  /** Reset all state: engines, schedulers, shared memory. */
  reset(): void {
    for (const sched of this._schedulers) {
      sched.reset();
    }
    this._allWarpSlots = [];
    this._sharedMemory.reset();
    this._sharedMemoryUsed = 0;
    this._registersAllocated = 0;
    this._activeBlocks = [];
    this._nextWarpId = 0;
    this._cycle = 0;
  }

  // --- Private helpers ---

  /**
   * Check if the executed instruction was a memory operation.
   *
   * Memory operations (LOAD/STORE) stall the warp for
   * memoryLatencyCycles to simulate global memory latency.
   */
  private _isMemoryInstruction(trace: EngineTrace): boolean {
    const desc = trace.description.toUpperCase();
    return desc.includes("LOAD") || desc.includes("STORE");
  }

  toString(): string {
    const active = this._allWarpSlots.filter(
      (w) => w.state !== WarpState.COMPLETED,
    ).length;
    return (
      `StreamingMultiprocessor(warps=${active}/${this._config.maxWarps}, ` +
      `occupancy=${(this.occupancy * 100).toFixed(1)}%, ` +
      `policy=${this._config.schedulingPolicy})`
    );
  }
}
