/**
 * Comprehensive tests for the GenericCompiler — the pluggable AST-to-bytecode
 * compiler framework.
 *
 * These tests verify every aspect of the GenericCompiler:
 *
 * 1. **Plugin registration and dispatch** — handlers are called for their
 *    registered rule names.
 * 2. **Pass-through** — single-child nodes without handlers are transparently
 *    forwarded.
 * 3. **Error handling** — multi-child nodes without handlers raise errors.
 * 4. **Instruction emission** — emit, emitJump, patchJump, currentOffset.
 * 5. **Pool management** — constant and name deduplication.
 * 6. **Scope management** — enter, exit, params, nesting.
 * 7. **Nested compilation** — compileNested saves/restores state.
 * 8. **Top-level compile** — appends HALT, returns CodeObject.
 * 9. **Integration** — a realistic "compile addition" scenario.
 */

import { describe, it, expect } from "vitest";
import {
  GenericCompiler,
  DefaultCompilerScope,
  CompilerError,
  UnhandledRuleError,
  isTokenNode,
} from "../src/generic-compiler.js";
import type {
  ASTNode,
  TokenNode,
  CompileHandler,
} from "../src/generic-compiler.js";
import { OpCode } from "../src/vm-types.js";

// =========================================================================
// Helpers — factory functions for building test ASTs
// =========================================================================

/**
 * Create an ASTNode with the given rule name and children.
 *
 * This is the universal non-terminal node. Every interior node in the
 * tree has a rule name and a list of children.
 */
function astNode(ruleName: string, children: (ASTNode | TokenNode)[]): ASTNode {
  return { ruleName, children };
}

/**
 * Create a TokenNode (leaf) with the given type and value.
 *
 * Token nodes represent actual source code tokens — numbers, identifiers,
 * operators, etc.
 */
function tokenNode(type: string, value: string): TokenNode {
  return { type, value };
}

// =========================================================================
// Plugin registration and dispatch
// =========================================================================

describe("Plugin registration and dispatch", () => {
  it("calls the registered handler for a matching ruleName", () => {
    /**
     * The most basic test: register a handler, compile a node with that
     * rule name, and verify the handler was called.
     */
    const compiler = new GenericCompiler();
    let called = false;

    compiler.registerRule("my_rule", (_c, _node) => {
      called = true;
    });

    compiler.compileNode(astNode("my_rule", []));
    expect(called).toBe(true);
  });

  it("passes the compiler and node to the handler", () => {
    /**
     * Handlers receive both the compiler instance (for emitting instructions)
     * and the AST node (for reading children). Verify both are correct.
     */
    const compiler = new GenericCompiler();
    const testNode = astNode("check_args", [tokenNode("NUM", "42")]);

    compiler.registerRule("check_args", (c, node) => {
      expect(c).toBe(compiler);
      expect(node).toBe(testNode);
    });

    compiler.compileNode(testNode);
  });

  it("dispatches different rules to different handlers", () => {
    /**
     * Multiple rules can be registered, and each gets its own handler.
     * Verify that the correct handler is called for each rule name.
     */
    const compiler = new GenericCompiler();
    const log: string[] = [];

    compiler.registerRule("rule_a", () => log.push("a"));
    compiler.registerRule("rule_b", () => log.push("b"));

    compiler.compileNode(astNode("rule_a", []));
    compiler.compileNode(astNode("rule_b", []));

    expect(log).toEqual(["a", "b"]);
  });

  it("later registration overwrites earlier for the same ruleName", () => {
    /**
     * If a handler is registered twice for the same rule, the second
     * registration wins. This allows plugins to override defaults.
     */
    const compiler = new GenericCompiler();
    let result = "";

    compiler.registerRule("overridable", () => {
      result = "first";
    });
    compiler.registerRule("overridable", () => {
      result = "second";
    });

    compiler.compileNode(astNode("overridable", []));
    expect(result).toBe("second");
  });
});

