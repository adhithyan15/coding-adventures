/**
 * Tests for the Starlark Compiler -- verifying all grammar rule handlers.
 *
 * These tests verify that the compiler correctly translates Starlark ASTs
 * into bytecode. We test each grammar rule handler by compiling small
 * Starlark programs and inspecting the resulting CodeObject.
 *
 * Test strategy:
 * - Use ``compileStarlark()`` for end-to-end source-to-bytecode tests
 * - Use ``createStarlarkCompiler()`` for more targeted AST-to-bytecode tests
 * - Check instructions, constants, and names in the resulting CodeObject
 */

import { describe, it, expect } from "vitest";
import {
  compileStarlark,
  createStarlarkCompiler,
  parseStringLiteral,
  Op,
} from "../src/index.js";
import { parseStarlark } from "@coding-adventures/starlark-parser";

// =========================================================================
// Helper: find opcodes in instructions
// =========================================================================

/**
 * Extract just the opcode values from a CodeObject's instructions.
 * This makes it easy to check "which opcodes were emitted?" without
 * worrying about operand details.
 */
function opcodes(code: { instructions: readonly { opcode: number }[] }): number[] {
  return code.instructions.map((i) => i.opcode);
}

/**
 * Find the first instruction with a given opcode.
 */
function findInstr(
  code: { instructions: readonly { opcode: number; operand?: number | string | null }[] },
  opcode: number
): { opcode: number; operand?: number | string | null } | undefined {
  return code.instructions.find((i) => i.opcode === opcode);
}

/**
 * Count instructions with a given opcode.
 */
function countOp(
  code: { instructions: readonly { opcode: number }[] },
  opcode: number
): number {
  return code.instructions.filter((i) => i.opcode === opcode).length;
}

// =========================================================================
// Integer Literals
// =========================================================================

describe("Integer Literals", () => {
  it("compiles a simple integer assignment", () => {
    const code = compileStarlark("x = 42\n");
    expect(code.constants).toContain(42);
    expect(code.names).toContain("x");
    expect(opcodes(code)).toContain(Op.LOAD_CONST);
    expect(opcodes(code)).toContain(Op.STORE_NAME);
    expect(opcodes(code)).toContain(Op.HALT);
  });

  it("compiles zero", () => {
    const code = compileStarlark("x = 0\n");
    expect(code.constants).toContain(0);
  });

  it("compiles negative integer via unary minus", () => {
    const code = compileStarlark("x = -1\n");
    expect(code.constants).toContain(1);
    expect(opcodes(code)).toContain(Op.NEGATE);
  });
});

// =========================================================================
// Float Literals
// =========================================================================

describe("Float Literals", () => {
  it("compiles a float assignment", () => {
    const code = compileStarlark("x = 3.14\n");
    expect(code.constants).toContain(3.14);
  });
});

// =========================================================================
// String Literals
// =========================================================================

describe("String Literals", () => {
  it("compiles a double-quoted string", () => {
    const code = compileStarlark('x = "hello"\n');
    expect(code.constants).toContain("hello");
  });

  it("compiles a single-quoted string", () => {
    const code = compileStarlark("x = 'world'\n");
    expect(code.constants).toContain("world");
  });
});

// =========================================================================
// Boolean and None Literals
// =========================================================================

describe("Boolean and None Literals", () => {
  it("compiles True", () => {
    const code = compileStarlark("x = True\n");
    expect(opcodes(code)).toContain(Op.LOAD_TRUE);
  });

  it("compiles False", () => {
    const code = compileStarlark("x = False\n");
    expect(opcodes(code)).toContain(Op.LOAD_FALSE);
  });

  it("compiles None", () => {
    const code = compileStarlark("x = None\n");
    expect(opcodes(code)).toContain(Op.LOAD_NONE);
  });
});

// =========================================================================
// Arithmetic Operations
// =========================================================================

