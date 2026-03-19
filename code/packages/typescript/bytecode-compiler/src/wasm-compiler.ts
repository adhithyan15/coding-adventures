/**
 * WASM Bytecode Compiler — Targeting WebAssembly.
 *
 * =================================================================
 * Chapter 4a.3: From Trees to WebAssembly Bytecode
 * =================================================================
 *
 * WebAssembly (WASM) is the newest of our three target VMs, standardized in 2017
 * by the W3C. Unlike the JVM (1995) and CLR (2002), which were designed for
 * general-purpose application development, WASM was designed specifically for the
 * web — a compact, fast, safe bytecode format that runs in browsers alongside
 * JavaScript.
 *
 * WASM's design philosophy differs from the JVM and CLR in several ways:
 *
 * 1. **Simplicity over compactness**: WASM uses a uniform encoding for most
 *    values. Where the JVM has ``iconst_0`` through ``iconst_5`` (saving one
 *    byte for common values), WASM always uses ``i32.const`` followed by a
 *    full 4-byte value. This makes the bytecode slightly larger but much
 *    simpler to encode and decode.
 *
 * 2. **Structured control flow**: WASM doesn't have ``goto`` — instead it uses
 *    structured blocks, loops, and if/else constructs. This makes it easier to
 *    validate and optimize, and prevents entire classes of security exploits.
 *    (Our simple language doesn't need control flow yet, but this is a key
 *    WASM design feature.)
 *
 * 3. **Module-based**: WASM code lives in modules with explicit imports and
 *    exports. There's no global mutable state accessible from outside. This
 *    sandboxing is crucial for running untrusted code in browsers.
 *
 * 4. **Stack validation**: WASM validates that the stack is balanced at function
 *    boundaries. Unlike the JVM (which needs explicit ``pop`` for expression
 *    statements), WASM handles stack cleanup implicitly at function boundaries.
 *
 * This module uses the same encoding as the existing ``wasm-simulator`` package,
 * so compiled code can be directly executed by that simulator.
 *
 * Opcode reference
 * ----------------
 * We use the following real WASM opcodes:
 *
 * | Instruction  | Byte | Description                               |
 * |--------------|------|-------------------------------------------|
 * | end          | 0x0B | End of function body                      |
 * | local.get    | 0x20 | Get local variable (+ 1-byte index)       |
 * | local.set    | 0x21 | Set local variable (+ 1-byte index)       |
 * | i32.const    | 0x41 | Push i32 constant (+ 4-byte LE int32)     |
 * | i32.add      | 0x6A | Integer addition                          |
 * | i32.sub      | 0x6B | Integer subtraction                       |
 * | i32.mul      | 0x6C | Integer multiplication                    |
 * | i32.div_s    | 0x6D | Integer signed division                   |
 *
 * Note: WASM distinguishes between signed and unsigned division. We use
 * ``i32.div_s`` (signed division) since our language treats all numbers as
 * signed integers.
 */

import type {
  Assignment,
  Expression,
  Program,
  Statement,
} from "@coding-adventures/parser";

// ---------------------------------------------------------------------------
// WASM Opcode constants — real values from the WebAssembly specification
// ---------------------------------------------------------------------------

// Function/block terminator
export const END = 0x0b;

// Local variable access (each followed by 1-byte index)
export const LOCAL_GET = 0x20;
export const LOCAL_SET = 0x21;

// Push 32-bit integer constant (followed by 4-byte little-endian int32)
export const I32_CONST = 0x41;

// Arithmetic
export const I32_ADD = 0x6a;
export const I32_SUB = 0x6b;
export const I32_MUL = 0x6c;
export const I32_DIV_S = 0x6d; // Signed division

// ---------------------------------------------------------------------------
// Operator-to-opcode mapping
// ---------------------------------------------------------------------------

/**
 * Maps source-level operator symbols to their WASM bytecode equivalents.
 *
 * WASM arithmetic instructions work the same way as JVM and CLR: pop two i32
 * values from the value stack, perform the operation, push the i32 result.
 *
 * Note that WASM distinguishes between signed and unsigned division. We use
 * ``i32.div_s`` (signed division) since our language treats all numbers as
 * signed integers.
 */
const WASM_OPERATOR_MAP: Record<string, number> = {
  "+": I32_ADD,
  "-": I32_SUB,
  "*": I32_MUL,
  "/": I32_DIV_S,
};

// ---------------------------------------------------------------------------
// WASMCodeObject — the compilation output
// ---------------------------------------------------------------------------

/**
 * The result of compiling an AST to WASM bytecode.
 *
 * WASM doesn't need a separate constant pool — all integer constants are
 * encoded inline in the bytecode using ``i32.const`` followed by 4 bytes.
 * This is simpler than the JVM's constant pool approach.
 */
export interface WASMCodeObject {
  /** The raw WASM bytecode bytes. */
  readonly bytecode: Uint8Array;
  /** The number of local variables declared. */
  readonly numLocals: number;
  /** Maps slot indices to variable names. */
  readonly localNames: readonly string[];
}

// ---------------------------------------------------------------------------
// WASMCompiler — the AST-to-WASM translator
// ---------------------------------------------------------------------------