// =========================================================================
// Pass-through single child nodes
// =========================================================================

describe("Pass-through single child nodes", () => {
  it("passes through a single-child ASTNode to its child", () => {
    /**
     * A node with one child and no handler should delegate to its child.
     * This handles grammar "wrapper" rules like:
     *     expression -> addition -> primary -> number_literal
     */
    const compiler = new GenericCompiler();
    let called = false;

    compiler.registerRule("inner", () => {
      called = true;
    });

    // "wrapper" has no handler, but it has one child with a handler.
    compiler.compileNode(astNode("wrapper", [astNode("inner", [])]));
    expect(called).toBe(true);
  });

  it("chains through multiple levels of single-child wrappers", () => {
    /**
     * Multiple layers of wrapper rules should all pass through until
     * we reach a node with a handler.
     */
    const compiler = new GenericCompiler();
    let called = false;

    compiler.registerRule("leaf", () => {
      called = true;
    });

    // Three levels of wrapping, all without handlers.
    const tree = astNode("level1", [
      astNode("level2", [astNode("level3", [astNode("leaf", [])])]),
    ]);

    compiler.compileNode(tree);
    expect(called).toBe(true);
  });
});

// =========================================================================
// Unhandled multi-child raises error
// =========================================================================

describe("Unhandled multi-child raises error", () => {
  it("throws UnhandledRuleError for multi-child node without handler", () => {
    /**
     * If a node has multiple children and no registered handler, the compiler
     * can't guess what to do — it raises UnhandledRuleError.
     */
    const compiler = new GenericCompiler();
    const node = astNode("unknown_rule", [
      tokenNode("A", "a"),
      tokenNode("B", "b"),
    ]);

    expect(() => compiler.compileNode(node)).toThrow(UnhandledRuleError);
  });

  it("error message includes the rule name", () => {
    /**
     * The error message should tell the developer which rule is missing
     * a handler, so they know what to register.
     */
    const compiler = new GenericCompiler();
    const node = astNode("missing_handler", [
      tokenNode("X", "x"),
      tokenNode("Y", "y"),
    ]);

    expect(() => compiler.compileNode(node)).toThrow(/missing_handler/);
  });

  it("UnhandledRuleError is a subclass of CompilerError", () => {
    /**
     * The error hierarchy lets callers catch broad (CompilerError) or
     * specific (UnhandledRuleError) errors as needed.
     */
    const error = new UnhandledRuleError("test_rule");
    expect(error).toBeInstanceOf(CompilerError);
    expect(error).toBeInstanceOf(Error);
  });
});

// =========================================================================
// Token pass-through (no-op)
// =========================================================================

describe("Token pass-through (no-op)", () => {
  it("compileToken is a no-op by default", () => {
    /**
     * When compileNode encounters a TokenNode, it calls compileToken,
     * which does nothing by default. No instructions should be emitted.
     */
    const compiler = new GenericCompiler();
    const before = compiler.instructions.length;

    compiler.compileNode(tokenNode("NUMBER", "42"));

    expect(compiler.instructions.length).toBe(before);
  });

  it("isTokenNode correctly identifies token nodes", () => {
    /**
     * The type guard should distinguish TokenNodes from ASTNodes.
     */
    expect(isTokenNode(tokenNode("NUM", "1"))).toBe(true);
    expect(isTokenNode(astNode("rule", []))).toBe(false);
  });

  it("single-child wrapper around token passes through silently", () => {
    /**
     * A wrapper node whose only child is a token should pass through
     * to the token, which is a no-op. No error, no instructions.
     */
    const compiler = new GenericCompiler();
    const node = astNode("wrapper", [tokenNode("IDENT", "x")]);

    // Should not throw — pass-through to token, which is a no-op.
    compiler.compileNode(node);
    expect(compiler.instructions.length).toBe(0);
  });
});

// =========================================================================
// Instruction emission
// =========================================================================

