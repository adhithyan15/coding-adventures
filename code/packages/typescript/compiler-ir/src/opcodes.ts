/**
 * IrOp — the opcode enumeration for the general-purpose IR.
 *
 * =============================================================================
 * Design Philosophy
 * =============================================================================
 *
 * This IR is **general-purpose** — designed to serve as the compilation target
 * for any compiled language, not just Brainfuck. The current v1 instruction set
 * is sufficient for Brainfuck; BASIC (the next planned frontend) will add
 * opcodes for multiplication, division, floating-point arithmetic, and string
 * operations.
 *
 * Key rules:
 *   1. Existing opcodes never change semantics — only new ones are appended.
 *   2. A new opcode is added only when a frontend needs it AND it cannot be
 *      efficiently expressed as a sequence of existing opcodes.
 *   3. All frontends and backends remain forward-compatible.
 *
 * =============================================================================
 * Opcode Groups
 * =============================================================================
 *
 *   Constants:    LOAD_IMM, LOAD_ADDR
 *   Memory:       LOAD_BYTE, STORE_BYTE, LOAD_WORD, STORE_WORD
 *   Arithmetic:   ADD, ADD_IMM, SUB, AND, AND_IMM
 *   Comparison:   CMP_EQ, CMP_NE, CMP_LT, CMP_GT
 *   Control Flow: LABEL, JUMP, BRANCH_Z, BRANCH_NZ, CALL, RET
 *   System:       SYSCALL, HALT
 *   Meta:         NOP, COMMENT
 */

// We use a plain enum (const enum would prevent roundtrip string conversion).
export enum IrOp {
  // ── Constants ────────────────────────────────────────────────────────────
  // Load an immediate integer value into a register.
  //   LOAD_IMM  v0, 42    →  v0 = 42
  LOAD_IMM = 0,

  // Load the address of a data label into a register.
  //   LOAD_ADDR v0, tape  →  v0 = &tape
  LOAD_ADDR = 1,

  // ── Memory ────────────────────────────────────────────────────────────────
  // Load a byte from memory: dst = mem[base + offset] (zero-extended).
  //   LOAD_BYTE v2, v0, v1  →  v2 = mem[v0 + v1] & 0xFF
  LOAD_BYTE = 2,

  // Store a byte to memory: mem[base + offset] = src & 0xFF.
  //   STORE_BYTE v2, v0, v1  →  mem[v0 + v1] = v2 & 0xFF
  STORE_BYTE = 3,

  // Load a machine word from memory: dst = *(word*)(base + offset).
  //   LOAD_WORD v2, v0, v1  →  v2 = *(int*)(v0 + v1)
  LOAD_WORD = 4,

  // Store a machine word to memory: *(word*)(base + offset) = src.
  //   STORE_WORD v2, v0, v1  →  *(int*)(v0 + v1) = v2
  STORE_WORD = 5,

  // ── Arithmetic ────────────────────────────────────────────────────────────
  // Register-register addition: dst = lhs + rhs.
  //   ADD v3, v1, v2  →  v3 = v1 + v2
  ADD = 6,

  // Register-immediate addition: dst = src + immediate.
  //   ADD_IMM v1, v1, 1  →  v1 = v1 + 1
  ADD_IMM = 7,

  // Register-register subtraction: dst = lhs - rhs.
  //   SUB v3, v1, v2  →  v3 = v1 - v2
  SUB = 8,

  // Register-register bitwise AND: dst = lhs & rhs.
  //   AND v3, v1, v2  →  v3 = v1 & v2
  AND = 9,

  // Register-immediate bitwise AND: dst = src & immediate.
  //   AND_IMM v2, v2, 255  →  v2 = v2 & 0xFF
  AND_IMM = 10,

  // ── Comparison ────────────────────────────────────────────────────────────
  // Set dst = 1 if lhs == rhs, else 0.
  //   CMP_EQ v4, v1, v2  →  v4 = (v1 == v2) ? 1 : 0
  CMP_EQ = 11,

  // Set dst = 1 if lhs != rhs, else 0.
  //   CMP_NE v4, v1, v2  →  v4 = (v1 != v2) ? 1 : 0
  CMP_NE = 12,

  // Set dst = 1 if lhs < rhs (signed), else 0.
  //   CMP_LT v4, v1, v2  →  v4 = (v1 < v2) ? 1 : 0
  CMP_LT = 13,

  // Set dst = 1 if lhs > rhs (signed), else 0.
  //   CMP_GT v4, v1, v2  →  v4 = (v1 > v2) ? 1 : 0
  CMP_GT = 14,

