/**
 * CLR IL Compiler — Targeting the Common Language Runtime.
 *
 * =================================================================
 * Chapter 4a.2: From Trees to CLR Intermediate Language
 * =================================================================
 *
 * The Common Language Runtime (CLR) is Microsoft's virtual machine, introduced
 * in 2002 as part of the .NET Framework. Like the JVM, it's a stack-based VM
 * that runs bytecode — but Microsoft calls it "Intermediate Language" (IL) or
 * sometimes "MSIL" (Microsoft Intermediate Language) or "CIL" (Common
 * Intermediate Language).
 *
 * The CLR was designed *after* the JVM, and its designers learned from both the
 * JVM's strengths and its limitations. Some notable differences:
 *
 * - **Richer short forms**: The CLR has dedicated opcodes for constants 0 through
 *   8 (the JVM only goes to 5). This reflects the observation that small numbers
 *   like 6, 7, 8 appear in real code more often than you'd expect.
 *
 * - **Signed byte encoding**: The CLR's ``ldc.i4.s`` uses a signed byte for
 *   values -128 to 127, just like the JVM's ``bipush``.
 *
 * - **Full 32-bit encoding**: For larger values, ``ldc.i4`` embeds a full 4-byte
 *   little-endian integer directly in the bytecode stream (5 bytes total). The
 *   JVM's ``ldc`` instead references a constant pool entry. The CLR approach is
 *   simpler but uses more space for large constants.
 *
 * This module compiles our AST into real CLR IL bytes — the same instruction
 * format that the C# compiler (``csc`` / ``dotnet build``) produces.
 *
 * Opcode reference
 * ----------------
 * We use the following real CLR IL opcodes:
 *
 * | Instruction  | Byte | Description                               |
 * |--------------|------|-------------------------------------------|
 * | ldloc.0      | 0x06 | Load local variable 0                     |
 * | ldloc.1      | 0x07 | Load local variable 1                     |
 * | ldloc.2      | 0x08 | Load local variable 2                     |
 * | ldloc.3      | 0x09 | Load local variable 3                     |
 * | stloc.0      | 0x0A | Store to local variable 0                 |
 * | stloc.1      | 0x0B | Store to local variable 1                 |
 * | stloc.2      | 0x0C | Store to local variable 2                 |
 * | stloc.3      | 0x0D | Store to local variable 3                 |
 * | ldloc.s      | 0x11 | Load local variable (short form, 2 bytes) |
 * | stloc.s      | 0x13 | Store to local variable (short form)      |
 * | ldc.i4.0     | 0x16 | Push int constant 0                       |
 * | ldc.i4.1     | 0x17 | Push int constant 1                       |
 * | ldc.i4.2     | 0x18 | Push int constant 2                       |
 * | ldc.i4.3     | 0x19 | Push int constant 3                       |
 * | ldc.i4.4     | 0x1A | Push int constant 4                       |
 * | ldc.i4.5     | 0x1B | Push int constant 5                       |
 * | ldc.i4.6     | 0x1C | Push int constant 6                       |
 * | ldc.i4.7     | 0x1D | Push int constant 7                       |
 * | ldc.i4.8     | 0x1E | Push int constant 8                       |
 * | ldc.i4.s     | 0x1F | Push signed byte (-128 to 127)            |
 * | ldc.i4       | 0x20 | Push 4-byte int32 (little-endian)         |
 * | pop          | 0x26 | Pop top value from stack                  |
 * | ret          | 0x2A | Return from method                        |
 * | add          | 0x58 | Integer addition                          |
 * | sub          | 0x59 | Integer subtraction                       |
 * | mul          | 0x5A | Integer multiplication                    |
 * | div          | 0x5B | Integer division                          |
 */

import type {
  Assignment,
  Expression,
  Program,
  Statement,
} from "@coding-adventures/parser";

// ---------------------------------------------------------------------------
// CLR IL Opcode constants — real values from the ECMA-335 specification
// ---------------------------------------------------------------------------

// Load local variable (short forms for slots 0-3)
export const LDLOC_0 = 0x06;
export const LDLOC_1 = 0x07;
export const LDLOC_2 = 0x08;
export const LDLOC_3 = 0x09;

// Store to local variable (short forms for slots 0-3)
export const STLOC_0 = 0x0a;
export const STLOC_1 = 0x0b;
export const STLOC_2 = 0x0c;
export const STLOC_3 = 0x0d;

// Generic load/store with index byte
export const LDLOC_S = 0x11;
export const STLOC_S = 0x13;

// Push integer constants 0-8 (single-byte instructions)
export const LDC_I4_0 = 0x16;
export const LDC_I4_1 = 0x17;
export const LDC_I4_2 = 0x18;
export const LDC_I4_3 = 0x19;
export const LDC_I4_4 = 0x1a;
export const LDC_I4_5 = 0x1b;
export const LDC_I4_6 = 0x1c;
export const LDC_I4_7 = 0x1d;
export const LDC_I4_8 = 0x1e;

// Push signed byte integer
export const LDC_I4_S = 0x1f;

// Push full 4-byte int32 (little-endian)
export const LDC_I4 = 0x20;