describe("Instruction emission", () => {
  it("emit appends an instruction with opcode only", () => {
    /**
     * Instructions like ADD, POP, HALT don't need an operand.
     * The emitted instruction should have just the opcode.
     */
    const compiler = new GenericCompiler();
    compiler.emit(OpCode.ADD);

    expect(compiler.instructions).toHaveLength(1);
    expect(compiler.instructions[0].opcode).toBe(OpCode.ADD);
    expect(compiler.instructions[0].operand).toBeUndefined();
  });

  it("emit appends an instruction with opcode and operand", () => {
    /**
     * Instructions like LOAD_CONST need an operand (the constant pool index).
     */
    const compiler = new GenericCompiler();
    compiler.emit(OpCode.LOAD_CONST, 0);

    expect(compiler.instructions).toHaveLength(1);
    expect(compiler.instructions[0].opcode).toBe(OpCode.LOAD_CONST);
    expect(compiler.instructions[0].operand).toBe(0);
  });

  it("emit returns sequential indices", () => {
    /**
     * Each call to emit returns the index where the instruction was placed.
     * The first instruction is at index 0, the second at 1, etc.
     */
    const compiler = new GenericCompiler();

    const idx0 = compiler.emit(OpCode.LOAD_CONST, 0);
    const idx1 = compiler.emit(OpCode.LOAD_CONST, 1);
    const idx2 = compiler.emit(OpCode.ADD);

    expect(idx0).toBe(0);
    expect(idx1).toBe(1);
    expect(idx2).toBe(2);
  });

  it("currentOffset reflects the number of emitted instructions", () => {
    /**
     * currentOffset is the index where the next instruction would be placed.
     * It starts at 0 and increments with each emit.
     */
    const compiler = new GenericCompiler();

    expect(compiler.currentOffset).toBe(0);
    compiler.emit(OpCode.ADD);
    expect(compiler.currentOffset).toBe(1);
    compiler.emit(OpCode.SUB);
    expect(compiler.currentOffset).toBe(2);
  });

  it("emit supports string operand", () => {
    /**
     * Some instructions may use a string operand (e.g., for debug info).
     */
    const compiler = new GenericCompiler();
    compiler.emit(OpCode.LOAD_CONST, "hello");

    expect(compiler.instructions[0].operand).toBe("hello");
  });

  it("emit supports null operand", () => {
    /**
     * null operands are useful for representing None/nil constants.
     */
    const compiler = new GenericCompiler();
    compiler.emit(OpCode.LOAD_CONST, null);

    expect(compiler.instructions[0].operand).toBe(null);
  });
});

// =========================================================================
// Jump patching
// =========================================================================

describe("Jump patching", () => {
  it("emitJump emits a placeholder with operand 0", () => {
    /**
     * emitJump creates a jump instruction with operand=0 as a placeholder.
     * The real target is filled in later by patchJump.
     */
    const compiler = new GenericCompiler();
    const idx = compiler.emitJump(OpCode.JUMP_IF_FALSE);

    expect(compiler.instructions[idx].opcode).toBe(OpCode.JUMP_IF_FALSE);
    expect(compiler.instructions[idx].operand).toBe(0);
  });

  it("patchJump with explicit target", () => {
    /**
     * patchJump can set a specific target address for the jump.
     */
    const compiler = new GenericCompiler();
    const jumpIdx = compiler.emitJump(OpCode.JUMP);
    compiler.emit(OpCode.ADD); // index 1
    compiler.emit(OpCode.SUB); // index 2

    compiler.patchJump(jumpIdx, 2);

    expect(compiler.instructions[jumpIdx].operand).toBe(2);
  });

  it("patchJump defaults to currentOffset", () => {
    /**
     * When no explicit target is given, patchJump uses currentOffset —
     * meaning "jump to whatever instruction comes next."
     */
    const compiler = new GenericCompiler();
    const jumpIdx = compiler.emitJump(OpCode.JUMP_IF_FALSE);
    compiler.emit(OpCode.ADD); // index 1
    compiler.emit(OpCode.SUB); // index 2

    // currentOffset is now 3 (next instruction would be at index 3).
    compiler.patchJump(jumpIdx);

    expect(compiler.instructions[jumpIdx].operand).toBe(3);
  });

  it("patchJump preserves the original opcode", () => {
    /**
     * Patching should only change the operand, not the opcode.
     * A JUMP_IF_FALSE should remain JUMP_IF_FALSE after patching.
     */
    const compiler = new GenericCompiler();
    const jumpIdx = compiler.emitJump(OpCode.JUMP_IF_FALSE);

    compiler.patchJump(jumpIdx, 10);

    expect(compiler.instructions[jumpIdx].opcode).toBe(OpCode.JUMP_IF_FALSE);
    expect(compiler.instructions[jumpIdx].operand).toBe(10);
  });

  it("emitJump returns the instruction index for later patching", () => {
    /**
     * The return value of emitJump is the index needed by patchJump.
     */
    const compiler = new GenericCompiler();
    compiler.emit(OpCode.LOAD_CONST, 0); // index 0
    const jumpIdx = compiler.emitJump(OpCode.JUMP); // index 1

    expect(jumpIdx).toBe(1);
  });
});