  // ── Control Flow ──────────────────────────────────────────────────────────
  // Define a label at this point in the instruction stream.
  // Labels produce no machine code — they just record an address.
  //   LABEL loop_start
  LABEL = 15,

  // Unconditional jump to a label.
  //   JUMP loop_start  →  PC = &loop_start
  JUMP = 16,

  // Conditional branch: jump to label if register == 0.
  //   BRANCH_Z v2, loop_end  →  if v2 == 0 then PC = &loop_end
  BRANCH_Z = 17,

  // Conditional branch: jump to label if register != 0.
  //   BRANCH_NZ v2, loop_end  →  if v2 != 0 then PC = &loop_end
  BRANCH_NZ = 18,

  // Call a subroutine at the given label. Pushes return address.
  //   CALL my_func
  CALL = 19,

  // Return from a subroutine. Pops return address.
  //   RET
  RET = 20,

  // ── System ────────────────────────────────────────────────────────────────
  // Invoke a system call. The syscall number is an immediate operand.
  // Arguments and return values follow the platform's syscall ABI.
  //   SYSCALL 1  →  ecall with a7=1 (write)
  SYSCALL = 21,

  // Halt execution. The program terminates.
  //   HALT  →  ecall with a7=10 (exit)
  HALT = 22,

  // ── Meta ──────────────────────────────────────────────────────────────────
  // No operation. Produces a single NOP instruction in the backend.
  //   NOP
  NOP = 23,

  // A human-readable comment. Produces no machine code.
  // Useful for debugging IR output.
  //   COMMENT "load tape base address"
  COMMENT = 24,
}

// ──────────────────────────────────────────────────────────────────────────────
// String representation
//
// Maps each opcode to its canonical text name. These names are used by
// the IR printer and parser for roundtrip fidelity.
//
// Truth table for the 24 opcodes:
//
//   Opcode enum value  →  canonical text name
//   0                  →  "LOAD_IMM"
//   1                  →  "LOAD_ADDR"
//   ...
//   24                 →  "COMMENT"
// ──────────────────────────────────────────────────────────────────────────────

const OP_NAMES = new Map<IrOp, string>([
  [IrOp.LOAD_IMM, "LOAD_IMM"],
  [IrOp.LOAD_ADDR, "LOAD_ADDR"],
  [IrOp.LOAD_BYTE, "LOAD_BYTE"],
  [IrOp.STORE_BYTE, "STORE_BYTE"],
  [IrOp.LOAD_WORD, "LOAD_WORD"],
  [IrOp.STORE_WORD, "STORE_WORD"],
  [IrOp.ADD, "ADD"],
  [IrOp.ADD_IMM, "ADD_IMM"],
  [IrOp.SUB, "SUB"],
  [IrOp.AND, "AND"],
  [IrOp.AND_IMM, "AND_IMM"],
  [IrOp.CMP_EQ, "CMP_EQ"],
  [IrOp.CMP_NE, "CMP_NE"],
  [IrOp.CMP_LT, "CMP_LT"],
  [IrOp.CMP_GT, "CMP_GT"],
  [IrOp.LABEL, "LABEL"],
  [IrOp.JUMP, "JUMP"],
  [IrOp.BRANCH_Z, "BRANCH_Z"],
  [IrOp.BRANCH_NZ, "BRANCH_NZ"],
  [IrOp.CALL, "CALL"],
  [IrOp.RET, "RET"],
  [IrOp.SYSCALL, "SYSCALL"],
  [IrOp.HALT, "HALT"],
  [IrOp.NOP, "NOP"],
  [IrOp.COMMENT, "COMMENT"],
]);

// Reverse map: text name → IrOp value, built at module load time.
const NAME_TO_OP = new Map<string, IrOp>();
for (const [op, name] of OP_NAMES) {
  NAME_TO_OP.set(name, op);
}

/**
 * Returns the canonical text name for an IR opcode.
 * Returns "UNKNOWN" for any opcode not in the table.
 *
 * @example
 *   opToString(IrOp.ADD_IMM)  // "ADD_IMM"
 *   opToString(IrOp.HALT)     // "HALT"
 */
export function opToString(op: IrOp): string {
  return OP_NAMES.get(op) ?? "UNKNOWN";
}

/**
 * Converts a text opcode name to its IrOp value.
 *
 * This is the inverse of opToString. Returns undefined if the name is
 * not recognised, so callers can check for unknown opcodes without
 * catching exceptions.
 *
 * @example
 *   parseOp("ADD_IMM")   // IrOp.ADD_IMM
 *   parseOp("BRANCH_Z")  // IrOp.BRANCH_Z
 *   parseOp("BOGUS")     // undefined
 */
export function parseOp(name: string): IrOp | undefined {
  return NAME_TO_OP.get(name);
}