describe("Arithmetic Operations", () => {
  it("compiles addition", () => {
    const code = compileStarlark("x = 1 + 2\n");
    expect(opcodes(code)).toContain(Op.ADD);
  });

  it("compiles subtraction", () => {
    const code = compileStarlark("x = 5 - 3\n");
    expect(opcodes(code)).toContain(Op.SUB);
  });

  it("compiles multiplication", () => {
    const code = compileStarlark("x = 2 * 3\n");
    expect(opcodes(code)).toContain(Op.MUL);
  });

  it("compiles division", () => {
    const code = compileStarlark("x = 10 / 3\n");
    expect(opcodes(code)).toContain(Op.DIV);
  });

  it("compiles floor division", () => {
    const code = compileStarlark("x = 10 // 3\n");
    expect(opcodes(code)).toContain(Op.FLOOR_DIV);
  });

  it("compiles modulo", () => {
    const code = compileStarlark("x = 10 % 3\n");
    expect(opcodes(code)).toContain(Op.MOD);
  });

  it("compiles exponentiation", () => {
    const code = compileStarlark("x = 2 ** 3\n");
    expect(opcodes(code)).toContain(Op.POWER);
  });

  it("compiles unary negation", () => {
    const code = compileStarlark("x = -5\n");
    expect(opcodes(code)).toContain(Op.NEGATE);
  });

  it("compiles chained addition", () => {
    const code = compileStarlark("x = 1 + 2 + 3\n");
    expect(countOp(code, Op.ADD)).toBe(2);
  });

  it("compiles mixed arithmetic", () => {
    const code = compileStarlark("x = 1 + 2 * 3\n");
    expect(opcodes(code)).toContain(Op.ADD);
    expect(opcodes(code)).toContain(Op.MUL);
  });
});

// =========================================================================
// Bitwise Operations
// =========================================================================

describe("Bitwise Operations", () => {
  it("compiles bitwise AND", () => {
    const code = compileStarlark("x = 5 & 3\n");
    expect(opcodes(code)).toContain(Op.BIT_AND);
  });

  it("compiles bitwise OR", () => {
    const code = compileStarlark("x = 5 | 3\n");
    expect(opcodes(code)).toContain(Op.BIT_OR);
  });

  it("compiles bitwise XOR", () => {
    const code = compileStarlark("x = 5 ^ 3\n");
    expect(opcodes(code)).toContain(Op.BIT_XOR);
  });

  it("compiles bitwise NOT", () => {
    const code = compileStarlark("x = ~5\n");
    expect(opcodes(code)).toContain(Op.BIT_NOT);
  });

  it("compiles left shift", () => {
    const code = compileStarlark("x = 1 << 3\n");
    expect(opcodes(code)).toContain(Op.LSHIFT);
  });

  it("compiles right shift", () => {
    const code = compileStarlark("x = 8 >> 2\n");
    expect(opcodes(code)).toContain(Op.RSHIFT);
  });
});

// =========================================================================
// Comparison Operations
// =========================================================================

describe("Comparison Operations", () => {
  it("compiles ==", () => {
    const code = compileStarlark("x = 1 == 2\n");
    expect(opcodes(code)).toContain(Op.CMP_EQ);
  });

  it("compiles !=", () => {
    const code = compileStarlark("x = 1 != 2\n");
    expect(opcodes(code)).toContain(Op.CMP_NE);
  });

  it("compiles <", () => {
    const code = compileStarlark("x = 1 < 2\n");
    expect(opcodes(code)).toContain(Op.CMP_LT);
  });

  it("compiles >", () => {
    const code = compileStarlark("x = 1 > 2\n");
    expect(opcodes(code)).toContain(Op.CMP_GT);
  });

  it("compiles <=", () => {
    const code = compileStarlark("x = 1 <= 2\n");
    expect(opcodes(code)).toContain(Op.CMP_LE);
  });

  it("compiles >=", () => {
    const code = compileStarlark("x = 1 >= 2\n");
    expect(opcodes(code)).toContain(Op.CMP_GE);
  });
});

// =========================================================================
// Boolean Operations
// =========================================================================

describe("Boolean Operations", () => {
  it("compiles not", () => {
    const code = compileStarlark("x = not True\n");
    expect(opcodes(code)).toContain(Op.NOT);
  });

  it("compiles or with short-circuit", () => {
    const code = compileStarlark("x = True or False\n");
    expect(opcodes(code)).toContain(Op.JUMP_IF_TRUE_OR_POP);
  });

  it("compiles and with short-circuit", () => {
    const code = compileStarlark("x = True and False\n");
    expect(opcodes(code)).toContain(Op.JUMP_IF_FALSE_OR_POP);
  });

  it("compiles chained or", () => {
    const code = compileStarlark("x = a or b or c\n");
    expect(countOp(code, Op.JUMP_IF_TRUE_OR_POP)).toBe(2);
  });

  it("compiles chained and", () => {
    const code = compileStarlark("x = a and b and c\n");
    expect(countOp(code, Op.JUMP_IF_FALSE_OR_POP)).toBe(2);
  });
});

