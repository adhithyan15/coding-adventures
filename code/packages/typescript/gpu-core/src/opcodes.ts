/**
 * Opcodes and Instructions -- the vocabulary of GPU core programs.
 *
 * === What is an Opcode? ===
 *
 * An opcode (operation code) is a number or name that tells the processor what
 * to do. It's like a verb in a sentence:
 *
 *     English:  "Add the first two numbers and store in the third"
 *     Assembly: FADD R2, R0, R1
 *
 * The opcode is FADD. The registers R0, R1, R2 are the operands.
 *
 * === Instruction Representation ===
 *
 * Real GPU hardware represents instructions as binary words (32 or 64 bits of
 * 1s and 0s packed together). But at this layer -- the processing element
 * simulator -- we use a structured TypeScript object instead:
 *
 *     Binary (real hardware): 01001000_00000010_00000000_00000001
 *     Our representation:     { opcode: Opcode.FADD, rd: 2, rs1: 0, rs2: 1 }
 *
 * Why? Because binary encoding is the job of the *assembler* layer above us.
 * The processing element receives already-decoded instructions from the
 * instruction cache. We're simulating what happens *after* decode.
 *
 * === The Instruction Set ===
 *
 * Our GenericISA has 16 opcodes organized into four categories:
 *
 *     Arithmetic:  FADD, FSUB, FMUL, FFMA, FNEG, FABS  (6 opcodes)
 *     Memory:      LOAD, STORE                           (2 opcodes)
 *     Data move:   MOV, LIMM                             (2 opcodes)
 *     Control:     BEQ, BLT, BNE, JMP, NOP, HALT         (6 opcodes)
 *
 * This is deliberately minimal. Real ISAs have hundreds of opcodes, but these
 * 16 are enough to write any floating-point program (they're Turing-complete
 * when combined with branches and memory).
 *
 * === Helper Constructors ===
 *
 * Writing programs as raw Instruction objects is verbose. The helper
 * functions (fadd, fmul, ffma, load, store, limm, halt, etc.) make programs
 * readable:
 *
 *     // Without helpers (verbose):
 *     const program = [
 *         { opcode: Opcode.LIMM, rd: 0, rs1: 0, rs2: 0, rs3: 0, immediate: 2.0 },
 *         { opcode: Opcode.LIMM, rd: 1, rs1: 0, rs2: 0, rs3: 0, immediate: 3.0 },
 *         { opcode: Opcode.FMUL, rd: 2, rs1: 0, rs2: 1, rs3: 0, immediate: 0 },
 *         { opcode: Opcode.HALT, rd: 0, rs1: 0, rs2: 0, rs3: 0, immediate: 0 },
 *     ];
 *
 *     // With helpers (clean):
 *     const program = [limm(0, 2.0), limm(1, 3.0), fmul(2, 0, 1), halt()];
 */

// ---------------------------------------------------------------------------
// Opcode enum -- the 16 operations our GPU core understands
// ---------------------------------------------------------------------------

/**
 * The set of operations a GPU core can perform.
 *
 * Organized by category:
 *
 * Floating-point arithmetic (uses fp-arithmetic package):
 *     FADD  -- add two registers
 *     FSUB  -- subtract two registers
 *     FMUL  -- multiply two registers
 *     FFMA  -- fused multiply-add (three source registers)
 *     FNEG  -- negate a register
 *     FABS  -- absolute value of a register
 *
 * Memory operations:
 *     LOAD  -- load float from memory into register
 *     STORE -- store register value to memory
 *
 * Data movement:
 *     MOV   -- copy one register to another
 *     LIMM  -- load an immediate (literal) float value
 *
 * Control flow:
 *     BEQ   -- branch if equal
 *     BLT   -- branch if less than
 *     BNE   -- branch if not equal
 *     JMP   -- unconditional jump
 *     NOP   -- no operation
 *     HALT  -- stop execution
 */
