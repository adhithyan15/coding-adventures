/**
 * @coding-adventures/compiler-ir — Intermediate Representation for the AOT Compiler Pipeline
 *
 * =============================================================================
 * What is IR?
 * =============================================================================
 *
 * An Intermediate Representation (IR) is a language-independent way to
 * describe computation. Think of it as an assembly language for an imaginary,
 * idealized computer — one with infinite registers and no platform quirks.
 *
 * The compiler pipeline works in stages:
 *
 *   Source code (Brainfuck)
 *       ↓  frontend (brainfuck-ir-compiler)
 *   IR instructions
 *       ↓  optimizer (compiler-ir-optimizer)
 *   Optimized IR instructions
 *       ↓  backend (codegen-riscv)
 *   Machine code (RISC-V .text section)
 *
 * This package defines the IR layer — the data structures that flow between
 * stages, plus utilities for printing and parsing IR text.
 *
 * =============================================================================
 * Package contents
 * =============================================================================
 *
 * Opcodes (opcodes.ts):
 *   - IrOp enum       — 25 opcodes covering constants, memory, arithmetic,
 *                       comparison, control flow, system calls, and meta ops
 *   - opToString()    — opcode → canonical text name (e.g., IrOp.ADD → "ADD")
 *   - parseOp()       — text name → opcode (e.g., "ADD" → IrOp.ADD)
 *
 * Types (types.ts):
 *   - IrRegister      — virtual register (v0, v1, ...) with toString
 *   - IrImmediate     — literal integer value with toString
 *   - IrLabel         — named jump target / data reference with toString
 *   - IrOperand       — discriminated union of the three operand types
 *   - IrInstruction   — { opcode, operands[], id } — one IR instruction
 *   - IrDataDecl      — data segment declaration (.data / .bss)
 *   - IrProgram       — complete program (instructions + data + metadata)
 *   - IDGenerator     — monotonic ID counter for instruction source mapping
 *
 * Printer (printer.ts):
 *   - printIr()       — IrProgram → canonical text format
 *
 * Parser (ir_parser.ts):
 *   - parseIr()       — canonical text → IrProgram (roundtrip inverse)
 */

// Opcodes
export { IrOp, opToString, parseOp } from "./opcodes.js";

// Types and constructors
export type { IrOperand, IrInstruction, IrDataDecl } from "./types.js";
export type { IrRegister, IrImmediate, IrLabel } from "./types.js";
export { IrProgram, IDGenerator, reg, imm, lbl, operandToString } from "./types.js";

// Printer
export { printIr } from "./printer.js";

// Parser
export { parseIr } from "./ir_parser.js";