// Stack manipulation
export const POP = 0x26;

// Return from method
export const RET = 0x2a;

// Arithmetic
export const ADD = 0x58;
export const SUB = 0x59;
export const MUL = 0x5a;
export const DIV = 0x5b;

// ---------------------------------------------------------------------------
// Operator-to-opcode mapping
// ---------------------------------------------------------------------------

/**
 * Maps source-level operator symbols to their CLR IL bytecode equivalents.
 *
 * The CLR arithmetic instructions work identically to the JVM's: pop two values
 * from the evaluation stack, perform the operation, push the result. The opcodes
 * are different (0x58 vs 0x60 for add), but the semantics are the same.
 */
const CLR_OPERATOR_MAP: Record<string, number> = {
  "+": ADD,
  "-": SUB,
  "*": MUL,
  "/": DIV,
};

// ---------------------------------------------------------------------------
// CLRCodeObject — the compilation output
// ---------------------------------------------------------------------------

/**
 * The result of compiling an AST to CLR IL bytecode.
 *
 * Unlike the JVM's ``JVMCodeObject``, the CLR code object does not need a
 * separate constant pool for integers. The CLR embeds integer constants
 * directly in the bytecode stream (using ``ldc.i4`` with 4 inline bytes),
 * rather than referencing a pool. This is one of the key design differences
 * between the two VMs.
 */
export interface CLRCodeObject {
  /** The raw CLR IL bytecode bytes. */
  readonly bytecode: Uint8Array;
  /** The number of local variable slots used. */
  readonly numLocals: number;
  /** Maps slot indices to variable names. */
  readonly localNames: readonly string[];
}

// ---------------------------------------------------------------------------
// CLRCompiler — the AST-to-IL translator
// ---------------------------------------------------------------------------

/**
 * Compiles an AST into CLR IL bytecode bytes.
 *
 * The CLR compiler follows the same pattern as the JVM compiler: walk the
 * AST in post-order, emitting stack-machine instructions for each node.
 * The differences are in the encoding details:
 *
 * - **Wider short-form range**: Constants 0-8 have dedicated single-byte
 *   opcodes (vs. 0-5 on the JVM).
 * - **Inline integers**: Large constants are embedded directly as 4-byte
 *   little-endian values, not stored in a separate constant pool.
 * - **Different opcode values**: ``add`` is 0x58 (vs. JVM's 0x60), etc.
 *
 * Example:
 *
 *     import { Parser } from "@coding-adventures/parser";
 *     import { tokenize } from "@coding-adventures/lexer";
 *     import { CLRCompiler } from "@coding-adventures/bytecode-compiler";
 *
 *     const tokens = tokenize("x = 1 + 2");
 *     const ast = new Parser(tokens).parse();
 *
 *     const compiler = new CLRCompiler();
 *     const code = compiler.compile(ast);
 *     // code.bytecode == Uint8Array([0x17, 0x18, 0x58, 0x0A, 0x2A])
 *     //   ldc.i4.1, ldc.i4.2, add, stloc.0, ret
 */
export class CLRCompiler {
  /** The growing bytecode buffer. */
  private bytecode: number[] = [];

  /** Maps local variable slot indices to their names. */
  private _locals: string[] = [];

  // -------------------------------------------------------------------
  // Public API
  // -------------------------------------------------------------------

  /**
   * Compile a full program AST into CLR IL bytecode.
   *
   * Walks every statement, emits bytes, then appends ``ret`` (0x2A).
   */
  compile(program: Program): CLRCodeObject {
    for (const statement of program.statements) {
      this.compileStatement(statement);
    }

    // Every CLR method body must end with a ret instruction.
    this.bytecode.push(RET);

    return {
      bytecode: new Uint8Array(this.bytecode),
      numLocals: this._locals.length,
      localNames: [...this._locals],
    };
  }

  // -------------------------------------------------------------------
  // Statement compilation
  // -------------------------------------------------------------------

  /**
   * Compile a single statement into CLR IL.
   *
   * Assignment statements compile the value and store it. Expression
   * statements compile the expression and pop the result. The CLR's
   * pop instruction (0x26) discards the top of the evaluation stack.
   */
  private compileStatement(stmt: Statement): void {
    if (stmt.kind === "Assignment") {
      this.compileAssignment(stmt);
    } else {
      this.compileExpression(stmt);
      this.bytecode.push(POP);
    }
  }

  /**
   * Compile ``name = expression`` into CLR IL.
   */
  private compileAssignment(node: Assignment): void {
    this.compileExpression(node.value);
    const slot = this.getLocalSlot(node.target.name);
    this.emitStloc(slot);
  }

  // -------------------------------------------------------------------
  // Expression compilation
  // -------------------------------------------------------------------

