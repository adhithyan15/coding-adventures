/**
 * JVM Bytecode Compiler — Targeting the Java Virtual Machine.
 *
 * =================================================================
 * Chapter 4a.1: From Trees to JVM Bytecode
 * =================================================================
 *
 * The Java Virtual Machine (JVM) is one of the most successful virtual machines
 * ever built. Created by James Gosling at Sun Microsystems in 1995, it has become
 * the runtime for not just Java, but also Kotlin, Scala, Clojure, and many other
 * languages.
 *
 * This module compiles our AST into *real* JVM bytecode bytes — the same format
 * that ``javac`` produces when it compiles ``.java`` files into ``.class`` files.
 * While we don't produce complete ``.class`` files (which need headers, constant
 * pool tables, method descriptors, etc.), we do emit the actual instruction bytes
 * that the JVM would execute inside a method body.
 *
 * How JVM bytecode works
 * ----------------------
 * The JVM is a **stack machine**, just like our custom VM. But where our VM uses
 * high-level instructions like ``LOAD_CONST 0`` (opcode + index), the JVM uses
 * compact byte-level encodings designed to minimize class file size. This was a
 * deliberate design choice in 1995 when bandwidth was expensive — Java applets
 * needed to download quickly over dial-up connections.
 *
 * The JVM has several clever encoding tricks:
 *
 * 1. **Short-form instructions**: Instead of always using ``bipush N`` (2 bytes)
 *    for small numbers, the JVM has dedicated single-byte opcodes for the most
 *    common values: ``iconst_0`` through ``iconst_5``. Since most programs use
 *    small numbers frequently, this saves significant space.
 *
 * 2. **Local variable slots**: Similarly, instead of always using ``istore N``
 *    (2 bytes), there are single-byte forms ``istore_0`` through ``istore_3``
 *    for the first four local variables. Most methods have fewer than four locals.
 *
 * 3. **Constant pool**: For values too large for ``bipush`` (-128 to 127), the
 *    JVM stores them in a constant pool and uses ``ldc`` (load constant) to
 *    reference them by index.
 *
 * These optimizations are why JVM bytecode is remarkably compact — a design
 * principle that influenced later VMs like the CLR and Dalvik.
 *
 * Opcode reference
 * ----------------
 * We use the following real JVM opcodes (values from the JVM specification):
 *
 * | Instruction  | Byte | Description                               |
 * |--------------|------|-------------------------------------------|
 * | iconst_0     | 0x03 | Push int constant 0                       |
 * | iconst_1     | 0x04 | Push int constant 1                       |
 * | iconst_2     | 0x05 | Push int constant 2                       |
 * | iconst_3     | 0x06 | Push int constant 3                       |
 * | iconst_4     | 0x07 | Push int constant 4                       |
 * | iconst_5     | 0x08 | Push int constant 5                       |
 * | bipush       | 0x10 | Push byte-sized int (-128 to 127)         |
 * | ldc          | 0x12 | Load from constant pool by index          |
 * | iload        | 0x15 | Load int from local variable (2 bytes)    |
 * | iload_0      | 0x1A | Load int from local variable 0            |
 * | iload_1      | 0x1B | Load int from local variable 1            |
 * | iload_2      | 0x1C | Load int from local variable 2            |
 * | iload_3      | 0x1D | Load int from local variable 3            |
 * | istore       | 0x36 | Store int to local variable (2 bytes)     |
 * | istore_0     | 0x3B | Store int to local variable 0             |
 * | istore_1     | 0x3C | Store int to local variable 1             |
 * | istore_2     | 0x3D | Store int to local variable 2             |
 * | istore_3     | 0x3E | Store int to local variable 3             |
 * | pop          | 0x57 | Pop top value from stack                  |
 * | iadd         | 0x60 | Integer addition                          |
 * | isub         | 0x64 | Integer subtraction                       |
 * | imul         | 0x68 | Integer multiplication                    |
 * | idiv         | 0x6C | Integer division                          |
 * | return       | 0xB1 | Return void from method                   |
 */

import type {
  Assignment,
  Expression,
  Program,
  Statement,
} from "@coding-adventures/parser";

// ---------------------------------------------------------------------------
// JVM Opcode constants — real values from the JVM specification
// ---------------------------------------------------------------------------

// Push integer constants 0-5 (single-byte instructions)
export const ICONST_0 = 0x03;
export const ICONST_1 = 0x04;
export const ICONST_2 = 0x05;
export const ICONST_3 = 0x06;
export const ICONST_4 = 0x07;
export const ICONST_5 = 0x08;

