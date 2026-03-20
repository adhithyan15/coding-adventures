/**
 * Protocols -- shared types for all compute unit simulators.
 *
 * === What is a Compute Unit? ===
 *
 * A compute unit is the organizational structure that wraps execution engines
 * (Layer 8) with scheduling, shared memory, register files, and caches to form
 * a complete computational building block. Think of it as the "factory floor"
 * analogy from the spec:
 *
 *     Workers         = execution engines (warps, wavefronts, systolic arrays)
 *     Floor manager   = warp/wavefront scheduler
 *     Shared toolbox  = shared memory / LDS (data accessible to all teams)
 *     Supply closet   = L1 cache (recent data kept nearby)
 *     Filing cabinets = register file (massive, partitioned among teams)
 *     Work orders     = thread blocks / work groups queued for execution
 *
 * Every vendor has a different name for this level of the hierarchy:
 *
 *     NVIDIA:   Streaming Multiprocessor (SM)
 *     AMD:      Compute Unit (CU) / Work Group Processor (WGP in RDNA)
 *     Intel:    Xe Core (or Subslice in older gen)
 *     Google:   Matrix Multiply Unit (MXU) + Vector/Scalar units
 *     Apple:    Neural Engine Core
 *
 * Despite the naming differences, they all serve the same purpose: take
 * execution engines, add scheduling and shared resources, and present a
 * coherent compute unit to the device layer above.
 *
 * === Protocol-Based Design ===
 *
 * Just like Layer 8 (parallel-execution-engine), we use TypeScript interfaces
 * to define a common contract that all compute units implement. This allows
 * higher layers to drive any compute unit uniformly, regardless of vendor.
 *
 * An interface in TypeScript is structural (duck-typed): if an object has the
 * right methods and properties, it satisfies the interface -- no explicit
 * "implements" keyword required. This is structural subtyping: if it looks
 * like a compute unit and steps like a compute unit, it IS a compute unit.
 */

import type { Instruction } from "@coding-adventures/gpu-core";
import type { EngineTrace } from "@coding-adventures/parallel-execution-engine";

// ---------------------------------------------------------------------------
// Architecture -- which vendor's compute unit this is
// ---------------------------------------------------------------------------

/**
 * Vendor architectures supported at the compute unit level.
 *
 * Each architecture represents a fundamentally different approach to
 * organizing parallel computation. They are NOT interchangeable -- each
 * has unique scheduling strategies, memory hierarchies, and execution
 * models.
 *
 * Comparison table:
 *
 *     Architecture      | Scheduling    | Memory Model  | Execution
 *     ------------------+---------------+---------------+--------------
 *     NVIDIA SM         | Warp sched.   | Shared mem    | SIMT warps
 *     AMD CU            | Wave sched.   | LDS           | SIMD wavefronts
 *     Google MXU        | Compile-time  | Weight buffer | Systolic array
 *     Intel Xe Core     | Thread disp.  | SLM           | SIMD + threads
 *     Apple ANE Core    | Compiler      | SRAM + DMA    | Scheduled MAC
 */
export enum Architecture {
  NVIDIA_SM = "nvidia_sm",
  AMD_CU = "amd_cu",
  GOOGLE_MXU = "google_mxu",
  INTEL_XE_CORE = "intel_xe_core",
  APPLE_ANE_CORE = "apple_ane_core",
}

// ---------------------------------------------------------------------------
// WarpState -- possible states of a warp in the scheduler
// ---------------------------------------------------------------------------

/**
 * Possible states of a warp (or wavefront, or thread) in the scheduler.
 *
 * A warp moves through these states during its lifetime:
 *
 *     READY --> RUNNING --> READY (if more instructions)
 *       |                    |
 *       |       +------------+
 *       |       |
 *       +--> STALLED_MEMORY    --> READY (when data arrives)
 *       +--> STALLED_BARRIER   --> READY (when all warps reach barrier)
 *       +--> STALLED_DEPENDENCY --> READY (when register available)
 *       +--> COMPLETED
 *
 * The scheduler's job is to find a READY warp and issue it to an engine.
 * When a warp stalls (e.g., on a memory access), the scheduler switches
 * to another READY warp -- this is how GPUs hide latency.
 */
