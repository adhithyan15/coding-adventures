/**
 * Comprehensive tests for the WASM Bytecode Compiler.
 *
 * These tests verify that the WASM compiler correctly translates AST nodes into
 * real WebAssembly bytecode bytes. The WASM compiler is simpler than JVM/CLR
 * because WASM uses uniform encoding (no short forms), but we still test:
 *
 * 1. **Number encoding** — Verify i32.const with 4-byte little-endian.
 * 2. **Local variable encoding** — Verify local.get/local.set with index byte.
 * 3. **Arithmetic operations** — Verify i32.add, i32.sub, i32.mul, i32.div_s.
 * 4. **End-to-end** — Full AST to bytecode verification.
 * 5. **WASM-specific** — No pop for expression statements, end instruction.
 */

import { describe, it, expect } from "vitest";
import type {
  NumberLiteral,
  StringLiteral,
  Name,
  BinaryOp,
  Assignment,
  Program,
  Expression,
} from "@coding-adventures/parser";
import {
  WASMCompiler,
  END,
  I32_ADD,
  I32_CONST,
  I32_DIV_S,
  I32_MUL,
  I32_SUB,
  LOCAL_GET,
  LOCAL_SET,
} from "../src/wasm-compiler.js";
import type { WASMCodeObject } from "../src/wasm-compiler.js";

// =========================================================================
// Helpers
// =========================================================================

function compileAst(prog: Program): WASMCodeObject {
  return new WASMCompiler().compile(prog);
}

function num(value: number): NumberLiteral {
  return { kind: "NumberLiteral", value };
}
function str(value: string): StringLiteral {
  return { kind: "StringLiteral", value };
}
function name(n: string): Name {
  return { kind: "Name", name: n };
}
function binop(left: Expression, op: string, right: Expression): BinaryOp {
  return { kind: "BinaryOp", left, op, right };
}
function assign(target: string, value: Expression): Assignment {
  return { kind: "Assignment", target: name(target), value };
}
function program(
  ...statements: readonly (Expression | Assignment)[]
): Program {
  return { kind: "Program", statements };
}

/** Build the expected bytes for an i32.const instruction. */
function i32ConstBytes(value: number): number[] {
  const buf = new ArrayBuffer(4);
  new DataView(buf).setInt32(0, value, true);
  const bytes = new Uint8Array(buf);
  return [I32_CONST, ...bytes];
}

// =========================================================================
// Number encoding tests
// =========================================================================

describe("TestNumberEncoding", () => {
  it("i32.const 0", () => {
    /** Number 0 should use i32.const + 4 zero bytes (no short form in WASM). */
    const code = compileAst(program(assign("x", num(0))));
    expect(code.bytecode[0]).toBe(I32_CONST);
    expect(readInt32LE(code.bytecode, 1)).toBe(0);
  });

  it("i32.const 1", () => {
    /** Number 1 should use i32.const + 4-byte encoding. */
    const code = compileAst(program(assign("x", num(1))));
    expect(code.bytecode[0]).toBe(I32_CONST);
    expect(readInt32LE(code.bytecode, 1)).toBe(1);
  });

  it("i32.const 42", () => {
    /** Number 42 should use i32.const + 4-byte encoding. */
    const code = compileAst(program(assign("x", num(42))));
    expect(code.bytecode.slice(0, 5)).toEqual(
      new Uint8Array(i32ConstBytes(42)),
    );
  });

  it("i32.const large", () => {
    /** Large number should use i32.const + 4-byte encoding. */
    const code = compileAst(program(assign("x", num(100000))));
    expect(code.bytecode.slice(0, 5)).toEqual(
      new Uint8Array(i32ConstBytes(100000)),
    );
  });

  it("i32.const negative", () => {
    /** Negative numbers should use i32.const with signed encoding. */
    const code = compileAst(program(assign("x", num(-1))));
    expect(code.bytecode[0]).toBe(I32_CONST);
    expect(readInt32LE(code.bytecode, 1)).toBe(-1);
  });

  it("i32.const max negative", () => {
    /** Large negative numbers should work with signed 32-bit encoding. */
    const code = compileAst(program(assign("x", num(-1000))));
    expect(code.bytecode.slice(0, 5)).toEqual(
      new Uint8Array(i32ConstBytes(-1000)),
    );
  });
});

