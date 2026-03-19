/**
 * Tests for the pipeline orchestrator.
 *
 * These tests verify that the full pipeline — lexer, parser, compiler, VM —
 * works end-to-end. Each test feeds source code into the pipeline and checks
 * that every stage produced the expected output.
 *
 * The tests are organized into five groups:
 *
 * 1. **TestPipelineBasic** — Simple programs that exercise the happy path.
 * 2. **TestPipelineComplex** — More involved programs: multiple statements,
 *    operator precedence, parentheses, strings.
 * 3. **TestAstToDict** — Unit tests for the AST-to-dictionary converter.
 * 4. **TestInstructionToText** — Unit tests for human-readable bytecode.
 * 5. **TestStageDataclasses** — Verify the structure of stage interfaces.
 */

import { describe, it, expect } from "vitest";

import {
  Pipeline,
  astToDict,
  instructionToText,
  OpCode,
} from "../src/index.js";

import type {
  PipelineResult,
  LexerStage,
  ParserStage,
  CompilerStage,
  VMStage,
  CodeObject,
  Instruction,
} from "../src/index.js";

// =========================================================================
// Group 1: Basic pipeline tests
// =========================================================================

describe("TestPipelineBasic", () => {
  /** Test the pipeline with simple single-statement programs. */

  it("simple assignment returns pipeline result", () => {
    /** Running `x = 1 + 2` should return a PipelineResult. */
    const result = new Pipeline().run("x = 1 + 2");
    expect(result).toBeDefined();
    expect(result.source).toBeDefined();
    expect(result.lexerStage).toBeDefined();
    expect(result.parserStage).toBeDefined();
    expect(result.compilerStage).toBeDefined();
    expect(result.vmStage).toBeDefined();
  });

  it("source is preserved", () => {
    /** The original source code should be captured in the result. */
    const result = new Pipeline().run("x = 1 + 2");
    expect(result.source).toBe("x = 1 + 2");
  });

  it("lexer stage has tokens", () => {
    /** The lexer stage should produce at least 6 tokens:
     * NAME, EQUALS, NUMBER, PLUS, NUMBER, EOF. */
    const result = new Pipeline().run("x = 1 + 2");
    expect(result.lexerStage.tokenCount).toBeGreaterThanOrEqual(6);
  });

  it("lexer stage source", () => {
    /** The lexer stage should capture the original source. */
    const result = new Pipeline().run("x = 1 + 2");
    expect(result.lexerStage.source).toBe("x = 1 + 2");
  });

  it("parser stage has ast", () => {
    /** The parser stage should produce a Program with one statement. */
    const result = new Pipeline().run("x = 1 + 2");
    const astDict = result.parserStage.astDict;
    expect(astDict["type"]).toBe("Program");
    expect(
      (astDict["statements"] as unknown[]).length,
    ).toBe(1);
  });

  it("parser stage assignment", () => {
    /** The AST should contain an Assignment to Name('x'). */
    const result = new Pipeline().run("x = 1 + 2");
    const stmt = (
      result.parserStage.astDict["statements"] as Record<string, unknown>[]
    )[0];
    expect(stmt["type"]).toBe("Assignment");
    expect(stmt["target"]).toEqual({ type: "Name", name: "x" });
  });

  it("compiler stage has instructions", () => {
    /** The compiler stage should produce at least one instruction. */
    const result = new Pipeline().run("x = 1 + 2");
    expect(result.compilerStage.instructionsText.length).toBeGreaterThan(0);
  });

  it("compiler stage constants", () => {
    /** The compiler should capture constants 1 and 2. */
    const result = new Pipeline().run("x = 1 + 2");
    expect(result.compilerStage.constants).toEqual([1, 2]);
  });

  it("compiler stage names", () => {
    /** The compiler should capture name 'x'. */
    const result = new Pipeline().run("x = 1 + 2");
    expect(result.compilerStage.names).toEqual(["x"]);
  });

  it("vm stage final variables", () => {
    /** The VM should compute x = 3. */
    const result = new Pipeline().run("x = 1 + 2");
    expect(result.vmStage.finalVariables).toEqual({ x: 3 });
  });

  it("vm stage has traces", () => {
    /** The VM should produce execution traces. */
    const result = new Pipeline().run("x = 1 + 2");
    expect(result.vmStage.traces.length).toBeGreaterThan(0);
  });

  it("vm stage output is array", () => {
    /** The VM output should be an array (possibly empty). */
    const result = new Pipeline().run("x = 1 + 2");
    expect(Array.isArray(result.vmStage.output)).toBe(true);
  });
});

