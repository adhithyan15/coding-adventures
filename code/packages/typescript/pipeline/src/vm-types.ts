/**
 * VM Types — Core types for the bytecode compiler and virtual machine.
 * =====================================================================
 *
 * These types define the "assembly language" of our virtual machine. They are
 * the bridge between the compiler (which produces bytecode) and the VM (which
 * executes it). In a full codebase, these would live in separate
 * `bytecode-compiler` and `virtual-machine` packages. Here, we define them
 * locally so the pipeline can be self-contained.
 *
 * The design mirrors real VMs:
 *
 * - **OpCode** — The instruction set, like x86 opcodes or JVM bytecodes.
 * - **Instruction** — One "line" of bytecode: an opcode plus an optional operand.
 * - **CodeObject** — A compiled unit of code (like a Java `.class` file).
 * - **VMTrace** — A snapshot of one execution step (for debugging/visualization).
 *
 * Why define these here rather than importing from a separate package?
 * -------------------------------------------------------------------
 * The `bytecode-compiler` and `virtual-machine` TypeScript packages don't exist
 * yet. Rather than blocking the pipeline on those ports, we define the minimal
 * set of types and implementations needed to make the pipeline work end-to-end.
 * When those packages are ported, the pipeline can switch to importing from them.
 */

// ===========================================================================
// OpCode Enumeration
// ===========================================================================
//
// Each opcode is a single byte value (0x00-0xFF), grouped by category using
// the high nibble:
//
//     0x0_ = stack operations
//     0x1_ = variable operations
//     0x2_ = arithmetic
//     0xF_ = VM control
//
// This grouping is a common convention. The JVM does something similar — all
// its "load" instructions are in one numeric range, all "store" instructions
// in another. It makes debugging easier because you can tell the *category*
// of an instruction just by glancing at its hex value.

/**
 * The complete instruction set for our virtual machine.
 *
 * We use a plain object with `as const` rather than a TypeScript `enum` because:
 * 1. Const objects are more tree-shakeable (better for bundlers).
 * 2. The numeric values are directly accessible without enum overhead.
 * 3. It matches the "data, not behavior" philosophy of bytecode formats.
 */
export const OpCode = {
  // -- Stack Operations (0x0_) -----------------------------------------------
  // These move values onto or off of the operand stack.

  /** Push a constant from the constants pool onto the stack. */
  LOAD_CONST: 0x01,

  /** Discard the top value on the stack. */
  POP: 0x02,

  /** Duplicate the top value on the stack. */
  DUP: 0x03,

  // -- Variable Operations (0x1_) --------------------------------------------
  // These store values in and retrieve values from variable storage.

  /** Pop the top of stack and store it in a named variable. */
  STORE_NAME: 0x10,

  /** Push the value of a named variable onto the stack. */
  LOAD_NAME: 0x11,

  // -- Arithmetic Operations (0x2_) ------------------------------------------
  // These pop two operands, perform a math operation, and push the result.

  /** Pop two values, push their sum. */
  ADD: 0x20,

  /** Pop two values, push their difference (a - b). */
  SUB: 0x21,

  /** Pop two values, push their product. */
  MUL: 0x22,

  /** Pop two values, push their quotient (a / b). */
  DIV: 0x23,

  // -- I/O Operations (0x6_) -------------------------------------------------

  /** Pop the top of stack and add it to the output list. */
  PRINT: 0x60,

  // -- VM Control (0xF_) -----------------------------------------------------

  /** Stop execution. Every program must end with HALT. */
  HALT: 0xff,
} as const;

/** The type of any valid opcode value. */
export type OpCodeValue = (typeof OpCode)[keyof typeof OpCode];

/**
 * Reverse lookup: opcode numeric value -> opcode name string.
 *
 * This is used by the instruction-to-text converter so it can show
 * "LOAD_CONST" instead of "1" in human-readable bytecode listings.
 */
export const OpCodeName: Record<number, string> = Object.fromEntries(
  Object.entries(OpCode).map(([name, value]) => [value, name]),
);

// ===========================================================================
// Instruction
// ===========================================================================