export enum Opcode {
  // Arithmetic
  FADD = "fadd",
  FSUB = "fsub",
  FMUL = "fmul",
  FFMA = "ffma",
  FNEG = "fneg",
  FABS = "fabs",

  // Memory
  LOAD = "load",
  STORE = "store",

  // Data movement
  MOV = "mov",
  LIMM = "limm",

  // Control flow
  BEQ = "beq",
  BLT = "blt",
  BNE = "bne",
  JMP = "jmp",
  NOP = "nop",
  HALT = "halt",
}

// ---------------------------------------------------------------------------
// Instruction -- a single GPU core instruction
// ---------------------------------------------------------------------------

/**
 * A single GPU core instruction.
 *
 * This is a structured representation of an instruction, not a binary
 * encoding. It contains all the information needed to execute the
 * instruction: the opcode and up to four operands.
 *
 * Fields:
 *     opcode:    What operation to perform (see Opcode enum).
 *     rd:        Destination register index (0-255).
 *     rs1:       First source register index (0-255).
 *     rs2:       Second source register index (0-255).
 *     rs3:       Third source register (used only by FFMA).
 *     immediate: A literal float value (used by LIMM, branch offsets,
 *                memory offsets). For branches, this is the number of
 *                instructions to skip (positive = forward, negative = back).
 */
export interface Instruction {
  readonly opcode: Opcode;
  readonly rd: number;
  readonly rs1: number;
  readonly rs2: number;
  readonly rs3: number;
  readonly immediate: number;
}

/**
 * Create an Instruction with sensible defaults for unspecified fields.
 *
 * Most instructions don't use all fields (e.g., HALT uses none, LIMM
 * only uses rd and immediate). This helper fills in zeros for the rest.
 */
function makeInstruction(
  partial: Partial<Instruction> & { opcode: Opcode },
): Instruction {
  return {
    rd: 0,
    rs1: 0,
    rs2: 0,
    rs3: 0,
    immediate: 0,
    ...partial,
  };
}

/**
 * Pretty-print an instruction in assembly-like syntax.
 *
 * This is used for trace output and debugging. It produces readable
 * strings like "FADD R2, R0, R1" or "LIMM R0, 3.14".
 */
export function formatInstruction(inst: Instruction): string {
  const op = inst.opcode.toUpperCase();
  switch (inst.opcode) {
    case Opcode.FADD:
    case Opcode.FSUB:
    case Opcode.FMUL:
      return `${op} R${inst.rd}, R${inst.rs1}, R${inst.rs2}`;
    case Opcode.FFMA:
      return `${op} R${inst.rd}, R${inst.rs1}, R${inst.rs2}, R${inst.rs3}`;
    case Opcode.FNEG:
    case Opcode.FABS:
      return `${op} R${inst.rd}, R${inst.rs1}`;
    case Opcode.LOAD:
      return `${op} R${inst.rd}, [R${inst.rs1}+${inst.immediate}]`;
    case Opcode.STORE:
      return `${op} [R${inst.rs1}+${inst.immediate}], R${inst.rs2}`;
    case Opcode.MOV:
      return `${op} R${inst.rd}, R${inst.rs1}`;
    case Opcode.LIMM:
      return `${op} R${inst.rd}, ${inst.immediate}`;
    case Opcode.BEQ:
    case Opcode.BLT:
    case Opcode.BNE: {
      const sign = inst.immediate >= 0 ? "+" : "";
      return `${op} R${inst.rs1}, R${inst.rs2}, ${sign}${Math.trunc(inst.immediate)}`;
    }
    case Opcode.JMP:
      return `${op} ${Math.trunc(inst.immediate)}`;
    case Opcode.NOP:
      return "NOP";
    case Opcode.HALT:
      return "HALT";
    default:
      return `${op} rd=${inst.rd} rs1=${inst.rs1} rs2=${inst.rs2}`;
  }
}

// ---------------------------------------------------------------------------
// Helper constructors -- make programs readable
// ---------------------------------------------------------------------------

