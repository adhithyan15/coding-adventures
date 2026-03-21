/**
 * Virtual Machine Types — Data structures for bytecode compilation targets.
 *
 * =================================================================
 * Why These Types Live Here
 * =================================================================
 *
 * In the Python version, the bytecode compiler imports ``CodeObject``,
 * ``Instruction``, and ``OpCode`` from the ``virtual_machine`` package.
 * Since the TypeScript virtual-machine package doesn't exist yet, we
 * define these types here — they're the "contract" between the compiler
 * (which produces bytecode) and the VM (which consumes it).
 *
 * When a TypeScript virtual-machine package is eventually created, these
 * types should be moved there and re-exported from this package for
 * backward compatibility.
 *
 * =================================================================
 * The OpCode Enum — Our VM's Instruction Set
 * =================================================================
 *
 * Each opcode is a single byte value (0x00-0xFF), giving us room for up
 * to 256 different instructions. We group them by category using the high
 * nibble:
 *
 *     0x0_ = stack operations
 *     0x1_ = variable operations
 *     0x2_ = arithmetic
 *     0x3_ = comparison
 *     0x4_ = control flow
 *     0x5_ = function operations
 *     0x6_ = I/O
 *     0xF_ = VM control
 *
 * This grouping is a common convention. The JVM does something similar —
 * all its "load" instructions are in one numeric range, all "store"
 * instructions in another. It makes debugging easier because you can tell
 * the *category* of an instruction just by glancing at its hex value.
 */

// =========================================================================
// OpCode — The instruction set
// =========================================================================

/**
 * The complete instruction set for our virtual machine.
 *
 * We use a plain object with ``as const`` rather than a TypeScript enum
 * because const objects produce cleaner JavaScript output and are easier
 * to iterate over. The values match the Python ``virtual_machine.OpCode``
 * enum exactly.
 */
export const OpCode = {
  // -- Stack Operations (0x0_) ------------------------------------------
  // These move values onto or off of the operand stack.

  /** Push a constant from the constants pool onto the stack.
   *  Operand: index into the CodeObject's ``constants`` list. */
  LOAD_CONST: 0x01,

  /** Discard the top value on the stack. No operand. */
  POP: 0x02,

  /** Duplicate the top value on the stack. No operand. */
  DUP: 0x03,

  // -- Variable Operations (0x1_) ----------------------------------------

  /** Pop the top of stack and store it in a named variable.
   *  Operand: index into the CodeObject's ``names`` list. */
  STORE_NAME: 0x10,

  /** Push the value of a named variable onto the stack.
   *  Operand: index into the CodeObject's ``names`` list. */
  LOAD_NAME: 0x11,

  /** Pop the top of stack and store it in a local variable slot.
   *  Operand: integer index of the local slot. */
  STORE_LOCAL: 0x12,

  /** Push the value from a local variable slot onto the stack.
   *  Operand: integer index of the local slot. */
  LOAD_LOCAL: 0x13,

  // -- Arithmetic Operations (0x2_) --------------------------------------
  // Pop two operands, perform a math operation, and push the result.

  /** Pop two values, push their sum. */
  ADD: 0x20,

  /** Pop two values, push their difference (a - b). */
  SUB: 0x21,

  /** Pop two values, push their product. */
  MUL: 0x22,

  /** Pop two values, push their quotient (a / b). Integer division. */
  DIV: 0x23,

  // -- Comparison Operations (0x3_) --------------------------------------

  /** Pop two values, push 1 if equal, 0 otherwise. */
  CMP_EQ: 0x30,

  /** Pop two values, push 1 if a < b, 0 otherwise. */
  CMP_LT: 0x31,

  /** Pop two values, push 1 if a > b, 0 otherwise. */
  CMP_GT: 0x32,

  // -- Control Flow (0x4_) -----------------------------------------------

  /** Unconditional jump: set PC to the operand value. */
  JUMP: 0x40,

  /** Conditional jump: pop top of stack, jump if falsy (0). */
  JUMP_IF_FALSE: 0x41,

  /** Conditional jump: pop top of stack, jump if truthy (non-zero). */
  JUMP_IF_TRUE: 0x42,

  // -- Function Operations (0x5_) ----------------------------------------

  /** Call a function. Operand: name index. */
  CALL: 0x50,

  /** Return from a function. */
  RETURN: 0x51,

  // -- I/O Operations (0x6_) ---------------------------------------------

  /** Pop the top of stack and print it. */
  PRINT: 0x60,

  // -- VM Control (0xF_) -------------------------------------------------

  /** Stop execution immediately. Every program should end with HALT. */
  HALT: 0xff,
} as const;

/**
 * The type of an individual opcode value.
 *
 * TypeScript's ``typeof OpCode[keyof typeof OpCode]`` extracts the union
 * of all numeric values in the OpCode object. This lets us type-check that
 * instructions only contain valid opcodes.
 */
export type OpCodeValue = (typeof OpCode)[keyof typeof OpCode];

// =========================================================================
// Instruction — A single VM operation
// =========================================================================

/**
 * A single VM instruction: an opcode plus an optional operand.
 *
 * Think of this as one line of assembly language:
 *
 *     ADD                -> opcode = ADD, operand = undefined
 *     LOAD_CONST 0       -> opcode = LOAD_CONST, operand = 0
 *     STORE_NAME 1       -> opcode = STORE_NAME, operand = 1
 *
 * Some instructions (like ADD, POP, HALT) don't need an operand — they
 * operate purely on what's already on the stack. Others (like LOAD_CONST,
 * JUMP) need an operand to know *which* constant to load or *where* to jump.
 *
 * In a real bytecode format, this would be encoded as raw bytes:
 *     [opcode_byte] [operand_bytes...]
 *
 * We use a TypeScript interface for clarity, but the concept is identical.
 */
