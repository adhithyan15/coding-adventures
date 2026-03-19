/**
 * Pipeline -- the fetch-decode-execute cycle that drives every CPU.
 *
 * === What is a pipeline? ===
 *
 * Every CPU operates by repeating three steps over and over:
 *
 *     +---------+     +---------+     +---------+
 *     |  FETCH  | --> | DECODE  | --> | EXECUTE | --> (repeat)
 *     +---------+     +---------+     +---------+
 *
 * 1. FETCH:   Read the next instruction from memory at the address stored
 *             in the Program Counter (PC). The instruction is just a number
 *             -- a pattern of bits that encodes what operation to perform.
 *
 * 2. DECODE:  Figure out what those bits mean. Which operation is it? (ADD?
 *             LOAD? BRANCH?) Which registers are involved? Is there an
 *             immediate value encoded in the instruction?
 *
 * 3. EXECUTE: Perform the operation. This might mean sending values through
 *             the ALU (for arithmetic), reading/writing memory (for loads
 *             and stores), or changing the PC (for branches/jumps).
 *
 * After execution, the PC is updated (usually PC += 4 for 32-bit instruction
 * sets) and the cycle repeats.
 *
 * === Why is it called a "pipeline"? ===
 *
 * In simple CPUs (like ours), these three stages happen one after another
 * for each instruction. But in modern CPUs, they overlap -- while one
 * instruction is being executed, the next one is being decoded, and the one
 * after that is being fetched. This is called "pipelining" and it's how
 * CPUs achieve high throughput.
 *
 * Think of it like a laundry pipeline:
 *   - Simple: wash shirt 1, dry shirt 1, fold shirt 1, THEN wash shirt 2...
 *   - Pipelined: while shirt 1 is drying, start washing shirt 2.
 *                While shirt 2 is drying and shirt 1 is being folded,
 *                start washing shirt 3.
 *
 * Our simulator starts with a simple non-pipelined design (one instruction
 * fully completes before the next begins) but exposes the pipeline stages
 * visibly so you can see what happens at each step.
 *
 * === Pipeline hazards (future) ===
 *
 * Pipelining introduces problems called "hazards":
 *   - Data hazard: instruction 2 needs the result of instruction 1, but
 *     instruction 1 hasn't finished yet
 *   - Control hazard: a branch instruction changes the PC, so the
 *     instructions we already fetched are wrong (pipeline "flush")
 *   - Structural hazard: two instructions need the same hardware unit
 *     at the same time
 *
 * These are fascinating problems that we'll explore as we add pipelining.
 */

// ---------------------------------------------------------------------------
// Pipeline stage enum
// ---------------------------------------------------------------------------

/**
 * The three stages of the fetch-decode-execute cycle.
 *
 * Each instruction passes through these stages in order:
 *
 *     FETCH -> DECODE -> EXECUTE
 *
 * In our simple (non-pipelined) CPU, only one stage is active at a time.
 * In a pipelined CPU, up to three instructions can be in different stages
 * simultaneously.
 */
export enum PipelineStage {
  FETCH = "fetch",
  DECODE = "decode",
  EXECUTE = "execute",
}

// ---------------------------------------------------------------------------
// Stage result types
// ---------------------------------------------------------------------------

/**
 * What the FETCH stage produces.
 *
 * The fetch stage reads raw bytes from memory at the current PC address.
 * It doesn't know what the bytes mean -- that's the decode stage's job.
 *
 * Example:
 *     PC = 0x00000004
 *     The 4 bytes at that address are: 0x00 0x20 0x81 0xB3
 *     rawInstruction = 0x002081B3
 *
 * In the pipeline diagram:
 *     +--------------------------------------+
 *     | FETCH                                |
 *     | PC: 0x00000004                       |
 *     | Read 4 bytes -> 0x002081B3           |
 *     +--------------------------------------+
 */
export interface FetchResult {
  /** Program Counter value when the fetch occurred. */
  pc: number;
  /** The raw 32-bit instruction word. */
  rawInstruction: number;
}

/**
 * What the DECODE stage produces.
 *
 * The decode stage takes the raw instruction bits and extracts the
 * meaningful fields: what operation, which registers, what immediate value.
 *
 * This is ISA-specific -- RISC-V, ARM, WASM, and 4004 all decode
 * differently. The CPU simulator provides this as a generic container;
 * the ISA simulator fills in the details.
 *
 * Example (RISC-V 'add x3, x1, x2'):
 *     mnemonic = "add"
 *     fields = { rd: 3, rs1: 1, rs2: 2, funct3: 0, funct7: 0 }
 *
 * In the pipeline diagram:
 *     +--------------------------------------+
 *     | DECODE                               |
 *     | 0x002081B3 -> add x3, x1, x2        |
 *     | rd=3, rs1=1, rs2=2                   |
 *     +--------------------------------------+
 */
