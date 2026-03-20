/**
 * WarpEngine -- SIMT parallel execution (NVIDIA CUDA / ARM Mali style).
 *
 * === What is SIMT? ===
 *
 * SIMT stands for "Single Instruction, Multiple Threads." NVIDIA invented this
 * term to describe how their GPU cores work. It's a hybrid between two older
 * concepts:
 *
 *     SISD (one instruction, one datum):
 *         Like a single CPU core. Our gpu-core package at Layer 9.
 *
 *     SIMD (one instruction, multiple data):
 *         Like AMD wavefronts. One instruction operates on a wide vector.
 *         There are no "threads" -- just lanes in a vector ALU.
 *
 *     SIMT (one instruction, multiple threads):
 *         Like NVIDIA warps. Multiple threads, each with its own registers
 *         and (logically) its own program counter. They USUALLY execute
 *         the same instruction, but CAN diverge.
 *
 * The key difference between SIMD and SIMT:
 *
 *     SIMD: "I have one wide ALU that processes 32 numbers at once."
 *     SIMT: "I have 32 tiny threads that happen to execute in lockstep."
 *
 * This distinction matters when threads need to take different paths (branches).
 * In SIMD, you just mask off lanes. In SIMT, the hardware manages a divergence
 * stack to serialize the paths and then reconverge.
 *
 * === How a Warp Works ===
 *
 * A warp is a group of threads (32 for NVIDIA, 16 for ARM Mali) that the
 * hardware schedules together. On each clock cycle:
 *
 *     1. The warp scheduler picks one instruction (at the warp's PC).
 *     2. That instruction is issued to ALL active threads simultaneously.
 *     3. Each thread executes the instruction on its OWN registers.
 *     4. If the instruction is a branch, threads may diverge.
 *
 *     +-----------------------------------------------------+
 *     |  Warp (32 threads)                                   |
 *     |                                                      |
 *     |  Active Mask: [1,1,1,1,1,1,1,1,...,1,1,1,1]          |
 *     |  PC: 0x004                                           |
 *     |                                                      |
 *     |  +------+ +------+ +------+       +------+           |
 *     |  | T0   | | T1   | | T2   |  ...  | T31  |           |
 *     |  |R0=1.0| |R0=2.0| |R0=3.0|       |R0=32.|           |
 *     |  |R1=0.5| |R1=0.5| |R1=0.5|       |R1=0.5|           |
 *     |  +------+ +------+ +------+       +------+           |
 *     |                                                      |
 *     |  Instruction: FMUL R2, R0, R1                        |
 *     |  Result: T0.R2=0.5, T1.R2=1.0, T2.R2=1.5, ...       |
 *     +-----------------------------------------------------+
 *
 * === Divergence: The Price of Flexibility ===
 *
 * When threads in a warp encounter a branch and disagree on which way to go,
 * the warp "diverges." The hardware serializes the paths:
 *
 *     Step 1: Evaluate the branch condition for ALL threads.
 *     Step 2: Threads that go "true" -> execute first (others masked off).
 *     Step 3: Push (reconvergence_pc, other_mask) onto the divergence stack.
 *     Step 4: When "true" path finishes, pop the stack.
 *     Step 5: Execute the "false" path (first group masked off).
 *     Step 6: At the reconvergence point, all threads are active again.
 *
 *     Example with 4 threads:
 *
 *     if (thread_id < 2):    Mask: [1,1,0,0]  <- threads 0,1 take true path
 *         path_A()           Only threads 0,1 execute
 *     else:                  Mask: [0,0,1,1]  <- threads 2,3 take false path
 *         path_B()           Only threads 2,3 execute
 *     // reconverge          Mask: [1,1,1,1]  <- all 4 threads active again
 *
 * This means divergent branches effectively halve your throughput -- the warp
 * runs both paths sequentially instead of simultaneously.
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
  type GPUCoreOptions,
} from "@coding-adventures/gpu-core";

import {
  type DivergenceInfo,
  type EngineTrace,
  ExecutionModel,
} from "./protocols.js";

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/**
 * Configuration for a SIMT warp engine.
 *
 * Real-world reference values:
 *
 *     Vendor      | Warp Width | Registers | Memory     | Max Divergence
 *     ------------+------------+-----------+------------+---------------
 *     NVIDIA      | 32         | 255       | 512 KB     | 32+ levels
 *     ARM Mali    | 16         | 64        | varies     | 16+ levels
 *     Our default | 32         | 32        | 1024 B     | 32 levels
 */
