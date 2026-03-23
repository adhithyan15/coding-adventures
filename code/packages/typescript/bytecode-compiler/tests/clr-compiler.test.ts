/**
 * Comprehensive tests for the CLR IL Compiler.
 *
 * These tests verify that the CLR compiler correctly translates AST nodes into
 * real CLR IL bytecode bytes. We test the same categories as the JVM compiler:
 *
 * 1. **Number encoding** — Verify the tiered encoding (ldc.i4.N, ldc.i4.s, ldc.i4).
 * 2. **Local variable encoding** — Verify short and long forms (stloc.N, stloc.s).
 * 3. **Arithmetic operations** — Verify add, sub, mul, div opcodes.
 * 4. **End-to-end** — Full AST to bytecode verification.
 * 5. **Edge cases** — Inline encoding, many variables, etc.
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
  CLRCompiler,
  ADD,
  DIV,
  LDC_I4,
  LDC_I4_0,
  LDC_I4_1,
  LDC_I4_8,
  LDC_I4_S,
  LDLOC_0,
  LDLOC_1,
  LDLOC_S,
  MUL,
  POP,
  RET,
  STLOC_0,
  STLOC_1,
  STLOC_3,
  STLOC_S,
  SUB,
} from "../src/clr-compiler.js";
import type { CLRCodeObject } from "../src/clr-compiler.js";

// =========================================================================
// Helpers
// =========================================================================

function compileAst(prog: Program): CLRCodeObject {
  return new CLRCompiler().compile(prog);
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
// Number encoding tests
// =========================================================================

describe("TestNumberEncoding", () => {
  it("ldc.i4.0", () => {
    /** Number 0 should use ldc.i4.0 (single byte 0x16). */
    const code = compileAst(program(assign("x", num(0))));
    expect(code.bytecode[0]).toBe(LDC_I4_0);
  });

  it("ldc.i4.1", () => {
    /** Number 1 should use ldc.i4.1 (single byte 0x17). */
    const code = compileAst(program(assign("x", num(1))));
    expect(code.bytecode[0]).toBe(LDC_I4_1);
  });

  it("ldc.i4.8", () => {
    /** Number 8 should use ldc.i4.8 (single byte 0x1E). */
    const code = compileAst(program(assign("x", num(8))));
    expect(code.bytecode[0]).toBe(LDC_I4_8);
  });

  it("ldc.i4.s for 9", () => {
    /** Number 9 exceeds ldc.i4.N range, should use ldc.i4.s (0x1F, 0x09). */
    const code = compileAst(program(assign("x", num(9))));
    expect(code.bytecode[0]).toBe(LDC_I4_S);
    expect(code.bytecode[1]).toBe(9);
  });

  it("ldc.i4.s for 100", () => {
    /** Number 100 should use ldc.i4.s (0x1F, 0x64). */
    const code = compileAst(program(assign("x", num(100))));
    expect(code.bytecode[0]).toBe(LDC_I4_S);
    expect(code.bytecode[1]).toBe(100);
  });

  it("ldc.i4.s for 127", () => {
    /** Number 127 (max positive signed byte) should use ldc.i4.s. */
    const code = compileAst(program(assign("x", num(127))));
    expect(code.bytecode[0]).toBe(LDC_I4_S);
    expect(code.bytecode[1]).toBe(127);
  });

  it("ldc.i4.s for negative", () => {
    /** Negative numbers in -128 to -1 range should use ldc.i4.s. */
    const code = compileAst(program(assign("x", num(-1))));
    expect(code.bytecode[0]).toBe(LDC_I4_S);
    expect(code.bytecode[1]).toBe(0xff); // -1 as unsigned byte
  });

  it("ldc.i4 for 128", () => {
    /** Number 128 exceeds ldc.i4.s range, should use ldc.i4 (5 bytes). */
    const code = compileAst(program(assign("x", num(128))));
    expect(code.bytecode[0]).toBe(LDC_I4);
    // Next 4 bytes should be 128 as little-endian int32
    expect(readInt32LE(code.bytecode, 1)).toBe(128);
  });

  it("ldc.i4 for large number", () => {
    /** Large numbers should use ldc.i4 with 4-byte little-endian encoding. */
    const code = compileAst(program(assign("x", num(100000))));
    expect(code.bytecode[0]).toBe(LDC_I4);
    expect(readInt32LE(code.bytecode, 1)).toBe(100000);
  });

  it("ldc.i4 for negative 129", () => {
    /** Number -129 exceeds ldc.i4.s range, should use ldc.i4. */
    const code = compileAst(program(assign("x", num(-129))));
    expect(code.bytecode[0]).toBe(LDC_I4);
    expect(readInt32LE(code.bytecode, 1)).toBe(-129);
  });
});