/** Read a 4-byte little-endian int32 from a Uint8Array at the given offset. */
function readInt32LE(bytes: Uint8Array, offset: number): number {
  const buf = new ArrayBuffer(4);
  const view = new DataView(buf);
  const arr = new Uint8Array(buf);
  arr[0] = bytes[offset];
  arr[1] = bytes[offset + 1];
  arr[2] = bytes[offset + 2];
  arr[3] = bytes[offset + 3];
  return view.getInt32(0, true);
}

// =========================================================================
// Local variable encoding tests
// =========================================================================

describe("TestLocalVariableEncoding", () => {
  it("local.set 0", () => {
    /** First variable should use local.set 0 (0x21, 0x00). */
    const code = compileAst(program(assign("x", num(1))));
    // i32.const 1 (5 bytes), local.set 0 (2 bytes), end (1 byte)
    expect(code.bytecode[5]).toBe(LOCAL_SET);
    expect(code.bytecode[6]).toBe(0);
  });

  it("local.set 1", () => {
    /** Second variable should use local.set 1 (0x21, 0x01). */
    const code = compileAst(
      program(assign("x", num(1)), assign("y", num(2))),
    );
    // First: i32.const(5) + local.set(2) = 7, then i32.const(5) + local.set(2)
    expect(code.bytecode[12]).toBe(LOCAL_SET);
    expect(code.bytecode[13]).toBe(1);
  });

  it("local.get 0", () => {
    /** Loading first variable should use local.get 0 (0x20, 0x00). */
    const code = compileAst(
      program(assign("x", num(1)), assign("y", name("x"))),
    );
    // First assignment: i32.const(5) + local.set(2) = 7 bytes
    // Then: local.get 0 (2 bytes)
    expect(code.bytecode[7]).toBe(LOCAL_GET);
    expect(code.bytecode[8]).toBe(0);
  });

  it("local.get 1", () => {
    /** Loading second variable should use local.get 1. */
    const code = compileAst(
      program(
        assign("x", num(1)),
        assign("y", num(2)),
        assign("z", name("y")),
      ),
    );
    // Two assignments: 7 + 7 = 14 bytes, then local.get 1
    expect(code.bytecode[14]).toBe(LOCAL_GET);
    expect(code.bytecode[15]).toBe(1);
  });

  it("many locals", () => {
    /** Variables beyond the first few use increasing slot indices. */
    const code = compileAst(
      program(
        assign("a", num(0)),
        assign("b", num(1)),
        assign("c", num(2)),
        assign("d", num(3)),
        assign("e", num(4)),
      ),
    );
    // Fifth variable should use slot 4
    // 4 assignments * 7 bytes = 28 bytes, then i32.const(5), local.set
    expect(code.bytecode[33]).toBe(LOCAL_SET);
    expect(code.bytecode[34]).toBe(4);
  });
});

// =========================================================================
// Arithmetic operation tests
// =========================================================================

describe("TestArithmeticOps", () => {
  it("i32.add", () => {
    const code = compileAst(program(assign("x", binop(num(1), "+", num(2)))));
    expect(code.bytecode).toContain(I32_ADD);
  });

  it("i32.sub", () => {
    const code = compileAst(program(assign("x", binop(num(5), "-", num(3)))));
    expect(code.bytecode).toContain(I32_SUB);
  });

  it("i32.mul", () => {
    const code = compileAst(program(assign("x", binop(num(4), "*", num(3)))));
    expect(code.bytecode).toContain(I32_MUL);
  });

  it("i32.div_s", () => {
    const code = compileAst(program(assign("x", binop(num(10), "/", num(2)))));
    expect(code.bytecode).toContain(I32_DIV_S);
  });
});

// =========================================================================
// End-to-end bytecode verification
// =========================================================================

