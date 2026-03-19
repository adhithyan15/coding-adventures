/**
 * Comprehensive tests for the Bytecode Compiler.
 *
 * These tests verify that the compiler correctly translates AST nodes into
 * bytecode instructions. We test at two levels:
 *
 * 1. **Unit tests** — Feed hand-built AST nodes into the compiler and verify
 *    the exact instructions, constants, and names that come out. These tests
 *    are precise and isolated from the lexer/parser.
 *
 * 2. **End-to-end tests** — Use ``compileSource`` to go from source code all
 *    the way to a CodeObject, then execute it on the VM and check the results.
 *    These tests verify that the full pipeline works together.
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
  BytecodeCompiler,
  compileSource,
  OpCode,
  VirtualMachine,
} from "../src/index.js";
import type { CodeObject, Instruction, OpCodeValue } from "../src/index.js";

// =========================================================================
// Helpers
// =========================================================================

/** Extract just the opcodes from a CodeObject, for quick comparison. */
function opcodes(code: CodeObject): OpCodeValue[] {
  return code.instructions.map((instr) => instr.opcode);
}

/** Extract just the operands from a CodeObject, for quick comparison. */
function operands(
  code: CodeObject,
): (number | string | null | undefined)[] {
  return code.instructions.map((instr) => instr.operand);
}

/** Shortcut: compile a Program AST into a CodeObject. */
function compileAst(program: Program): CodeObject {
  return new BytecodeCompiler().compile(program);
}

// Helper constructors for AST nodes — these match the parser's interface
// with the `kind` discriminant field.

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

function program(...statements: readonly (Expression | Assignment)[]): Program {
  return { kind: "Program", statements };
}

// =========================================================================
// Unit tests — AST node to bytecode
// =========================================================================

describe("TestNumberLiteral", () => {
  it("number literal produces LOAD_CONST, POP, HALT", () => {
    /** A number expression statement should: load the constant, pop it
     *  (because it's not assigned), then halt. */
    const code = compileAst(program(num(42)));

    expect(opcodes(code)).toEqual([OpCode.LOAD_CONST, OpCode.POP, OpCode.HALT]);
    expect(operands(code)).toEqual([0, undefined, undefined]);
    expect(code.constants).toEqual([42]);
    expect(code.names).toEqual([]);
  });

  it("number literal zero", () => {
    /** Zero is a valid constant and should be handled normally. */
    const code = compileAst(program(num(0)));

    expect(code.constants).toEqual([0]);
    expect(opcodes(code)).toEqual([OpCode.LOAD_CONST, OpCode.POP, OpCode.HALT]);
  });
});

describe("TestStringLiteral", () => {
  it("string literal produces LOAD_CONST, POP, HALT", () => {
    /** A string expression statement: load, pop (unused), halt. */
    const code = compileAst(program(str("hello")));

    expect(opcodes(code)).toEqual([OpCode.LOAD_CONST, OpCode.POP, OpCode.HALT]);
    expect(operands(code)).toEqual([0, undefined, undefined]);
    expect(code.constants).toEqual(["hello"]);
    expect(code.names).toEqual([]);
  });

  it("empty string", () => {
    /** Empty strings are valid constants. */
    const code = compileAst(program(str("")));
    expect(code.constants).toEqual([""]);
  });
});

describe("TestNameReference", () => {
  it("name produces LOAD_NAME, POP, HALT", () => {
    /** A bare name reference: look up the variable, pop the result, halt. */
    const code = compileAst(program(name("x")));

    expect(opcodes(code)).toEqual([OpCode.LOAD_NAME, OpCode.POP, OpCode.HALT]);
    expect(operands(code)).toEqual([0, undefined, undefined]);
    expect(code.constants).toEqual([]);
    expect(code.names).toEqual(["x"]);
  });
});