  /**
   * Compile an expression, leaving exactly one value on the stack.
   *
   * @throws TypeError if the node type is not recognized.
   */
  compileExpression(node: Expression): void {
    switch (node.kind) {
      case "NumberLiteral":
        this.emitNumber(node.value);
        break;

      case "StringLiteral":
        // The CLR handles strings via the ldstr instruction in real IL,
        // but for our purposes we treat string values like large constants
        // and encode them with ldc.i4 (which would be incorrect for real
        // .NET, but consistent with our simplified model).
        // For now, we raise an error since our language primarily handles
        // integers and the CLR doesn't have a simple string constant pool
        // like the JVM.
        throw new TypeError(
          `CLR compiler does not support string literals yet. ` +
            `Got: "${node.value}"`,
        );

      case "Name": {
        const slot = this.getLocalSlot(node.name);
        this.emitLdloc(slot);
        break;
      }

      case "BinaryOp": {
        this.compileExpression(node.left);
        this.compileExpression(node.right);
        const opcode = CLR_OPERATOR_MAP[node.op];
        this.bytecode.push(opcode);
        break;
      }

      default: {
        const exhaustive: never = node;
        throw new TypeError(
          `Unknown expression type: ${(exhaustive as { kind: string }).kind}. ` +
            `The CLR compiler doesn't know how to handle this AST node.`,
        );
      }
    }
  }

  // -------------------------------------------------------------------
  // Number encoding — the CLR's three tiers
  // -------------------------------------------------------------------

  /**
   * Emit the most compact IL to push an integer onto the stack.
   *
   * The CLR has three encoding tiers, similar to the JVM but with a
   * wider short-form range:
   *
   * 1. **ldc.i4.N** (1 byte): For values 0 through 8. The CLR extends
   *    the short-form range by three compared to the JVM (which stops at
   *    5). This is because 6, 7, and 8 appear frequently enough in real
   *    code to justify dedicated opcodes.
   *
   * 2. **ldc.i4.s N** (2 bytes): For values -128 through 127. Like the
   *    JVM's ``bipush``, this uses a single signed byte after the opcode.
   *
   * 3. **ldc.i4 N** (5 bytes): For everything else. Unlike the JVM (which
   *    uses a constant pool), the CLR embeds the full 4-byte little-endian
   *    integer directly in the bytecode stream. This is simpler (no pool
   *    management) but uses more space for large constants that appear
   *    repeatedly.
   */
  private emitNumber(value: number): void {
    if (value >= 0 && value <= 8) {
      // Tier 1: Single-byte ldc.i4.N.
      // ldc.i4.0 is at 0x16, ldc.i4.1 at 0x17, ..., ldc.i4.8 at 0x1E.
      this.bytecode.push(LDC_I4_0 + value);
    } else if (value >= -128 && value <= 127) {
      // Tier 2: Two-byte ldc.i4.s.
      // The value is encoded as a signed byte after the opcode.
      this.bytecode.push(LDC_I4_S);
      this.bytecode.push(value & 0xff);
    } else {
      // Tier 3: Five-byte ldc.i4.
      // The opcode (0x20) is followed by the value as a 4-byte
      // little-endian signed int32. This is a key difference from the
      // JVM: no constant pool needed, but 5 bytes per large constant.
      this.bytecode.push(LDC_I4);
      this.emitInt32LE(value);
    }
  }

  // -------------------------------------------------------------------
  // Local variable encoding
  // -------------------------------------------------------------------

  /**
   * Emit a stloc instruction for the given local variable slot.
   *
   * 1. **stloc.N** (1 byte): For slots 0 through 3.
   * 2. **stloc.s N** (2 bytes): For slot 4 and above.
   */
  private emitStloc(slot: number): void {
    if (slot <= 3) {
      this.bytecode.push(STLOC_0 + slot);
    } else {
      this.bytecode.push(STLOC_S);
      this.bytecode.push(slot);
    }
  }

  /**
   * Emit a ldloc instruction for the given local variable slot.
   *
   * 1. **ldloc.N** (1 byte): For slots 0 through 3.
   * 2. **ldloc.s N** (2 bytes): For slot 4 and above.
   */
  private emitLdloc(slot: number): void {
    if (slot <= 3) {
      this.bytecode.push(LDLOC_0 + slot);
    } else {
      this.bytecode.push(LDLOC_S);
      this.bytecode.push(slot);
    }
  }

  // -------------------------------------------------------------------
  // Local slot management
  // -------------------------------------------------------------------

  /**
   * Get (or assign) a local variable slot for the given name.
   */
  private getLocalSlot(name: string): number {
    const existing = this._locals.indexOf(name);
    if (existing !== -1) {
      return existing;
    }
    this._locals.push(name);
    return this._locals.length - 1;
  }

  // -------------------------------------------------------------------
  // Byte encoding helpers
  // -------------------------------------------------------------------

  /**
   * Emit a 4-byte little-endian signed int32 into the bytecode stream.
   *
   * We use a DataView on an ArrayBuffer for correct endianness handling.
   * Little-endian is the native byte order of x86 processors and is what
   * the CLR specification requires for inline integer constants.
   */
  private emitInt32LE(value: number): void {
    const buf = new ArrayBuffer(4);
    const view = new DataView(buf);
    view.setInt32(0, value, true); // true = little-endian
    const bytes = new Uint8Array(buf);
    for (const b of bytes) {
      this.bytecode.push(b);
    }
  }
}