// =========================================================================
// Group 2: Complex pipeline tests
// =========================================================================

describe("TestPipelineComplex", () => {
  /** Test the pipeline with multi-statement and complex programs. */

  it("multiple assignments", () => {
    /** Multiple assignments should all be captured. */
    const source = "a = 10\nb = 20\nc = a + b";
    const result = new Pipeline().run(source);
    expect(result.vmStage.finalVariables).toEqual({
      a: 10,
      b: 20,
      c: 30,
    });
  });

  it("operator precedence", () => {
    /** Multiplication should bind tighter than addition. */
    const result = new Pipeline().run("x = 1 + 2 * 3");
    expect(result.vmStage.finalVariables).toEqual({ x: 7 });
  });

  it("parentheses", () => {
    /** Parentheses should override default precedence. */
    const result = new Pipeline().run("x = (1 + 2) * 3");
    expect(result.vmStage.finalVariables).toEqual({ x: 9 });
  });

  it("string assignment", () => {
    /** String literals should be handled correctly. */
    const result = new Pipeline().run('x = "hello"');
    expect(result.vmStage.finalVariables).toEqual({ x: "hello" });
  });

  it("subtraction", () => {
    /** Subtraction should work correctly. */
    const result = new Pipeline().run("x = 10 - 3");
    expect(result.vmStage.finalVariables).toEqual({ x: 7 });
  });

  it("division", () => {
    /** Division should work correctly. */
    const result = new Pipeline().run("x = 10 / 2");
    expect(result.vmStage.finalVariables).toEqual({ x: 5 });
  });

  it("complex expression", () => {
    /** A complex nested expression should evaluate correctly. */
    const result = new Pipeline().run("x = (10 + 20) * (3 - 1)");
    expect(result.vmStage.finalVariables).toEqual({ x: 60 });
  });

  it("variable reuse", () => {
    /** Variables should be reusable across statements. */
    const source = "x = 5\ny = x * 2";
    const result = new Pipeline().run(source);
    expect(result.vmStage.finalVariables).toEqual({ x: 5, y: 10 });
  });

  it("multiple statements have multiple ast nodes", () => {
    /** Multiple statements should produce multiple AST nodes. */
    const source = "a = 1\nb = 2";
    const result = new Pipeline().run(source);
    expect(
      (result.parserStage.astDict["statements"] as unknown[]).length,
    ).toBe(2);
  });

  it("traces count increases with complexity", () => {
    /** More instructions should produce more traces. */
    const simple = new Pipeline().run("x = 1");
    const complex_ = new Pipeline().run("x = 1 + 2 * 3");
    expect(complex_.vmStage.traces.length).toBeGreaterThan(
      simple.vmStage.traces.length,
    );
  });
});

// =========================================================================
// Group 3: AST-to-dict conversion tests
// =========================================================================

