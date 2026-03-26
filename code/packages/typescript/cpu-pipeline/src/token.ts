/**
 * PipelineToken, PipelineStage, StageCategory, and PipelineConfig --
 * the data structures that define what flows through the pipeline and
 * how the pipeline itself is structured.
 *
 * # Assembly Line Analogy
 *
 * A CPU pipeline is like a factory assembly line:
 *
 *     Single-cycle (no pipeline):
 *     Instr 1: [IF][ID][EX][MEM][WB]
 *     Instr 2:                       [IF][ID][EX][MEM][WB]
 *     Throughput: 1 instruction every 5 cycles
 *
 *     Pipelined:
 *     Instr 1: [IF][ID][EX][MEM][WB]
 *     Instr 2:     [IF][ID][EX][MEM][WB]
 *     Instr 3:         [IF][ID][EX][MEM][WB]
 *     Throughput: 1 instruction every 1 cycle (after filling)
 *
 * The PipelineToken is the "tray" on the assembly line. It starts empty
 * at the IF stage, gets filled with decoded information at ID, gets
 * computed results at EX, memory data at MEM, and delivers results at WB.
 */

// =========================================================================
// StageCategory -- classifies pipeline stages by function
// =========================================================================

/**
 * StageCategory classifies pipeline stages by their function.
 *
 * Every stage in a pipeline does one of these five jobs, regardless of
 * how many stages the pipeline has. A 5-stage pipeline has one stage per
 * category. A 13-stage pipeline might have 2 fetch stages, 2 decode
 * stages, 3 execute stages, etc.
 *
 * This classification determines:
 * - Which callback to invoke for each stage
 * - Where to insert stall bubbles
 * - Which stages to flush on a misprediction
 */
export enum StageCategory {
  /** Stages that read instructions from the instruction cache. */
  Fetch = "fetch",
  /** Stages that decode the instruction and read registers. */
  Decode = "decode",
  /** Stages that perform computation (ALU, branch resolution). */
  Execute = "execute",
  /** Stages that access data memory (loads and stores). */
  Memory = "memory",
  /** Stages that write results back to the register file. Always the final stage. */
  Writeback = "writeback",
}

// =========================================================================
// PipelineStage -- definition of a single stage
// =========================================================================

/**
 * PipelineStage defines a single stage in the pipeline.
 *
 * A stage has a short name (used in diagrams), a description (for humans),
 * and a category (for the pipeline to know what callback to invoke).
 *
 * Example stages:
 *
 *     { name: "IF",  description: "Instruction Fetch", category: StageCategory.Fetch }
 *     { name: "EX1", description: "Execute - ALU",     category: StageCategory.Execute }
 */
export interface PipelineStage {
  /** Short name like "IF", "ID", "EX1". */
  name: string;
  /** Human-readable description. */
  description: string;
  /** What kind of work this stage does. */
  category: StageCategory;
}

// =========================================================================
// PipelineToken -- a unit of work flowing through the pipeline
// =========================================================================

/**
 * PipelineToken represents one instruction moving through the pipeline.
 *
 * Think of it as a tray on an assembly line. The tray starts empty at the
 * IF stage, gets filled with decoded information at ID, gets computed
 * results at EX, gets memory data at MEM, and delivers results at WB.
 *
 * The token is ISA-independent. The ISA decoder fills in the fields via
 * callbacks. The pipeline itself never looks at instruction semantics --
 * it only moves tokens between stages and handles stalls/flushes.
 *
 * # Token Lifecycle
 *
 *     IF stage:  FetchFunc fills in pc and rawInstruction
 *     ID stage:  DecodeFunc fills in opcode, registers, control signals
 *     EX stage:  ExecuteFunc fills in aluResult, branchTaken, branchTarget
 *     MEM stage: MemoryFunc fills in memData (for loads)
 *     WB stage:  WritebackFunc uses writeData to update register file
 *
 * # Bubbles
 *
 * A "bubble" is a special token that represents NO instruction. Bubbles
 * are inserted when the pipeline stalls or flushes. A bubble flows through
 * the pipeline like a normal token but does nothing at each stage.
 */