describe("TestAssignment", () => {
  it("simple assignment", () => {
    /** ``x = 42`` should: load 42, store in x, halt. No POP needed because
     *  STORE_NAME already pops the value. */
    const code = compileAst(program(assign("x", num(42))));

    expect(opcodes(code)).toEqual([
      OpCode.LOAD_CONST,
      OpCode.STORE_NAME,
      OpCode.HALT,
    ]);
    expect(operands(code)).toEqual([0, 0, undefined]);
    expect(code.constants).toEqual([42]);
    expect(code.names).toEqual(["x"]);
  });

  it("assignment with string", () => {
    /** ``name = "alice"`` should store a string constant. */
    const code = compileAst(program(assign("name", str("alice"))));

    expect(code.constants).toEqual(["alice"]);
    expect(code.names).toEqual(["name"]);
    expect(opcodes(code)).toEqual([
      OpCode.LOAD_CONST,
      OpCode.STORE_NAME,
      OpCode.HALT,
    ]);
  });
});

describe("TestBinaryOp", () => {
  it("addition", () => {
    /** ``1 + 2`` as an expression statement: load both, add, pop, halt. */
    const code = compileAst(program(binop(num(1), "+", num(2))));

    expect(opcodes(code)).toEqual([
      OpCode.LOAD_CONST,
      OpCode.LOAD_CONST,
      OpCode.ADD,
      OpCode.POP,
      OpCode.HALT,
    ]);
    expect(code.constants).toEqual([1, 2]);
  });

  it("subtraction", () => {
    /** ``5 - 3`` should emit SUB. */
    const code = compileAst(program(binop(num(5), "-", num(3))));
    expect(opcodes(code)).toContain(OpCode.SUB);
  });

  it("multiplication", () => {
    /** ``4 * 7`` should emit MUL. */
    const code = compileAst(program(binop(num(4), "*", num(7))));
    expect(opcodes(code)).toContain(OpCode.MUL);
  });

  it("division", () => {
    /** ``10 / 2`` should emit DIV. */
    const code = compileAst(program(binop(num(10), "/", num(2))));
    expect(opcodes(code)).toContain(OpCode.DIV);
  });
});

describe("TestComplexExpressions", () => {
  it("assignment with binary op", () => {
    /** ``x = 1 + 2`` should compile the addition, then store in x. */
    const code = compileAst(program(assign("x", binop(num(1), "+", num(2)))));

    expect(opcodes(code)).toEqual([
      OpCode.LOAD_CONST, // 1
      OpCode.LOAD_CONST, // 2
      OpCode.ADD,
      OpCode.STORE_NAME, // x
      OpCode.HALT,
    ]);
    expect(code.constants).toEqual([1, 2]);
    expect(code.names).toEqual(["x"]);
  });

  it("nested binary ops respects tree structure", () => {
    /**
     * ``x = 1 + 2 * 3`` — the parser builds the tree with * binding tighter,
     * so the compiler should emit the multiplication before the addition.
     *
     * AST: Assignment(x, BinaryOp(1, +, BinaryOp(2, *, 3)))
     */
    const code = compileAst(
      program(assign("x", binop(num(1), "+", binop(num(2), "*", num(3))))),
    );

    expect(opcodes(code)).toEqual([
      OpCode.LOAD_CONST, // 1
      OpCode.LOAD_CONST, // 2
      OpCode.LOAD_CONST, // 3
      OpCode.MUL,
      OpCode.ADD,
      OpCode.STORE_NAME, // x
      OpCode.HALT,
    ]);
    expect(code.constants).toEqual([1, 2, 3]);
  });

  it("binary op with name operands", () => {
    /** ``a + b`` should emit LOAD_NAME for both operands. */
    const code = compileAst(program(binop(name("a"), "+", name("b"))));

    expect(opcodes(code)).toEqual([
      OpCode.LOAD_NAME,
      OpCode.LOAD_NAME,
      OpCode.ADD,
      OpCode.POP,
      OpCode.HALT,
    ]);
    expect(code.names).toEqual(["a", "b"]);
  });
});