describe("TestAstToDict", () => {
  /** Test the astToDict helper function. */

  it("number literal", () => {
    /** A NumberLiteral should convert to a dict with type and value. */
    expect(astToDict({ kind: "NumberLiteral", value: 42 })).toEqual({
      type: "NumberLiteral",
      value: 42,
    });
  });

  it("string literal", () => {
    /** A StringLiteral should convert to a dict with type and value. */
    expect(astToDict({ kind: "StringLiteral", value: "hello" })).toEqual({
      type: "StringLiteral",
      value: "hello",
    });
  });

  it("name", () => {
    /** A Name should convert to a dict with type and name. */
    expect(astToDict({ kind: "Name", name: "x" })).toEqual({
      type: "Name",
      name: "x",
    });
  });

  it("binary op", () => {
    /** A BinaryOp should convert recursively. */
    const node = {
      kind: "BinaryOp" as const,
      left: { kind: "NumberLiteral" as const, value: 1 },
      op: "+",
      right: { kind: "NumberLiteral" as const, value: 2 },
    };
    const d = astToDict(node) as Record<string, unknown>;
    expect(d["type"]).toBe("BinaryOp");
    expect(d["op"]).toBe("+");
    expect(d["left"]).toEqual({ type: "NumberLiteral", value: 1 });
    expect(d["right"]).toEqual({ type: "NumberLiteral", value: 2 });
  });

  it("assignment", () => {
    /** An Assignment should convert with target and value. */
    const node = {
      kind: "Assignment" as const,
      target: { kind: "Name" as const, name: "x" },
      value: { kind: "NumberLiteral" as const, value: 42 },
    };
    const d = astToDict(node) as Record<string, unknown>;
    expect(d["type"]).toBe("Assignment");
    expect(d["target"]).toEqual({ type: "Name", name: "x" });
    expect(d["value"]).toEqual({ type: "NumberLiteral", value: 42 });
  });

  it("program", () => {
    /** A Program should convert with a statements list. */
    const node = {
      kind: "Program" as const,
      statements: [
        {
          kind: "Assignment" as const,
          target: { kind: "Name" as const, name: "x" },
          value: { kind: "NumberLiteral" as const, value: 1 },
        },
      ],
    };
    const d = astToDict(node) as Record<string, unknown>;
    expect(d["type"]).toBe("Program");
    expect((d["statements"] as unknown[]).length).toBe(1);
  });

  it("unknown type fallback", () => {
    /** Unknown types should get a fallback dict with type and repr. */
    const d = astToDict("something else") as Record<string, unknown>;
    expect(d["type"]).toBe("string");
    expect(d["repr"]).toBeDefined();
  });
});

// =========================================================================
// Group 4: Instruction-to-text conversion tests
// =========================================================================

describe("TestInstructionToText", () => {
  /** Test the instructionToText helper function. */

  it("load const with resolution", () => {
    /** LOAD_CONST should resolve to the actual constant value. */
    const code: CodeObject = {
      instructions: [{ opcode: OpCode.LOAD_CONST, operand: 0 }],
      constants: [42],
      names: [],
    };
    const text = instructionToText(code.instructions[0], code);
    expect(text).toBe("LOAD_CONST 0 (42)");
  });

  it("store name with resolution", () => {
    /** STORE_NAME should resolve to the actual variable name. */
    const code: CodeObject = {
      instructions: [{ opcode: OpCode.STORE_NAME, operand: 0 }],
      constants: [],
      names: ["x"],
    };
    const text = instructionToText(code.instructions[0], code);
    expect(text).toBe("STORE_NAME 0 ('x')");
  });

  it("load name with resolution", () => {
    /** LOAD_NAME should resolve to the actual variable name. */
    const code: CodeObject = {
      instructions: [{ opcode: OpCode.LOAD_NAME, operand: 0 }],
      constants: [],
      names: ["y"],
    };
    const text = instructionToText(code.instructions[0], code);
    expect(text).toBe("LOAD_NAME 0 ('y')");
  });

  it("add no operand", () => {
    /** ADD should just show the opcode name. */
    const code: CodeObject = {
      instructions: [{ opcode: OpCode.ADD }],
      constants: [],
      names: [],
    };
    const text = instructionToText(code.instructions[0], code);
    expect(text).toBe("ADD");
  });

  it("halt no operand", () => {
    /** HALT should just show the opcode name. */
    const code: CodeObject = {
      instructions: [{ opcode: OpCode.HALT }],
      constants: [],
      names: [],
    };
    const text = instructionToText(code.instructions[0], code);
    expect(text).toBe("HALT");
  });

  it("out of bounds operand", () => {
    /** Out-of-bounds operand should fall back to raw display. */
    const code: CodeObject = {
      instructions: [{ opcode: OpCode.LOAD_CONST, operand: 99 }],
      constants: [42],
      names: [],
    };
    const text = instructionToText(code.instructions[0], code);
    expect(text).toBe("LOAD_CONST 99");
  });
});

// =========================================================================
// Group 5: Stage structure tests
// =========================================================================

