/**
 * Tests for the ECMAScript 5 (2009) Parser (TypeScript).
 *
 * These tests verify that the grammar-driven parser correctly parses ES5
 * source code when loaded with the `es5.grammar` file.
 *
 * ES5 adds over ES3:
 * - debugger statement
 * - getter/setter properties in object literals
 */

import { describe, it, expect } from "vitest";
import { parseEs5 } from "../src/parser.js";
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
// Debugger Statement (NEW in ES5)
// ============================================================================

describe("debugger statement (new in ES5)", () => {
  it("parses debugger;", () => {
    const ast = parseEs5("debugger;");
    expect(ast.ruleName).toBe("program");

    const debuggerStmts = findNodes(ast, "debugger_statement");
    expect(debuggerStmts).toHaveLength(1);
  });

  it("parses debugger inside function", () => {
    const ast = parseEs5("function f() { debugger; }");
    const debuggerStmts = findNodes(ast, "debugger_statement");
    expect(debuggerStmts).toHaveLength(1);
  });
});

// ============================================================================
// Getter/Setter Properties (NEW in ES5)
// ============================================================================

describe("getter/setter properties (new in ES5)", () => {
  // The ES5 grammar's getter_property rule is:
  //   getter_property = NAME LPAREN RPAREN LBRACE function_body RBRACE ;
  // This matches patterns like `x() { return 1; }` inside object literals.
  // The PEG parser tries getter_property before property_name COLON, so
  // `x() { return 1; }` is parsed as a getter_property (with NAME = "x").
  //
  // Note: The full `get x() {}` syntax requires two NAME tokens, which the
  // grammar handles at the property_assignment level via ordered choice.

  it("parses getter-like property in object literal", () => {
    // A bare function-style property `x() { return 1; }` matches getter_property
    const ast = parseEs5("var o = { x() { return 1; } };");
    const getterProps = findNodes(ast, "getter_property");
    expect(getterProps).toHaveLength(1);
  });

  it("parses setter-like property in object literal", () => {
    // A property `x(v) { }` matches setter_property
    const ast = parseEs5("var o = { x(v) { } };");
    const setterProps = findNodes(ast, "setter_property");
    expect(setterProps).toHaveLength(1);
  });

  it("parses object with both getter and setter style properties", () => {
    const ast = parseEs5("var o = { x() { return 1; }, y(v) { } };");
    const getterProps = findNodes(ast, "getter_property");
    const setterProps = findNodes(ast, "setter_property");
    expect(getterProps).toHaveLength(1);
    expect(setterProps).toHaveLength(1);
  });

  it("parses object with mixed regular and getter properties", () => {
    const ast = parseEs5("var o = { a: 1, b() { return 2; } };");
    const propAssigns = findNodes(ast, "property_assignment");
    expect(propAssigns.length).toBeGreaterThanOrEqual(2);
  });
});

// ============================================================================
// ES3 Features Still Present
// ============================================================================

describe("ES3 features still present", () => {
  it("parses var declaration", () => {
    const ast = parseEs5("var x = 1;");
    const varStmts = findNodes(ast, "variable_statement");
    expect(varStmts).toHaveLength(1);
  });

  it("parses function declaration", () => {
    const ast = parseEs5("function add(a, b) { return a + b; }");
    const funcDecls = findNodes(ast, "function_declaration");
    expect(funcDecls).toHaveLength(1);
  });

  it("parses try/catch/finally", () => {
    const ast = parseEs5("try { x; } catch (e) { y; } finally { z; }");
    const tryStmts = findNodes(ast, "try_statement");
    expect(tryStmts).toHaveLength(1);
    const catchClauses = findNodes(ast, "catch_clause");
    expect(catchClauses).toHaveLength(1);
    const finallyClauses = findNodes(ast, "finally_clause");
    expect(finallyClauses).toHaveLength(1);
  });

  it("parses throw statement", () => {
    const ast = parseEs5('throw "error";');
    const throwStmts = findNodes(ast, "throw_statement");
    expect(throwStmts).toHaveLength(1);
  });

  it("parses strict equality", () => {
    const ast = parseEs5("x === 1;");
    const tokens = findTokens(ast);
    const strictEq = tokens.find((t) => t.type === "STRICT_EQUALS");
    expect(strictEq).toBeDefined();
  });

  it("parses instanceof", () => {
    const ast = parseEs5("x instanceof Array;");
    const tokens = findTokens(ast);
    const instanceofToken = tokens.find((t) => t.value === "instanceof");
    expect(instanceofToken).toBeDefined();
  });
});