export enum WarpState {
  READY = "ready",
  RUNNING = "running",
  STALLED_MEMORY = "stalled_memory",
  STALLED_BARRIER = "stalled_barrier",
  STALLED_DEPENDENCY = "stalled_dependency",
  COMPLETED = "completed",
}

// ---------------------------------------------------------------------------
// SchedulingPolicy -- how the scheduler picks which warp to issue
// ---------------------------------------------------------------------------

/**
 * How the warp scheduler picks which warp to issue next.
 *
 * Real GPUs use sophisticated scheduling policies that balance throughput,
 * fairness, and latency hiding. Here are the most common ones:
 *
 *     Policy       | Strategy              | Used by
 *     -------------+-----------------------+--------------
 *     ROUND_ROBIN  | Fair rotation         | Teaching, some AMD
 *     GREEDY       | Most-ready-first      | Throughput-focused
 *     OLDEST_FIRST | Longest-waiting-first | Fairness-focused
 *     GTO          | Same warp til stall   | NVIDIA (common)
 *     LRR          | Skip-stalled rotation | AMD (common)
 *
 * GTO (Greedy-Then-Oldest) is particularly interesting: it keeps issuing
 * from the same warp until it stalls, then switches to the oldest ready
 * warp. This reduces context-switch overhead because warps that don't
 * stall get maximum throughput.
 */
export enum SchedulingPolicy {
  ROUND_ROBIN = "round_robin",
  GREEDY = "greedy",
  OLDEST_FIRST = "oldest_first",
  GTO = "gto",
  LRR = "lrr",
}

// ---------------------------------------------------------------------------
// WorkItem -- a unit of parallel work dispatched to a compute unit
// ---------------------------------------------------------------------------

/**
 * A unit of parallel work dispatched to a compute unit.
 *
 * In CUDA terms, this is a **thread block** (or cooperative thread array).
 * In OpenCL terms, this is a **work group**.
 * In TPU terms, this is a **tile** of a matrix operation.
 * In NPU terms, this is an **inference tile**.
 *
 * The WorkItem is the bridge between the application (which says "compute
 * this") and the hardware (which says "here are my execution engines").
 * The compute unit takes a WorkItem and decomposes it into warps/wavefronts
 * /tiles that can run on the engines.
 *
 * === Thread Block Decomposition (NVIDIA example) ===
 *
 * A WorkItem with threadCount=256 on an NVIDIA SM:
 *
 *     WorkItem(threadCount=256)
 *     +-- Warp 0:  threads 0-31    (first 32 threads)
 *     +-- Warp 1:  threads 32-63
 *     +-- Warp 2:  threads 64-95
 *     +-- ...
 *     +-- Warp 7:  threads 224-255 (last 32 threads)
 *
 * All 8 warps share the same shared memory and can synchronize with
 * __syncthreads(). This is how threads cooperate on shared data.
 */
export interface WorkItem {
  readonly workId: number;
  readonly program: Instruction[] | null;
  readonly threadCount: number;
  readonly perThreadData: Readonly<Record<number, Readonly<Record<number, number>>>>;
  readonly inputData: readonly (readonly number[])[] | null;
  readonly weightData: readonly (readonly number[])[] | null;
  readonly schedule: readonly unknown[] | null;
  readonly sharedMemBytes: number;
  readonly registersPerThread: number;
}

/**
 * Create a WorkItem with sensible defaults.
 *
 * Only workId is required. All other fields default to values that
 * represent a minimal work item (32 threads, no program, no data).
 */
export function makeWorkItem(
  partial: Partial<WorkItem> & { workId: number },
): WorkItem {
  return {
    program: null,
    threadCount: 32,
    perThreadData: {},
    inputData: null,
    weightData: null,
    schedule: null,
    sharedMemBytes: 0,
    registersPerThread: 32,
    ...partial,
  };
}

