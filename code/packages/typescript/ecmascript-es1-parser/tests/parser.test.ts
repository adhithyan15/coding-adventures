/**
 * Tests for the ECMAScript 1 (1997) Parser (TypeScript).
 *
 * These tests verify that the grammar-driven parser correctly parses ES1
 * source code when loaded with the `es1.grammar` file.
 *
 * ES1 grammar features:
 * - `var` declarations (no let/const)
 * - Function declarations and expressions
 * - All 14 statement types (no try/catch)
 * - Full expression precedence chain
 * - Object and array literals
 */

import { describe, it, expect } from "vitest";
import { parseEs1 } from "../src/parser.js";
import { isASTNode, findNodes, collectTokens } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import type { Token } from "@coding-adventures/lexer";

/**
 * Helper: collect all tokens from an AST node (leaf traversal).
 */
function findTokens(node: ASTNode): Token[] {
  const tokens: Token[] = [];
  for (const child of node.children) {
    if (isASTNode(child)) {
      tokens.push(...findTokens(child));
    } else {
      tokens.push(child as Token);
    }
  }
  return tokens;
}

// ============================================================================
// Variable Declarations
// ============================================================================

describe("variable declarations", () => {
  it("parses var x = 1 + 2;", () => {
    const ast = parseEs1("var x = 1 + 2;");
    expect(ast.ruleName).toBe("program");

    const varStmts = findNodes(ast, "variable_statement");
    expect(varStmts).toHaveLength(1);

    const tokens = findTokens(varStmts[0]);
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords[0].value).toBe("var");

    const names = tokens.filter((t) => t.type === "NAME");
    expect(names[0].value).toBe("x");
  });

  it("parses var with multiple declarators: var a = 1, b = 2;", () => {
    const ast = parseEs1("var a = 1, b = 2;");
    const varDecls = findNodes(ast, "variable_declaration");
    expect(varDecls).toHaveLength(2);
  });

  it("parses var without initializer: var x;", () => {
    const ast = parseEs1("var x;");
    const varStmts = findNodes(ast, "variable_statement");
    expect(varStmts).toHaveLength(1);
  });
});

// ============================================================================
// Function Declarations
// ============================================================================

describe("function declarations", () => {
  it("parses function declaration", () => {
    const ast = parseEs1("function add(a, b) { return a + b; }");
    expect(ast.ruleName).toBe("program");

    const funcDecls = findNodes(ast, "function_declaration");
    expect(funcDecls).toHaveLength(1);
  });

  it("parses function with no parameters", () => {
    const ast = parseEs1("function noop() { }");
    const funcDecls = findNodes(ast, "function_declaration");
    expect(funcDecls).toHaveLength(1);
  });
});

// ============================================================================
// Expression Statements
// ============================================================================

describe("expression statements", () => {
  it("parses 1 + 2;", () => {
    const ast = parseEs1("1 + 2;");
    expect(ast.ruleName).toBe("program");

    const exprStmts = findNodes(ast, "expression_statement");
    expect(exprStmts).toHaveLength(1);
  });

  it("parses function call: foo();", () => {
    const ast = parseEs1("foo();");
    const exprStmts = findNodes(ast, "expression_statement");
    expect(exprStmts).toHaveLength(1);

    const callExprs = findNodes(ast, "call_expression");
    expect(callExprs).toHaveLength(1);
  });
});

// ============================================================================
// Control Flow Statements
// ============================================================================