describe("TestEndToEnd", () => {
  it("x equals 1 plus 2", () => {
    const code = compileAst(program(assign("x", binop(num(1), "+", num(2)))));
    const expected = new Uint8Array([
      ...i32ConstBytes(1), // i32.const 1 (5 bytes)
      ...i32ConstBytes(2), // i32.const 2 (5 bytes)
      I32_ADD, // i32.add (1 byte)
      LOCAL_SET,
      0, // local.set 0 (2 bytes)
      END, // end (1 byte)
    ]);
    expect(code.bytecode).toEqual(expected);
  });

  it("no pop for expression statement", () => {
    /** WASM doesn't need pop for expression statements. */
    const code = compileAst(program(binop(num(1), "+", num(2))));
    // No pop instruction — WASM handles stack at function boundary
    const expected = new Uint8Array([
      ...i32ConstBytes(1),
      ...i32ConstBytes(2),
      I32_ADD,
      END,
    ]);
    expect(code.bytecode).toEqual(expected);
  });

  it("two assignments", () => {
    const code = compileAst(
      program(assign("x", num(1)), assign("y", num(2))),
    );
    const expected = new Uint8Array([
      ...i32ConstBytes(1), // i32.const 1
      LOCAL_SET,
      0, // local.set 0 (x)
      ...i32ConstBytes(2), // i32.const 2
      LOCAL_SET,
      1, // local.set 1 (y)
      END, // end
    ]);
    expect(code.bytecode).toEqual(expected);
  });

  it("empty program", () => {
    const code = compileAst(program());
    expect(code.bytecode).toEqual(new Uint8Array([END]));
    expect(code.numLocals).toBe(0);
    expect(code.localNames).toEqual([]);
  });

  it("ends with end", () => {
    const code = compileAst(program(assign("x", num(42))));
    expect(code.bytecode[code.bytecode.length - 1]).toBe(END);
  });

  it("variable load and store", () => {
    /** ``x = 1; y = x`` should load x and store into y. */
    const code = compileAst(
      program(assign("x", num(1)), assign("y", name("x"))),
    );
    const expected = new Uint8Array([
      ...i32ConstBytes(1), // i32.const 1
      LOCAL_SET,
      0, // local.set 0 (x)
      LOCAL_GET,
      0, // local.get 0 (x)
      LOCAL_SET,
      1, // local.set 1 (y)
      END, // end
    ]);
    expect(code.bytecode).toEqual(expected);
  });
});

// =========================================================================
// Local names mapping tests
// =========================================================================

describe("TestLocalNames", () => {
  it("single variable", () => {
    const code = compileAst(program(assign("x", num(1))));
    expect(code.localNames).toEqual(["x"]);
    expect(code.numLocals).toBe(1);
  });

  it("multiple variables", () => {
    const code = compileAst(
      program(assign("x", num(1)), assign("y", num(2)), assign("z", num(3))),
    );
    expect(code.localNames).toEqual(["x", "y", "z"]);
    expect(code.numLocals).toBe(3);
  });

  it("reassignment reuses slot", () => {
    const code = compileAst(
      program(assign("x", num(1)), assign("x", num(2))),
    );
    expect(code.localNames).toEqual(["x"]);
    expect(code.numLocals).toBe(1);
  });
});

// =========================================================================
// Return type tests
// =========================================================================

describe("TestReturnType", () => {
  it("returns WASMCodeObject", () => {
    const code = compileAst(program(num(1)));
    expect(code).toHaveProperty("bytecode");
    expect(code).toHaveProperty("numLocals");
    expect(code).toHaveProperty("localNames");
  });

  it("bytecode is Uint8Array", () => {
    const code = compileAst(program(num(1)));
    expect(code.bytecode).toBeInstanceOf(Uint8Array);
  });
});

// =========================================================================
// Error handling tests
// =========================================================================

describe("TestErrorHandling", () => {
  it("unknown expression raises TypeError", () => {
    const compiler = new WASMCompiler();
    const fakeNode = { kind: "FakeNode" } as unknown as Expression;
    expect(() => compiler.compileExpression(fakeNode)).toThrow(TypeError);
    expect(() => compiler.compileExpression(fakeNode)).toThrow(
      /Unknown expression type/,
    );
  });

  it("string literal raises TypeError", () => {
    expect(() => compileAst(program(assign("x", str("hello"))))).toThrow(
      TypeError,
    );
    expect(() => compileAst(program(assign("x", str("hello"))))).toThrow(
      /WASM compiler does not support string/,
    );
  });
});