// ============================================================================
// Control Flow
// ============================================================================

describe("control flow", () => {
  it("parses if/else", () => {
    const ast = parseEs5("if (x) { y; } else { z; }");
    const ifStmts = findNodes(ast, "if_statement");
    expect(ifStmts).toHaveLength(1);
  });

  it("parses while loop", () => {
    const ast = parseEs5("while (x) { y; }");
    const whileStmts = findNodes(ast, "while_statement");
    expect(whileStmts).toHaveLength(1);
  });

  it("parses for loop", () => {
    const ast = parseEs5("for (var i = 0; i < 10; i++) { x; }");
    const forStmts = findNodes(ast, "for_statement");
    expect(forStmts).toHaveLength(1);
  });

  it("parses switch statement", () => {
    const ast = parseEs5("switch (x) { case 1: y; break; default: z; }");
    const switchStmts = findNodes(ast, "switch_statement");
    expect(switchStmts).toHaveLength(1);
  });
});

// ============================================================================
// Expression Precedence
// ============================================================================

describe("expression precedence", () => {
  it("parses additive expression", () => {
    const ast = parseEs5("1 + 2;");
    const addExprs = findNodes(ast, "additive_expression");
    expect(addExprs.length).toBeGreaterThanOrEqual(1);
  });

  it("parses logical operators", () => {
    const ast = parseEs5("a && b || c;");
    const orExprs = findNodes(ast, "logical_or_expression");
    expect(orExprs.length).toBeGreaterThanOrEqual(1);
  });

  it("parses ternary conditional", () => {
    const ast = parseEs5("a ? b : c;");
    const condExprs = findNodes(ast, "conditional_expression");
    expect(condExprs.length).toBeGreaterThanOrEqual(1);
  });
});

// ============================================================================
// Literals
// ============================================================================

describe("literals", () => {
  it("parses array literal", () => {
    const ast = parseEs5("var x = [1, 2, 3];");
    const arrayLits = findNodes(ast, "array_literal");
    expect(arrayLits).toHaveLength(1);
  });

  it("parses object literal", () => {
    const ast = parseEs5('var x = { a: 1, b: "two" };');
    const objLits = findNodes(ast, "object_literal");
    expect(objLits).toHaveLength(1);
  });

  it("parses function expression", () => {
    const ast = parseEs5("var f = function(x) { return x; };");
    const funcExprs = findNodes(ast, "function_expression");
    expect(funcExprs).toHaveLength(1);
  });
});

// ============================================================================
// Edge Cases
// ============================================================================

describe("edge cases", () => {
  it("parses empty program", () => {
    const ast = parseEs5("");
    expect(ast.ruleName).toBe("program");
  });

  it("parses empty block", () => {
    const ast = parseEs5("{ }");
    const blocks = findNodes(ast, "block");
    expect(blocks).toHaveLength(1);
  });

  it("parses multiple statements", () => {
    const ast = parseEs5("var x = 1; debugger; try { x; } catch (e) { y; }");
    const varStmts = findNodes(ast, "variable_statement");
    const debuggerStmts = findNodes(ast, "debugger_statement");
    const tryStmts = findNodes(ast, "try_statement");
    expect(varStmts).toHaveLength(1);
    expect(debuggerStmts).toHaveLength(1);
    expect(tryStmts).toHaveLength(1);
  });
});
