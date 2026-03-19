/**
 * Assembler — Layer 5 of the computing stack.
 *
 * Translates human-readable ARM assembly language into binary machine code
 * using a two-pass assembly algorithm.
 *
 * The assembler bridges the gap between text that humans can write:
 *
 *     MOV R0, #1
 *     MOV R1, #2
 *     ADD R2, R0, R1
 *     HLT
 *
 * And the raw bytes that the CPU executes:
 *
 *     01 00 A0 E3  02 00 A0 E3  01 20 80 E0  FF FF FF FF
 *
 * This package sits between the ARM simulator (which defines the binary
 * encoding) and the compiler (which generates assembly source).
 *
 *   Logic Gates -> Arithmetic -> CPU -> ARM -> [Assembler] -> Lexer -> Parser -> Compiler -> VM
 */

// --- Core types ---
export type { AssemblyError, AssemblyResult, Operand, OperandType, ParsedLine } from "./types.js";

// --- Constants ---
export {
  CONDITION_CODES,
  OPCODES,
  REGISTER_ALIASES,
  FLAG_ONLY_INSTRUCTIONS,
  NO_RN_INSTRUCTIONS,
  BRANCH_INSTRUCTIONS,
  MEMORY_INSTRUCTIONS,
  HLT_INSTRUCTION,
} from "./types.js";

// --- Parser ---
export { parse } from "./parser.js";
export { parseRegister, parseNumber } from "./parser.js";

// --- Encoder (individual instruction encoding) ---
export {
  encodeDataProcessing,
  encodeBranch,
  encodeMemory,
  encodeMovImm,
  encodeAdd,
  encodeSub,
  encodeHlt,
  encodeImmediate,
  instructionsToBytes,
} from "./encoder.js";

// --- Assembler (two-pass assembly) ---
export { Assembler, assemble } from "./assembler.js";
