/**
 * IR Operand Types and Data Structures
 *
 * =============================================================================
 * IR Operand Types
 * =============================================================================
 *
 * Every IR instruction operates on operands. There are three kinds:
 *
 *   IrRegister  — a virtual register (v0, v1, v2, ...)
 *   IrImmediate — a literal integer value
 *   IrLabel     — a named jump target or data label
 *
 * TypeScript does not have a sealed interface mechanism like Go's unexported
 * interface methods, so we use a discriminated union with a "kind" tag instead.
 * The union type IrOperand ensures only the three known operand types exist.
 *
 * =============================================================================
 * Data Flow
 * =============================================================================
 *
 * IrProgram
 *   ├── Instructions: IrInstruction[]   ← the linear instruction stream
 *   ├── Data: IrDataDecl[]              ← .data/.bss segment declarations
 *   ├── EntryLabel: string              ← where execution begins
 *   └── Version: number                 ← IR version (1 = Brainfuck subset)
 */

import { IrOp } from "./opcodes.js";

// ──────────────────────────────────────────────────────────────────────────────
// IrRegister — a virtual register
//
// Virtual registers are named v0, v1, v2, ... (the index field).
// There are infinitely many — the backend's register allocator maps
// them to physical registers.
//
// Example:
//   { kind: "register", index: 0 }  →  "v0"
//   { kind: "register", index: 5 }  →  "v5"
// ──────────────────────────────────────────────────────────────────────────────

export interface IrRegister {
  readonly kind: "register";
  readonly index: number;
}

/**
 * Create a virtual register operand.
 *
 * @param index - The register index (0-based, e.g., 0 → v0).
 */
export function reg(index: number): IrRegister {
  return { kind: "register", index };
}

/**
 * Returns the string representation of an IrRegister.
 * v0, v1, v5, etc.
 */
export function regToString(r: IrRegister): string {
  return `v${r.index}`;
}

// ──────────────────────────────────────────────────────────────────────────────
// IrImmediate — a literal integer value
//
// Immediates are signed integers that appear directly in instructions.
//
// Example:
//   { kind: "immediate", value: 42 }   →  "42"
//   { kind: "immediate", value: -1 }   →  "-1"
//   { kind: "immediate", value: 255 }  →  "255"
// ──────────────────────────────────────────────────────────────────────────────

export interface IrImmediate {
  readonly kind: "immediate";
  readonly value: number;
}

/**
 * Create an immediate integer operand.
 *
 * @param value - The signed integer value.
 */
export function imm(value: number): IrImmediate {
  return { kind: "immediate", value };
}

/**
 * Returns the string representation of an IrImmediate.
 * Just the decimal number, e.g., "42" or "-1".
 */
export function immToString(i: IrImmediate): string {
  return String(i.value);
}

// ──────────────────────────────────────────────────────────────────────────────
// IrLabel — a named target for jumps, branches, calls, or data references
//
// Labels are strings like "loop_0_start", "_start", "tape", "__trap_oob".
// They resolve to addresses during code generation.
//
// Example:
//   { kind: "label", name: "_start" }       →  "_start"
//   { kind: "label", name: "loop_0_end" }   →  "loop_0_end"
// ──────────────────────────────────────────────────────────────────────────────

export interface IrLabel {
  readonly kind: "label";
  readonly name: string;
}

/**
 * Create a label operand.
 *
 * @param name - The label name (e.g., "_start", "loop_0_end").
 */
export function lbl(name: string): IrLabel {
  return { kind: "label", name };
}

/**
 * Returns the string representation of an IrLabel.
 * Just the label name, e.g., "_start".
 */
export function lblToString(l: IrLabel): string {
  return l.name;
}

// ──────────────────────────────────────────────────────────────────────────────
// IrOperand — discriminated union of all operand types
//
// TypeScript's discriminated union pattern allows exhaustive checking:
//
//   function printOperand(op: IrOperand): string {
//     switch (op.kind) {
//       case "register":  return regToString(op);
//       case "immediate": return immToString(op);
//       case "label":     return lblToString(op);
//     }
//   }
//
// The TypeScript compiler verifies that all three cases are handled.
// ──────────────────────────────────────────────────────────────────────────────

export type IrOperand = IrRegister | IrImmediate | IrLabel;

/**
 * Returns the canonical string representation of any IrOperand.
 * Dispatches on the "kind" discriminant.
 */
export function operandToString(op: IrOperand): string {
  switch (op.kind) {
    case "register":
      return regToString(op);
    case "immediate":
      return immToString(op);
    case "label":
      return lblToString(op);
  }
}

// ──────────────────────────────────────────────────────────────────────────────
// IrInstruction — a single IR instruction
//
// Every instruction has:
//   - opcode:   what operation to perform (ADD_IMM, BRANCH_Z, etc.)
//   - operands: the arguments (registers, immediates, labels)
//   - id:       a unique monotonic integer for source mapping
//
// The id field is the key that connects this instruction to the source
// map chain. Each instruction gets a unique ID assigned by the IDGenerator,
// and that ID flows through all pipeline stages.
//
// Examples:
//   { opcode: IrOp.ADD_IMM, operands: [v1, v1, 1], id: 3 }
//     →  ADD_IMM    v1, v1, 1  ; #3
//
//   { opcode: IrOp.BRANCH_Z, operands: [v2, loop_0_end], id: 7 }
//     →  BRANCH_Z   v2, loop_0_end  ; #7
// ──────────────────────────────────────────────────────────────────────────────