describe("TestMultipleStatements", () => {
  it("two assignments", () => {
    /** ``x = 1`` then ``y = 2`` — each gets its own constant and name. */
    const code = compileAst(program(assign("x", num(1)), assign("y", num(2))));

    expect(opcodes(code)).toEqual([
      OpCode.LOAD_CONST, // 1
      OpCode.STORE_NAME, // x
      OpCode.LOAD_CONST, // 2
      OpCode.STORE_NAME, // y
      OpCode.HALT,
    ]);
    expect(code.constants).toEqual([1, 2]);
    expect(code.names).toEqual(["x", "y"]);
  });

  it("assignment then expression", () => {
    /** ``x = 42`` then ``x`` — the second statement is an expression
     *  that should be popped. */
    const code = compileAst(program(assign("x", num(42)), name("x")));

    expect(opcodes(code)).toEqual([
      OpCode.LOAD_CONST, // 42
      OpCode.STORE_NAME, // x
      OpCode.LOAD_NAME, // x
      OpCode.POP,
      OpCode.HALT,
    ]);
  });
});

describe("TestDeduplication", () => {
  it("constant deduplication", () => {
    /** Using the same number twice should reuse the same constant index. */
    const code = compileAst(
      program(assign("x", num(1)), assign("y", num(1))),
    );

    // Both LOAD_CONST instructions should reference index 0
    expect(code.constants).toEqual([1]);
    const loadConsts = code.instructions.filter(
      (i) => i.opcode === OpCode.LOAD_CONST,
    );
    expect(loadConsts.every((i) => i.operand === 0)).toBe(true);
  });

  it("name deduplication", () => {
    /** Referencing the same variable twice should reuse the same name index. */
    const code = compileAst(
      program(assign("x", num(1)), assign("x", num(2))),
    );

    // Both STORE_NAME instructions should reference index 0
    expect(code.names).toEqual(["x"]);
    const storeNames = code.instructions.filter(
      (i) => i.opcode === OpCode.STORE_NAME,
    );
    expect(storeNames.every((i) => i.operand === 0)).toBe(true);
  });

  it("mixed deduplication", () => {
    /** Constants and names are deduplicated independently. */
    const code = compileAst(
      program(
        assign("x", num(5)),
        assign("y", binop(name("x"), "+", num(5))),
      ),
    );

    // 5 appears twice but should be stored once
    expect(code.constants).toEqual([5]);
    // x appears in both statements (store and load), should be stored once
    expect(code.names.filter((n) => n === "x").length).toBeLessThanOrEqual(1);
    // No duplicates in name pool
    expect(code.names.length).toBe(new Set(code.names).size);
  });
});

describe("TestEmptyProgram", () => {
  it("empty program produces just HALT", () => {
    /** An empty program should still produce a valid CodeObject with HALT. */
    const code = compileAst(program());

    expect(opcodes(code)).toEqual([OpCode.HALT]);
    expect(code.constants).toEqual([]);
    expect(code.names).toEqual([]);
  });
});

describe("TestCompilerReturnType", () => {
  it("returns code object", () => {
    /** compile() should return a CodeObject. */
    const code = compileAst(program(num(1)));
    expect(code).toHaveProperty("instructions");
    expect(code).toHaveProperty("constants");
    expect(code).toHaveProperty("names");
  });

  it("code object has instructions", () => {
    /** The CodeObject should have an instructions list. */
    const code = compileAst(program(num(1)));
    expect(Array.isArray(code.instructions)).toBe(true);
    for (const instr of code.instructions) {
      expect(instr).toHaveProperty("opcode");
    }
  });
});

