/**
 * Tests for the ECMAScript 3 (1999) Parser (TypeScript).
 *
 * These tests verify that the grammar-driven parser correctly parses ES3
 * source code when loaded with the `es3.grammar` file.
 *
 * ES3 adds over ES1:
 * - try/catch/finally/throw statements
 * - === and !== in equality expressions
 * - `instanceof` in relational expressions
 * - REGEX as a primary expression
 */

import { describe, it, expect } from "vitest";
import { parseEs3 } from "../src/parser.js";
import { isASTNode, findNodes } from "@coding-adventures/parser";
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
// ES1 Features Still Present
// ============================================================================

describe("ES1 features still present", () => {
  it("parses var declaration", () => {
    const ast = parseEs3("var x = 1 + 2;");
    expect(ast.ruleName).toBe("program");
    const varStmts = findNodes(ast, "variable_statement");
    expect(varStmts).toHaveLength(1);
  });

  it("parses function declaration", () => {
    const ast = parseEs3("function add(a, b) { return a + b; }");
    const funcDecls = findNodes(ast, "function_declaration");
    expect(funcDecls).toHaveLength(1);
  });

  it("parses if/else", () => {
    const ast = parseEs3("if (x) { y; } else { z; }");
    const ifStmts = findNodes(ast, "if_statement");
    expect(ifStmts).toHaveLength(1);
  });

  it("parses for loop", () => {
    const ast = parseEs3("for (var i = 0; i < 10; i++) { x; }");
    const forStmts = findNodes(ast, "for_statement");
    expect(forStmts).toHaveLength(1);
  });

  it("parses switch statement", () => {
    const ast = parseEs3("switch (x) { case 1: y; break; default: z; }");
    const switchStmts = findNodes(ast, "switch_statement");
    expect(switchStmts).toHaveLength(1);
  });
});

// ============================================================================
// Try/Catch/Finally (NEW in ES3)
// ============================================================================

describe("try/catch/finally (new in ES3)", () => {
  it("parses try/catch", () => {
    const ast = parseEs3("try { x; } catch (e) { y; }");
    const tryStmts = findNodes(ast, "try_statement");
    expect(tryStmts).toHaveLength(1);

    const catchClauses = findNodes(ast, "catch_clause");
    expect(catchClauses).toHaveLength(1);
  });

  it("parses try/finally", () => {
    const ast = parseEs3("try { x; } finally { z; }");
    const tryStmts = findNodes(ast, "try_statement");
    expect(tryStmts).toHaveLength(1);

    const finallyClauses = findNodes(ast, "finally_clause");
    expect(finallyClauses).toHaveLength(1);
  });

  it("parses try/catch/finally", () => {
    const ast = parseEs3("try { x; } catch (e) { y; } finally { z; }");
    const tryStmts = findNodes(ast, "try_statement");
    expect(tryStmts).toHaveLength(1);

    const catchClauses = findNodes(ast, "catch_clause");
    expect(catchClauses).toHaveLength(1);

    const finallyClauses = findNodes(ast, "finally_clause");
    expect(finallyClauses).toHaveLength(1);
  });
});

// ============================================================================
// Throw Statement (NEW in ES3)
// ============================================================================

describe("throw statement (new in ES3)", () => {
  it("parses throw statement", () => {
    const ast = parseEs3('throw "error";');
    const throwStmts = findNodes(ast, "throw_statement");
    expect(throwStmts).toHaveLength(1);
  });

  it("parses throw with new Error", () => {
    const ast = parseEs3('throw new Error("oops");');
    const throwStmts = findNodes(ast, "throw_statement");
    expect(throwStmts).toHaveLength(1);
  });
});

// ============================================================================
// Strict Equality in Expressions (NEW in ES3)
// ============================================================================

describe("strict equality in expressions (new in ES3)", () => {
  it("parses expression with ===", () => {
    const ast = parseEs3("x === 1;");
    const eqExprs = findNodes(ast, "equality_expression");
    expect(eqExprs.length).toBeGreaterThanOrEqual(1);

    // Find the STRICT_EQUALS token
    const tokens = findTokens(ast);
    const strictEq = tokens.find((t) => t.type === "STRICT_EQUALS");
    expect(strictEq).toBeDefined();
  });

  it("parses expression with !==", () => {
    const ast = parseEs3("x !== 1;");
    const tokens = findTokens(ast);
    const strictNeq = tokens.find((t) => t.type === "STRICT_NOT_EQUALS");
    expect(strictNeq).toBeDefined();
  });
});

// ============================================================================
// instanceof (NEW in ES3)
// ============================================================================

describe("instanceof (new in ES3)", () => {
  it("parses instanceof expression", () => {
    const ast = parseEs3("x instanceof Array;");
    const relExprs = findNodes(ast, "relational_expression");
    expect(relExprs.length).toBeGreaterThanOrEqual(1);

    const tokens = findTokens(ast);
    const instanceofToken = tokens.find((t) => t.value === "instanceof");
    expect(instanceofToken).toBeDefined();
  });
});

// ============================================================================
// Expression Precedence
// ============================================================================

describe("expression precedence", () => {
  it("parses additive expression", () => {
    const ast = parseEs3("1 + 2;");
    const addExprs = findNodes(ast, "additive_expression");
    expect(addExprs.length).toBeGreaterThanOrEqual(1);
  });

  it("parses logical operators", () => {
    const ast = parseEs3("a && b || c;");
    const orExprs = findNodes(ast, "logical_or_expression");
    expect(orExprs.length).toBeGreaterThanOrEqual(1);
  });

  it("parses ternary conditional", () => {
    const ast = parseEs3("a ? b : c;");
    const condExprs = findNodes(ast, "conditional_expression");
    expect(condExprs.length).toBeGreaterThanOrEqual(1);
  });
});

// ============================================================================
// Literals
// ============================================================================

describe("literals", () => {
  it("parses array literal", () => {
    const ast = parseEs3("var x = [1, 2, 3];");
    const arrayLits = findNodes(ast, "array_literal");
    expect(arrayLits).toHaveLength(1);
  });

  it("parses object literal", () => {
    const ast = parseEs3('var x = { a: 1, b: "two" };');
    const objLits = findNodes(ast, "object_literal");
    expect(objLits).toHaveLength(1);
  });
});

// ============================================================================
// Edge Cases
// ============================================================================

describe("edge cases", () => {
  it("parses empty program", () => {
    const ast = parseEs3("");
    expect(ast.ruleName).toBe("program");
  });

  it("parses empty block", () => {
    const ast = parseEs3("{ }");
    const blocks = findNodes(ast, "block");
    expect(blocks).toHaveLength(1);
  });

  it("parses multiple statements", () => {
    const ast = parseEs3("var x = 1; try { x; } catch (e) { y; }");
    const varStmts = findNodes(ast, "variable_statement");
    const tryStmts = findNodes(ast, "try_statement");
    expect(varStmts).toHaveLength(1);
    expect(tryStmts).toHaveLength(1);
  });
});