export interface IrInstruction {
  readonly opcode: IrOp;
  readonly operands: readonly IrOperand[];
  readonly id: number;
}

// ──────────────────────────────────────────────────────────────────────────────
// IrDataDecl — a data segment declaration
//
// Declares a named region of memory with a given size and initial byte
// value. For Brainfuck, this is the tape:
//
//   { label: "tape", size: 30000, init: 0 }
//     →  .data tape 30000 0
//
// The init value is repeated for every byte in the region. init=0 means
// zero-initialized (equivalent to .bss in most formats).
// ──────────────────────────────────────────────────────────────────────────────

export interface IrDataDecl {
  readonly label: string;
  readonly size: number;
  readonly init: number; // initial byte value (usually 0)
}

// ──────────────────────────────────────────────────────────────────────────────
// IrProgram — a complete IR program
//
// An IrProgram contains:
//   - instructions: the linear sequence of IR instructions
//   - data:         data segment declarations (.bss, .data)
//   - entryLabel:   the label where execution begins
//   - version:      IR version number (1 = Brainfuck subset)
//
// The instructions array is ordered — execution flows from index 0
// to length-1, with jumps/branches altering the flow.
// ──────────────────────────────────────────────────────────────────────────────

export class IrProgram {
  /**
   * The linear sequence of IR instructions. Execution begins at index 0
   * and flows forward unless a jump/branch redirects it.
   */
  public instructions: IrInstruction[] = [];

  /**
   * Data segment declarations. Each declares a named region of memory
   * with a fixed size and initial fill value.
   */
  public data: IrDataDecl[] = [];

  /**
   * The label where execution begins. The linker/loader uses this to set
   * the program counter before the first instruction runs.
   */
  public entryLabel: string;

  /**
   * IR version number. Version 1 is the Brainfuck subset. Future versions
   * will extend the opcode set for BASIC, Lua, etc.
   *
   * Invariant: existing opcodes never change semantics between versions.
   */
  public version: number;

  /**
   * Create a new IrProgram with the given entry label.
   * Starts with version 1 (the Brainfuck-sufficient subset).
   *
   * @param entryLabel - The label where execution begins (e.g., "_start").
   */
  constructor(entryLabel: string) {
    this.entryLabel = entryLabel;
    this.version = 1;
  }

  /**
   * Append an instruction to the end of the instruction stream.
   *
   * Instructions are stored in execution order. The first instruction
   * appended is the first to execute (unless a jump redirects flow).
   *
   * @param instr - The instruction to append.
   */
  addInstruction(instr: IrInstruction): void {
    this.instructions.push(instr);
  }

  /**
   * Append a data segment declaration.
   *
   * Data declarations are stored in the order they are added. The linker
   * places them in the .data or .bss section in that order.
   *
   * @param decl - The data declaration to append.
   */
  addData(decl: IrDataDecl): void {
    this.data.push(decl);
  }
}

// ──────────────────────────────────────────────────────────────────────────────
// IDGenerator — produces unique monotonic instruction IDs
//
// Every IR instruction in the pipeline needs a unique ID for source
// mapping. The IDGenerator ensures no two instructions ever share an ID,
// even across multiple compiler invocations within the same process.
//
// Usage:
//   const gen = new IDGenerator();
//   const id1 = gen.next();  // 0
//   const id2 = gen.next();  // 1
//   const id3 = gen.next();  // 2
//   gen.current();           // 3 (next value to be returned)
//
// Why monotonic?
// --------------
// Source maps work by matching instruction IDs across pipeline stages.
// If IDs could repeat or be recycled, a source-map entry for instruction
// #42 in stage A might accidentally match a *different* instruction #42
// in stage B. Monotonic IDs prevent this class of bug.
// ──────────────────────────────────────────────────────────────────────────────

export class IDGenerator {
  private nextId: number;

  /**
   * Create a new ID generator.
   *
   * @param start - The starting value (default 0). Use a non-zero start
   *   when multiple compilers contribute instructions to the same program
   *   and IDs must not collide.
   */
  constructor(start = 0) {
    this.nextId = start;
  }

  /**
   * Returns the next unique ID and increments the counter.
   * This is the primary method. Call it once per instruction emitted.
   *
   * @returns The next available unique ID.
   */
  next(): number {
    return this.nextId++;
  }

  /**
   * Returns the current counter value without incrementing.
   * This is the ID that will be returned by the next call to next().
   * Useful for recording the start ID before emitting a batch of
   * instructions, then computing the range [start, current()).
   *
   * @returns The counter value (= next ID that will be allocated).
   */
  current(): number {
    return this.nextId;
  }
}
