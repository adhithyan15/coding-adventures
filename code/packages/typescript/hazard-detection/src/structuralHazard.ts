/**
 * Structural hazard detection — when hardware resources collide.
 *
 * === What Is a Structural Hazard? ===
 *
 * A structural hazard occurs when two instructions need the same hardware
 * resource in the same clock cycle. It's like two people trying to use
 * the same bathroom at the same time — someone has to wait.
 *
 * === Common Examples ===
 *
 * 1. **Single-Port Memory** (fetch + data access conflict):
 *    The CPU needs to fetch an instruction (IF stage) AND load/store
 *    data (MEM stage) in the same cycle, but the memory only has one
 *    port (one "door" for reading/writing).
 *
 *        Cycle 4:
 *        Instr A:  [IF] ← needs to read instruction from memory
 *        Instr B:       [MEM] ← needs to read/write data from memory
 *                  ↑ CONFLICT! Memory can only serve one request.
 *
 *    Solution: Split L1 cache into L1I (instructions) and L1D (data).
 *    Each has its own port, so fetch and data access happen in parallel.
 *    This is what ALL modern CPUs do.
 *
 * 2. **Single ALU** (two ALU instructions at once):
 *    In a simple pipeline, there's only one ALU. If two instructions
 *    both need the ALU in the same cycle, one must wait. This mostly
 *    matters for superscalar CPUs that try to execute multiple
 *    instructions per cycle.
 *
 *        IF → ID → [EX] → MEM → WB   ← uses ALU
 *        IF → ID → [EX] → MEM → WB   ← also uses ALU (superscalar)
 *                   ↑ CONFLICT! Only one ALU available.
 *
 *    Solution: Add more ALUs (superscalar CPUs have 2-8+ ALUs).
 *
 * 3. **Single FP Unit** (two floating-point instructions at once):
 *    Floating-point units are expensive (lots of transistors), so many
 *    CPUs have fewer FP units than integer ALUs. Two FP instructions
 *    may conflict.
 *
 * === For Our Basic 5-Stage Pipeline ===
 *
 * With split L1I/L1D caches (the default), structural hazards are rare.
 * The main case is when two instructions in adjacent stages both need
 * the same execution unit (ALU or FP unit). We detect this for
 * completeness and for future superscalar extensions.
 *
 * === Configurability ===
 *
 * The detector is configurable:
 * - numAlus: How many ALU units are available (default: 1)
 * - numFpUnits: How many FP units are available (default: 1)
 * - splitCaches: Whether L1I and L1D are separate (default: true)
 *
 * With enough resources, structural hazards disappear entirely.
 * This is exactly how real CPUs evolved — adding more hardware to
 * eliminate stalls.
 */

import { HazardAction, HazardResult, PipelineSlot } from "./types.js";

/**
 * Detects structural hazards — two instructions competing for hardware.
 *
 * === How Detection Works ===
 *
 * We check two things each cycle:
 *
 * 1. **Execution unit conflict**: Is the ID-stage instruction about to
 *    enter EX, while the EX-stage instruction is still using the same
 *    execution unit? With 1 ALU, two ALU instructions can't both be
 *    in EX. With 2 ALUs, they can.
 *
 *    The check is:
 *    - Both ID and EX need the ALU, and we have fewer ALUs than needed?
 *    - Both ID and EX need the FP unit, and we have fewer FP units?
 *
 *    For a single-issue pipeline (1 instruction enters EX per cycle),
 *    this only matters when the execution unit is MULTI-CYCLE (i.e.,
 *    the EX-stage instruction hasn't finished yet and is still occupying
 *    the unit). For simplicity, we flag the conflict whenever both stages
 *    want the same unit and there's only one of it.
 *
 * 2. **Memory port conflict**: Is IF trying to fetch while MEM is
 *    accessing data, and we have a single (shared) cache?
 *
 *    With splitCaches=true (default), no conflict — each cache has
 *    its own port. With splitCaches=false, fetch and data access
 *    compete for the single memory port.
 */
export class StructuralHazardDetector {
  private readonly _numAlus: number;
  private readonly _numFpUnits: number;
  private readonly _splitCaches: boolean;