export interface PipelineToken {
  // --- Instruction identity ---

  /** Program counter -- the memory address of this instruction. */
  pc: number;
  /** Raw instruction bits as fetched from memory. */
  rawInstruction: number;
  /** Decoded instruction name (e.g., "ADD", "LDR", "BEQ"). */
  opcode: string;

  // --- Decoded operands (set by ID stage callback) ---

  /** First source register number (-1 means unused). */
  rs1: number;
  /** Second source register number (-1 means unused). */
  rs2: number;
  /** Destination register number (-1 means unused). */
  rd: number;
  /** Sign-extended immediate value from the instruction. */
  immediate: number;

  // --- Control signals (set by ID stage callback) ---

  /** True if this instruction writes a register. */
  regWrite: boolean;
  /** True if this instruction reads from data memory. */
  memRead: boolean;
  /** True if this instruction writes to data memory. */
  memWrite: boolean;
  /** True if this instruction is a branch (conditional or unconditional). */
  isBranch: boolean;
  /** True if this is a halt/stop instruction. */
  isHalt: boolean;

  // --- Computed values (filled during execution) ---

  /** Output of the ALU in the EX stage. */
  aluResult: number;
  /** Data read from memory in the MEM stage. */
  memData: number;
  /** Final value to write to the destination register. */
  writeData: number;
  /** True if the branch was actually taken (resolved in EX). */
  branchTaken: boolean;
  /** Actual branch target address (resolved in EX). */
  branchTarget: number;

  // --- Pipeline metadata ---

  /** True if this token represents a NOP/bubble. */
  isBubble: boolean;
  /**
   * Maps stage name to the cycle number when the token entered that stage.
   * Example: { "IF": 1, "ID": 2, "EX": 4, "MEM": 5, "WB": 6 }
   */
  stageEntered: Record<string, number>;
  /** Which stage provided a forwarded value, if any. Empty string means none. */
  forwardedFrom: string;
}

/**
 * Creates a new empty token with default register values.
 *
 * The token starts with all register fields set to -1 (unused) and
 * all control signals set to false. The fetch callback will fill in
 * the PC and raw instruction; the decode callback fills in everything else.
 */
export function newToken(): PipelineToken {
  return {
    pc: 0,
    rawInstruction: 0,
    opcode: "",
    rs1: -1,
    rs2: -1,
    rd: -1,
    immediate: 0,
    regWrite: false,
    memRead: false,
    memWrite: false,
    isBranch: false,
    isHalt: false,
    aluResult: 0,
    memData: 0,
    writeData: 0,
    branchTaken: false,
    branchTarget: 0,
    isBubble: false,
    stageEntered: {},
    forwardedFrom: "",
  };
}

/**
 * Creates a new bubble token.
 *
 * A bubble is a "do nothing" instruction that occupies a pipeline stage
 * without performing any useful work. It is the pipeline equivalent of
 * a "no-op" on an assembly line.
 */
export function newBubble(): PipelineToken {
  return {
    ...newToken(),
    isBubble: true,
  };
}

/**
 * Returns a human-readable representation of a token.
 *
 * - Bubbles display as "---"
 * - Normal tokens display their opcode and PC
 */
export function tokenToString(t: PipelineToken): string {
  if (t.isBubble) return "---";
  if (t.opcode !== "") return `${t.opcode}@${t.pc}`;
  return `instr@${t.pc}`;
}

/**
 * Returns a deep copy of a token.
 *
 * This is necessary because tokens are passed between pipeline stages
 * via pipeline registers. Each register holds its own copy so that
 * modifying a token in one stage does not affect the copy in the
 * pipeline register.
 */