// =========================================================================
// Constant pool
// =========================================================================

describe("Constant pool", () => {
  it("addConstant adds a new value and returns its index", () => {
    /**
     * The first constant gets index 0, the second gets index 1, etc.
     */
    const compiler = new GenericCompiler();

    const idx = compiler.addConstant(42);

    expect(idx).toBe(0);
    expect(compiler.constants).toEqual([42]);
  });

  it("addConstant deduplicates identical values", () => {
    /**
     * Adding the same value twice should return the same index and not
     * create a duplicate entry in the pool.
     */
    const compiler = new GenericCompiler();

    const idx1 = compiler.addConstant(42);
    const idx2 = compiler.addConstant(42);

    expect(idx1).toBe(0);
    expect(idx2).toBe(0);
    expect(compiler.constants).toEqual([42]);
  });

  it("addConstant handles multiple distinct values", () => {
    /**
     * Different values get different indices, stored in order.
     */
    const compiler = new GenericCompiler();

    const i0 = compiler.addConstant(1);
    const i1 = compiler.addConstant("hello");
    const i2 = compiler.addConstant(null);
    const i3 = compiler.addConstant(2);

    expect(i0).toBe(0);
    expect(i1).toBe(1);
    expect(i2).toBe(2);
    expect(i3).toBe(3);
    expect(compiler.constants).toEqual([1, "hello", null, 2]);
  });

  it("addConstant distinguishes numbers from strings", () => {
    /**
     * The number 0 and the string "0" are different constants and should
     * not be deduplicated against each other.
     */
    const compiler = new GenericCompiler();

    const i0 = compiler.addConstant(0);
    const i1 = compiler.addConstant("0");

    expect(i0).toBe(0);
    expect(i1).toBe(1);
    expect(compiler.constants).toEqual([0, "0"]);
  });
});

// =========================================================================
// Name pool
// =========================================================================

describe("Name pool", () => {
  it("addName adds a new name and returns its index", () => {
    /**
     * Names work just like constants — first entry is index 0.
     */
    const compiler = new GenericCompiler();

    const idx = compiler.addName("x");

    expect(idx).toBe(0);
    expect(compiler.names).toEqual(["x"]);
  });

  it("addName deduplicates identical names", () => {
    /**
     * The same variable name used twice should reference the same index.
     */
    const compiler = new GenericCompiler();

    const idx1 = compiler.addName("x");
    const idx2 = compiler.addName("x");

    expect(idx1).toBe(0);
    expect(idx2).toBe(0);
    expect(compiler.names).toEqual(["x"]);
  });

  it("addName handles multiple distinct names", () => {
    /**
     * Different names get different indices.
     */
    const compiler = new GenericCompiler();

    const i0 = compiler.addName("x");
    const i1 = compiler.addName("y");
    const i2 = compiler.addName("z");

    expect(i0).toBe(0);
    expect(i1).toBe(1);
    expect(i2).toBe(2);
    expect(compiler.names).toEqual(["x", "y", "z"]);
  });
});