// =========================================================================
// Local variable encoding tests
// =========================================================================

describe("TestLocalVariableEncoding", () => {
  it("stloc.0", () => {
    /** First variable should use stloc.0 (0x0A). */
    const code = compileAst(program(assign("x", num(1))));
    expect(code.bytecode[1]).toBe(STLOC_0);
  });

  it("stloc.1", () => {
    /** Second variable should use stloc.1 (0x0B). */
    const code = compileAst(
      program(assign("x", num(1)), assign("y", num(2))),
    );
    // bytecode: ldc.i4.1, stloc.0, ldc.i4.2, stloc.1, ret
    expect(code.bytecode[3]).toBe(STLOC_1);
  });

  it("stloc.3", () => {
    /** Fourth variable should use stloc.3 (0x0D). */
    const code = compileAst(
      program(
        assign("a", num(0)),
        assign("b", num(1)),
        assign("c", num(2)),
        assign("d", num(3)),
      ),
    );
    // Each assignment: 1 byte ldc.i4.N + 1 byte stloc.N = 2 bytes
    expect(code.bytecode[7]).toBe(STLOC_3);
  });

  it("stloc.s for slot 4", () => {
    /** Fifth+ variable should use stloc.s (0x13) + index byte. */
    const code = compileAst(
      program(
        assign("a", num(0)),
        assign("b", num(1)),
        assign("c", num(2)),
        assign("d", num(3)),
        assign("e", num(4)),
      ),
    );
    // First 4 assignments: 8 bytes, then ldc.i4.4 (1 byte), stloc.s, 4
    expect(code.bytecode[9]).toBe(STLOC_S);
    expect(code.bytecode[10]).toBe(4);
  });

  it("ldloc.0", () => {
    /** Loading first variable should use ldloc.0 (0x06). */
    const code = compileAst(
      program(assign("x", num(1)), assign("y", name("x"))),
    );
    // bytecode: ldc.i4.1, stloc.0, ldloc.0, stloc.1, ret
    expect(code.bytecode[2]).toBe(LDLOC_0);
  });

  it("ldloc.1", () => {
    /** Loading second variable should use ldloc.1 (0x07). */
    const code = compileAst(
      program(
        assign("x", num(1)),
        assign("y", num(2)),
        assign("z", name("y")),
      ),
    );
    // bytecode: ldc.i4.1, stloc.0, ldc.i4.2, stloc.1, ldloc.1, stloc.2, ret
    expect(code.bytecode[4]).toBe(LDLOC_1);
  });

  it("ldloc.s for slot 4", () => {
    /** Loading 5th+ variable should use ldloc.s (0x11) + index byte. */
    const code = compileAst(
      program(
        assign("a", num(0)),
        assign("b", num(1)),
        assign("c", num(2)),
        assign("d", num(3)),
        assign("e", num(4)),
        assign("f", name("e")),
      ),
    );
    // After 5 assignments (11 bytes), we have ldloc.s 4, stloc.s 5
    expect(code.bytecode[11]).toBe(LDLOC_S);
    expect(code.bytecode[12]).toBe(4);
  });
});

// =========================================================================
// Arithmetic operation tests
// =========================================================================

