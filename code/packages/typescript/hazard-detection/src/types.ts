/**
 * Shared data types for pipeline hazard detection.
 *
 * === Why These Types Exist ===
 *
 * A CPU pipeline is like an assembly line: each stage works on a different
 * instruction simultaneously. But sometimes instructions interfere with each
 * other — one instruction needs a result that another hasn't produced yet,
 * or two instructions fight over the same hardware resource.
 *
 * The hazard detection unit needs to know what each pipeline stage is doing
 * WITHOUT knowing the specifics of the instruction set. It doesn't care
 * whether you're running ARM, RISC-V, or x86 — it only needs to know:
 *
 *   1. Which registers does this instruction READ?
 *   2. Which register does it WRITE?
 *   3. Is it a branch? Was it predicted correctly?
 *   4. What hardware resources does it need (ALU, FP unit, memory)?
 *
 * These types capture exactly that information, nothing more.
 *
 * === The Pipeline Stages (5-Stage Classic) ===
 *
 *     IF → ID → EX → MEM → WB
 *     │    │    │     │     │
 *     │    │    │     │     └─ Write Back: write result to register file
 *     │    │    │     └─ Memory: load/store data from/to memory
 *     │    │    └─ Execute: ALU computes result
 *     │    └─ Instruction Decode: read registers, detect hazards
 *     └─ Instruction Fetch: grab instruction from memory
 *
 * The hazard unit sits between ID and EX. It peeks at what's in each stage
 * and decides: "Can ID proceed, or do we need to stall/forward/flush?"
 */

// ---------------------------------------------------------------------------
// PipelineSlot — what the hazard unit sees in each pipeline stage
// ---------------------------------------------------------------------------

/**
 * Information about an instruction occupying a pipeline stage.
 *
 * This is ISA-independent — whatever decoder is plugged in extracts this
 * info from raw instruction bits. The hazard unit only cares about register
 * numbers and resource usage, not opcodes.
 *
 * === Fields Explained ===
 *
 * valid:
 *     Is there actually an instruction here? After a flush or at startup,
 *     stages contain "bubbles" (empty slots) — valid=false.
 *
 * pc:
 *     Program counter. Useful for debugging ("which instruction caused
 *     the hazard?"), not used for hazard logic itself.
 *
 * sourceRegs:
 *     Array of register numbers this instruction READS. For example,
 *     ADD R1, R2, R3 reads R2 and R3, so sourceRegs = [2, 3].
 *     Treated as readonly — never mutated after construction.
 *
 * destReg:
 *     The register this instruction WRITES. ADD R1, R2, R3 writes R1,
 *     so destReg = 1. Instructions that don't write (like a store or
 *     a branch) have destReg = null.
 *
 * destValue:
 *     The computed result, if available. After the EX stage, the ALU
 *     result is known. After MEM, the loaded value is known. This is
 *     what gets forwarded to avoid stalls.
 *
 * isBranch / branchTaken / branchPredictedTaken:
 *     Branch instructions change the flow of execution. The predictor
 *     guesses the outcome during IF; the actual outcome is known in EX.
 *     If the guess was wrong, we must flush the pipeline.
 *
 * memRead / memWrite:
 *     Load (memRead=true) and store (memWrite=true) instructions.
 *     Loads are special because the value isn't available until after
 *     MEM — so a load followed immediately by a use MUST stall.
 *
 * usesAlu / usesFp:
 *     Which execution unit does this instruction need? Most instructions
 *     use the ALU. Floating-point ops use the FP unit. If two instructions
 *     in the pipeline need the same unit at the same time, that's a
 *     structural hazard.
 *
 * === Example: Encoding "ADD R1, R2, R3" ===
 *
 *     {
 *         valid: true,
 *         pc: 0x1000,
 *         sourceRegs: [2, 3],   // reads R2 and R3
 *         destReg: 1,           // writes R1
 *         destValue: null,      // not computed yet (still in ID)
 *         isBranch: false,
 *         branchTaken: false,
 *         branchPredictedTaken: false,
 *         memRead: false,
 *         memWrite: false,
 *         usesAlu: true,
 *         usesFp: false,
 *     }
 */
export interface PipelineSlotFields {
  valid?: boolean;
  pc?: number;
  sourceRegs?: readonly number[];
  destReg?: number | null;
  destValue?: number | null;
  isBranch?: boolean;
  branchTaken?: boolean;
  branchPredictedTaken?: boolean;
  memRead?: boolean;
  memWrite?: boolean;
  usesAlu?: boolean;
  usesFp?: boolean;
}

/**
 * A frozen (immutable) snapshot of a pipeline stage's contents.
 *
 * We use a class rather than a plain object so we can provide sensible
 * defaults for every field. An empty `new PipelineSlot()` represents a
 * "bubble" — an empty pipeline stage (valid=false).
 */
export class PipelineSlot {
  readonly valid: boolean;
  readonly pc: number;
  readonly sourceRegs: readonly number[];
  readonly destReg: number | null;
  readonly destValue: number | null;
  readonly isBranch: boolean;
  readonly branchTaken: boolean;
  readonly branchPredictedTaken: boolean;
  readonly memRead: boolean;
  readonly memWrite: boolean;
  readonly usesAlu: boolean;
  readonly usesFp: boolean;