export interface Instruction {
  /** The operation to perform. */
  readonly opcode: OpCodeValue;

  /** Optional data for the operation (constant pool index, name index, etc.). */
  readonly operand?: number | string | null;
}

// =========================================================================
// CodeObject — A compiled unit of code
// =========================================================================

/**
 * A compiled unit of code — the bytecode equivalent of a source file.
 *
 * This is our version of Java's ``.class`` file or Python's ``code`` object.
 * It bundles together everything the VM needs to execute a piece of code:
 *
 * 1. **instructions** — The ordered list of operations to perform.
 * 2. **constants** — A pool of literal values (numbers, strings) referenced
 *    by LOAD_CONST instructions. Instead of embedding "42" directly in
 *    the instruction stream, we store it here and reference it by index.
 * 3. **names** — A pool of identifier strings (variable names) referenced
 *    by STORE_NAME/LOAD_NAME instructions.
 *
 * Why pools?
 * ----------
 * Real bytecode formats use constant pools extensively. The JVM's constant
 * pool stores strings, class names, method signatures, and numeric
 * literals. Our two pools (constants + names) are a simplified version
 * of the same idea.
 *
 * Example:
 * --------
 * To represent ``x = 42``:
 *
 *     constants = [42]
 *     names = ["x"]
 *     instructions = [
 *       { opcode: LOAD_CONST, operand: 0 },   // push constants[0] = 42
 *       { opcode: STORE_NAME, operand: 0 },    // pop and store in names[0] = "x"
 *       { opcode: HALT },                       // stop
 *     ]
 */
export interface CodeObject {
  /** The ordered list of bytecode instructions. */
  readonly instructions: readonly Instruction[];

  /** The constant pool — literal values referenced by LOAD_CONST. */
  readonly constants: readonly (number | string)[];

  /** The name pool — variable names referenced by STORE_NAME / LOAD_NAME. */
  readonly names: readonly string[];
}

// =========================================================================
// VirtualMachine — A minimal interpreter for end-to-end tests
// =========================================================================

/**
 * A minimal stack-based bytecode interpreter.
 *
 * This is a simplified version of the full VirtualMachine from the Python
 * ``virtual_machine`` package. It supports only the opcodes needed to run
 * the bytecode compiler's end-to-end tests: LOAD_CONST, POP, STORE_NAME,
 * LOAD_NAME, ADD, SUB, MUL, DIV, and HALT.
 *
 * When a full TypeScript virtual-machine package is created, this class
 * should be replaced by an import from that package.
 *
 * The Fetch-Decode-Execute Cycle
 * ------------------------------
 * Like every processor (real or virtual), this VM runs in a loop:
 *
 *   1. **Fetch** — Read the instruction at ``pc``.
 *   2. **Decode** — Look at the opcode to determine what to do.
 *   3. **Execute** — Perform the operation (push, pop, add, etc.).
 *   4. **Advance** — Move ``pc`` to the next instruction.
 *   5. **Repeat** — Go back to step 1.
 */
export class VirtualMachine {
  /** Named variable storage (like global scope). */
  variables: Record<string, number | string> = {};

  /** The operand stack where all computation happens. */
  private stack: (number | string)[] = [];

  /**
   * Execute a CodeObject, running its instructions to completion.
   *
   * After execution, the ``variables`` property contains all variables
   * that were assigned during the program's run.
   */
  execute(code: CodeObject): void {
    let pc = 0;

    while (pc < code.instructions.length) {
      const instr = code.instructions[pc];

      switch (instr.opcode) {
        case OpCode.LOAD_CONST: {
          // Push a constant from the pool onto the stack.
          const value = code.constants[instr.operand as number];
          this.stack.push(value);
          break;
        }

        case OpCode.POP: {
          // Discard the top value.
          this.stack.pop();
          break;
        }

        case OpCode.STORE_NAME: {
          // Pop the top value and bind it to a named variable.
          const name = code.names[instr.operand as number];
          const value = this.stack.pop()!;
          this.variables[name] = value;
          break;
        }

        case OpCode.LOAD_NAME: {
          // Look up a named variable and push its value.
          const name = code.names[instr.operand as number];
          const value = this.variables[name];
          if (value === undefined) {
            throw new Error(`Undefined variable: ${name}`);
          }
          this.stack.push(value);
          break;
        }

        case OpCode.ADD: {
          // Pop two values, push their sum.
          const b = this.stack.pop() as number;
          const a = this.stack.pop() as number;
          this.stack.push(a + b);
          break;
        }

        case OpCode.SUB: {
          // Pop two values, push their difference.
          const b = this.stack.pop() as number;
          const a = this.stack.pop() as number;
          this.stack.push(a - b);
          break;
        }

        case OpCode.MUL: {
          // Pop two values, push their product.
          const b = this.stack.pop() as number;
          const a = this.stack.pop() as number;
          this.stack.push(a * b);
          break;
        }

        case OpCode.DIV: {
          // Pop two values, push their quotient (integer division).
          const b = this.stack.pop() as number;
          const a = this.stack.pop() as number;
          this.stack.push(Math.trunc(a / b));
          break;
        }

        case OpCode.HALT: {
          // Stop execution.
          return;
        }

        default:
          throw new Error(
            `Unknown opcode: 0x${instr.opcode.toString(16).padStart(2, "0")}`,
          );
      }

      pc++;
    }
  }
}
