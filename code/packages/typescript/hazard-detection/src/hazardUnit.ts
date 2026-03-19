/**
 * Combined hazard detection unit — the pipeline's traffic controller.
 *
 * === What Is the Hazard Unit? ===
 *
 * The hazard unit is a single hardware module that runs ALL hazard
 * detectors every clock cycle and returns ONE decision to the pipeline.
 * Think of it as an air traffic controller: it monitors all the "planes"
 * (instructions) in the pipeline and issues commands to prevent collisions.
 *
 * === Why a Combined Unit? ===
 *
 * Multiple hazard types can occur simultaneously:
 * - A data hazard (RAW) AND a control hazard (branch misprediction)
 *   could happen at the same time.
 * - A structural hazard AND a data hazard could overlap.
 *
 * The combined unit resolves conflicts between detectors by using a
 * strict priority system.
 *
 * === Priority System ===
 *
 *     FLUSH > STALL > FORWARD > NONE
 *
 *     1. FLUSH (highest priority):
 *        A branch misprediction means we're executing WRONG instructions.
 *        Nothing else matters — flush the pipeline immediately. Even if
 *        there's a data hazard, it's on a wrong instruction that's about
 *        to be thrown away.
 *
 *     2. STALL (second priority):
 *        We can't proceed because data isn't ready yet. The pipeline must
 *        freeze. Even if forwarding could help with one register, a stall
 *        on another register takes precedence.
 *
 *     3. FORWARD (third priority):
 *        A data dependency exists, but we can resolve it by forwarding.
 *        This is the "best case" for hazards — zero penalty.
 *
 *     4. NONE (lowest priority):
 *        All clear. The pipeline flows normally.
 *
 * === Statistics Tracking ===
 *
 * The hazard unit maintains a history of all decisions, which is useful
 * for performance analysis:
 * - stallCount: total stall cycles (directly reduces throughput)
 * - flushCount: total flushes (each costs 2 wasted cycles)
 * - forwardCount: total forwards (zero penalty, but indicates
 *   dependency density in the code)
 *
 * A well-optimized program (or compiler) minimizes stalls and flushes.
 */

import { ControlHazardDetector } from "./controlHazard.js";
import { DataHazardDetector } from "./dataHazard.js";
import { StructuralHazardDetector } from "./structuralHazard.js";
import { HazardAction, HazardResult, PipelineSlot } from "./types.js";

/**
 * Combined hazard detection unit — runs all detectors each cycle.
 *
 * === Usage Example ===
 *
 *     // Create the unit (configurable hardware resources)
 *     const unit = new HazardUnit({ numAlus: 1, numFpUnits: 1, splitCaches: true });
 *
 *     // Each cycle, pass in the four pipeline stages:
 *     const result = unit.check(ifStage, idStage, exStage, memStage);
 *
 *     // Act on the result:
 *     if (result.action === HazardAction.FLUSH) {
 *         pipeline.flushIfAndId();
 *     } else if (result.action === HazardAction.STALL) {
 *         pipeline.insertBubble();
 *     } else if (result.action === HazardAction.FORWARD_FROM_EX ||
 *                result.action === HazardAction.FORWARD_FROM_MEM) {
 *         pipeline.forward(result.forwardedValue);
 *     } else {
 *         pipeline.proceedNormally();
 *     }
 *
 *     // Check performance stats:
 *     console.log(`Total stalls: ${unit.stallCount}`);
 *     console.log(`Total flushes: ${unit.flushCount}`);
 */
export class HazardUnit {
  readonly dataDetector: DataHazardDetector;
  readonly controlDetector: ControlHazardDetector;
  readonly structuralDetector: StructuralHazardDetector;
  private readonly _history: HazardResult[] = [];

  /**
   * Create a hazard unit with configurable hardware resources.
   *
   * @param numAlus - Number of integer ALUs. Affects structural hazard detection.
   * @param numFpUnits - Number of floating-point units. Affects structural hazard detection.
   * @param splitCaches - Whether L1I and L1D caches are separate. Affects structural
   *     hazard detection for memory port conflicts.
   */
  constructor(
    {
      numAlus = 1,
      numFpUnits = 1,
      splitCaches = true,
    }: {
      numAlus?: number;
      numFpUnits?: number;
      splitCaches?: boolean;
    } = {},
  ) {
    this.dataDetector = new DataHazardDetector();
    this.controlDetector = new ControlHazardDetector();
    this.structuralDetector = new StructuralHazardDetector({
      numAlus,
      numFpUnits,
      splitCaches,
    });
  }

