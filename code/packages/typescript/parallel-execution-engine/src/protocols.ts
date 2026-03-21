/**
 * Protocols -- the unified interface for all parallel execution engines.
 *
 * === What is a Parallel Execution Engine? ===
 *
 * At Layer 9 (gpu-core), we built a single processing element -- one tiny
 * compute unit that executes one instruction at a time. Useful for learning,
 * but real accelerators never run just ONE core. They run THOUSANDS in parallel.
 *
 * Layer 8 is where parallelism happens. It takes many Layer 9 cores (or
 * simpler processing elements) and orchestrates them to execute together.
 * But HOW they're orchestrated differs fundamentally across architectures:
 *
 *     NVIDIA GPU:   32 threads in a "warp" -- each has its own registers,
 *                   but they execute the same instruction (SIMT).
 *
 *     AMD GPU:      32/64 "lanes" in a "wavefront" -- one instruction stream,
 *                   one wide vector ALU, explicit execution mask (SIMD).
 *
 *     Google TPU:   NxN grid of multiply-accumulate units -- data FLOWS
 *                   through the array, no instructions at all (Systolic).
 *
 *     Apple NPU:    Array of MACs driven by a compiler-generated schedule --
 *                   no runtime scheduler, just a fixed plan (Scheduled MAC).
 *
 *     Intel GPU:    SIMD8 execution units with multiple hardware threads --
 *                   a hybrid of SIMD and multi-threading (Subslice).
 *
 * Despite these radical differences, ALL of them share a common interface:
 * "advance one clock cycle, tell me what happened, report utilization."
 * That common interface is the ParallelExecutionEngine interface.
 *
 * === Flynn's Taxonomy -- A Quick Refresher ===
 *
 * In 1966, Michael Flynn classified computer architectures:
 *
 *     +-------------------+-----------------+---------------------+
 *     |                   | Single Data     | Multiple Data       |
 *     +-------------------+-----------------+---------------------+
 *     | Single Instr.     | SISD (old CPU)  | SIMD (vector proc.) |
 *     | Multiple Instr.   | MISD (rare)     | MIMD (multi-core)   |
 *     +-------------------+-----------------+---------------------+
 *
 * Modern accelerators don't fit neatly into these boxes:
 * - NVIDIA coined "SIMT" because warps are neither pure SIMD nor pure MIMD.
 * - Systolic arrays don't have "instructions" at all.
 * - NPU scheduled arrays are driven by static compiler schedules.
 *
 * Our ExecutionModel enum captures these real-world execution models.
 */

// ---------------------------------------------------------------------------
// ExecutionModel -- the five parallel execution paradigms
// ---------------------------------------------------------------------------

/**
 * The five parallel execution models supported by this package.
 *
 * Each model represents a fundamentally different way to organize parallel
 * computation. They are NOT interchangeable -- each has different properties
 * around divergence, synchronization, and data movement.
 *
 * Think of these as "architectural philosophies":
 *
 *     SIMT:          "Give every thread its own identity, execute together"
 *     SIMD:          "One instruction, wide ALU, explicit masking"
 *     SYSTOLIC:      "Data flows through a grid -- no instructions needed"
 *     SCHEDULED_MAC: "Compiler decides everything -- hardware just executes"
 *     VLIW:          "Pack multiple ops into one wide instruction word"
 *
 * Comparison table:
 *
 *     Model          | Has PC? | Has threads? | Divergence?     | Used by
 *     ---------------+---------+--------------+-----------------+----------
 *     SIMT           | Yes*    | Yes          | HW-managed      | NVIDIA
 *     SIMD           | Yes     | No (lanes)   | Explicit mask   | AMD
 *     SYSTOLIC       | No      | No           | N/A             | Google TPU
 *     SCHEDULED_MAC  | No      | No           | Compile-time    | Apple NPU
 *     VLIW           | Yes     | No           | Predicated      | Qualcomm
 *
 *     * SIMT: each thread logically has its own PC, but they usually share one.
 */
export enum ExecutionModel {
  SIMT = "simt",
  SIMD = "simd",
  SYSTOLIC = "systolic",
  SCHEDULED_MAC = "scheduled_mac",
  VLIW = "vliw",
}