describe("TestStageStructure", () => {
  /** Verify the structure and types of stage fields. */

  it("lexer stage tokens are array", () => {
    /** Tokens should be an array. */
    const result = new Pipeline().run("x = 1");
    expect(Array.isArray(result.lexerStage.tokens)).toBe(true);
  });

  it("parser stage ast dict is object", () => {
    /** AST dict should be a plain object. */
    const result = new Pipeline().run("x = 1");
    expect(typeof result.parserStage.astDict).toBe("object");
  });

  it("compiler stage code has instructions", () => {
    /** The CodeObject should have an instructions array. */
    const result = new Pipeline().run("x = 1");
    expect(result.compilerStage.code.instructions).toBeDefined();
    expect(Array.isArray(result.compilerStage.code.instructions)).toBe(true);
  });

  it("vm stage traces are array", () => {
    /** Traces should be an array. */
    const result = new Pipeline().run("x = 1");
    expect(Array.isArray(result.vmStage.traces)).toBe(true);
  });

  it("vm stage final variables is object", () => {
    /** Final variables should be an object. */
    const result = new Pipeline().run("x = 1");
    expect(typeof result.vmStage.finalVariables).toBe("object");
  });
});

// =========================================================================
// Group 6: BytecodeCompiler unit tests
// =========================================================================

describe("TestBytecodeCompiler", () => {
  /** Direct tests for the BytecodeCompiler. */

  it("compiles simple assignment", () => {
    const result = new Pipeline().run("x = 42");
    const code = result.compilerStage.code;
    expect(code.constants).toEqual([42]);
    expect(code.names).toEqual(["x"]);
    // Should have: LOAD_CONST 0, STORE_NAME 0, HALT
    expect(code.instructions.length).toBe(3);
  });

  it("compiles binary operation", () => {
    const result = new Pipeline().run("x = 1 + 2");
    const code = result.compilerStage.code;
    expect(code.constants).toEqual([1, 2]);
    // LOAD_CONST 0, LOAD_CONST 1, ADD, STORE_NAME 0, HALT
    expect(code.instructions.length).toBe(5);
  });

  it("deduplicates constants", () => {
    /** If the same constant appears twice, it should be stored once. */
    const result = new Pipeline().run("x = 1 + 1");
    const code = result.compilerStage.code;
    // The constant 1 should appear only once in the pool
    expect(code.constants).toEqual([1]);
  });

  it("deduplicates names", () => {
    /** If a variable is used in assignment and expression, the name appears once. */
    const result = new Pipeline().run("x = 5\ny = x");
    const code = result.compilerStage.code;
    // "x" and "y" should each appear once
    expect(code.names).toEqual(["x", "y"]);
  });
});

// =========================================================================
// Group 7: VirtualMachine unit tests
// =========================================================================

describe("TestVirtualMachine", () => {
  /** Direct tests for the VirtualMachine. */

  it("executes simple assignment", () => {
    const result = new Pipeline().run("x = 42");
    expect(result.vmStage.finalVariables).toEqual({ x: 42 });
  });

  it("traces have correct structure", () => {
    const result = new Pipeline().run("x = 1");
    const trace = result.vmStage.traces[0];
    expect(trace.pc).toBeDefined();
    expect(trace.instruction).toBeDefined();
    expect(trace.stackBefore).toBeDefined();
    expect(trace.stackAfter).toBeDefined();
    expect(trace.variables).toBeDefined();
    expect(trace.description).toBeDefined();
  });

  it("trace descriptions are human readable", () => {
    const result = new Pipeline().run("x = 42");
    // First trace should be about pushing a constant
    expect(result.vmStage.traces[0].description).toContain("Push constant");
  });

  it("stack evolves correctly", () => {
    const result = new Pipeline().run("x = 1 + 2");
    // After LOAD_CONST 1, stack should be [1]
    expect(result.vmStage.traces[0].stackAfter).toEqual([1]);
    // After LOAD_CONST 2, stack should be [1, 2]
    expect(result.vmStage.traces[1].stackAfter).toEqual([1, 2]);
    // After ADD, stack should be [3]
    expect(result.vmStage.traces[2].stackAfter).toEqual([3]);
  });
});