/** FADD Rd, Rs1, Rs2 -- floating-point addition: Rd = Rs1 + Rs2. */
export function fadd(rd: number, rs1: number, rs2: number): Instruction {
  return makeInstruction({ opcode: Opcode.FADD, rd, rs1, rs2 });
}

/** FSUB Rd, Rs1, Rs2 -- floating-point subtraction: Rd = Rs1 - Rs2. */
export function fsub(rd: number, rs1: number, rs2: number): Instruction {
  return makeInstruction({ opcode: Opcode.FSUB, rd, rs1, rs2 });
}

/** FMUL Rd, Rs1, Rs2 -- floating-point multiplication: Rd = Rs1 * Rs2. */
export function fmul(rd: number, rs1: number, rs2: number): Instruction {
  return makeInstruction({ opcode: Opcode.FMUL, rd, rs1, rs2 });
}

/** FFMA Rd, Rs1, Rs2, Rs3 -- fused multiply-add: Rd = Rs1 * Rs2 + Rs3. */
export function ffma(
  rd: number,
  rs1: number,
  rs2: number,
  rs3: number,
): Instruction {
  return makeInstruction({ opcode: Opcode.FFMA, rd, rs1, rs2, rs3 });
}

/** FNEG Rd, Rs1 -- negate: Rd = -Rs1. */
export function fneg(rd: number, rs1: number): Instruction {
  return makeInstruction({ opcode: Opcode.FNEG, rd, rs1 });
}

/** FABS Rd, Rs1 -- absolute value: Rd = |Rs1|. */
export function fabsOp(rd: number, rs1: number): Instruction {
  return makeInstruction({ opcode: Opcode.FABS, rd, rs1 });
}

/** LOAD Rd, [Rs1+offset] -- load float from memory into register. */
export function load(
  rd: number,
  rs1: number,
  offset: number = 0,
): Instruction {
  return makeInstruction({ opcode: Opcode.LOAD, rd, rs1, immediate: offset });
}

/** STORE [Rs1+offset], Rs2 -- store register value to memory. */
export function store(
  rs1: number,
  rs2: number,
  offset: number = 0,
): Instruction {
  return makeInstruction({
    opcode: Opcode.STORE,
    rs1,
    rs2,
    immediate: offset,
  });
}

/** MOV Rd, Rs1 -- copy register: Rd = Rs1. */
export function mov(rd: number, rs1: number): Instruction {
  return makeInstruction({ opcode: Opcode.MOV, rd, rs1 });
}

/** LIMM Rd, value -- load immediate float: Rd = value. */
export function limm(rd: number, value: number): Instruction {
  return makeInstruction({ opcode: Opcode.LIMM, rd, immediate: value });
}

/** BEQ Rs1, Rs2, offset -- branch if equal. */
export function beq(rs1: number, rs2: number, offset: number): Instruction {
  return makeInstruction({ opcode: Opcode.BEQ, rs1, rs2, immediate: offset });
}

/** BLT Rs1, Rs2, offset -- branch if less than. */
export function blt(rs1: number, rs2: number, offset: number): Instruction {
  return makeInstruction({ opcode: Opcode.BLT, rs1, rs2, immediate: offset });
}

/** BNE Rs1, Rs2, offset -- branch if not equal. */
export function bne(rs1: number, rs2: number, offset: number): Instruction {
  return makeInstruction({ opcode: Opcode.BNE, rs1, rs2, immediate: offset });
}

/** JMP target -- unconditional jump to absolute address. */
export function jmp(target: number): Instruction {
  return makeInstruction({ opcode: Opcode.JMP, immediate: target });
}

/** NOP -- no operation, advance program counter. */
export function nop(): Instruction {
  return makeInstruction({ opcode: Opcode.NOP });
}

/** HALT -- stop execution. */
export function halt(): Instruction {
  return makeInstruction({ opcode: Opcode.HALT });
}