export interface DecodeResult {
  /** Human-readable instruction name. */
  mnemonic: string;
  /** Decoded fields (ISA-specific). */
  fields: Record<string, number>;
  /** The raw instruction (for display). */
  rawInstruction: number;
}

/**
 * What the EXECUTE stage produces.
 *
 * The execute stage performs the actual operation and records what changed.
 *
 * Example (add x3, x1, x2 where x1=1, x2=2):
 *     description = "x3 = x1 + x2 = 1 + 2 = 3"
 *     registersChanged = { x3: 3 }
 *     memoryChanged = {}
 *     nextPc = 12  (PC + 4, normal sequential execution)
 *
 * In the pipeline diagram:
 *     +--------------------------------------+
 *     | EXECUTE                              |
 *     | add x3, x1, x2                      |
 *     | ALU: 1 + 2 = 3                      |
 *     | Write x3 = 3                        |
 *     | PC -> 12                            |
 *     +--------------------------------------+
 */
export interface ExecuteResult {
  /** Human-readable description of what happened. */
  description: string;
  /** Which registers changed and to what values. */
  registersChanged: Record<string, number>;
  /** Which memory addresses changed. */
  memoryChanged: Record<number, number>;
  /** The new program counter value. */
  nextPc: number;
  /** Did this instruction halt the CPU? */
  halted: boolean;
}

// ---------------------------------------------------------------------------
// Pipeline trace
// ---------------------------------------------------------------------------

/**
 * A complete record of one instruction's journey through the pipeline.
 *
 * This is the main data structure for visualization. It captures what
 * happened at each stage, allowing you to see the full pipeline:
 *
 *     +----------------------------------------------------------+
 *     | Instruction #0                                           |
 *     +--------------+------------------+-----------------------+
 *     | FETCH        | DECODE           | EXECUTE               |
 *     | PC: 0x0000   | addi x1, x0, 1  | x1 = 0 + 1 = 1       |
 *     | -> 0x00100093| rd=1, rs1=0,     | Write x1 = 1          |
 *     |              | imm=1            | PC -> 4               |
 *     +--------------+------------------+-----------------------+
 *
 * Example:
 *     const trace: PipelineTrace = {
 *         cycle: 0,
 *         fetch: { pc: 0, rawInstruction: 0x00100093 },
 *         decode: { mnemonic: "addi", fields: { rd: 1, rs1: 0, imm: 1 }, rawInstruction: 0x00100093 },
 *         execute: { description: "x1 = 0 + 1 = 1", registersChanged: { x1: 1 }, memoryChanged: {}, nextPc: 4, halted: false },
 *         registerSnapshot: { R0: 0, R1: 1 },
 *     };
 */
export interface PipelineTrace {
  /** Which instruction number this is (0, 1, 2, ...). */
  cycle: number;
  /** Result of the fetch stage. */
  fetch: FetchResult;
  /** Result of the decode stage. */
  decode: DecodeResult;
  /** Result of the execute stage. */
  execute: ExecuteResult;
  /** Snapshot of all register values after execution. */
  registerSnapshot: Record<string, number>;
}

/**
 * Format a pipeline trace as a visual pipeline diagram.
 *
 * Returns a multi-line string showing all three stages side by side.
 *
 * Example output:
 *     --- Cycle 0 ---
 *       FETCH              | DECODE             | EXECUTE
 *       PC: 0x0000         | addi x1, x0, 1     | x1 = 1
 *       -> 0x00100093      | rd=1 rs1=0 imm=1   | PC -> 4
 */
export function formatPipeline(trace: PipelineTrace): string {
  const fetchLines = [
    "FETCH",
    `PC: 0x${trace.fetch.pc.toString(16).toUpperCase().padStart(4, "0")}`,
    `-> 0x${trace.fetch.rawInstruction.toString(16).toUpperCase().padStart(8, "0")}`,
  ];

  const decodeLines = [
    "DECODE",
    trace.decode.mnemonic,
    Object.entries(trace.decode.fields)
      .map(([k, v]) => `${k}=${v}`)
      .join(" "),
  ];

  const executeLines = [
    "EXECUTE",
    trace.execute.description,
    `PC -> ${trace.execute.nextPc}`,
  ];

  // Pad all columns to same number of lines
  const maxLines = Math.max(
    fetchLines.length,
    decodeLines.length,
    executeLines.length
  );
  while (fetchLines.length < maxLines) fetchLines.push("");
  while (decodeLines.length < maxLines) decodeLines.push("");
  while (executeLines.length < maxLines) executeLines.push("");

  // Format as columns
  const colWidth = 20;
  const result = [`--- Cycle ${trace.cycle} ---`];
  for (let i = 0; i < maxLines; i++) {
    const f = fetchLines[i].padEnd(colWidth);
    const d = decodeLines[i].padEnd(colWidth);
    const e = executeLines[i].padEnd(colWidth);
    result.push(`  ${f} | ${d} | ${e}`);
  }

  return result.join("\n");
}