// =========================================================================
// Scope management
// =========================================================================

describe("Scope management", () => {
  it("enterScope creates a new scope and sets it as current", () => {
    /**
     * After enterScope, compiler.scope should point to the new scope.
     */
    const compiler = new GenericCompiler();
    expect(compiler.scope).toBeNull();

    const scope = compiler.enterScope();

    expect(compiler.scope).toBe(scope);
    expect(scope.parent).toBeNull();
  });

  it("enterScope with params pre-assigns local slots", () => {
    /**
     * Parameters should be assigned to local slots starting from 0.
     */
    const compiler = new GenericCompiler();

    const scope = compiler.enterScope(["x", "y", "z"]);

    expect(scope.getLocal("x")).toBe(0);
    expect(scope.getLocal("y")).toBe(1);
    expect(scope.getLocal("z")).toBe(2);
    expect(scope.numLocals).toBe(3);
  });

  it("exitScope restores the parent scope", () => {
    /**
     * After exiting a scope, compiler.scope should point to the parent.
     */
    const compiler = new GenericCompiler();
    compiler.enterScope();
    const inner = compiler.enterScope();

    const exited = compiler.exitScope();

    expect(exited).toBe(inner);
    expect(compiler.scope).not.toBeNull();
    expect(compiler.scope!.parent).toBeNull(); // back to the outer scope
  });

  it("nested scopes link via parent pointers", () => {
    /**
     * Each scope's parent points to the enclosing scope, forming a chain.
     */
    const compiler = new GenericCompiler();
    const outer = compiler.enterScope(["a"]);
    const inner = compiler.enterScope(["b"]);

    expect(inner.parent).toBe(outer);
    expect(outer.parent).toBeNull();
  });

  it("exitScope throws CompilerError when not in a scope", () => {
    /**
     * Calling exitScope when scope is null should throw, not silently
     * return null.
     */
    const compiler = new GenericCompiler();

    expect(() => compiler.exitScope()).toThrow(CompilerError);
  });

  it("exitScope returns the exited scope for inspection", () => {
    /**
     * The returned scope can be inspected for numLocals, etc.
     */
    const compiler = new GenericCompiler();
    const scope = compiler.enterScope(["x", "y"]);
    scope.addLocal("temp");

    const exited = compiler.exitScope();

    expect(exited.numLocals).toBe(3);
    expect(exited.getLocal("x")).toBe(0);
    expect(exited.getLocal("temp")).toBe(2);
  });
});

// =========================================================================
// CompilerScope class (DefaultCompilerScope)
// =========================================================================