// =========================================================================
// Variable Operations
// =========================================================================

describe("Variable Operations", () => {
  it("compiles variable assignment", () => {
    const code = compileStarlark("x = 1\n");
    expect(code.names).toContain("x");
    expect(opcodes(code)).toContain(Op.STORE_NAME);
  });

  it("compiles variable reference", () => {
    const code = compileStarlark("y = x\n");
    expect(code.names).toContain("x");
    expect(code.names).toContain("y");
    expect(opcodes(code)).toContain(Op.LOAD_NAME);
    expect(opcodes(code)).toContain(Op.STORE_NAME);
  });

  it("compiles multiple assignments", () => {
    const code = compileStarlark("x = 1\ny = 2\n");
    expect(code.names).toContain("x");
    expect(code.names).toContain("y");
    expect(countOp(code, Op.STORE_NAME)).toBe(2);
  });
});

// =========================================================================
// Expression Statements
// =========================================================================

describe("Expression Statements", () => {
  it("compiles expression statement with POP", () => {
    const code = compileStarlark("42\n");
    expect(opcodes(code)).toContain(Op.LOAD_CONST);
    expect(opcodes(code)).toContain(Op.POP);
  });
});

// =========================================================================
// Augmented Assignment
// =========================================================================

describe("Augmented Assignment", () => {
  it("compiles +=", () => {
    const code = compileStarlark("x = 1\nx += 2\n");
    expect(opcodes(code)).toContain(Op.ADD);
    // Should have LOAD_NAME for x, then ADD, then STORE_NAME for x
    expect(countOp(code, Op.STORE_NAME)).toBe(2);
  });

  it("compiles -=", () => {
    const code = compileStarlark("x = 10\nx -= 3\n");
    expect(opcodes(code)).toContain(Op.SUB);
  });

  it("compiles *=", () => {
    const code = compileStarlark("x = 5\nx *= 2\n");
    expect(opcodes(code)).toContain(Op.MUL);
  });
});

// =========================================================================
// If Statements
// =========================================================================

describe("If Statements", () => {
  it("compiles simple if", () => {
    const code = compileStarlark("if True:\n  x = 1\n");
    expect(opcodes(code)).toContain(Op.JUMP_IF_FALSE);
    expect(opcodes(code)).toContain(Op.JUMP);
  });

  it("compiles if-else", () => {
    const code = compileStarlark("if True:\n  x = 1\nelse:\n  x = 2\n");
    expect(opcodes(code)).toContain(Op.JUMP_IF_FALSE);
    expect(countOp(code, Op.STORE_NAME)).toBe(2);
  });

  it("compiles if-elif-else", () => {
    const code = compileStarlark(
      "if x:\n  a = 1\nelif y:\n  a = 2\nelse:\n  a = 3\n"
    );
    // Should have two JUMP_IF_FALSE (one for if, one for elif)
    expect(countOp(code, Op.JUMP_IF_FALSE)).toBe(2);
  });
});

// =========================================================================
// For Loops
// =========================================================================

describe("For Loops", () => {
  it("compiles simple for loop", () => {
    const code = compileStarlark("for x in items:\n  y = x\n");
    expect(opcodes(code)).toContain(Op.GET_ITER);
    expect(opcodes(code)).toContain(Op.FOR_ITER);
    expect(opcodes(code)).toContain(Op.JUMP);
  });

  it("compiles for with break", () => {
    const code = compileStarlark("for x in items:\n  break\n");
    expect(opcodes(code)).toContain(Op.GET_ITER);
    expect(opcodes(code)).toContain(Op.FOR_ITER);
    // break emits a JUMP that gets patched
    expect(countOp(code, Op.JUMP)).toBeGreaterThanOrEqual(2);
  });

  it("compiles for with continue", () => {
    const code = compileStarlark("for x in items:\n  continue\n");
    expect(opcodes(code)).toContain(Op.GET_ITER);
    // continue emits a JUMP back to the loop top
    expect(countOp(code, Op.JUMP)).toBeGreaterThanOrEqual(2);
  });
});

// =========================================================================
// Function Definitions
// =========================================================================