// ---------------------------------------------------------------------------
// ComputeUnitTrace -- record of one clock cycle across the compute unit
// ---------------------------------------------------------------------------

/**
 * Record of one clock cycle across the entire compute unit.
 *
 * Captures scheduler decisions, engine activity, memory accesses, and
 * resource utilization -- everything needed to understand what the compute
 * unit did in one cycle.
 *
 * === Why Trace Everything? ===
 *
 * Tracing is how you learn what GPUs actually do. Without traces, a GPU
 * is a black box: data in, data out, who knows what happened inside.
 * With traces, you can see:
 *
 * - Which warp the scheduler picked and why
 * - How many warps are stalled on memory
 * - What occupancy looks like cycle by cycle
 * - Where bank conflicts happen in shared memory
 *
 * This is the same information that tools like NVIDIA Nsight Compute
 * show for real GPUs. Our traces are simpler but serve the same
 * educational purpose.
 */
export interface ComputeUnitTrace {
  readonly cycle: number;
  readonly unitName: string;
  readonly architecture: Architecture;
  readonly schedulerAction: string;
  readonly activeWarps: number;
  readonly totalWarps: number;
  readonly engineTraces: Readonly<Record<number, EngineTrace>>;
  readonly sharedMemoryUsed: number;
  readonly sharedMemoryTotal: number;
  readonly registerFileUsed: number;
  readonly registerFileTotal: number;
  readonly occupancy: number;
  readonly l1Hits: number;
  readonly l1Misses: number;
}

/**
 * Create a ComputeUnitTrace with sensible defaults for optional fields.
 */
export function makeComputeUnitTrace(
  partial: Omit<ComputeUnitTrace, "l1Hits" | "l1Misses"> &
    Partial<Pick<ComputeUnitTrace, "l1Hits" | "l1Misses">>,
): ComputeUnitTrace {
  return {
    l1Hits: 0,
    l1Misses: 0,
    ...partial,
  };
}

/**
 * Pretty-print the trace for educational display.
 *
 * Returns a multi-line string showing scheduler action, occupancy,
 * resource usage, and per-engine details.
 *
 * Example output:
 *
 *     [Cycle 5] SM (nvidia_sm) -- 75.0% occupancy (48/64 warps)
 *       Scheduler: issued warp 3 (GTO policy)
 *       Shared memory: 49152/98304 bytes (50.0%)
 *       Registers: 32768/65536 (50.0%)
 *       Engine 0: FMUL R2, R0, R1 -- 32/32 threads active
 *       Engine 1: (idle)
 */
export function formatComputeUnitTrace(trace: ComputeUnitTrace): string {
  const occPct = `${(trace.occupancy * 100).toFixed(1)}%`;
  const lines: string[] = [
    `[Cycle ${trace.cycle}] ${trace.unitName} ` +
      `(${trace.architecture}) ` +
      `-- ${occPct} occupancy ` +
      `(${trace.activeWarps}/${trace.totalWarps} warps)`,
  ];
  lines.push(`  Scheduler: ${trace.schedulerAction}`);

  if (trace.sharedMemoryTotal > 0) {
    const smemPct =
      (trace.sharedMemoryUsed / trace.sharedMemoryTotal) * 100;
    lines.push(
      `  Shared memory: ${trace.sharedMemoryUsed}` +
        `/${trace.sharedMemoryTotal} bytes (${smemPct.toFixed(1)}%)`,
    );
  }

  if (trace.registerFileTotal > 0) {
    const regPct =
      (trace.registerFileUsed / trace.registerFileTotal) * 100;
    lines.push(
      `  Registers: ${trace.registerFileUsed}` +
        `/${trace.registerFileTotal} (${regPct.toFixed(1)}%)`,
    );
  }

  const engineIds = Object.keys(trace.engineTraces)
    .map(Number)
    .sort((a, b) => a - b);
  for (const eid of engineIds) {
    lines.push(
      `  Engine ${eid}: ${trace.engineTraces[eid].description}`,
    );
  }

  return lines.join("\n");
}