describe("DefaultCompilerScope", () => {
  it("addLocal assigns consecutive slot indices", () => {
    /**
     * Each new local variable gets the next available slot index.
     */
    const scope = new DefaultCompilerScope(null);

    expect(scope.addLocal("a")).toBe(0);
    expect(scope.addLocal("b")).toBe(1);
    expect(scope.addLocal("c")).toBe(2);
  });

  it("addLocal deduplicates — same name returns same slot", () => {
    /**
     * Adding a name that already exists should return the existing slot
     * index, not create a new one.
     */
    const scope = new DefaultCompilerScope(null);

    const i1 = scope.addLocal("x");
    const i2 = scope.addLocal("x");

    expect(i1).toBe(0);
    expect(i2).toBe(0);
    expect(scope.numLocals).toBe(1);
  });

  it("getLocal returns the slot index for known variables", () => {
    /**
     * getLocal should find variables that were added via addLocal or
     * pre-assigned as parameters.
     */
    const scope = new DefaultCompilerScope(null, ["param"]);
    scope.addLocal("local");

    expect(scope.getLocal("param")).toBe(0);
    expect(scope.getLocal("local")).toBe(1);
  });

  it("getLocal returns undefined for unknown variables", () => {
    /**
     * Variables not in this scope return undefined (not an error).
     * The caller (language plugin) decides what to do — maybe check
     * the parent scope, maybe throw an error.
     */
    const scope = new DefaultCompilerScope(null);

    expect(scope.getLocal("nonexistent")).toBeUndefined();
  });

  it("numLocals reflects the total count", () => {
    /**
     * numLocals includes both parameters and explicitly added locals.
     */
    const scope = new DefaultCompilerScope(null, ["a", "b"]);
    scope.addLocal("c");

    expect(scope.numLocals).toBe(3);
  });

  it("numLocals starts at 0 for empty scope", () => {
    /**
     * An empty scope has no locals.
     */
    const scope = new DefaultCompilerScope(null);
    expect(scope.numLocals).toBe(0);
  });

  it("params are pre-assigned before addLocal", () => {
    /**
     * Parameters get the lowest slots (0, 1, ...), and addLocal picks
     * up from where they left off.
     */
    const scope = new DefaultCompilerScope(null, ["x", "y"]);
    const tempSlot = scope.addLocal("temp");

    expect(scope.getLocal("x")).toBe(0);
    expect(scope.getLocal("y")).toBe(1);
    expect(tempSlot).toBe(2);
  });
});

// =========================================================================
// Nested code object compilation
// =========================================================================

describe("Nested code object compilation", () => {
  it("compileNested returns a separate CodeObject", () => {
    /**
     * Nested compilation should produce a self-contained CodeObject
     * with its own instructions, constants, and names.
     */
    const compiler = new GenericCompiler();

    compiler.registerRule("body", (c, _node) => {
      c.emit(OpCode.LOAD_CONST, c.addConstant(99));
      c.addName("local_var");
    });

    const nested = compiler.compileNested(astNode("body", []));

    expect(nested.instructions).toHaveLength(1);
    expect(nested.instructions[0].opcode).toBe(OpCode.LOAD_CONST);
    expect(nested.constants).toEqual([99]);
    expect(nested.names).toEqual(["local_var"]);
  });

  it("compileNested restores outer state", () => {
    /**
     * After compileNested returns, the compiler's state (instructions,
     * constants, names) should be back to what it was before the call.
     */
    const compiler = new GenericCompiler();

    // Set up some outer state first.
    compiler.emit(OpCode.LOAD_CONST, compiler.addConstant(1));
    compiler.addName("outer_var");

    const outerInstrCount = compiler.instructions.length;
    const outerConstCount = compiler.constants.length;
    const outerNameCount = compiler.names.length;

    compiler.registerRule("inner_body", (c, _node) => {
      c.emit(OpCode.ADD);
      c.addConstant(999);
      c.addName("inner_var");
    });

    compiler.compileNested(astNode("inner_body", []));

    // Outer state should be restored.
    expect(compiler.instructions.length).toBe(outerInstrCount);
    expect(compiler.constants.length).toBe(outerConstCount);
    expect(compiler.names.length).toBe(outerNameCount);
    expect(compiler.constants).toEqual([1]);
    expect(compiler.names).toEqual(["outer_var"]);
  });

  it("compileNested does not pollute outer instructions", () => {
    /**
     * Instructions emitted during nested compilation should NOT appear
     * in the outer instruction list.
     */
    const compiler = new GenericCompiler();
    compiler.emit(OpCode.LOAD_CONST, 0); // outer instruction

    compiler.registerRule("nested", (c, _node) => {
      c.emit(OpCode.ADD);
      c.emit(OpCode.SUB);
      c.emit(OpCode.MUL);
    });

    compiler.compileNested(astNode("nested", []));

    // Outer should still have just the one instruction.
    expect(compiler.instructions).toHaveLength(1);
    expect(compiler.instructions[0].opcode).toBe(OpCode.LOAD_CONST);
  });
});