describe("Function Definitions", () => {
  it("compiles simple function definition", () => {
    const code = compileStarlark("def f():\n  return 1\n");
    expect(opcodes(code)).toContain(Op.MAKE_FUNCTION);
    expect(opcodes(code)).toContain(Op.STORE_NAME);
    expect(code.names).toContain("f");
  });

  it("compiles function with parameters", () => {
    const code = compileStarlark("def add(a, b):\n  return a + b\n");
    expect(opcodes(code)).toContain(Op.MAKE_FUNCTION);
    expect(code.names).toContain("add");
  });

  it("compiles function call with no args", () => {
    const code = compileStarlark("f()\n");
    expect(opcodes(code)).toContain(Op.CALL_FUNCTION);
    const callInstr = findInstr(code, Op.CALL_FUNCTION);
    expect(callInstr?.operand).toBe(0);
  });

  it("compiles function call with positional args", () => {
    const code = compileStarlark("f(1, 2)\n");
    expect(opcodes(code)).toContain(Op.CALL_FUNCTION);
    const callInstr = findInstr(code, Op.CALL_FUNCTION);
    expect(callInstr?.operand).toBe(2);
  });

  it("compiles function call with keyword args", () => {
    const code = compileStarlark("f(x=1)\n");
    expect(opcodes(code)).toContain(Op.CALL_FUNCTION_KW);
  });

  it("compiles return with value", () => {
    const code = compileStarlark("def f():\n  return 42\n");
    // The nested code object should contain RETURN
    // Check that MAKE_FUNCTION is emitted
    expect(opcodes(code)).toContain(Op.MAKE_FUNCTION);
  });

  it("compiles return without value", () => {
    const code = compileStarlark("def f():\n  return\n");
    expect(opcodes(code)).toContain(Op.MAKE_FUNCTION);
  });
});

// =========================================================================
// Pass Statement
// =========================================================================

describe("Pass Statement", () => {
  it("compiles pass as no-op", () => {
    const code = compileStarlark("pass\n");
    // pass emits nothing -- just HALT
    expect(opcodes(code)).toEqual([Op.HALT]);
  });
});

// =========================================================================
// List Literals
// =========================================================================

describe("List Literals", () => {
  it("compiles empty list", () => {
    const code = compileStarlark("x = []\n");
    expect(opcodes(code)).toContain(Op.BUILD_LIST);
    const buildList = findInstr(code, Op.BUILD_LIST);
    expect(buildList?.operand).toBe(0);
  });

  it("compiles list with elements", () => {
    const code = compileStarlark("x = [1, 2, 3]\n");
    expect(opcodes(code)).toContain(Op.BUILD_LIST);
    const buildList = findInstr(code, Op.BUILD_LIST);
    expect(buildList?.operand).toBe(3);
  });
});

// =========================================================================
// Dict Literals
// =========================================================================

describe("Dict Literals", () => {
  it("compiles empty dict", () => {
    const code = compileStarlark("x = {}\n");
    expect(opcodes(code)).toContain(Op.BUILD_DICT);
    const buildDict = findInstr(code, Op.BUILD_DICT);
    expect(buildDict?.operand).toBe(0);
  });

  it("compiles dict with entries", () => {
    const code = compileStarlark('x = {"a": 1, "b": 2}\n');
    expect(opcodes(code)).toContain(Op.BUILD_DICT);
    const buildDict = findInstr(code, Op.BUILD_DICT);
    expect(buildDict?.operand).toBe(2);
  });
});

// =========================================================================
// Tuple and Parenthesized Expressions
// =========================================================================

describe("Tuple Expressions", () => {
  it("compiles empty tuple", () => {
    const code = compileStarlark("x = ()\n");
    expect(opcodes(code)).toContain(Op.BUILD_TUPLE);
    const buildTuple = findInstr(code, Op.BUILD_TUPLE);
    expect(buildTuple?.operand).toBe(0);
  });

  it("compiles parenthesized expression (not a tuple)", () => {
    const code = compileStarlark("x = (1 + 2)\n");
    expect(opcodes(code)).toContain(Op.ADD);
    // Should NOT have BUILD_TUPLE since (expr) without comma is just grouping
    expect(opcodes(code)).not.toContain(Op.BUILD_TUPLE);
  });
});

// =========================================================================
// Ternary Expression
// =========================================================================

describe("Ternary Expression", () => {
  it("compiles ternary if-else expression", () => {
    const code = compileStarlark("x = 1 if True else 2\n");
    expect(opcodes(code)).toContain(Op.JUMP_IF_FALSE);
    expect(opcodes(code)).toContain(Op.JUMP);
    expect(code.constants).toContain(1);
    expect(code.constants).toContain(2);
  });
});

// =========================================================================
// Attribute Access
// =========================================================================