describe("TestArithmeticOps", () => {
  it("add", () => {
    const code = compileAst(program(assign("x", binop(num(1), "+", num(2)))));
    expect(code.bytecode).toContain(ADD);
  });

  it("sub", () => {
    const code = compileAst(program(assign("x", binop(num(5), "-", num(3)))));
    expect(code.bytecode).toContain(SUB);
  });

  it("mul", () => {
    const code = compileAst(program(assign("x", binop(num(4), "*", num(3)))));
    expect(code.bytecode).toContain(MUL);
  });

  it("div", () => {
    const code = compileAst(program(assign("x", binop(num(10), "/", num(2)))));
    expect(code.bytecode).toContain(DIV);
  });
});

// =========================================================================
// End-to-end bytecode verification
// =========================================================================

describe("TestEndToEnd", () => {
  it("x equals 1 plus 2", () => {
    const code = compileAst(program(assign("x", binop(num(1), "+", num(2)))));
    const expected = new Uint8Array([
      LDC_I4_1, // ldc.i4.1
      LDC_I4_1 + 1, // ldc.i4.2
      ADD, // add
      STLOC_0, // stloc.0
      RET, // ret
    ]);
    expect(code.bytecode).toEqual(expected);
  });

  it("x equals 100", () => {
    const code = compileAst(program(assign("x", num(100))));
    const expected = new Uint8Array([LDC_I4_S, 100, STLOC_0, RET]);
    expect(code.bytecode).toEqual(expected);
  });

  it("expression statement emits pop", () => {
    const code = compileAst(program(binop(num(1), "+", num(2))));
    const expected = new Uint8Array([
      LDC_I4_1, // ldc.i4.1
      LDC_I4_1 + 1, // ldc.i4.2
      ADD, // add
      POP, // pop
      RET, // ret
    ]);
    expect(code.bytecode).toEqual(expected);
  });

  it("two assignments", () => {
    const code = compileAst(
      program(assign("x", num(1)), assign("y", num(2))),
    );
    const expected = new Uint8Array([
      LDC_I4_1, // ldc.i4.1
      STLOC_0, // stloc.0 (x)
      LDC_I4_1 + 1, // ldc.i4.2
      STLOC_1, // stloc.1 (y)
      RET, // ret
    ]);
    expect(code.bytecode).toEqual(expected);
  });

  it("empty program", () => {
    const code = compileAst(program());
    expect(code.bytecode).toEqual(new Uint8Array([RET]));
    expect(code.numLocals).toBe(0);
    expect(code.localNames).toEqual([]);
  });

  it("ends with ret", () => {
    const code = compileAst(program(assign("x", num(42))));
    expect(code.bytecode[code.bytecode.length - 1]).toBe(RET);
  });

  it("ldc.i4 end to end", () => {
    /** ``x = 1000`` should use ldc.i4 with 4-byte little-endian. */
    const code = compileAst(program(assign("x", num(1000))));
    // Build expected: ldc.i4 + int32LE(1000) + stloc.0 + ret
    const buf = new ArrayBuffer(4);
    new DataView(buf).setInt32(0, 1000, true);
    const int32Bytes = new Uint8Array(buf);
    const expected = new Uint8Array([
      LDC_I4,
      ...int32Bytes,
      STLOC_0,
      RET,
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
  it("returns CLRCodeObject", () => {
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
    const compiler = new CLRCompiler();
    const fakeNode = { kind: "FakeNode" } as unknown as Expression;
    expect(() => compiler.compileExpression(fakeNode)).toThrow(TypeError);
    expect(() => compiler.compileExpression(fakeNode)).toThrow(
      /Unknown expression type/,
    );
  });

  it("string literal raises TypeError", () => {
    /** String literals are not yet supported in the CLR compiler. */
    expect(() => compileAst(program(assign("x", str("hello"))))).toThrow(
      TypeError,
    );
    expect(() => compileAst(program(assign("x", str("hello"))))).toThrow(
      /CLR compiler does not support string/,
    );
  });
});