export interface WarpConfig {
  readonly warpWidth: number;
  readonly numRegisters: number;
  readonly memoryPerThread: number;
  readonly floatFormat: FloatFormat;
  readonly maxDivergenceDepth: number;
  readonly isa: InstructionSet;
  readonly independentThreadScheduling: boolean;
}

/**
 * Create a WarpConfig with sensible defaults.
 */
export function makeWarpConfig(partial: Partial<WarpConfig> = {}): WarpConfig {
  return {
    warpWidth: 32,
    numRegisters: 32,
    memoryPerThread: 1024,
    floatFormat: FP32,
    maxDivergenceDepth: 32,
    isa: new GenericISA(),
    independentThreadScheduling: false,
    ...partial,
  };
}

// ---------------------------------------------------------------------------
// Per-thread context
// ---------------------------------------------------------------------------

/**
 * Per-thread execution context in a SIMT warp.
 *
 * Each thread in the warp has:
 * - threadId: its position in the warp (0 to warpWidth-1)
 * - core: a full GPUCore instance with its own registers and memory
 * - active: whether this thread is currently executing (false = masked off)
 * - pc: per-thread program counter (used in independent scheduling mode)
 *
 * In NVIDIA hardware, each CUDA thread has 255 registers. In our simulator,
 * each thread gets a full GPUCore instance, which is heavier but lets us
 * reuse all the existing instruction execution infrastructure.
 */
export interface ThreadContext {
  readonly threadId: number;
  readonly core: GPUCore;
  active: boolean;
  pc: number;
}

// ---------------------------------------------------------------------------
// Divergence stack entry
// ---------------------------------------------------------------------------

/**
 * One entry on the divergence stack.
 *
 * When threads diverge at a branch, we push an entry recording:
 * - reconvergencePc: where threads should rejoin
 * - savedMask: which threads took the OTHER path (will run later)
 *
 * This is the pre-Volta divergence handling mechanism. The stack allows
 * nested divergence -- if threads diverge again while already diverged,
 * another entry is pushed.
 *
 * Divergence stack example (4 threads, nested branches):
 *
 *     Stack (top -> bottom):
 *     +--------------------------------------------+
 *     | reconvergencePc=10, savedMask=[0,0,1,0]    |  <- inner branch
 *     +--------------------------------------------+
 *     | reconvergencePc=20, savedMask=[0,0,0,1]    |  <- outer branch
 *     +--------------------------------------------+
 */
export interface DivergenceStackEntry {
  readonly reconvergencePc: number;
  readonly savedMask: readonly boolean[];
}

// ---------------------------------------------------------------------------
// WarpEngine -- the SIMT parallel execution engine
// ---------------------------------------------------------------------------

/**
 * SIMT warp execution engine (NVIDIA CUDA / ARM Mali style).
 *
 * Manages N threads executing in lockstep with hardware divergence support.
 * Each thread is backed by a real GPUCore instance from the gpu-core package.
 *
 * === Usage Pattern ===
 *
 *     1. Create engine with config
 *     2. Load program (same program goes to all threads)
 *     3. Set per-thread register values (give each thread different data)
 *     4. Step or run (engine issues instructions to all active threads)
 *     5. Read results from per-thread registers
 *
 * Example:
 *     import { WarpEngine, makeWarpConfig } from "@coding-adventures/parallel-execution-engine";
 *     import { limm, fmul, halt } from "@coding-adventures/gpu-core";
 *     const engine = new WarpEngine(makeWarpConfig({ warpWidth: 4 }));
 *     engine.loadProgram([limm(0, 2.0), limm(1, 3.0), fmul(2, 0, 1), halt()]);
 *     const traces = engine.run();
 *     engine.threads[0].core.registers.readFloat(2);  // 6.0
 */