// Push byte-sized integer (-128 to 127)
export const BIPUSH = 0x10;

// Load from constant pool
export const LDC = 0x12;

// Load integer from local variable
export const ILOAD = 0x15; // Generic form: iload + index byte
export const ILOAD_0 = 0x1a;
export const ILOAD_1 = 0x1b;
export const ILOAD_2 = 0x1c;
export const ILOAD_3 = 0x1d;

// Store integer to local variable
export const ISTORE = 0x36; // Generic form: istore + index byte
export const ISTORE_0 = 0x3b;
export const ISTORE_1 = 0x3c;
export const ISTORE_2 = 0x3d;
export const ISTORE_3 = 0x3e;

// Stack manipulation
export const POP = 0x57;

// Arithmetic
export const IADD = 0x60;
export const ISUB = 0x64;
export const IMUL = 0x68;
export const IDIV = 0x6c;

// Return void
export const RETURN = 0xb1;

// ---------------------------------------------------------------------------
// Operator-to-opcode mapping
// ---------------------------------------------------------------------------

/**
 * Maps source-level operator symbols to their JVM bytecode equivalents.
 *
 * Just like the original compiler's OPERATOR_MAP, this table separates the data
 * (which opcode corresponds to which operator) from the logic (how to compile a
 * binary operation). The JVM arithmetic opcodes operate on the top two values of
 * the operand stack, popping both and pushing the result — exactly the same
 * semantics as our custom VM.
 */
const JVM_OPERATOR_MAP: Record<string, number> = {
  "+": IADD,
  "-": ISUB,
  "*": IMUL,
  "/": IDIV,
};

// ---------------------------------------------------------------------------
// JVMCodeObject — the compilation output
// ---------------------------------------------------------------------------

/**
 * The result of compiling an AST to JVM bytecode.
 *
 * This is analogous to the ``CodeObject`` from our custom VM compiler, but
 * instead of high-level ``Instruction`` objects, it contains raw bytes — the
 * actual byte sequence that a JVM would execute.
 *
 * In a real ``.class`` file, the bytecode would be inside the ``Code``
 * attribute of a method, the constants would be in the class-level constant
 * pool with type tags and UTF-8 encoding, and num_locals would be the
 * ``max_locals`` field. We simplify all of this to flat arrays.
 */
export interface JVMCodeObject {
  /** The raw JVM bytecode bytes. */
  readonly bytecode: Uint8Array;
  /** The constant pool — values referenced by ``ldc`` instructions. */
  readonly constants: readonly (number | string)[];
  /** The number of local variable slots used. */
  readonly numLocals: number;
  /** Maps slot indices to variable names. */
  readonly localNames: readonly string[];
}

// ---------------------------------------------------------------------------
// JVMCompiler — the AST-to-bytecode translator
// ---------------------------------------------------------------------------

/**
 * Compiles an AST into JVM bytecode bytes.
 *
 * This compiler walks the same AST that our custom ``BytecodeCompiler`` uses,
 * but instead of emitting ``Instruction`` objects, it emits raw bytes using
 * real JVM opcode values. The result is a ``JVMCodeObject`` containing the
 * bytecode bytes, a constant pool, and local variable metadata.
 *
 * The compiler uses the JVM's compact encoding scheme:
 *
 * - Small integers (0-5) use dedicated single-byte ``iconst_N`` instructions
 * - Medium integers (-128 to 127) use two-byte ``bipush N`` instructions
 * - Larger integers use ``ldc`` with a constant pool reference
 * - The first four local variables use single-byte ``istore_N``/``iload_N``
 * - Additional locals use two-byte ``istore N``/``iload N``
 *
 * This tiered encoding is a hallmark of JVM design — optimize for the common
 * case (small numbers and few variables) while still supporting the general
 * case.
 *
 * Example:
 *
 *     import { Parser } from "@coding-adventures/parser";
 *     import { tokenize } from "@coding-adventures/lexer";
 *     import { JVMCompiler } from "@coding-adventures/bytecode-compiler";
 *
 *     const tokens = tokenize("x = 1 + 2");
 *     const ast = new Parser(tokens).parse();
 *
 *     const compiler = new JVMCompiler();
 *     const code = compiler.compile(ast);
 *     // code.bytecode == Uint8Array([0x04, 0x05, 0x60, 0x3B, 0xB1])
 *     //   iconst_1, iconst_2, iadd, istore_0, return
 */