  /**
   * Configure the structural hazard detector.
   *
   * @param numAlus - Number of integer ALU units. With 1 ALU (default), two
   *     ALU-using instructions in EX simultaneously causes a stall.
   *     With 2+ ALUs, they can execute in parallel.
   * @param numFpUnits - Number of floating-point execution units. Same logic as ALUs.
   * @param splitCaches - If true (default), L1I and L1D are separate — no memory port
   *     conflict between IF and MEM stages. If false, a shared cache
   *     means IF and MEM can't access memory in the same cycle.
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
    this._numAlus = numAlus;
    this._numFpUnits = numFpUnits;
    this._splitCaches = splitCaches;
  }

  /**
   * Check for structural hazards between pipeline stages.
   *
   * @param idStage - Instruction about to enter EX. We check if it needs the same
   *     resources as the instruction currently in EX.
   * @param exStage - Instruction currently in EX. Occupying an execution unit.
   * @param ifStage - Instruction being fetched. Used for memory port conflict check.
   * @param memStage - Instruction accessing memory. Used for memory port conflict check.
   * @returns STALL if a structural hazard is detected, NONE otherwise.
   */
  detect(
    idStage: PipelineSlot,
    exStage: PipelineSlot,
    ifStage?: PipelineSlot | null,
    memStage?: PipelineSlot | null,
  ): HazardResult {
    // --- Check execution unit conflicts ---
    // Both instructions must be valid (non-bubble) to conflict.
    const execResult = this._checkExecutionUnitConflict(idStage, exStage);
    if (execResult.action !== HazardAction.NONE) {
      return execResult;
    }

    // --- Check memory port conflicts ---
    if (ifStage != null && memStage != null) {
      const memResult = this._checkMemoryPortConflict(ifStage, memStage);
      if (memResult.action !== HazardAction.NONE) {
        return memResult;
      }
    }

    return new HazardResult({
      action: HazardAction.NONE,
      reason: "no structural hazards — all resources available",
    });
  }

  /**
   * Check if ID and EX need the same execution unit.
   *
   * === Logic ===
   *
   * For ALU conflict:
   *     Both idStage.usesAlu AND exStage.usesAlu must be true,
   *     AND numAlus must be < 2 (only 1 ALU to share).
   *
   * For FP conflict:
   *     Both idStage.usesFp AND exStage.usesFp must be true,
   *     AND numFpUnits must be < 2.
   *
   * === Truth Table for ALU Conflict (1 ALU) ===
   *
   *     ID.usesAlu | EX.usesAlu | Conflict?
   *     -----------+-----------+----------
   *     false      | false     | No
   *     false      | true      | No  (ID doesn't need ALU)
   *     true       | false     | No  (EX doesn't need ALU)
   *     true       | true      | YES (both need the 1 ALU)
   */
  private _checkExecutionUnitConflict(
    idStage: PipelineSlot,
    exStage: PipelineSlot,
  ): HazardResult {
    if (!idStage.valid || !exStage.valid) {
      return new HazardResult({
        action: HazardAction.NONE,
        reason: "one or both stages are empty (bubble)",
      });
    }

    // ALU conflict: both need ALU, but we only have 1.
    if (idStage.usesAlu && exStage.usesAlu && this._numAlus < 2) {
      return new HazardResult({
        action: HazardAction.STALL,
        stallCycles: 1,
        reason:
          `structural hazard: both ID (PC=0x${idStage.pc.toString(16).toUpperCase().padStart(4, "0")}) ` +
          `and EX (PC=0x${exStage.pc.toString(16).toUpperCase().padStart(4, "0")}) need the ALU, ` +
          `but only ${this._numAlus} ALU available`,
      });
    }

    // FP unit conflict: both need FP, but we only have 1.
    if (idStage.usesFp && exStage.usesFp && this._numFpUnits < 2) {
      return new HazardResult({
        action: HazardAction.STALL,
        stallCycles: 1,
        reason:
          `structural hazard: both ID (PC=0x${idStage.pc.toString(16).toUpperCase().padStart(4, "0")}) ` +
          `and EX (PC=0x${exStage.pc.toString(16).toUpperCase().padStart(4, "0")}) need the FP unit, ` +
          `but only ${this._numFpUnits} FP unit available`,
      });
    }

    return new HazardResult({
      action: HazardAction.NONE,
      reason: "no execution unit conflict",
    });
  }

  /**
   * Check if IF and MEM both need the memory bus.
   *
   * This only matters when splitCaches is false (shared L1 cache).
   * With split caches, IF reads from L1I and MEM reads/writes L1D
   * independently — no conflict.
   *
   * === When Does This Happen? ===
   *
   * IF always needs memory (to fetch the next instruction).
   * MEM only needs memory when it's a load (memRead) or store
   * (memWrite). So the conflict occurs when:
   *
   *     ifStage.valid AND memStage.valid AND
   *     (memStage.memRead OR memStage.memWrite) AND
   *     NOT splitCaches
   */
  private _checkMemoryPortConflict(
    ifStage: PipelineSlot,
    memStage: PipelineSlot,
  ): HazardResult {
    // With split caches, fetch and data access never conflict.
    if (this._splitCaches) {
      return new HazardResult({
        action: HazardAction.NONE,
        reason: "split caches — no memory port conflict",
      });
    }

    // Both stages must be valid and MEM must actually access memory.
    if (
      ifStage.valid &&
      memStage.valid &&
      (memStage.memRead || memStage.memWrite)
    ) {
      const accessType = memStage.memRead ? "load" : "store";
      return new HazardResult({
        action: HazardAction.STALL,
        stallCycles: 1,
        reason:
          `structural hazard: IF (fetch at PC=0x${ifStage.pc.toString(16).toUpperCase().padStart(4, "0")}) ` +
          `and MEM (${accessType} at PC=0x${memStage.pc.toString(16).toUpperCase().padStart(4, "0")}) ` +
          `both need the shared memory bus`,
      });
    }

    return new HazardResult({
      action: HazardAction.NONE,
      reason: "no memory port conflict",
    });
  }
}