describe("TestUnknownExpressionType", () => {
  it("unknown expression raises TypeError", () => {
    /** Passing an unrecognized AST node should raise TypeError. */
    const compiler = new BytecodeCompiler();
    const fakeNode = { kind: "FakeNode" } as unknown as Expression;
    expect(() => compiler.compileExpression(fakeNode)).toThrow(TypeError);
    expect(() => compiler.compileExpression(fakeNode)).toThrow(
      /Unknown expression type/,
    );
  });
});

// =========================================================================
// End-to-end tests — source code -> VM execution
// =========================================================================

describe("TestEndToEnd", () => {
  it("simple assignment", () => {
    /** ``x = 1 + 2`` should result in x == 3. */
    const code = compileSource("x = 1 + 2");
    const vm = new VirtualMachine();
    vm.execute(code);
    expect(vm.variables["x"]).toBe(3);
  });

  it("multiple assignments", () => {
    /** Multiple assignments should all be accessible in the VM. */
    const code = compileSource("a = 10\nb = 20\nc = a + b");
    const vm = new VirtualMachine();
    vm.execute(code);
    expect(vm.variables["a"]).toBe(10);
    expect(vm.variables["b"]).toBe(20);
    expect(vm.variables["c"]).toBe(30);
  });

  it("arithmetic operations", () => {
    /** Test all four arithmetic operations end-to-end. */
    const code = compileSource("a = 10 + 5\nb = 10 - 5\nc = 10 * 5\nd = 10 / 5");
    const vm = new VirtualMachine();
    vm.execute(code);
    expect(vm.variables["a"]).toBe(15);
    expect(vm.variables["b"]).toBe(5);
    expect(vm.variables["c"]).toBe(50);
    expect(vm.variables["d"]).toBe(2);
  });

  it("expression with precedence", () => {
    /** ``x = 2 + 3 * 4`` should respect multiplication precedence. */
    const code = compileSource("x = 2 + 3 * 4");
    const vm = new VirtualMachine();
    vm.execute(code);
    expect(vm.variables["x"]).toBe(14); // 2 + (3 * 4) = 14, not (2+3)*4 = 20
  });

  it("variable reuse", () => {
    /** A variable can be assigned, then used in a later expression. */
    const code = compileSource("x = 10\ny = x + 5");
    const vm = new VirtualMachine();
    vm.execute(code);
    expect(vm.variables["x"]).toBe(10);
    expect(vm.variables["y"]).toBe(15);
  });

  it("variable reassignment", () => {
    /** A variable can be reassigned to a new value. */
    const code = compileSource("x = 1\nx = 2");
    const vm = new VirtualMachine();
    vm.execute(code);
    expect(vm.variables["x"]).toBe(2);
  });

  it("compile source returns CodeObject", () => {
    /** compileSource should return a CodeObject. */
    const code = compileSource("x = 42");
    expect(code).toHaveProperty("instructions");
    expect(code).toHaveProperty("constants");
    expect(code).toHaveProperty("names");
  });

  it("compile source with keywords", () => {
    /** compileSource should accept optional keywords parameter. */
    const code = compileSource("x = 1", ["if", "else"]);
    const vm = new VirtualMachine();
    vm.execute(code);
    expect(vm.variables["x"]).toBe(1);
  });

  it("chain of operations", () => {
    /** A longer program exercising multiple features. */
    const code = compileSource("a = 1\nb = 2\nc = 3\nresult = a + b * c");
    const vm = new VirtualMachine();
    vm.execute(code);
    // b * c = 6, a + 6 = 7
    expect(vm.variables["result"]).toBe(7);
  });
});

describe("TestCompileSourceConvenience", () => {
  it("basic usage", () => {
    /** Simplest possible usage. */
    const code = compileSource("42");
    expect(code).toHaveProperty("instructions");
    expect(code.instructions.length).toBeGreaterThan(0);
  });

  it("ends with HALT", () => {
    /** Every compiled program should end with HALT. */
    const code = compileSource("x = 1");
    const lastInstr = code.instructions[code.instructions.length - 1];
    expect(lastInstr.opcode).toBe(OpCode.HALT);
  });
});