// =========================================================================
// Top-level compile
// =========================================================================

describe("Top-level compile", () => {
  it("appends HALT instruction at the end", () => {
    /**
     * compile() should automatically append a HALT instruction so the
     * VM knows to stop.
     */
    const compiler = new GenericCompiler();
    compiler.registerRule("program", (c, _node) => {
      c.emit(OpCode.LOAD_CONST, c.addConstant(42));
    });

    const code = compiler.compile(astNode("program", []));

    const last = code.instructions[code.instructions.length - 1];
    expect(last.opcode).toBe(OpCode.HALT);
  });

  it("supports custom halt opcode", () => {
    /**
     * Some backends may use a different halt instruction. The second
     * parameter to compile() lets you specify it.
     */
    const compiler = new GenericCompiler();
    compiler.registerRule("prog", () => {});

    const customHalt = 0xfe; // arbitrary custom opcode
    const code = compiler.compile(astNode("prog", []), customHalt);

    const last = code.instructions[code.instructions.length - 1];
    expect(last.opcode).toBe(customHalt);
  });

  it("returns a CodeObject with instructions, constants, and names", () => {
    /**
     * The returned CodeObject should contain all accumulated state.
     */
    const compiler = new GenericCompiler();
    compiler.registerRule("root", (c, _node) => {
      c.emit(OpCode.LOAD_CONST, c.addConstant(10));
      c.emit(OpCode.STORE_NAME, c.addName("x"));
    });

    const code = compiler.compile(astNode("root", []));

    expect(code).toHaveProperty("instructions");
    expect(code).toHaveProperty("constants");
    expect(code).toHaveProperty("names");
    expect(code.instructions.length).toBe(3); // LOAD_CONST, STORE_NAME, HALT
    expect(code.constants).toEqual([10]);
    expect(code.names).toEqual(["x"]);
  });

  it("empty program produces just HALT", () => {
    /**
     * A program with no rules that fire should still produce a valid
     * CodeObject with just a HALT instruction.
     */
    const compiler = new GenericCompiler();
    compiler.registerRule("empty", () => {});

    const code = compiler.compile(astNode("empty", []));

    expect(code.instructions).toHaveLength(1);
    expect(code.instructions[0].opcode).toBe(OpCode.HALT);
  });
});

// =========================================================================
// Integration test: compile addition expression
// =========================================================================