describe("Attribute Access", () => {
  it("compiles attribute access", () => {
    const code = compileStarlark("x = obj.attr\n");
    expect(opcodes(code)).toContain(Op.LOAD_ATTR);
    expect(code.names).toContain("attr");
  });
});

// =========================================================================
// Subscript Access
// =========================================================================

describe("Subscript Access", () => {
  it("compiles subscript access", () => {
    const code = compileStarlark("x = lst[0]\n");
    expect(opcodes(code)).toContain(Op.LOAD_SUBSCRIPT);
  });
});

// =========================================================================
// String Literal Parsing
// =========================================================================

describe("parseStringLiteral", () => {
  it("strips double quotes", () => {
    expect(parseStringLiteral('"hello"')).toBe("hello");
  });

  it("strips single quotes", () => {
    expect(parseStringLiteral("'world'")).toBe("world");
  });

  it("strips triple double quotes", () => {
    expect(parseStringLiteral('"""multi"""')).toBe("multi");
  });

  it("strips triple single quotes", () => {
    expect(parseStringLiteral("'''multi'''")).toBe("multi");
  });

  it("handles \\n escape", () => {
    expect(parseStringLiteral('"line1\\nline2"')).toBe("line1\nline2");
  });

  it("handles \\t escape", () => {
    expect(parseStringLiteral('"tab\\there"')).toBe("tab\there");
  });

  it("handles \\\\ escape", () => {
    expect(parseStringLiteral('"back\\\\slash"')).toBe("back\\slash");
  });

  it("handles \\\" escape", () => {
    expect(parseStringLiteral('"say \\"hello\\""')).toBe('say "hello"');
  });

  it("handles \\' escape", () => {
    expect(parseStringLiteral("\"it\\'s\"")).toBe("it's");
  });

  it("handles \\r escape", () => {
    expect(parseStringLiteral('"cr\\rhere"')).toBe("cr\rhere");
  });

  it("handles \\0 escape", () => {
    expect(parseStringLiteral('"null\\0here"')).toBe("null\0here");
  });

  it("preserves unknown escapes", () => {
    expect(parseStringLiteral('"test\\xvalue"')).toBe("test\\xvalue");
  });
});

// =========================================================================
// Factory Function
// =========================================================================

describe("createStarlarkCompiler", () => {
  it("returns a compiler that can compile Starlark ASTs", () => {
    const compiler = createStarlarkCompiler();
    const ast = parseStarlark("x = 1\n");
    const code = compiler.compile(ast, Op.HALT);
    expect(code.constants).toContain(1);
    expect(code.names).toContain("x");
  });

  it("handles empty file", () => {
    const code = compileStarlark("\n");
    expect(opcodes(code)).toEqual([Op.HALT]);
  });
});

// =========================================================================
// Unary Plus (no-op)
// =========================================================================

describe("Unary Plus", () => {
  it("compiles unary plus as no-op", () => {
    const code = compileStarlark("x = +5\n");
    // unary + should NOT emit NEGATE
    expect(opcodes(code)).not.toContain(Op.NEGATE);
    expect(code.constants).toContain(5);
  });
});

// =========================================================================
// Method Calls (suffix chaining)
// =========================================================================

describe("Method Calls", () => {
  it("compiles method call (dot + call)", () => {
    const code = compileStarlark("x = obj.method()\n");
    expect(opcodes(code)).toContain(Op.LOAD_ATTR);
    expect(opcodes(code)).toContain(Op.CALL_FUNCTION);
  });

  it("compiles method call with args", () => {
    const code = compileStarlark("x = obj.method(1, 2)\n");
    expect(opcodes(code)).toContain(Op.LOAD_ATTR);
    expect(opcodes(code)).toContain(Op.CALL_FUNCTION);
    const callInstr = findInstr(code, Op.CALL_FUNCTION);
    expect(callInstr?.operand).toBe(2);
  });
});

// =========================================================================
// Break/Continue errors
// =========================================================================

describe("Break/Continue outside loop", () => {
  it("throws on break outside loop", () => {
    expect(() => compileStarlark("break\n")).toThrow("'break' outside of a for loop");
  });

  it("throws on continue outside loop", () => {
    expect(() => compileStarlark("continue\n")).toThrow("'continue' outside of a for loop");
  });
});

// =========================================================================
// Complex expressions
// =========================================================================