export class WarpEngine {
  private readonly _config: WarpConfig;
  private _cycle: number = 0;
  private _program: Instruction[] = [];
  private _threads: ThreadContext[];
  private _divergenceStack: DivergenceStackEntry[] = [];
  private _allHalted: boolean = false;

  constructor(config: WarpConfig) {
    this._config = config;

    // Create one GPUCore per thread. Each thread is an independent
    // processing element with its own registers and local memory.
    this._threads = Array.from({ length: config.warpWidth }, (_, i) => ({
      threadId: i,
      core: new GPUCore({
        isa: config.isa,
        fmt: config.floatFormat,
        numRegisters: config.numRegisters,
        memorySize: config.memoryPerThread,
      }),
      active: true,
      pc: 0,
    }));
  }

  // --- Properties ---

  /** Engine name for traces. */
  get name(): string {
    return "WarpEngine";
  }

  /** Number of threads in this warp. */
  get width(): number {
    return this._config.warpWidth;
  }

  /** This is a SIMT engine. */
  get executionModel(): ExecutionModel {
    return ExecutionModel.SIMT;
  }

  /** Access to per-thread contexts (for reading results). */
  get threads(): readonly ThreadContext[] {
    return this._threads;
  }

  /** Which threads are currently active (not masked off). */
  get activeMask(): boolean[] {
    return this._threads.map((t) => t.active);
  }

  /** True if ALL threads have executed a HALT instruction. */
  get halted(): boolean {
    return this._allHalted;
  }

  /** The configuration this engine was created with. */
  get config(): WarpConfig {
    return this._config;
  }

  // --- Program loading ---

  /**
   * Load the same program into all threads.
   *
   * In real NVIDIA hardware, all threads in a warp share the same
   * instruction memory. We simulate this by loading the same program
   * into each thread's GPUCore.
   */
  loadProgram(program: Instruction[]): void {
    this._program = [...program];
    for (const thread of this._threads) {
      thread.core.loadProgram(this._program);
      thread.active = true;
      thread.pc = 0;
    }
    this._allHalted = false;
    this._cycle = 0;
    this._divergenceStack = [];
  }

  // --- Per-thread register setup ---

  /**
   * Set a register value for a specific thread.
   *
   * This is how you give each thread different data to work on.
   * In a real GPU kernel, each thread would compute its global index
   * and use it to load different data from memory. In our simulator,
   * we pre-load the data into registers.
   */
  setThreadRegister(threadId: number, reg: number, value: number): void {
    if (threadId < 0 || threadId >= this._config.warpWidth) {
      throw new RangeError(
        `Thread ID ${threadId} out of range [0, ${this._config.warpWidth})`,
      );
    }
    this._threads[threadId].core.registers.writeFloat(reg, value);
  }

  // --- Execution ---