  /**
   * Run all hazard detectors and return the highest-priority action.
   *
   * This method is called ONCE per clock cycle. It runs all three
   * hazard detectors and returns the single most critical action
   * the pipeline should take.
   *
   * @param ifStage - Instruction being fetched.
   * @param idStage - Instruction being decoded.
   * @param exStage - Instruction being executed.
   * @param memStage - Instruction in memory access stage.
   * @returns The highest-priority hazard result. The pipeline should act
   *     on this result's action field.
   *
   * === Detection Order ===
   *
   * We run all detectors regardless of what earlier ones found,
   * because the history should record what WOULD have happened.
   * The final result uses the highest-priority action.
   *
   * However, the ORDER of detection doesn't matter for correctness —
   * only the priority comparison at the end determines the result.
   * We check control first because flushes are most critical.
   */
  check(
    ifStage: PipelineSlot,
    idStage: PipelineSlot,
    exStage: PipelineSlot,
    memStage: PipelineSlot,
  ): HazardResult {
    // --- 1. Control hazards (check first — highest priority) ---
    // If there's a misprediction, everything else is moot.
    const controlResult = this.controlDetector.detect(exStage);

    // --- 2. Data hazards (forwarding or stalling) ---
    const dataResult = this.dataDetector.detect(idStage, exStage, memStage);

    // --- 3. Structural hazards (resource conflicts) ---
    const structuralResult = this.structuralDetector.detect(
      idStage,
      exStage,
      ifStage,
      memStage,
    );

    // --- Pick the highest-priority result ---
    // Priority: FLUSH > STALL > FORWARD_FROM_EX > FORWARD_FROM_MEM > NONE
    const finalResult = pickHighestPriority(
      controlResult,
      dataResult,
      structuralResult,
    );

    // Record in history for statistics.
    this._history.push(finalResult);

    return finalResult;
  }

  /**
   * Complete history of hazard results, one per cycle.
   *
   * Useful for debugging and performance analysis. Each entry
   * corresponds to one call to check().
   */
  get history(): HazardResult[] {
    return [...this._history];
  }

  /**
   * Total stall cycles across all instructions.
   *
   * Each stall wastes one pipeline cycle. A high stall count
   * indicates the code has many data dependencies that can't be
   * resolved by forwarding (typically load-use patterns).
   */
  get stallCount(): number {
    return this._history.reduce((sum, r) => sum + r.stallCycles, 0);
  }

  /**
   * Total pipeline flushes (branch mispredictions).
   *
   * Each flush wastes 2 cycles (IF and ID stages are discarded).
   * A high flush count indicates the branch predictor is struggling,
   * or the code has many hard-to-predict branches.
   */
  get flushCount(): number {
    return this._history.filter((r) => r.action === HazardAction.FLUSH).length;
  }

  /**
   * Total forwarding operations.
   *
   * Forwarding resolves data hazards with zero penalty. A high
   * forward count isn't bad — it means the forwarding hardware
   * is earning its keep. Without it, these would all be stalls.
   */
  get forwardCount(): number {
    return this._history.filter(
      (r) =>
        r.action === HazardAction.FORWARD_FROM_EX ||
        r.action === HazardAction.FORWARD_FROM_MEM,
    ).length;
  }
}

/**
 * Return the hazard result with the highest-priority action.
 *
 * === Priority Map ===
 *
 *     FLUSH           = 4  (most urgent — wrong instructions in pipeline)
 *     STALL           = 3  (urgent — would get wrong data)
 *     FORWARD_FROM_EX = 2  (optimization — grab data from EX)
 *     FORWARD_FROM_MEM= 1  (optimization — grab data from MEM)
 *     NONE            = 0  (all clear)
 *
 * @param results - Variable number of hazard results to compare.
 * @returns The one with the highest-priority action. Ties go to the first one
 *     encountered (which is fine — same priority means same urgency).
 */
export function pickHighestPriority(
  ...results: HazardResult[]
): HazardResult {
  const priority: Record<HazardAction, number> = {
    [HazardAction.NONE]: 0,
    [HazardAction.FORWARD_FROM_MEM]: 1,
    [HazardAction.FORWARD_FROM_EX]: 2,
    [HazardAction.STALL]: 3,
    [HazardAction.FLUSH]: 4,
  };

  let best = results[0];
  for (let i = 1; i < results.length; i++) {
    if (priority[results[i].action] > priority[best.action]) {
      best = results[i];
    }
  }
  return best;
}