/**
 * A single VM instruction: an opcode plus an optional operand.
 *
 * Think of this as one line of assembly language:
 *
 *     ADD                -> opcode=ADD, operand=undefined
 *     LOAD_CONST 0       -> opcode=LOAD_CONST, operand=0
 *     STORE_NAME 1       -> opcode=STORE_NAME, operand=1
 *
 * Some instructions (like ADD, POP, HALT) don't need an operand — they
 * operate purely on what's already on the stack. Others (like LOAD_CONST)
 * need an operand to know *which* constant to load.
 *
 * In a real bytecode format, this would be encoded as raw bytes:
 *     [opcode_byte] [operand_bytes...]
 *
 * We use a plain object for clarity, but the concept is identical.
 */
export interface Instruction {
  /** The operation to perform. */
  readonly opcode: OpCodeValue;

  /**
   * Optional data for the operation.
   *
   * - For LOAD_CONST: index into the constants pool.
   * - For STORE_NAME/LOAD_NAME: index into the names pool.
   * - For stack/arithmetic ops: undefined (not used).
   */
  readonly operand?: number;
}

// ===========================================================================
// CodeObject
// ===========================================================================

/**
 * A compiled unit of code — the bytecode equivalent of a source file.
 *
 * This is our version of Java's `.class` file or Python's `code` object.
 * It bundles together everything the VM needs to execute a piece of code:
 *
 * 1. **instructions** — The ordered list of operations to perform.
 * 2. **constants** — A pool of literal values (numbers, strings) referenced
 *    by LOAD_CONST instructions. Instead of embedding "42" directly in
 *    the instruction stream, we store it here and reference it by index.
 * 3. **names** — A pool of identifier strings (variable names) referenced
 *    by STORE_NAME/LOAD_NAME instructions.
 *
 * **Why pools?**
 * Real bytecode formats use constant pools extensively. The JVM's constant
 * pool stores strings, class names, method signatures, and numeric
 * literals. Our two pools (constants + names) are a simplified version
 * of the same idea.
 *
 * **Example:**
 * To represent `x = 42`:
 *     constants = [42]
 *     names = ["x"]
 *     instructions = [
 *         { opcode: LOAD_CONST, operand: 0 },   // Push constants[0] -> 42
 *         { opcode: STORE_NAME, operand: 0 },    // Pop into names[0] -> "x"
 *         { opcode: HALT },
 *     ]
 */
export interface CodeObject {
  /** The ordered list of bytecode instructions. */
  readonly instructions: readonly Instruction[];

  /** The constant pool — literal values referenced by LOAD_CONST. */
  readonly constants: readonly (number | string)[];

  /** The name pool — variable names referenced by STORE_NAME/LOAD_NAME. */
  readonly names: readonly string[];
}

// ===========================================================================
// VMTrace
// ===========================================================================

/**
 * A snapshot of one execution step — the VM's "black box recorder."
 *
 * Every time the VM executes an instruction, it produces a VMTrace
 * capturing the complete state before and after. This serves two purposes:
 *
 * 1. **Debugging** — You can replay the entire execution step by step,
 *    seeing exactly what happened to the stack, variables, and output.
 *
 * 2. **Visualization** — The pipeline visualizer (the HTML renderer) can
 *    animate the VM's execution, showing values flowing onto and off of
 *    the stack, variables changing, etc.
 *
 * Think of it like a flight recorder (black box) on an airplane — it
 * records everything so you can reconstruct what happened.
 */
export interface VMTrace {
  /** The program counter *before* this instruction executed. */
  readonly pc: number;

  /** The instruction that was executed in this step. */
  readonly instruction: Instruction;

  /** A snapshot of the stack before the instruction ran. */
  readonly stackBefore: readonly unknown[];

  /** A snapshot of the stack after the instruction ran. */
  readonly stackAfter: readonly unknown[];

  /** A snapshot of all named variables after the instruction ran. */
  readonly variables: Readonly<Record<string, unknown>>;

  /** If this instruction was PRINT, the string that was printed. */
  readonly output?: string;

  /** A human-readable explanation of what this step did. */
  readonly description: string;
}