// ---------------------------------------------------------------------------
// DivergenceInfo -- tracking branch divergence (SIMT/SIMD only)
// ---------------------------------------------------------------------------

/**
 * Information about branch divergence during one execution step.
 *
 * === What is Divergence? ===
 *
 * When a group of threads/lanes encounters a branch (if/else), some may
 * take the "true" path and others the "false" path. This is called
 * "divergence" -- the threads are no longer executing in lockstep.
 *
 *     Before branch:    All 8 threads active: [1, 1, 1, 1, 1, 1, 1, 1]
 *     Branch condition:  thread_id < 4?
 *     After branch:     Only 4 active:        [1, 1, 1, 1, 0, 0, 0, 0]
 *                       The other 4 will run later.
 *
 * Divergence is the enemy of GPU performance. When half the threads are
 * masked off, you're wasting half your hardware. Real GPU code tries to
 * minimize divergence by ensuring threads in the same warp/wavefront
 * take the same path.
 *
 * Fields:
 *     activeMaskBefore: Which units were active BEFORE the branch.
 *     activeMaskAfter:  Which units are active AFTER the branch.
 *     reconvergencePc:  The instruction address where all units rejoin.
 *                       -1 if not applicable (e.g., SIMD explicit mask).
 *     divergenceDepth:  How many nested divergent branches we're inside.
 *                       0 means no divergence. Higher = more serialization.
 */
export interface DivergenceInfo {
  readonly activeMaskBefore: readonly boolean[];
  readonly activeMaskAfter: readonly boolean[];
  readonly reconvergencePc: number;
  readonly divergenceDepth: number;
}

/**
 * Create a DivergenceInfo with sensible defaults.
 */
export function makeDivergenceInfo(
  partial: Partial<DivergenceInfo> & {
    activeMaskBefore: readonly boolean[];
    activeMaskAfter: readonly boolean[];
  },
): DivergenceInfo {
  return {
    reconvergencePc: -1,
    divergenceDepth: 0,
    ...partial,
  };
}

// ---------------------------------------------------------------------------
// DataflowInfo -- tracking data movement (Systolic only)
// ---------------------------------------------------------------------------

/**
 * Information about data flow in a systolic array.
 *
 * === What is Dataflow Execution? ===
 *
 * In a systolic array, there are no instructions. Instead, data "flows"
 * through a grid of processing elements, like water flowing through pipes.
 * Each PE does a multiply-accumulate and passes data to its neighbor.
 *
 * This interface tracks the state of every PE in the grid so we can
 * visualize how data pulses through the array cycle by cycle.
 *
 * Fields:
 *     peStates:       2D grid of PE state descriptions.
 *                     peStates[row][col] = "acc=3.14, in=2.0"
 *     dataPositions:  Where each input value currently is in the array.
 *                     Maps input_id to [row, col] position.
 */
export interface DataflowInfo {
  readonly peStates: readonly (readonly string[])[];
  readonly dataPositions: Readonly<Record<string, readonly [number, number]>>;
}

/**
 * Create a DataflowInfo with sensible defaults.
 */
export function makeDataflowInfo(
  partial: Partial<DataflowInfo> & {
    peStates: readonly (readonly string[])[];
  },
): DataflowInfo {
  return {
    dataPositions: {},
    ...partial,
  };
}

// ---------------------------------------------------------------------------
// EngineTrace -- the unified trace record for all engines
// ---------------------------------------------------------------------------

/**
 * Record of one parallel execution step across ALL parallel units.
 *
 * === Why a Unified Trace? ===
 *
 * Every engine -- warp, wavefront, systolic, MAC array -- produces one
 * EngineTrace per clock cycle. This lets higher layers (and tests, and
 * visualization tools) treat all engines uniformly.
 *
 * The trace captures:
 * 1. WHAT happened (description, per-unit details)
 * 2. WHO was active (activeMask, utilization)
 * 3. HOW efficient it was (activeCount / totalCount)
 * 4. Engine-specific details (divergence for SIMT, dataflow for systolic)
 *
 * Example trace from a 4-thread warp:
 *
 *     {
 *         cycle: 3,
 *         engineName: "WarpEngine",
 *         executionModel: ExecutionModel.SIMT,
 *         description: "FADD R2, R0, R1 -- 3/4 threads active",
 *         unitTraces: {
 *             0: "R2 = 1.0 + 2.0 = 3.0",
 *             1: "R2 = 3.0 + 4.0 = 7.0",
 *             2: "(masked -- diverged)",
 *             3: "R2 = 5.0 + 6.0 = 11.0",
 *         },
 *         activeMask: [true, true, false, true],
 *         activeCount: 3,
 *         totalCount: 4,
 *         utilization: 0.75,
 *     }
 */