// ---------------------------------------------------------------------------
// SharedMemory -- programmer-visible scratchpad with bank conflict detection
// ---------------------------------------------------------------------------

/**
 * Shared memory with bank conflict detection.
 *
 * === What is Shared Memory? ===
 *
 * Shared memory is a small, fast, programmer-managed scratchpad that's
 * visible to all threads in a thread block. It's the GPU equivalent of
 * a team whiteboard -- everyone on the team can read and write to it.
 *
 * Performance comparison:
 *
 *     Memory Level      | Latency    | Bandwidth
 *     ------------------+------------+------------
 *     Registers         | 0 cycles   | unlimited
 *     Shared memory     | ~1-4 cycles| ~10 TB/s
 *     L1 cache          | ~30 cycles | ~2 TB/s
 *     Global (VRAM)     | ~400 cycles| ~1 TB/s
 *
 * That's a 100x latency difference between shared memory and global
 * memory. Kernels that reuse data should load it into shared memory
 * once and access it from there.
 *
 * === Bank Conflicts -- The Hidden Performance Trap ===
 *
 * Shared memory is divided into **banks** (typically 32). Each bank can
 * serve one request per cycle. If two threads access the same bank but
 * at different addresses, they **serialize** -- this is a bank conflict.
 *
 * Bank mapping (32 banks, 4 bytes per bank):
 *
 *     Address 0x00 -> Bank 0    Address 0x04 -> Bank 1    ...
 *     Address 0x80 -> Bank 0    Address 0x84 -> Bank 1    ...
 *
 * The bank for an address is: Math.floor(address / bankWidth) % numBanks
 */
export class SharedMemory {
  /** Total bytes of shared memory. */
  readonly size: number;

  /** Number of memory banks (typically 32). */
  readonly numBanks: number;

  /** Bytes per bank (typically 4). */
  readonly bankWidth: number;

  /** Internal data storage as a Float32Array-backed buffer. */
  private _data: DataView;

  /** Raw ArrayBuffer backing the DataView. */
  private _buffer: ArrayBuffer;

  /** Total number of read/write accesses. */
  private _totalAccesses: number = 0;

  /** Total bank conflicts detected. */
  private _totalConflicts: number = 0;

  constructor(size: number, numBanks: number = 32, bankWidth: number = 4) {
    this.size = size;
    this.numBanks = numBanks;
    this.bankWidth = bankWidth;
    this._buffer = new ArrayBuffer(size);
    this._data = new DataView(this._buffer);
  }

  /**
   * Read a 4-byte float from shared memory.
   *
   * @param address  Byte address to read from (must be 4-byte aligned).
   * @param threadId Which thread is reading (for conflict tracking).
   * @returns The float value at that address.
   * @throws RangeError if address is out of range.
   */
  read(address: number, threadId: number): number {
    if (address < 0 || address + 4 > this.size) {
      throw new RangeError(
        `Shared memory address ${address} out of range [0, ${this.size})`,
      );
    }
    this._totalAccesses += 1;
    return this._data.getFloat32(address, true); // little-endian
  }

  /**
   * Write a 4-byte float to shared memory.
   *
   * @param address  Byte address to write to (must be 4-byte aligned).
   * @param value    The float value to write.
   * @param threadId Which thread is writing (for conflict tracking).
   * @throws RangeError if address is out of range.
   */
  write(address: number, value: number, threadId: number): void {
    if (address < 0 || address + 4 > this.size) {
      throw new RangeError(
        `Shared memory address ${address} out of range [0, ${this.size})`,
      );
    }
    this._totalAccesses += 1;
    this._data.setFloat32(address, value, true); // little-endian
  }