  /**
   * Execute one cycle: issue one instruction to all active threads.
   *
   * On each step:
   * 1. Find the instruction at the current warp PC.
   * 2. Issue it to all active (non-masked) threads.
   * 3. Detect divergence on branch instructions.
   * 4. Handle reconvergence when appropriate.
   * 5. Build and return an EngineTrace.
   */
  step(clockEdge: { cycle: number }): EngineTrace {
    this._cycle += 1;

    // If all halted, produce a no-op trace
    if (this._allHalted) {
      return this._makeHaltedTrace();
    }

    // Check for reconvergence
    this._checkReconvergence();

    // Find active, non-halted threads
    const activeThreads = this._threads.filter(
      (t) => t.active && !t.core.halted,
    );

    if (activeThreads.length === 0) {
      // All threads are either halted or masked off.
      // Check if we need to pop the divergence stack.
      if (this._divergenceStack.length > 0) {
        return this._popDivergenceAndTrace();
      }
      this._allHalted = true;
      return this._makeHaltedTrace();
    }

    // Save pre-step mask for divergence tracking
    const maskBefore = this._threads.map((t) => t.active);

    // Execute the instruction on all active, non-halted threads
    const unitTraces: Record<number, string> = {};
    const branchTakenThreads: number[] = [];
    const branchNotTakenThreads: number[] = [];

    for (const thread of this._threads) {
      if (thread.active && !thread.core.halted) {
        try {
          const trace = thread.core.step();
          unitTraces[thread.threadId] = trace.description;

          // Detect branch divergence: check if different threads
          // ended up at different PCs after a branch instruction.
          if (trace.nextPc !== trace.pc + 1 && !trace.halted) {
            branchTakenThreads.push(thread.threadId);
          } else if (!trace.halted) {
            branchNotTakenThreads.push(thread.threadId);
          }

          if (trace.halted) {
            unitTraces[thread.threadId] = "HALTED";
          }
        } catch {
          thread.active = false;
          unitTraces[thread.threadId] = "(error -- deactivated)";
        }
      } else if (thread.core.halted) {
        unitTraces[thread.threadId] = "(halted)";
      } else {
        unitTraces[thread.threadId] = "(masked off)";
      }
    }

    // Handle divergence: if some threads branched and others didn't,
    // we have divergence.
    let divergenceInfo: DivergenceInfo | null = null;
    if (branchTakenThreads.length > 0 && branchNotTakenThreads.length > 0) {
      divergenceInfo = this._handleDivergence(
        branchTakenThreads,
        branchNotTakenThreads,
        maskBefore,
      );
    }

    // Check if all threads are now halted
    if (this._threads.every((t) => t.core.halted)) {
      this._allHalted = true;
    }

    // Build the trace
    const currentMask = this._threads.map(
      (t) => t.active && !t.core.halted,
    );
    const activeCount = currentMask.filter(Boolean).length;
    const total = this._config.warpWidth;

    // Get a description from the first active instruction
    const skipStates = new Set([
      "(masked off)",
      "(halted)",
      "(error -- deactivated)",
    ]);
    let firstActiveDesc = "no active threads";
    for (const thread of this._threads) {
      const desc = unitTraces[thread.threadId];
      if (desc !== undefined && !skipStates.has(desc)) {
        firstActiveDesc = desc;
        break;
      }
    }

    return {
      cycle: this._cycle,
      engineName: this.name,
      executionModel: this.executionModel,
      description: `${firstActiveDesc} -- ${activeCount}/${total} threads active`,
      unitTraces,
      activeMask: currentMask,
      activeCount,
      totalCount: total,
      utilization: total > 0 ? activeCount / total : 0.0,
      divergenceInfo,
    };
  }