export function cloneToken(t: PipelineToken | null): PipelineToken | null {
  if (t === null) return null;
  return {
    ...t,
    stageEntered: { ...t.stageEntered },
  };
}

// =========================================================================
// PipelineConfig -- configuration for the pipeline
// =========================================================================

/**
 * PipelineConfig holds the configuration for a pipeline.
 *
 * The key insight: a pipeline's behavior is determined entirely by its
 * stage configuration and execution width. Everything else (instruction
 * semantics, hazard handling) is injected via callbacks.
 */
export interface PipelineConfig {
  /** Pipeline stages in order, from first to last. */
  stages: PipelineStage[];
  /**
   * Number of instructions the pipeline can process per cycle.
   * Width 1 = scalar pipeline. Width > 1 = superscalar (future extension).
   */
  executionWidth: number;
}

/**
 * Returns the standard 5-stage RISC pipeline configuration.
 *
 * This matches the MIPS R2000 (1985) and is the foundation for
 * understanding all modern CPU pipelines:
 *
 *     IF -> ID -> EX -> MEM -> WB
 */
export function classic5Stage(): PipelineConfig {
  return {
    stages: [
      { name: "IF", description: "Instruction Fetch", category: StageCategory.Fetch },
      { name: "ID", description: "Instruction Decode", category: StageCategory.Decode },
      { name: "EX", description: "Execute", category: StageCategory.Execute },
      { name: "MEM", description: "Memory Access", category: StageCategory.Memory },
      { name: "WB", description: "Write Back", category: StageCategory.Writeback },
    ],
    executionWidth: 1,
  };
}

/**
 * Returns a 13-stage pipeline inspired by ARM Cortex-A78.
 *
 * Modern high-performance CPUs split the classic 5 stages into many
 * sub-stages to enable higher clock frequencies. The tradeoff: a branch
 * misprediction now costs 10+ cycles instead of 2.
 */
export function deep13Stage(): PipelineConfig {
  return {
    stages: [
      { name: "IF1", description: "Fetch 1 - TLB lookup", category: StageCategory.Fetch },
      { name: "IF2", description: "Fetch 2 - cache read", category: StageCategory.Fetch },
      { name: "IF3", description: "Fetch 3 - align/buffer", category: StageCategory.Fetch },
      { name: "ID1", description: "Decode 1 - pre-decode", category: StageCategory.Decode },
      { name: "ID2", description: "Decode 2 - full decode", category: StageCategory.Decode },
      { name: "ID3", description: "Decode 3 - register read", category: StageCategory.Decode },
      { name: "EX1", description: "Execute 1 - ALU", category: StageCategory.Execute },
      { name: "EX2", description: "Execute 2 - shift/multiply", category: StageCategory.Execute },
      { name: "EX3", description: "Execute 3 - result select", category: StageCategory.Execute },
      { name: "MEM1", description: "Memory 1 - address calc", category: StageCategory.Memory },
      { name: "MEM2", description: "Memory 2 - cache access", category: StageCategory.Memory },
      { name: "MEM3", description: "Memory 3 - data align", category: StageCategory.Memory },
      { name: "WB", description: "Write Back", category: StageCategory.Writeback },
    ],
    executionWidth: 1,
  };
}

/**
 * Returns the number of stages in the pipeline.
 */
export function numStages(config: PipelineConfig): number {
  return config.stages.length;
}

/**
 * Validates that a pipeline configuration is well-formed.
 *
 * Rules:
 * - Must have at least 2 stages
 * - Execution width must be at least 1
 * - All stage names must be unique
 * - Must have at least one fetch stage and one writeback stage
 *
 * Returns null if valid, or an error message string.
 */