/**
 * Compiles an AST into WASM bytecode bytes.
 *
 * The WASM compiler is the simplest of our three backends because WASM uses
 * a uniform encoding: every integer constant is 5 bytes (opcode + 4-byte
 * value), and every local variable access is 2 bytes (opcode + index). There
 * are no short forms to choose between.
 *
 * This simplicity is intentional in WASM's design. The bytecode is meant to
 * be generated by compilers (not written by hand), so encoding simplicity
 * matters more than bytecode compactness. The browser's JIT compiler will
 * optimize the native code anyway.
 *
 * Another WASM-specific detail: expression statements don't need an explicit
 * ``pop`` instruction. WASM validates the stack at function boundaries, and
 * the ``end`` instruction handles any remaining stack cleanup. This means
 * we can simply omit the pop for bare expression statements.
 *
 * Example:
 *
 *     import { Parser } from "@coding-adventures/parser";
 *     import { tokenize } from "@coding-adventures/lexer";
 *     import { WASMCompiler } from "@coding-adventures/bytecode-compiler";
 *
 *     const tokens = tokenize("x = 1 + 2");
 *     const ast = new Parser(tokens).parse();
 *
 *     const compiler = new WASMCompiler();
 *     const code = compiler.compile(ast);
 *     // code.bytecode contains:
 *     //   0x41 0x01 0x00 0x00 0x00  (i32.const 1)
 *     //   0x41 0x02 0x00 0x00 0x00  (i32.const 2)
 *     //   0x6A                       (i32.add)
 *     //   0x21 0x00                  (local.set 0)
 *     //   0x0B                       (end)
 */
export class WASMCompiler {
  /** The growing bytecode buffer. */
  private bytecode: number[] = [];

  /** Maps local variable slot indices to their names. */
  private _locals: string[] = [];

  // -------------------------------------------------------------------
  // Public API
  // -------------------------------------------------------------------

  /**
   * Compile a full program AST into WASM bytecode.
   *
   * Walks every statement, emits bytes, then appends ``end`` (0x0B)
   * to terminate the function body.
   */
  compile(program: Program): WASMCodeObject {
    for (const statement of program.statements) {
      this.compileStatement(statement);
    }

    // Every WASM function body ends with 'end' (0x0B).
    // This marks the end of the function's expression sequence.
    this.bytecode.push(END);

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
   * Compile a single statement into WASM bytecode.
   *
   * WASM differs from JVM and CLR here: expression statements don't need
   * an explicit ``pop``. WASM's stack validation happens at function
   * boundaries (the ``end`` instruction), so extra values on the stack
   * are handled implicitly. This means we can simply compile the expression
   * without worrying about stack cleanup.
   *
   * In practice, a real WASM compiler *would* emit ``drop`` (0x1A) for
   * expression statements to keep the stack clean within the function body.
   * But for our simple programs (which always end with ``end``), omitting
   * it works correctly with the wasm-simulator.
   */
  private compileStatement(stmt: Statement): void {
    if (stmt.kind === "Assignment") {
      this.compileAssignment(stmt);
    } else {
      // WASM: no explicit pop needed for expression statements.
      // The stack is validated at the function boundary.
      this.compileExpression(stmt);
    }
  }

  /**
   * Compile ``name = expression`` into WASM bytecode.
   */
  private compileAssignment(node: Assignment): void {
    this.compileExpression(node.value);
    const slot = this.getLocalSlot(node.target.name);
    this.bytecode.push(LOCAL_SET);
    this.bytecode.push(slot);
  }

  // -------------------------------------------------------------------
  // Expression compilation
  // -------------------------------------------------------------------

  /**
   * Compile an expression, leaving exactly one value on the stack.
   *
   * WASM's encoding is refreshingly uniform compared to JVM and CLR:
   * - Every integer uses ``i32.const`` + 4 bytes (no short forms)
   * - Every local access uses ``local.get/set`` + 1 byte index
   *
   * @throws TypeError if the node type is not recognized.
   */
  compileExpression(node: Expression): void {
    switch (node.kind) {
      case "NumberLiteral": {
        // WASM always uses i32.const followed by 4-byte little-endian.
        // No short forms, no constant pool — just the value inline.
        this.bytecode.push(I32_CONST);
        this.emitInt32LE(node.value);
        break;
      }

      case "StringLiteral":
        throw new TypeError(
          `WASM compiler does not support string literals yet. ` +
            `Got: "${node.value}"`,
        );

      case "Name": {
        const slot = this.getLocalSlot(node.name);
        this.bytecode.push(LOCAL_GET);
        this.bytecode.push(slot);
        break;
      }

      case "BinaryOp": {
        this.compileExpression(node.left);
        this.compileExpression(node.right);
        const opcode = WASM_OPERATOR_MAP[node.op];
        this.bytecode.push(opcode);
        break;
      }

      default: {
        const exhaustive: never = node;
        throw new TypeError(
          `Unknown expression type: ${(exhaustive as { kind: string }).kind}. ` +
            `The WASM compiler doesn't know how to handle this AST node.`,
        );
      }
    }
  }

  // -------------------------------------------------------------------
  // Local slot management
  // -------------------------------------------------------------------

  /**
   * Get (or assign) a local variable slot for the given name.
   *
   * WASM local variables are indexed by a simple integer, just like the
   * JVM and CLR. The encoding is always 1 byte for the index (after the
   * opcode), supporting up to 256 local variables in our simplified model.
   * Real WASM uses LEB128 encoding for the index, which supports larger
   * values.
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