describe("Complex Expressions", () => {
  it("compiles nested arithmetic with proper precedence", () => {
    const code = compileStarlark("x = (1 + 2) * 3\n");
    expect(opcodes(code)).toContain(Op.ADD);
    expect(opcodes(code)).toContain(Op.MUL);
  });

  it("compiles multiple statements", () => {
    const code = compileStarlark("x = 1\ny = x + 2\nz = x * y\n");
    expect(code.names).toContain("x");
    expect(code.names).toContain("y");
    expect(code.names).toContain("z");
    expect(countOp(code, Op.STORE_NAME)).toBe(3);
  });

  it("compiles constant deduplication", () => {
    const code = compileStarlark("x = 42\ny = 42\n");
    // 42 should only appear once in the constant pool
    const count42 = code.constants.filter((c) => c === 42).length;
    expect(count42).toBe(1);
  });

  it("compiles name deduplication", () => {
    const code = compileStarlark("x = 1\nx = 2\n");
    // "x" should only appear once in the name pool
    const countX = code.names.filter((n) => n === "x").length;
    expect(countX).toBe(1);
  });
});

// =========================================================================
// Semicolons (simple_stmt with multiple small_stmts)
// =========================================================================

describe("Semicolons", () => {
  it("compiles multiple statements on one line", () => {
    const code = compileStarlark("x = 1; y = 2\n");
    expect(code.names).toContain("x");
    expect(code.names).toContain("y");
    expect(countOp(code, Op.STORE_NAME)).toBe(2);
  });
});

// =========================================================================
// Nested if-elif chains
// =========================================================================

describe("Nested Control Flow", () => {
  it("compiles nested if inside for", () => {
    const code = compileStarlark(
      "for x in items:\n  if x:\n    y = x\n"
    );
    expect(opcodes(code)).toContain(Op.GET_ITER);
    expect(opcodes(code)).toContain(Op.FOR_ITER);
    expect(opcodes(code)).toContain(Op.JUMP_IF_FALSE);
  });
});

// =========================================================================
// Function definitions with defaults
// =========================================================================

describe("Function Definitions with Defaults", () => {
  it("compiles function with default parameter", () => {
    const code = compileStarlark("def f(x=1):\n  return x\n");
    expect(opcodes(code)).toContain(Op.MAKE_FUNCTION);
    // Default value 1 should be in constants
    expect(code.constants).toContain(1);
  });
});

// =========================================================================
// Load Statement
// =========================================================================

describe("Load Statement", () => {
  it("compiles load statement", () => {
    const code = compileStarlark('load("module.star", "func")\n');
    expect(opcodes(code)).toContain(Op.LOAD_MODULE);
    expect(opcodes(code)).toContain(Op.DUP);
    expect(opcodes(code)).toContain(Op.IMPORT_FROM);
    expect(opcodes(code)).toContain(Op.STORE_NAME);
    expect(opcodes(code)).toContain(Op.POP);
  });
});

// =========================================================================
// Lambda Expressions
// =========================================================================

describe("Lambda Expressions", () => {
  it("compiles simple lambda", () => {
    const code = compileStarlark("f = lambda: 1\n");
    expect(opcodes(code)).toContain(Op.MAKE_FUNCTION);
  });

  it("compiles lambda with parameters", () => {
    const code = compileStarlark("f = lambda x, y: x\n");
    expect(opcodes(code)).toContain(Op.MAKE_FUNCTION);
  });

  it("compiles lambda with default parameter", () => {
    const code = compileStarlark("f = lambda x=1: x\n");
    expect(opcodes(code)).toContain(Op.MAKE_FUNCTION);
    expect(code.constants).toContain(1);
  });
});

// =========================================================================
// Tuple Unpacking
// =========================================================================

describe("Tuple Unpacking", () => {
  it("compiles tuple unpacking assignment", () => {
    const code = compileStarlark("a, b = 1, 2\n");
    expect(opcodes(code)).toContain(Op.UNPACK_SEQUENCE);
    expect(countOp(code, Op.STORE_NAME)).toBe(2);
  });
});

// =========================================================================
// For Loop with Multiple Variables
// =========================================================================

describe("For Loop Multiple Vars", () => {
  it("compiles for with multiple loop vars", () => {
    const code = compileStarlark("for x, y in items:\n  z = x\n");
    expect(opcodes(code)).toContain(Op.UNPACK_SEQUENCE);
    expect(opcodes(code)).toContain(Op.GET_ITER);
    expect(opcodes(code)).toContain(Op.FOR_ITER);
  });
});