export class JVMCompiler {
  /** The growing bytecode buffer. We use a regular array for efficient
   *  appending, then convert to Uint8Array at the end. */
  private bytecode: number[] = [];

  /** The constant pool for values too large for inline encoding. */
  private _constants: (number | string)[] = [];

  /** Maps local variable slot indices to their names. The first variable
   *  assigned gets slot 0, the second gets slot 1, etc. */
  private _locals: string[] = [];

  // -------------------------------------------------------------------
  // Public API
  // -------------------------------------------------------------------

  /**
   * Compile a full program AST into JVM bytecode.
   *
   * Walks every statement in the program, emitting bytes for each one,
   * then appends a ``return`` instruction (0xB1) to end the method.
   *
   * @param program - The root AST node, as produced by ``Parser.parse()``.
   * @returns Contains the raw bytecode, constant pool, and local variable info.
   */
  compile(program: Program): JVMCodeObject {
    for (const statement of program.statements) {
      this.compileStatement(statement);
    }

    // Every JVM method must end with a return instruction.
    // We use 'return' (0xB1) which returns void — appropriate since our
    // programs don't have an explicit return value.
    this.bytecode.push(RETURN);

    return {
      bytecode: new Uint8Array(this.bytecode),
      constants: this._constants,
      numLocals: this._locals.length,
      localNames: [...this._locals],
    };
  }

  // -------------------------------------------------------------------
  // Statement compilation
  // -------------------------------------------------------------------

  /**
   * Compile a single statement into JVM bytecode.
   *
   * Assignment statements compile the value expression and then store the
   * result into a local variable slot. Expression statements compile the
   * expression and then pop the result off the stack (since nobody captures
   * it). The JVM requires the operand stack to be balanced — you can't leave
   * stray values on it.
   */
  private compileStatement(stmt: Statement): void {
    if (stmt.kind === "Assignment") {
      this.compileAssignment(stmt);
    } else {
      // Expression statement: evaluate for side effects, then discard.
      // The JVM's pop instruction (0x57) removes the top value from the
      // operand stack, keeping things tidy.
      this.compileExpression(stmt);
      this.bytecode.push(POP);
    }
  }

  /**
   * Compile ``name = expression`` into JVM bytecode.
   *
   * First compiles the right-hand side (pushes value onto stack), then
   * emits an ``istore`` instruction to pop and store into a local slot.
   *
   * The local slot is determined by the order in which variables are first
   * seen: the first variable gets slot 0, the second gets slot 1, etc.
   * This mirrors how ``javac`` assigns local variable slots.
   */
  private compileAssignment(node: Assignment): void {
    // Compile the value expression — puts result on the operand stack.
    this.compileExpression(node.value);

    // Determine which local slot this variable maps to.
    const slot = this.getLocalSlot(node.target.name);

    // Emit the appropriate istore instruction.
    this.emitIstore(slot);
  }

  // -------------------------------------------------------------------
  // Expression compilation — the recursive heart
  // -------------------------------------------------------------------

  /**
   * Compile an expression, leaving exactly one value on the stack.
   *
   * This is the recursive core of the compiler. Each expression type has
   * its own compilation strategy, but they all share the same contract:
   * after compilation, exactly one new value sits on top of the operand
   * stack.
   *
   * @throws TypeError if the node type is not recognized.
   */
  compileExpression(node: Expression): void {
    switch (node.kind) {
      case "NumberLiteral":
        this.emitNumber(node.value);
        break;

      case "StringLiteral": {
        // Strings always go through the constant pool.
        // The JVM's ldc instruction can load both integers and strings
        // from the constant pool.
        const constIndex = this.addConstant(node.value);
        this.bytecode.push(LDC);
        this.bytecode.push(constIndex);
        break;
      }

      case "Name": {
        // Load a local variable onto the stack.
        const slot = this.getLocalSlot(node.name);
        this.emitIload(slot);
        break;
      }

      case "BinaryOp": {
        // Post-order traversal: compile left, compile right, emit operator.
        // The JVM arithmetic instructions (iadd, isub, etc.) pop two values
        // and push the result, just like our custom VM.
        this.compileExpression(node.left);
        this.compileExpression(node.right);
        const opcode = JVM_OPERATOR_MAP[node.op];
        this.bytecode.push(opcode);
        break;
      }

      default: {
        const exhaustive: never = node;
        throw new TypeError(
          `Unknown expression type: ${(exhaustive as { kind: string }).kind}. ` +
            `The JVM compiler doesn't know how to handle this AST node.`,
        );
      }
    }
  }