export interface EngineTrace {
  readonly cycle: number;
  readonly engineName: string;
  readonly executionModel: ExecutionModel;
  readonly description: string;
  readonly unitTraces: Readonly<Record<number, string>>;
  readonly activeMask: readonly boolean[];
  readonly activeCount: number;
  readonly totalCount: number;
  readonly utilization: number;
  readonly divergenceInfo?: DivergenceInfo | null;
  readonly dataflowInfo?: DataflowInfo | null;
}

/**
 * Pretty-print the trace for educational display.
 *
 * Returns a multi-line string showing the cycle, engine, utilization,
 * and per-unit details. Example output:
 *
 *     [Cycle 3] WarpEngine (SIMT) -- 75.0% utilization (3/4 active)
 *       FADD R2, R0, R1 -- 3/4 threads active
 *       Unit 0: R2 = 1.0 + 2.0 = 3.0
 *       Unit 1: R2 = 3.0 + 4.0 = 7.0
 *       Unit 2: (masked -- diverged)
 *       Unit 3: R2 = 5.0 + 6.0 = 11.0
 */
export function formatEngineTrace(trace: EngineTrace): string {
  const pct = `${(trace.utilization * 100).toFixed(1)}%`;
  const lines: string[] = [
    `[Cycle ${trace.cycle}] ${trace.engineName} ` +
      `(${trace.executionModel.toUpperCase()}) ` +
      `-- ${pct} utilization (${trace.activeCount}/${trace.totalCount} active)`,
  ];
  lines.push(`  ${trace.description}`);

  const unitIds = Object.keys(trace.unitTraces)
    .map(Number)
    .sort((a, b) => a - b);
  for (const unitId of unitIds) {
    lines.push(`  Unit ${unitId}: ${trace.unitTraces[unitId]}`);
  }

  if (trace.divergenceInfo != null) {
    const di = trace.divergenceInfo;
    lines.push(
      `  Divergence: depth=${di.divergenceDepth}, ` +
        `reconvergence_pc=${di.reconvergencePc}`,
    );
  }

  return lines.join("\n");
}

// ---------------------------------------------------------------------------
// ParallelExecutionEngine -- the interface all engines implement
// ---------------------------------------------------------------------------

/**
 * The common interface for all parallel execution engines.
 *
 * === Interface Design ===
 *
 * This interface captures the minimal shared behavior of ALL parallel
 * execution engines, regardless of execution model:
 *
 * 1. name  -- identify which engine this is
 * 2. width -- how many parallel units (threads, lanes, PEs, MACs)
 * 3. executionModel -- which paradigm (SIMT, SIMD, systolic, etc.)
 * 4. step() -- advance one clock cycle
 * 5. halted -- is all work complete?
 * 6. reset() -- return to initial state
 *
 * === Why so minimal? ===
 *
 * Different engines have radically different APIs:
 * - WarpEngine has loadProgram(), setThreadRegister()
 * - SystolicArray has loadWeights(), feedInput()
 * - MACArrayEngine has loadSchedule(), loadInputs()
 *
 * Those are engine-specific. The interface only captures what they ALL share,
 * so that Layer 7 (the compute unit) can drive any engine uniformly.
 */
export interface ParallelExecutionEngine {
  /** Engine name: 'WarpEngine', 'WavefrontEngine', etc. */
  readonly name: string;

  /** Parallelism width (threads, lanes, PEs, MACs). */
  readonly width: number;

  /** Which parallel execution model this engine uses. */
  readonly executionModel: ExecutionModel;

  /** True if all work is complete. */
  readonly halted: boolean;

  /** Advance one clock cycle. Returns a trace of what happened. */
  step(clockEdge: { cycle: number }): EngineTrace;

  /** Reset to initial state. */
  reset(): void;
}