// =========================================================================
// List Comprehensions
// =========================================================================

describe("List Comprehensions", () => {
  it("compiles list comprehension", () => {
    const code = compileStarlark("x = [i for i in items]\n");
    expect(opcodes(code)).toContain(Op.BUILD_LIST);
    expect(opcodes(code)).toContain(Op.GET_ITER);
    expect(opcodes(code)).toContain(Op.FOR_ITER);
    expect(opcodes(code)).toContain(Op.LIST_APPEND);
  });

  it("compiles list comprehension with filter", () => {
    const code = compileStarlark("x = [i for i in items if i]\n");
    expect(opcodes(code)).toContain(Op.BUILD_LIST);
    expect(opcodes(code)).toContain(Op.GET_ITER);
    expect(opcodes(code)).toContain(Op.LIST_APPEND);
    expect(opcodes(code)).toContain(Op.JUMP_IF_FALSE);
  });
});

// =========================================================================
// Dict Comprehensions
// =========================================================================

describe("Dict Comprehensions", () => {
  it("compiles dict comprehension", () => {
    const code = compileStarlark("x = {k: v for k, v in items}\n");
    expect(opcodes(code)).toContain(Op.BUILD_DICT);
    expect(opcodes(code)).toContain(Op.GET_ITER);
    expect(opcodes(code)).toContain(Op.FOR_ITER);
    expect(opcodes(code)).toContain(Op.DICT_SET);
  });
});

// =========================================================================
// Tuple Expressions in Parens
// =========================================================================

describe("Tuple in Parens", () => {
  it("compiles multi-element tuple in parens", () => {
    const code = compileStarlark("x = (1, 2, 3)\n");
    expect(opcodes(code)).toContain(Op.BUILD_TUPLE);
  });
});

// =========================================================================
// Subscript Access (simple indexing)
// =========================================================================

describe("Subscript with Index", () => {
  it("compiles simple subscript with integer index", () => {
    const code = compileStarlark("x = lst[0]\n");
    expect(opcodes(code)).toContain(Op.LOAD_SUBSCRIPT);
  });

  it("compiles subscript with variable index", () => {
    const code = compileStarlark("x = lst[i]\n");
    expect(opcodes(code)).toContain(Op.LOAD_SUBSCRIPT);
  });
});

// =========================================================================
// Function Definitions with Varargs/Kwargs
// =========================================================================

describe("Function Varargs/Kwargs", () => {
  it("compiles function with *args", () => {
    const code = compileStarlark("def f(*args):\n  return args\n");
    expect(opcodes(code)).toContain(Op.MAKE_FUNCTION);
  });

  it("compiles function with **kwargs", () => {
    const code = compileStarlark("def f(**kwargs):\n  return kwargs\n");
    expect(opcodes(code)).toContain(Op.MAKE_FUNCTION);
  });
});

// =========================================================================
// Load Statement with Alias
// =========================================================================

describe("Load Statement with Alias", () => {
  it("compiles load with alias", () => {
    const code = compileStarlark('load("mod.star", alias = "func")\n');
    expect(opcodes(code)).toContain(Op.LOAD_MODULE);
    expect(opcodes(code)).toContain(Op.IMPORT_FROM);
    expect(code.names).toContain("alias");
  });
});

// =========================================================================
// Expression List (Tuple Creation)
// =========================================================================

describe("Expression List", () => {
  it("compiles expression list with trailing comma as tuple", () => {
    const code = compileStarlark("x = 1,\n");
    expect(opcodes(code)).toContain(Op.BUILD_TUPLE);
  });

  it("compiles multi-expression list as tuple", () => {
    const code = compileStarlark("x = 1, 2, 3\n");
    expect(opcodes(code)).toContain(Op.BUILD_TUPLE);
  });
});

// =========================================================================
// Function calls with keyword and positional args
// =========================================================================

describe("Function Calls Mixed Args", () => {
  it("compiles call with mixed positional and keyword args", () => {
    const code = compileStarlark("f(1, x=2)\n");
    expect(opcodes(code)).toContain(Op.CALL_FUNCTION_KW);
  });

  it("compiles call with *args unpacking", () => {
    const code = compileStarlark("f(*items)\n");
    expect(opcodes(code)).toContain(Op.CALL_FUNCTION);
  });

  it("compiles call with **kwargs unpacking", () => {
    const code = compileStarlark("f(**items)\n");
    expect(opcodes(code)).toContain(Op.CALL_FUNCTION);
  });
});