  /**
   * Run until all threads halt or maxCycles reached.
   *
   * Creates clock edges internally to drive execution. Each cycle
   * produces one EngineTrace.
   */
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
      throw new Error(`WarpEngine: max_cycles (${maxCycles}) reached`);
    }
    return traces;
  }

  /**
   * Reset the engine to its initial state.
   *
   * Resets all thread cores, reactivates all threads, clears the
   * divergence stack, and reloads the program (if one was loaded).
   */
  reset(): void {
    for (const thread of this._threads) {
      thread.core.reset();
      thread.active = true;
      thread.pc = 0;
      if (this._program.length > 0) {
        thread.core.loadProgram(this._program);
      }
    }
    this._divergenceStack = [];
    this._allHalted = false;
    this._cycle = 0;
  }

  // --- Divergence handling (private) ---

  /**
   * Handle a divergent branch by pushing onto the divergence stack.
   */
  private _handleDivergence(
    takenThreads: number[],
    notTakenThreads: number[],
    maskBefore: boolean[],
  ): DivergenceInfo {
    // The reconvergence PC is the maximum PC among all active threads
    // after the branch. This is a simplified heuristic.
    const allPcs = [...takenThreads, ...notTakenThreads].map(
      (tid) => this._threads[tid].core.pc,
    );
    const reconvergencePc = Math.max(...allPcs);

    // Build the saved mask: threads that took the "not taken" path
    const savedMask = Array.from(
      { length: this._config.warpWidth },
      () => false,
    );
    for (const tid of notTakenThreads) {
      savedMask[tid] = true;
      this._threads[tid].active = false;
    }

    // Push onto the divergence stack
    if (this._divergenceStack.length < this._config.maxDivergenceDepth) {
      this._divergenceStack.push({
        reconvergencePc,
        savedMask,
      });
    }

    const maskAfter = this._threads.map((t) => t.active);

    return {
      activeMaskBefore: maskBefore,
      activeMaskAfter: maskAfter,
      reconvergencePc,
      divergenceDepth: this._divergenceStack.length,
    };
  }

  /**
   * Check if active threads have reached a reconvergence point.
   */
  private _checkReconvergence(): void {
    if (this._divergenceStack.length === 0) return;

    const entry = this._divergenceStack[this._divergenceStack.length - 1];
    const activeThreads = this._threads.filter(
      (t) => t.active && !t.core.halted,
    );

    if (activeThreads.length === 0) return;

    // Check if all active threads have reached the reconvergence PC
    const allAtReconvergence = activeThreads.every(
      (t) => t.core.pc >= entry.reconvergencePc,
    );

    if (allAtReconvergence) {
      this._divergenceStack.pop();
      // Reactivate the saved threads
      for (let tid = 0; tid < entry.savedMask.length; tid++) {
        if (entry.savedMask[tid] && !this._threads[tid].core.halted) {
          this._threads[tid].active = true;
        }
      }
    }
  }

  /**
   * Pop the divergence stack and produce a trace for the switch.
   */
  private _popDivergenceAndTrace(): EngineTrace {
    const entry = this._divergenceStack.pop()!;

    // Reactivate saved threads
    for (let tid = 0; tid < entry.savedMask.length; tid++) {
      if (entry.savedMask[tid] && !this._threads[tid].core.halted) {
        this._threads[tid].active = true;
      }
    }

    const currentMask = this._threads.map(
      (t) => t.active && !t.core.halted,
    );
    const activeCount = currentMask.filter(Boolean).length;

    return {
      cycle: this._cycle,
      engineName: this.name,
      executionModel: this.executionModel,
      description: `Divergence stack pop -- reactivated ${activeCount} threads`,
      unitTraces: Object.fromEntries(
        this._threads.map((t) => [
          t.threadId,
          entry.savedMask[t.threadId] ? "reactivated" : "(waiting)",
        ]),
      ),
      activeMask: currentMask,
      activeCount,
      totalCount: this._config.warpWidth,
      utilization:
        this._config.warpWidth > 0
          ? activeCount / this._config.warpWidth
          : 0.0,
    };
  }

  /** Produce a trace for when all threads are halted. */
  private _makeHaltedTrace(): EngineTrace {
    return {
      cycle: this._cycle,
      engineName: this.name,
      executionModel: this.executionModel,
      description: "All threads halted",
      unitTraces: Object.fromEntries(
        this._threads.map((t) => [t.threadId, "(halted)"]),
      ),
      activeMask: Array.from(
        { length: this._config.warpWidth },
        () => false,
      ),
      activeCount: 0,
      totalCount: this._config.warpWidth,
      utilization: 0.0,
    };
  }

  /** String representation for debugging. */
  toString(): string {
    const active = this._threads.filter((t) => t.active).length;
    const halted = this._threads.filter((t) => t.core.halted).length;
    return (
      `WarpEngine(width=${this._config.warpWidth}, ` +
      `active=${active}, halted_threads=${halted}, ` +
      `divergence_depth=${this._divergenceStack.length})`
    );
  }
}
