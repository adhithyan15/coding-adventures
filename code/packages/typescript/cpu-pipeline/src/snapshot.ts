/**
 * PipelineSnapshot and PipelineStats -- capturing and measuring pipeline state.
 *
 * PipelineSnapshot is a "photograph" of the assembly line at one moment in time.
 * PipelineStats tracks performance metrics across the entire execution.
 */

import type { PipelineToken } from "./token.js";

// =========================================================================
// PipelineSnapshot -- the complete state of the pipeline at one moment
// =========================================================================

/**
 * PipelineSnapshot captures the full state of the pipeline at a single
 * point in time (one clock cycle).
 *
 * Example snapshot for a 5-stage pipeline at cycle 7:
 *
 *     Cycle 7:
 *       IF:  instr@28  (fetching instruction at PC=28)
 *       ID:  ADD@24    (decoding an ADD instruction)
 *       EX:  SUB@20    (executing a SUB)
 *       MEM: ---       (bubble -- pipeline was stalled here)
 *       WB:  LDR@12    (writing back a load result)
 */
export interface PipelineSnapshot {
  /** Clock cycle number when this snapshot was taken (starts at 1). */
  cycle: number;
  /** Maps stage name to the token currently occupying that stage. */
  stages: Record<string, PipelineToken>;
  /** True if the pipeline was stalled during this cycle. */
  stalled: boolean;
  /** True if a pipeline flush occurred during this cycle. */
  flushing: boolean;
  /** Current program counter (address of next fetch). */
  pc: number;
}

/**
 * Returns a compact string representation of a pipeline snapshot.
 */
export function snapshotToString(s: PipelineSnapshot): string {
  return `[cycle ${s.cycle}] PC=${s.pc} stalled=${s.stalled} flushing=${s.flushing}`;
}

// =========================================================================
// PipelineStats -- execution statistics
// =========================================================================

/**
 * PipelineStats tracks performance statistics across the pipeline's execution.
 *
 * These are the same metrics that hardware performance counters measure
 * in real CPUs. They answer: "How efficiently is the pipeline being used?"
 *
 * # Key Metrics
 *
 * IPC (Instructions Per Cycle):
 *     IPC = instructionsCompleted / totalCycles
 *     Ideal: 1.0 | With stalls: < 1.0 | Superscalar: > 1.0
 *
 * CPI (Cycles Per Instruction):
 *     CPI = totalCycles / instructionsCompleted
 *     Ideal: 1.0 | Typical: 1.2-2.0
 */
export class PipelineStats {
  /** Number of clock cycles the pipeline has executed. */
  totalCycles: number = 0;

  /** Number of non-bubble instructions that reached the final (writeback) stage. */
  instructionsCompleted: number = 0;

  /** Number of cycles where the pipeline was stalled. */
  stallCycles: number = 0;

  /** Number of cycles where a flush occurred. */
  flushCycles: number = 0;

  /**
   * Total number of stage-cycles occupied by bubbles.
   * If 3 stages hold bubbles for 1 cycle, that contributes 3.
   */
  bubbleCycles: number = 0;

  /**
   * Returns the instructions per cycle.
   *
   *   IPC = 1.0: perfect pipeline utilization (ideal)
   *   IPC < 1.0: some cycles are wasted (stalls, flushes)
   *   IPC > 1.0: superscalar execution
   *
   * Returns 0.0 if no cycles have been executed.
   */
  ipc(): number {
    if (this.totalCycles === 0) return 0.0;
    return this.instructionsCompleted / this.totalCycles;
  }

  /**
   * Returns cycles per instruction (inverse of IPC).
   *
   *   CPI = 1.0: one cycle per instruction (ideal)
   *   CPI = 1.5: 50% overhead from stalls and flushes
   *
   * Returns 0.0 if no instructions have completed.
   */
  cpi(): number {
    if (this.instructionsCompleted === 0) return 0.0;
    return this.totalCycles / this.instructionsCompleted;
  }

  /**
   * Returns a formatted summary of pipeline statistics.
   */
  toString(): string {
    return (
      `PipelineStats{cycles=${this.totalCycles}, completed=${this.instructionsCompleted}, ` +
      `IPC=${this.ipc().toFixed(3)}, CPI=${this.cpi().toFixed(3)}, ` +
      `stalls=${this.stallCycles}, flushes=${this.flushCycles}, bubbles=${this.bubbleCycles}}`
    );
  }
}