export function validateConfig(config: PipelineConfig): string | null {
  if (config.stages.length < 2) {
    return `pipeline must have at least 2 stages, got ${config.stages.length}`;
  }
  if (config.executionWidth < 1) {
    return `execution width must be at least 1, got ${config.executionWidth}`;
  }

  // Check for unique stage names.
  const seen = new Set<string>();
  for (const s of config.stages) {
    if (seen.has(s.name)) {
      return `duplicate stage name: "${s.name}"`;
    }
    seen.add(s.name);
  }

  // Check for required categories.
  const hasFetch = config.stages.some(s => s.category === StageCategory.Fetch);
  const hasWriteback = config.stages.some(s => s.category === StageCategory.Writeback);

  if (!hasFetch) {
    return "pipeline must have at least one fetch stage";
  }
  if (!hasWriteback) {
    return "pipeline must have at least one writeback stage";
  }

  return null;
}

// =========================================================================
// HazardAction and HazardResponse -- pipeline hazard signaling
// =========================================================================

/**
 * HazardAction represents the action the hazard unit tells the pipeline to take.
 *
 * These are "traffic signals" for the pipeline:
 *
 *     None:             Green light -- pipeline flows normally
 *     Stall:            Red light -- freeze earlier stages, insert bubble
 *     Flush:            Emergency stop -- discard speculative instructions
 *     ForwardFromEX:    Shortcut -- grab value from EX stage output
 *     ForwardFromMEM:   Shortcut -- grab value from MEM stage output
 *
 * Priority: Flush > Stall > Forward > None
 */
export enum HazardAction {
  None = "NONE",
  ForwardFromEX = "FORWARD_FROM_EX",
  ForwardFromMEM = "FORWARD_FROM_MEM",
  Stall = "STALL",
  Flush = "FLUSH",
}

/**
 * HazardResponse is the full response from the hazard detection callback.
 *
 * It tells the pipeline what to do and provides additional context
 * (forwarded values, stall duration, flush target).
 */
export interface HazardResponse {
  /** The hazard action to take. */
  action: HazardAction;
  /** Value to forward (only used for FORWARD actions). */
  forwardValue: number;
  /** Stage that provided the forwarded value. */
  forwardSource: string;
  /** Number of stages to stall (typically 1). */
  stallStages: number;
  /** Number of stages to flush on a misprediction. */
  flushCount: number;
  /** Correct PC to fetch from after a flush. */
  redirectPC: number;
}

/**
 * Creates a default HazardResponse with no action.
 */
export function noHazard(): HazardResponse {
  return {
    action: HazardAction.None,
    forwardValue: 0,
    forwardSource: "",
    stallStages: 0,
    flushCount: 0,
    redirectPC: 0,
  };
}

// =========================================================================
// Callback function types
// =========================================================================

/**
 * FetchFunc fetches the raw instruction bits at the given program counter.
 * In a real CPU, this reads from the instruction cache (L1I).
 */
export type FetchFunc = (pc: number) => number;

/**
 * DecodeFunc decodes a raw instruction and fills in the token's fields.
 * Returns the token with all decoded fields filled in.
 */
export type DecodeFunc = (rawInstruction: number, token: PipelineToken) => PipelineToken;

/**
 * ExecuteFunc performs the ALU operation for the instruction.
 * Returns the token with ALUResult, BranchTaken, and BranchTarget filled in.
 */
export type ExecuteFunc = (token: PipelineToken) => PipelineToken;

/**
 * MemoryFunc performs the memory access (load/store) for the instruction.
 */
export type MemoryFunc = (token: PipelineToken) => PipelineToken;

/**
 * WritebackFunc writes the instruction's result to the register file.
 */
export type WritebackFunc = (token: PipelineToken) => void;

/**
 * HazardFunc checks for hazards given the current pipeline stage contents.
 * The stages array is ordered from first stage (IF) to last stage (WB).
 * A null entry means the stage is empty.
 */
export type HazardFunc = (stages: (PipelineToken | null)[]) => HazardResponse;

/**
 * PredictFunc predicts the next PC given the current PC.
 * Used by the IF stage to speculatively fetch the next instruction.
 */
export type PredictFunc = (pc: number) => number;
