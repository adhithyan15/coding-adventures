/**
 * @coding-adventures/wasm-opcodes
 *
 * Complete WASM 1.0 opcode table with metadata (name, immediates, stack effects, category)
 *
 * This package is part of the coding-adventures monorepo, a ground-up
 * implementation of the computing stack from transistors to operating systems.
 *
 * Usage:
 *   import { getOpcode, getOpcodeByName, OPCODES, OPCODES_BY_NAME } from "@coding-adventures/wasm-opcodes";
 *
 *   getOpcode(0x6A)          // → { name: "i32.add", opcode: 106, category: "numeric_i32", ... }
 *   getOpcodeByName("i32.add") // → same OpcodeInfo
 */

export const VERSION = "0.1.0";

export type { OpcodeInfo } from "./wasm_opcodes.js";
export { OPCODES, OPCODES_BY_NAME, getOpcode, getOpcodeByName } from "./wasm_opcodes.js";
