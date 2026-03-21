/**
 * Comprehensive tests for the JVM Bytecode Compiler.
 *
 * These tests verify that the JVM compiler correctly translates AST nodes into
 * real JVM bytecode bytes. We test at multiple levels:
 *
 * 1. **Number encoding** — Verify the tiered encoding (iconst, bipush, ldc).
 * 2. **Local variable encoding** — Verify short and long forms (istore_N, istore).
 * 3. **Arithmetic operations** — Verify iadd, isub, imul, idiv opcodes.
 * 4. **End-to-end** — Full AST to bytecode verification.
 * 5. **Edge cases** — Constant deduplication, many variables, etc.
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
  JVMCompiler,
  BIPUSH,
  IADD,
  ICONST_0,
  ICONST_1,
  ICONST_5,
  IDIV,
  ILOAD,
  ILOAD_0,
  ILOAD_1,
  IMUL,
  ISTORE,
  ISTORE_0,
  ISTORE_1,
  ISTORE_3,
  ISUB,
  LDC,
  POP,
  RETURN,
} from "../src/jvm-compiler.js";
import type { JVMCodeObject } from "../src/jvm-compiler.js";

// =========================================================================
// Helpers
// =========================================================================

function compileAst(prog: Program): JVMCodeObject {
  return new JVMCompiler().compile(prog);
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

// =========================================================================
// Number encoding tests
// =========================================================================

describe("TestNumberEncoding", () => {
  it("iconst_0", () => {
    /** Number 0 should use iconst_0 (single byte 0x03). */
    const code = compileAst(program(assign("x", num(0))));
    expect(code.bytecode[0]).toBe(ICONST_0);
  });

  it("iconst_1", () => {
    /** Number 1 should use iconst_1 (single byte 0x04). */
    const code = compileAst(program(assign("x", num(1))));
    expect(code.bytecode[0]).toBe(ICONST_1);
  });

  it("iconst_5", () => {
    /** Number 5 should use iconst_5 (single byte 0x08). */
    const code = compileAst(program(assign("x", num(5))));
    expect(code.bytecode[0]).toBe(ICONST_5);
  });

  it("bipush for 6", () => {
    /** Number 6 exceeds iconst range, should use bipush (0x10, 0x06). */
    const code = compileAst(program(assign("x", num(6))));
    expect(code.bytecode[0]).toBe(BIPUSH);
    expect(code.bytecode[1]).toBe(6);
  });

  it("bipush for 100", () => {
    /** Number 100 should use bipush (0x10, 0x64). */
    const code = compileAst(program(assign("x", num(100))));
    expect(code.bytecode[0]).toBe(BIPUSH);
    expect(code.bytecode[1]).toBe(100);
  });

  it("bipush for 127", () => {
    /** Number 127 (max bipush positive) should use bipush. */
    const code = compileAst(program(assign("x", num(127))));
    expect(code.bytecode[0]).toBe(BIPUSH);
    expect(code.bytecode[1]).toBe(127);
  });

  it("bipush for negative", () => {
    /** Negative numbers in -128 to -1 range should use bipush. */
    const code = compileAst(program(assign("x", num(-1))));
    expect(code.bytecode[0]).toBe(BIPUSH);
    expect(code.bytecode[1]).toBe(0xff); // -1 as unsigned byte
  });

  it("ldc for 128", () => {
    /** Number 128 exceeds bipush range, should use ldc + constant pool. */
    const code = compileAst(program(assign("x", num(128))));
    expect(code.bytecode[0]).toBe(LDC);
    expect(code.bytecode[1]).toBe(0); // constant pool index 0
    expect(code.constants).toEqual([128]);
  });

  it("ldc for large number", () => {
    /** Large numbers should use ldc with constant pool. */
    const code = compileAst(program(assign("x", num(1000))));
    expect(code.bytecode[0]).toBe(LDC);
    expect(code.constants).toContain(1000);
  });

  it("ldc for negative 129", () => {
    /** Number -129 exceeds bipush range, should use ldc. */
    const code = compileAst(program(assign("x", num(-129))));
    expect(code.bytecode[0]).toBe(LDC);
    expect(code.constants).toContain(-129);
  });
});

// =========================================================================
// Local variable encoding tests
// =========================================================================