  // -------------------------------------------------------------------
  // Number encoding — the JVM's tiered approach
  // -------------------------------------------------------------------

  /**
   * Emit the most compact bytecode to push an integer onto the stack.
   *
   * The JVM has three ways to push an integer, each suited to a different
   * range:
   *
   * 1. **iconst_N** (1 byte): For values 0 through 5. These are the most
   *    common integer values in programs, so they get dedicated single-byte
   *    opcodes. ``iconst_0`` is 0x03, ``iconst_5`` is 0x08.
   *
   * 2. **bipush N** (2 bytes): For values -128 through 127. The ``bipush``
   *    opcode (0x10) is followed by a single signed byte. This covers most
   *    loop counters, array indices, and small constants.
   *
   * 3. **ldc index** (2 bytes): For anything else. The value is stored in
   *    the constant pool, and ``ldc`` (0x12) loads it by index. This
   *    handles large numbers at the cost of an extra constant pool entry.
   */
  private emitNumber(value: number): void {
    if (value >= 0 && value <= 5) {
      // Tier 1: Single-byte iconst_N.
      // iconst_0 is at 0x03, iconst_1 at 0x04, ..., iconst_5 at 0x08.
      this.bytecode.push(ICONST_0 + value);
    } else if (value >= -128 && value <= 127) {
      // Tier 2: Two-byte bipush.
      // The value is encoded as a signed byte after the opcode.
      this.bytecode.push(BIPUSH);
      this.bytecode.push(value & 0xff);
    } else {
      // Tier 3: Constant pool reference via ldc.
      // Store the value in the constant pool and emit ldc + index.
      const constIndex = this.addConstant(value);
      this.bytecode.push(LDC);
      this.bytecode.push(constIndex);
    }
  }

  // -------------------------------------------------------------------
  // Local variable encoding — another tiered approach
  // -------------------------------------------------------------------

  /**
   * Emit an istore instruction for the given local variable slot.
   *
   * Like number encoding, local variable access has two tiers:
   *
   * 1. **istore_N** (1 byte): For slots 0 through 3. These cover the first
   *    four local variables, which is enough for most simple methods.
   *
   * 2. **istore N** (2 bytes): For slot 4 and above. The generic form uses
   *    the ``istore`` opcode (0x36) followed by the slot index as a byte.
   */
  private emitIstore(slot: number): void {
    if (slot <= 3) {
      // Short form: istore_0 (0x3B) through istore_3 (0x3E)
      this.bytecode.push(ISTORE_0 + slot);
    } else {
      // Long form: istore + slot index byte
      this.bytecode.push(ISTORE);
      this.bytecode.push(slot);
    }
  }

  /**
   * Emit an iload instruction for the given local variable slot.
   *
   * Same tiered approach as istore:
   *
   * 1. **iload_N** (1 byte): For slots 0 through 3.
   * 2. **iload N** (2 bytes): For slot 4 and above.
   */
  private emitIload(slot: number): void {
    if (slot <= 3) {
      // Short form: iload_0 (0x1A) through iload_3 (0x1D)
      this.bytecode.push(ILOAD_0 + slot);
    } else {
      // Long form: iload + slot index byte
      this.bytecode.push(ILOAD);
      this.bytecode.push(slot);
    }
  }

  // -------------------------------------------------------------------
  // Pool management
  // -------------------------------------------------------------------

  /**
   * Add a value to the constant pool, returning its index. Deduplicates.
   *
   * The constant pool stores values that are too large or complex to encode
   * inline in the bytecode. Each unique value is stored once, and subsequent
   * references reuse the same index.
   *
   * In a real JVM ``.class`` file, the constant pool is much more elaborate,
   * with type tags (CONSTANT_Integer, CONSTANT_String, etc.) and cross-
   * references between entries. Our simplified version is a flat list.
   */
  private addConstant(value: number | string): number {
    const existing = this._constants.indexOf(value);
    if (existing !== -1) {
      return existing;
    }
    this._constants.push(value);
    return this._constants.length - 1;
  }

  /**
   * Get (or assign) a local variable slot for the given name.
   *
   * Local variables in the JVM are stored in a numbered array of slots.
   * Each variable gets a unique slot index, assigned in the order they
   * are first encountered. This is the same strategy ``javac`` uses for
   * local variables in a method.
   */
  private getLocalSlot(name: string): number {
    const existing = this._locals.indexOf(name);
    if (existing !== -1) {
      return existing;
    }
    this._locals.push(name);
    return this._locals.length - 1;
  }
}