// =========================================================================
// Augmented assignment with all operators
// =========================================================================

describe("All Augmented Assignments", () => {
  it("compiles /=", () => {
    const code = compileStarlark("x = 10\nx /= 2\n");
    expect(opcodes(code)).toContain(Op.DIV);
  });

  it("compiles //=", () => {
    const code = compileStarlark("x = 10\nx //= 3\n");
    expect(opcodes(code)).toContain(Op.FLOOR_DIV);
  });

  it("compiles %=", () => {
    const code = compileStarlark("x = 10\nx %= 3\n");
    expect(opcodes(code)).toContain(Op.MOD);
  });

  it("compiles &=", () => {
    const code = compileStarlark("x = 5\nx &= 3\n");
    expect(opcodes(code)).toContain(Op.BIT_AND);
  });

  it("compiles |=", () => {
    const code = compileStarlark("x = 5\nx |= 3\n");
    expect(opcodes(code)).toContain(Op.BIT_OR);
  });

  it("compiles ^=", () => {
    const code = compileStarlark("x = 5\nx ^= 3\n");
    expect(opcodes(code)).toContain(Op.BIT_XOR);
  });

  it("compiles <<=", () => {
    const code = compileStarlark("x = 1\nx <<= 3\n");
    expect(opcodes(code)).toContain(Op.LSHIFT);
  });

  it("compiles >>=", () => {
    const code = compileStarlark("x = 8\nx >>= 2\n");
    expect(opcodes(code)).toContain(Op.RSHIFT);
  });

  it("compiles **=", () => {
    const code = compileStarlark("x = 2\nx **= 3\n");
    expect(opcodes(code)).toContain(Op.POWER);
  });
});

// =========================================================================
// Nested Function Scopes
// =========================================================================

describe("Function Scopes", () => {
  it("uses LOAD_LOCAL inside function for parameters", () => {
    // The nested code object should use LOAD_LOCAL for param access
    const code = compileStarlark("def f(x):\n  y = x\n");
    expect(opcodes(code)).toContain(Op.MAKE_FUNCTION);
  });
});

// =========================================================================
// Generator Expression (via paren)
// =========================================================================

describe("Generator Expression", () => {
  it("compiles generator expression in parens", () => {
    const code = compileStarlark("x = (i for i in items)\n");
    expect(opcodes(code)).toContain(Op.BUILD_LIST); // compiled as list for now
    expect(opcodes(code)).toContain(Op.GET_ITER);
    expect(opcodes(code)).toContain(Op.LIST_APPEND);
  });
});

// =========================================================================
// In/Not In comparisons
// =========================================================================

describe("In/Not In Comparisons", () => {
  it("compiles 'in' comparison", () => {
    const code = compileStarlark("x = 1 in lst\n");
    expect(opcodes(code)).toContain(Op.CMP_IN);
  });

  it("compiles 'not in' comparison", () => {
    const code = compileStarlark("x = 1 not in lst\n");
    expect(opcodes(code)).toContain(Op.CMP_NOT_IN);
  });
});

// =========================================================================
// Adjacent String Concatenation
// =========================================================================

describe("Adjacent String Concatenation", () => {
  it("compiles adjacent strings into one constant", () => {
    const code = compileStarlark('x = "hello" "world"\n');
    expect(code.constants).toContain("helloworld");
  });
});

// =========================================================================
// Instruction ordering verification
// =========================================================================

describe("Instruction Ordering", () => {
  it("emits HALT as the last instruction", () => {
    const code = compileStarlark("x = 1\n");
    const lastInstr = code.instructions[code.instructions.length - 1];
    expect(lastInstr.opcode).toBe(Op.HALT);
  });

  it("emits LOAD_CONST before STORE_NAME for assignment", () => {
    const code = compileStarlark("x = 42\n");
    const loadIdx = code.instructions.findIndex((i) => i.opcode === Op.LOAD_CONST);
    const storeIdx = code.instructions.findIndex((i) => i.opcode === Op.STORE_NAME);
    expect(loadIdx).toBeLessThan(storeIdx);
  });

  it("emits operands before operator for binary ops", () => {
    const code = compileStarlark("x = 1 + 2\n");
    const firstLoad = code.instructions.findIndex((i) => i.opcode === Op.LOAD_CONST);
    const addIdx = code.instructions.findIndex((i) => i.opcode === Op.ADD);
    expect(firstLoad).toBeLessThan(addIdx);
  });
});