describe("TestLocalVariableEncoding", () => {
  it("istore_0", () => {
    /** First variable should use istore_0 (0x3B). */
    const code = compileAst(program(assign("x", num(1))));
    // bytecode: iconst_1, istore_0, return
    expect(code.bytecode[1]).toBe(ISTORE_0);
  });

  it("istore_1", () => {
    /** Second variable should use istore_1 (0x3C). */
    const code = compileAst(
      program(assign("x", num(1)), assign("y", num(2))),
    );
    // bytecode: iconst_1, istore_0, iconst_2, istore_1, return
    expect(code.bytecode[3]).toBe(ISTORE_1);
  });

  it("istore_3", () => {
    /** Fourth variable should use istore_3 (0x3E). */
    const code = compileAst(
      program(
        assign("a", num(0)),
        assign("b", num(1)),
        assign("c", num(2)),
        assign("d", num(3)),
      ),
    );
    expect(code.bytecode[7]).toBe(ISTORE_3);
  });

  it("istore generic for slot 4", () => {
    /** Fifth+ variable should use istore (0x36) + index byte. */
    const code = compileAst(
      program(
        assign("a", num(0)),
        assign("b", num(1)),
        assign("c", num(2)),
        assign("d", num(3)),
        assign("e", num(4)),
      ),
    );
    // Find the last istore sequence — should be istore 4
    expect(code.bytecode[9]).toBe(ISTORE);
    expect(code.bytecode[10]).toBe(4);
  });

  it("iload_0", () => {
    /** Loading first variable should use iload_0 (0x1A). */
    const code = compileAst(
      program(assign("x", num(1)), assign("y", name("x"))),
    );
    // bytecode: iconst_1, istore_0, iload_0, istore_1, return
    expect(code.bytecode[2]).toBe(ILOAD_0);
  });

  it("iload_1", () => {
    /** Loading second variable should use iload_1 (0x1B). */
    const code = compileAst(
      program(
        assign("x", num(1)),
        assign("y", num(2)),
        assign("z", name("y")),
      ),
    );
    // bytecode: iconst_1, istore_0, iconst_2, istore_1, iload_1, istore_2, return
    expect(code.bytecode[4]).toBe(ILOAD_1);
  });

  it("iload generic for slot 4", () => {
    /** Loading 5th+ variable should use iload (0x15) + index byte. */
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
    // After 5 assignments (11 bytes), we have iload 4, istore ...
    expect(code.bytecode[11]).toBe(ILOAD);
    expect(code.bytecode[12]).toBe(4);
  });
});

// =========================================================================
// Arithmetic operation tests
// =========================================================================

describe("TestArithmeticOps", () => {
  it("iadd", () => {
    /** Addition should emit iadd (0x60). */
    const code = compileAst(program(assign("x", binop(num(1), "+", num(2)))));
    expect(code.bytecode).toContain(IADD);
  });

  it("isub", () => {
    /** Subtraction should emit isub (0x64). */
    const code = compileAst(program(assign("x", binop(num(5), "-", num(3)))));
    expect(code.bytecode).toContain(ISUB);
  });

  it("imul", () => {
    /** Multiplication should emit imul (0x68). */
    const code = compileAst(program(assign("x", binop(num(4), "*", num(3)))));
    expect(code.bytecode).toContain(IMUL);
  });

  it("idiv", () => {
    /** Division should emit idiv (0x6C). */
    const code = compileAst(program(assign("x", binop(num(10), "/", num(2)))));
    expect(code.bytecode).toContain(IDIV);
  });
});

// =========================================================================
// End-to-end bytecode verification
// =========================================================================

