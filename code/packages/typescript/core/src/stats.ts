/**
 * CoreStats -- aggregate statistics from all core sub-components.
 *
 * Collects stats from the pipeline, branch predictor, hazard unit,
 * and cache hierarchy into a single view.
 */

import type { PredictionStats } from "@coding-adventures/branch-predictor";
import type { CacheStats } from "@coding-adventures/cache";
import type { PipelineStats } from "@coding-adventures/cpu-pipeline";

/**
 * CoreStats collects performance statistics from every sub-component.
 *
 * Key Metrics:
 *
 *     IPC = instructionsCompleted / totalCycles
 *         1.0 = perfect | < 1.0 = stalls/flushes | > 1.0 = superscalar
 *
 *     CPI = totalCycles / instructionsCompleted
 *         1.0 = ideal | > 1.0 = wasted cycles
 */
export class CoreStats {
  instructionsCompleted: number = 0;
  totalCycles: number = 0;
  pipelineStats: PipelineStats | null = null;
  predictorStats: PredictionStats | null = null;
  cacheStats: Record<string, CacheStats> = {};
  forwardCount: number = 0;
  stallCount: number = 0;
  flushCount: number = 0;

  /** Returns instructions per cycle. */
  ipc(): number {
    if (this.totalCycles === 0) return 0.0;
    return this.instructionsCompleted / this.totalCycles;
  }

  /** Returns cycles per instruction. */
  cpi(): number {
    if (this.instructionsCompleted === 0) return 0.0;
    return this.totalCycles / this.instructionsCompleted;
  }

  /** Returns a formatted summary of all statistics. */
  toString(): string {
    let result = "Core Statistics:\n";
    result += `  Instructions completed: ${this.instructionsCompleted}\n`;
    result += `  Total cycles:           ${this.totalCycles}\n`;
    result += `  IPC: ${this.ipc().toFixed(3)}   CPI: ${this.cpi().toFixed(3)}\n`;
    result += "\n";
    result += "Hazards:\n";
    result += `  Forwards: ${this.forwardCount}\n`;
    result += `  Stalls:   ${this.stallCount}\n`;
    result += `  Flushes:  ${this.flushCount}\n`;
    return result;
  }
}
