/**
 * Tests for the Python Parser (TypeScript).
 *
 * These tests verify that the grammar-driven parser, when loaded with the
 * `python.grammar` file, correctly parses Python source code into ASTs.
 *
 * The key insight: **no new parser code was written**. The same
 * `GrammarParser` engine that handles any language handles Python —
 * only the grammar file differs.
 */

import { describe, it, expect } from "vitest";
import { parsePython } from "../src/parser.js";
import { isASTNode } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import type { Token } from "@coding-adventures/lexer";

// ---------------------------------------------------------------------------
// Helpers — navigate AST structure
// ---------------------------------------------------------------------------

/** Recursively find all nodes with a given rule name. */
function findNodes(node: ASTNode, ruleName: string): ASTNode[] {
  const results: ASTNode[] = [];
  if (node.ruleName === ruleName) results.push(node);
  for (const child of node.children) {
    if (isASTNode(child)) {
      results.push(...findNodes(child, ruleName));
    }
  }
  return results;
}

/** Recursively collect all Token leaves from an AST. */
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

// ---------------------------------------------------------------------------
// Simple Assignments
// ---------------------------------------------------------------------------

describe("assignments", () => {
  it("parses x = 1 — simple assignment", () => {
    const ast = parsePython("x = 1");
    expect(ast.ruleName).toBe("program");

    const assignments = findNodes(ast, "assignment");
    expect(assignments).toHaveLength(1);

    const tokens = findTokens(assignments[0]);
    const names = tokens.filter((t) => t.type === "NAME");
    expect(names).toHaveLength(1);
    expect(names[0].value).toBe("x");
  });

  it("parses x = 1 + 2 — assignment with arithmetic", () => {
    const ast = parsePython("x = 1 + 2");

    const expressions = findNodes(ast, "expression");
    expect(expressions.length).toBeGreaterThanOrEqual(1);

    const exprTokens = findTokens(expressions[0]);
    const plusTokens = exprTokens.filter((t) => t.type === "PLUS");
    expect(plusTokens).toHaveLength(1);
  });
});

// ---------------------------------------------------------------------------
// Operator Precedence
// ---------------------------------------------------------------------------

describe("operator precedence", () => {
  it("parses 1 + 2 * 3 — multiplication before addition", () => {
    const ast = parsePython("1 + 2 * 3");

    const expressions = findNodes(ast, "expression");
    expect(expressions.length).toBeGreaterThanOrEqual(1);

    // PLUS should be at expression level
    const exprDirectTokens = expressions[0].children.filter(
      (c) => !isASTNode(c) && (c as Token).type === "PLUS"
    );
    expect(exprDirectTokens).toHaveLength(1);

    // STAR should be inside a term
    const terms = findNodes(ast, "term");
    const starTerms = terms.filter((t) =>
      t.children.some((c) => !isASTNode(c) && (c as Token).value === "*")
    );
    expect(starTerms).toHaveLength(1);
  });

  it("parses (1 + 2) * 3 — parentheses override precedence", () => {
    const ast = parsePython("(1 + 2) * 3");

    const factors = findNodes(ast, "factor");
    const parenFactors = factors.filter((f) =>
      f.children.some((c) => !isASTNode(c) && (c as Token).type === "LPAREN")
    );
    expect(parenFactors).toHaveLength(1);
  });
});

// ---------------------------------------------------------------------------
// Multiple Statements
// ---------------------------------------------------------------------------

describe("multiple statements", () => {
  it("parses two assignments separated by newline", () => {
    const ast = parsePython("x = 1\ny = 2");
    expect(ast.ruleName).toBe("program");

    const assignments = findNodes(ast, "assignment");
    expect(assignments).toHaveLength(2);
  });

  it("parses a bare expression as a statement", () => {
    const ast = parsePython("1 + 2");

    const exprStmts = findNodes(ast, "expression_stmt");
    expect(exprStmts).toHaveLength(1);
  });
});