describe("TestEndToEnd", () => {
  it("x equals 1 plus 2", () => {
    /** ``x = 1 + 2`` should produce: iconst_1, iconst_2, iadd, istore_0, return. */
    const code = compileAst(program(assign("x", binop(num(1), "+", num(2)))));

    const expected = new Uint8Array([
      ICONST_1, // iconst_1 (push 1)
      ICONST_1 + 1, // iconst_2 (push 2)
      IADD, // iadd (1 + 2 = 3)
      ISTORE_0, // istore_0 (store in x)
      RETURN, // return
    ]);
    expect(code.bytecode).toEqual(expected);
  });

  it("x equals 100", () => {
    /** ``x = 100`` should use bipush encoding. */
    const code = compileAst(program(assign("x", num(100))));

    const expected = new Uint8Array([
      BIPUSH, // bipush
      100, // value 100
      ISTORE_0, // istore_0
      RETURN, // return
    ]);
    expect(code.bytecode).toEqual(expected);
  });

  it("expression statement emits pop", () => {
    /** A bare expression statement should emit pop (0x57) after evaluation. */
    const code = compileAst(program(binop(num(1), "+", num(2))));

    const expected = new Uint8Array([
      ICONST_1, // iconst_1
      ICONST_1 + 1, // iconst_2
      IADD, // iadd
      POP, // pop (discard result)
      RETURN, // return
    ]);
    expect(code.bytecode).toEqual(expected);
  });

  it("two assignments", () => {
    /** Two assignments use different local slots. */
    const code = compileAst(
      program(assign("x", num(1)), assign("y", num(2))),
    );

    const expected = new Uint8Array([
      ICONST_1, // iconst_1
      ISTORE_0, // istore_0 (x)
      ICONST_1 + 1, // iconst_2
      ISTORE_1, // istore_1 (y)
      RETURN, // return
    ]);
    expect(code.bytecode).toEqual(expected);
  });

  it("empty program", () => {
    /** An empty program should just produce return. */
    const code = compileAst(program());
    expect(code.bytecode).toEqual(new Uint8Array([RETURN]));
    expect(code.constants).toEqual([]);
    expect(code.numLocals).toBe(0);
    expect(code.localNames).toEqual([]);
  });

  it("ends with return", () => {
    /** Every compiled program should end with return (0xB1). */
    const code = compileAst(program(assign("x", num(42))));
    expect(code.bytecode[code.bytecode.length - 1]).toBe(RETURN);
  });
});

// =========================================================================
// Constant pool tests
// =========================================================================

describe("TestConstantPool", () => {
  it("constant deduplication", () => {
    /** Using the same large number twice should reuse the constant pool entry. */
    const code = compileAst(
      program(assign("x", num(200)), assign("y", num(200))),
    );

    // 200 should appear only once in the constant pool
    expect(code.constants).toEqual([200]);
    // Both ldc instructions should reference index 0
    const ldcIndices: number[] = [];
    for (let i = 0; i < code.bytecode.length; i++) {
      if (code.bytecode[i] === LDC) {
        ldcIndices.push(code.bytecode[i + 1]);
      }
    }
    expect(ldcIndices).toEqual([0, 0]);
  });

  it("different constants get different indices", () => {
    /** Different large numbers get separate constant pool entries. */
    const code = compileAst(
      program(assign("x", num(200)), assign("y", num(300))),
    );
    expect(code.constants).toEqual([200, 300]);
  });

  it("string in constant pool", () => {
    /** String literals should be stored in the constant pool. */
    const code = compileAst(program(assign("x", str("hello"))));
    expect(code.constants).toContain("hello");
    expect(code.bytecode[0]).toBe(LDC);
  });
});

// =========================================================================
// Local names mapping tests
// =========================================================================

describe("TestLocalNames", () => {
  it("single variable", () => {
    /** One variable: slot 0 = x. */
    const code = compileAst(program(assign("x", num(1))));
    expect(code.localNames).toEqual(["x"]);
    expect(code.numLocals).toBe(1);
  });

  it("multiple variables", () => {
    /** Multiple variables: slots assigned in order of first appearance. */
    const code = compileAst(
      program(assign("x", num(1)), assign("y", num(2)), assign("z", num(3))),
    );
    expect(code.localNames).toEqual(["x", "y", "z"]);
    expect(code.numLocals).toBe(3);
  });

  it("reassignment reuses slot", () => {
    /** Reassigning a variable should reuse its existing slot. */
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
  it("returns JVMCodeObject", () => {
    /** compile() should return a JVMCodeObject. */
    const code = compileAst(program(num(1)));
    expect(code).toHaveProperty("bytecode");
    expect(code).toHaveProperty("constants");
    expect(code).toHaveProperty("numLocals");
    expect(code).toHaveProperty("localNames");
  });

  it("bytecode is Uint8Array", () => {
    /** The bytecode field should be a Uint8Array. */
    const code = compileAst(program(num(1)));
    expect(code.bytecode).toBeInstanceOf(Uint8Array);
  });
});

// =========================================================================
// Error handling tests
// =========================================================================

describe("TestErrorHandling", () => {
  it("unknown expression raises TypeError", () => {
    /** Passing an unrecognized AST node should raise TypeError. */
    const compiler = new JVMCompiler();
    const fakeNode = { kind: "FakeNode" } as unknown as Expression;
    expect(() => compiler.compileExpression(fakeNode)).toThrow(TypeError);
    expect(() => compiler.compileExpression(fakeNode)).toThrow(
      /Unknown expression type/,
    );
  });
});