  /**
   * Detect bank conflicts for a set of simultaneous accesses.
   *
   * Given a list of addresses (one per thread), determine which
   * accesses conflict (hit the same bank). Returns a list of conflict
   * groups -- each group is a list of thread indices that conflict.
   *
   * === How Bank Conflict Detection Works ===
   *
   * 1. Compute the bank for each address:
   *    bank = Math.floor(address / bankWidth) % numBanks
   *
   * 2. Group threads by bank.
   *
   * 3. Any bank accessed by more than one thread is a conflict.
   *    The threads in that bank must serialize -- taking N cycles
   *    for N conflicting accesses instead of 1 cycle.
   *
   * @param addresses List of byte addresses, one per thread.
   * @returns List of conflict groups. Each group is a list of thread
   *          indices that conflict. Groups of size 1 (no conflict) are
   *          NOT included -- only actual conflicts.
   *
   * @example
   *     const smem = new SharedMemory(1024);
   *     // Threads 0 and 2 both hit bank 0 (addresses 0 and 128)
   *     smem.checkBankConflicts([0, 4, 128, 12]);
   *     // Returns [[0, 2]]  -- threads 0 and 2 conflict on bank 0
   */
  checkBankConflicts(addresses: number[]): number[][] {
    // Map bank -> list of thread indices
    const bankToThreads: Map<number, number[]> = new Map();
    for (let threadIdx = 0; threadIdx < addresses.length; threadIdx++) {
      const bank =
        Math.floor(addresses[threadIdx] / this.bankWidth) % this.numBanks;
      if (!bankToThreads.has(bank)) {
        bankToThreads.set(bank, []);
      }
      bankToThreads.get(bank)!.push(threadIdx);
    }

    // Find conflicts (banks with more than one thread)
    const conflicts: number[][] = [];
    for (const threads of bankToThreads.values()) {
      if (threads.length > 1) {
        conflicts.push(threads);
        this._totalConflicts += threads.length - 1;
      }
    }

    return conflicts;
  }

  /** Reset all data and statistics. */
  reset(): void {
    this._buffer = new ArrayBuffer(this.size);
    this._data = new DataView(this._buffer);
    this._totalAccesses = 0;
    this._totalConflicts = 0;
  }

  /** Total number of read/write accesses. */
  get totalAccesses(): number {
    return this._totalAccesses;
  }

  /** Total bank conflicts detected. */
  get totalConflicts(): number {
    return this._totalConflicts;
  }
}

// ---------------------------------------------------------------------------
// ComputeUnit interface -- the unified interface
// ---------------------------------------------------------------------------

/**
 * Any compute unit: SM, CU, MXU, Xe Core, ANE Core.
 *
 * A compute unit manages multiple execution engines, schedules work
 * across them, and provides shared resources. It's the integration
 * point between raw parallel execution and the device layer above.
 *
 * === Why an Interface? ===
 *
 * Despite radical differences between NVIDIA SMs, AMD CUs, and Google
 * MXUs, they all share this common interface:
 *
 * 1. dispatch(work) -- accept work
 * 2. step(clockEdge) -- advance one cycle
 * 3. run(maxCycles) -- run until done
 * 4. idle -- is all work complete?
 * 5. reset() -- clear all state
 *
 * This lets the device layer above treat all compute units uniformly,
 * the same way a factory manager can manage different production lines
 * without knowing the details of each machine.
 */
export interface ComputeUnit {
  /** Unit name: 'SM', 'CU', 'MXU', 'XeCore', 'ANECore'. */
  readonly name: string;

  /** Which vendor architecture this compute unit belongs to. */
  readonly architecture: Architecture;

  /** True if no work remains and all engines are idle. */
  readonly idle: boolean;

  /** Accept a work item (thread block, work group, tile). */
  dispatch(work: WorkItem): void;

  /** Advance one clock cycle across all engines and the scheduler. */
  step(clockEdge: { cycle: number }): ComputeUnitTrace;

  /** Run until all dispatched work is complete. */
  run(maxCycles?: number): ComputeUnitTrace[];

  /** Reset all state: engines, scheduler, shared memory, caches. */
  reset(): void;
}