describe("control flow statements", () => {
  it("parses if statement", () => {
    const ast = parseEs1("if (x) { y; }");
    const ifStmts = findNodes(ast, "if_statement");
    expect(ifStmts).toHaveLength(1);
  });

  it("parses if/else statement", () => {
    const ast = parseEs1("if (x) { y; } else { z; }");
    const ifStmts = findNodes(ast, "if_statement");
    expect(ifStmts).toHaveLength(1);
  });

  it("parses while loop", () => {
    const ast = parseEs1("while (x) { y; }");
    const whileStmts = findNodes(ast, "while_statement");
    expect(whileStmts).toHaveLength(1);
  });

  it("parses do-while loop", () => {
    const ast = parseEs1("do { x; } while (y);");
    const doWhileStmts = findNodes(ast, "do_while_statement");
    expect(doWhileStmts).toHaveLength(1);
  });

  it("parses for loop", () => {
    const ast = parseEs1("for (var i = 0; i < 10; i++) { x; }");
    const forStmts = findNodes(ast, "for_statement");
    expect(forStmts).toHaveLength(1);
  });

  it("parses for-in loop", () => {
    const ast = parseEs1("for (var k in obj) { x; }");
    const forInStmts = findNodes(ast, "for_in_statement");
    expect(forInStmts).toHaveLength(1);
  });

  it("parses switch statement", () => {
    const ast = parseEs1("switch (x) { case 1: y; break; default: z; }");
    const switchStmts = findNodes(ast, "switch_statement");
    expect(switchStmts).toHaveLength(1);
  });

  it("parses return statement", () => {
    const ast = parseEs1("function f() { return 42; }");
    const returnStmts = findNodes(ast, "return_statement");
    expect(returnStmts).toHaveLength(1);
  });

  it("parses break and continue", () => {
    const ast = parseEs1("while (true) { break; continue; }");
    const breakStmts = findNodes(ast, "break_statement");
    const continueStmts = findNodes(ast, "continue_statement");
    expect(breakStmts).toHaveLength(1);
    expect(continueStmts).toHaveLength(1);
  });
});

// ============================================================================
// Operator Precedence
// ============================================================================

describe("operator precedence", () => {
  it("parses additive expression with correct structure", () => {
    const ast = parseEs1("1 + 2;");
    const addExprs = findNodes(ast, "additive_expression");
    expect(addExprs.length).toBeGreaterThanOrEqual(1);
  });

  it("parses multiplicative inside additive", () => {
    const ast = parseEs1("1 + 2 * 3;");
    const mulExprs = findNodes(ast, "multiplicative_expression");
    expect(mulExprs.length).toBeGreaterThanOrEqual(1);
  });

  it("parses logical operators", () => {
    const ast = parseEs1("a && b || c;");
    const orExprs = findNodes(ast, "logical_or_expression");
    expect(orExprs.length).toBeGreaterThanOrEqual(1);
    const andExprs = findNodes(ast, "logical_and_expression");
    expect(andExprs.length).toBeGreaterThanOrEqual(1);
  });

  it("parses ternary conditional", () => {
    const ast = parseEs1("a ? b : c;");
    const condExprs = findNodes(ast, "conditional_expression");
    expect(condExprs.length).toBeGreaterThanOrEqual(1);
  });
});

// ============================================================================
// Literals
// ============================================================================

describe("literals", () => {
  it("parses array literal", () => {
    const ast = parseEs1("var x = [1, 2, 3];");
    const arrayLits = findNodes(ast, "array_literal");
    expect(arrayLits).toHaveLength(1);
  });

  it("parses object literal", () => {
    const ast = parseEs1('var x = { a: 1, b: "two" };');
    const objLits = findNodes(ast, "object_literal");
    expect(objLits).toHaveLength(1);
  });

  it("parses function expression", () => {
    const ast = parseEs1("var f = function(x) { return x; };");
    const funcExprs = findNodes(ast, "function_expression");
    expect(funcExprs).toHaveLength(1);
  });
});

// ============================================================================
// Multiple Statements
// ============================================================================

describe("multiple statements", () => {
  it("parses two variable declarations", () => {
    const ast = parseEs1("var x = 1; var y = 2;");
    const varStmts = findNodes(ast, "variable_statement");
    expect(varStmts).toHaveLength(2);
  });

  it("parses mixed statements", () => {
    const ast = parseEs1("var x = 1; foo(); if (x) { bar(); }");
    const varStmts = findNodes(ast, "variable_statement");
    const exprStmts = findNodes(ast, "expression_statement");
    const ifStmts = findNodes(ast, "if_statement");
    expect(varStmts).toHaveLength(1);
    expect(exprStmts.length).toBeGreaterThanOrEqual(1);
    expect(ifStmts).toHaveLength(1);
  });
});

// ============================================================================
// Empty Program
// ============================================================================

describe("edge cases", () => {
  it("parses empty program", () => {
    const ast = parseEs1("");
    expect(ast.ruleName).toBe("program");
  });

  it("parses empty block", () => {
    const ast = parseEs1("{ }");
    const blocks = findNodes(ast, "block");
    expect(blocks).toHaveLength(1);
  });

  it("parses empty statement (lone semicolon)", () => {
    const ast = parseEs1(";");
    const emptyStmts = findNodes(ast, "empty_statement");
    expect(emptyStmts).toHaveLength(1);
  });
});