  constructor(fields: PipelineSlotFields = {}) {
    this.valid = fields.valid ?? false;
    this.pc = fields.pc ?? 0;
    this.sourceRegs = fields.sourceRegs ?? [];
    this.destReg = fields.destReg ?? null;
    this.destValue = fields.destValue ?? null;
    this.isBranch = fields.isBranch ?? false;
    this.branchTaken = fields.branchTaken ?? false;
    this.branchPredictedTaken = fields.branchPredictedTaken ?? false;
    this.memRead = fields.memRead ?? false;
    this.memWrite = fields.memWrite ?? false;
    this.usesAlu = fields.usesAlu ?? true;
    this.usesFp = fields.usesFp ?? false;
  }
}

// ---------------------------------------------------------------------------
// HazardAction — what the hazard unit tells the pipeline to do
// ---------------------------------------------------------------------------

/**
 * The action the hazard unit instructs the pipeline to take.
 *
 * Think of these as traffic signals for the pipeline:
 *
 * === NONE (Green Light) ===
 * Everything is fine. The pipeline flows normally.
 *
 * === FORWARD_FROM_EX (Yellow Shortcut from EX) ===
 * "The value you need is right HERE in the EX stage — grab it!"
 * Instead of waiting for the instruction to reach WB, we wire the
 * EX output directly back to the ID input. No time lost.
 *
 *     Without forwarding:         With forwarding:
 *     ADD R1, R2, R3  [EX]       ADD R1, R2, R3  [EX] ──┐
 *     SUB R4, R1, R5  [ID] STALL SUB R4, R1, R5  [ID] ←─┘ OK!
 *
 * === FORWARD_FROM_MEM (Yellow Shortcut from MEM) ===
 * Same idea, but the value comes from the MEM stage. This happens
 * when there's a 2-instruction gap, or after a load completes.
 *
 * === STALL (Red Light) ===
 * "STOP! You can't proceed yet." The pipeline freezes the IF and ID
 * stages and inserts a bubble (NOP) into EX. This happens when
 * forwarding can't help — typically a load-use hazard:
 *
 *     LW R1, [addr]   [EX]  ← value won't be ready until after MEM
 *     ADD R4, R1, R5  [ID]  ← needs R1 NOW — must wait 1 cycle
 *
 * === FLUSH (Emergency Stop) ===
 * "WRONG WAY! Throw out everything!" A branch was mispredicted, so
 * the instructions that were fetched after it are WRONG. We must
 * discard them (replace with bubbles) and restart from the correct PC.
 *
 *     BEQ R1, R2, target  [EX]  ← discovers branch IS taken
 *     wrong_instr_1        [ID]  ← FLUSH (replace with bubble)
 *     wrong_instr_2        [IF]  ← FLUSH (replace with bubble)
 */
export enum HazardAction {
  NONE = "none",
  FORWARD_FROM_EX = "forward_ex",
  FORWARD_FROM_MEM = "forward_mem",
  STALL = "stall",
  FLUSH = "flush",
}

// ---------------------------------------------------------------------------
// HazardResult — the complete verdict from hazard detection
// ---------------------------------------------------------------------------

/**
 * Complete result from hazard detection — may include multiple details.
 *
 * === Why a Structured Result? ===
 *
 * A simple "stall or not" boolean isn't enough. The pipeline needs to know:
 * - WHAT action to take (forward? stall? flush?)
 * - The forwarded VALUE (if forwarding)
 * - WHERE it came from (for debugging: "forwarded from EX" vs "from MEM")
 * - HOW MANY cycles to stall (usually 1, but could be more)
 * - HOW MANY stages to flush (branch misprediction flushes IF and ID)
 * - WHY (human-readable explanation for debugging and learning)
 *
 * === Examples ===
 *
 * No hazard:
 *     { action: HazardAction.NONE, reason: "no dependencies" }
 *
 * RAW hazard resolved by forwarding from EX:
 *     {
 *         action: HazardAction.FORWARD_FROM_EX,
 *         forwardedValue: 42,
 *         forwardedFrom: "EX",
 *         reason: "R1 produced by ADD in EX, forwarded to SUB in ID",
 *     }
 *
 * Load-use stall:
 *     {
 *         action: HazardAction.STALL,
 *         stallCycles: 1,
 *         reason: "R1 loaded by LW in EX, needed by ADD in ID — must wait",
 *     }
 *
 * Branch misprediction flush:
 *     {
 *         action: HazardAction.FLUSH,
 *         flushCount: 2,
 *         reason: "BEQ mispredicted: predicted not-taken, actually taken",
 *     }
 */
export interface HazardResultFields {
  action?: HazardAction;
  forwardedValue?: number | null;
  forwardedFrom?: string;
  stallCycles?: number;
  flushCount?: number;
  reason?: string;
}

export class HazardResult {
  readonly action: HazardAction;
  readonly forwardedValue: number | null;
  readonly forwardedFrom: string;
  readonly stallCycles: number;
  readonly flushCount: number;
  readonly reason: string;

  constructor(fields: HazardResultFields = {}) {
    this.action = fields.action ?? HazardAction.NONE;
    this.forwardedValue = fields.forwardedValue ?? null;
    this.forwardedFrom = fields.forwardedFrom ?? "";
    this.stallCycles = fields.stallCycles ?? 0;
    this.flushCount = fields.flushCount ?? 0;
    this.reason = fields.reason ?? "";
  }
}