describe("Integration: compile addition expression", () => {
  /**
   * A realistic scenario: compile the expression ``1 + 2`` using the
   * GenericCompiler with language-specific handlers.
   *
   * The AST for ``1 + 2`` looks like:
   *
   *     ASTNode("expression", [
   *       ASTNode("addition", [
   *         ASTNode("number", [TokenNode("NUMBER", "1")]),
   *         TokenNode("PLUS", "+"),
   *         ASTNode("number", [TokenNode("NUMBER", "2")]),
   *       ]),
   *     ])
   *
   * We register handlers for "addition" and "number". The "expression"
   * wrapper passes through to "addition" because it has one child.
   */

  it("compiles 1 + 2 to LOAD_CONST, LOAD_CONST, ADD, HALT", () => {
    const compiler = new GenericCompiler();

    // Handler for number literals: extract the value from the token child.
    compiler.registerRule("number", (c, node) => {
      const token = node.children[0] as TokenNode;
      const value = Number(token.value);
      const index = c.addConstant(value);
      c.emit(OpCode.LOAD_CONST, index);
    });

    // Handler for addition: compile left, compile right, emit ADD.
    compiler.registerRule("addition", (c, node) => {
      c.compileNode(node.children[0]); // left operand (number)
      c.compileNode(node.children[2]); // right operand (number), skip PLUS token
      c.emit(OpCode.ADD);
    });

    // Build the AST for "1 + 2".
    const ast = astNode("expression", [
      astNode("addition", [
        astNode("number", [tokenNode("NUMBER", "1")]),
        tokenNode("PLUS", "+"),
        astNode("number", [tokenNode("NUMBER", "2")]),
      ]),
    ]);

    const code = compiler.compile(ast);

    // Verify the instruction sequence.
    expect(code.instructions.map((i) => i.opcode)).toEqual([
      OpCode.LOAD_CONST, // push 1
      OpCode.LOAD_CONST, // push 2
      OpCode.ADD, // pop both, push 3
      OpCode.HALT, // stop
    ]);

    // Verify the constant pool.
    expect(code.constants).toEqual([1, 2]);

    // Verify operands.
    expect(code.instructions[0].operand).toBe(0); // constants[0] = 1
    expect(code.instructions[1].operand).toBe(1); // constants[1] = 2
  });

  it("compiles nested 1 + 2 + 3 with left-associative grouping", () => {
    /**
     * ``1 + 2 + 3`` is parsed as ``(1 + 2) + 3``, producing a nested tree:
     *
     *     addition
     *     /    |    \
     *   addition  "+"  number(3)
     *   /  |  \
     * number(1) "+" number(2)
     */
    const compiler = new GenericCompiler();

    compiler.registerRule("number", (c, node) => {
      const token = node.children[0] as TokenNode;
      c.emit(OpCode.LOAD_CONST, c.addConstant(Number(token.value)));
    });

    compiler.registerRule("addition", (c, node) => {
      c.compileNode(node.children[0]);
      c.compileNode(node.children[2]);
      c.emit(OpCode.ADD);
    });

    const ast = astNode("addition", [
      astNode("addition", [
        astNode("number", [tokenNode("NUMBER", "1")]),
        tokenNode("PLUS", "+"),
        astNode("number", [tokenNode("NUMBER", "2")]),
      ]),
      tokenNode("PLUS", "+"),
      astNode("number", [tokenNode("NUMBER", "3")]),
    ]);

    const code = compiler.compile(ast);

    expect(code.instructions.map((i) => i.opcode)).toEqual([
      OpCode.LOAD_CONST, // 1
      OpCode.LOAD_CONST, // 2
      OpCode.ADD, // 1 + 2
      OpCode.LOAD_CONST, // 3
      OpCode.ADD, // (1+2) + 3
      OpCode.HALT,
    ]);
    expect(code.constants).toEqual([1, 2, 3]);
  });

  it("compiles variable assignment and lookup", () => {
    /**
     * ``x = 42`` followed by ``x`` — tests STORE_NAME and LOAD_NAME.
     */
    const compiler = new GenericCompiler();

    compiler.registerRule("number", (c, node) => {
      const token = node.children[0] as TokenNode;
      c.emit(OpCode.LOAD_CONST, c.addConstant(Number(token.value)));
    });

    compiler.registerRule("assignment", (c, node) => {
      const nameToken = node.children[0] as TokenNode;
      c.compileNode(node.children[2]); // compile the value
      c.emit(OpCode.STORE_NAME, c.addName(nameToken.value));
    });

    compiler.registerRule("name_ref", (c, node) => {
      const token = node.children[0] as TokenNode;
      c.emit(OpCode.LOAD_NAME, c.addName(token.value));
    });

    compiler.registerRule("program", (c, node) => {
      for (const child of node.children) {
        c.compileNode(child);
      }
    });

    const ast = astNode("program", [
      astNode("assignment", [
        tokenNode("IDENT", "x"),
        tokenNode("EQUALS", "="),
        astNode("number", [tokenNode("NUMBER", "42")]),
      ]),
      astNode("name_ref", [tokenNode("IDENT", "x")]),
    ]);

    const code = compiler.compile(ast);

    expect(code.instructions.map((i) => i.opcode)).toEqual([
      OpCode.LOAD_CONST, // 42
      OpCode.STORE_NAME, // x
      OpCode.LOAD_NAME, // x
      OpCode.HALT,
    ]);
    expect(code.constants).toEqual([42]);
    expect(code.names).toEqual(["x"]);
  });
});
