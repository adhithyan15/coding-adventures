/**
 * Tests for the Ruby Parser (TypeScript).
 *
 * These tests verify that the grammar-driven parser correctly parses Ruby
 * source code when loaded with the `ruby.grammar` file.
 */

import { describe, it, expect } from "vitest";
import { parseRuby } from "../src/parser.js";
import { isASTNode } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import type { Token } from "@coding-adventures/lexer";

function findNodes(node: ASTNode, ruleName: string): ASTNode[] {
  const results: ASTNode[] = [];
  if (node.ruleName === ruleName) results.push(node);
  for (const child of node.children) {
    if (isASTNode(child)) results.push(...findNodes(child, ruleName));
  }
  return results;
}

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

describe("assignments", () => {
  it("parses x = 1", () => {
    const ast = parseRuby("x = 1");
    expect(ast.ruleName).toBe("program");

    const assignments = findNodes(ast, "assignment");
    expect(assignments).toHaveLength(1);
  });

  it("parses x = 1 + 2", () => {
    const ast = parseRuby("x = 1 + 2");

    const expressions = findNodes(ast, "expression");
    expect(expressions.length).toBeGreaterThanOrEqual(1);

    const exprTokens = findTokens(expressions[0]);
    const plusTokens = exprTokens.filter((t) => t.type === "PLUS");
    expect(plusTokens).toHaveLength(1);
  });
});

describe("method calls", () => {
  it("parses puts(\"hello\")", () => {
    const ast = parseRuby('puts("hello")');

    const methodCalls = findNodes(ast, "method_call");
    expect(methodCalls).toHaveLength(1);

    const tokens = findTokens(methodCalls[0]);
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords[0].value).toBe("puts");
  });

  it("parses add(1, 2)", () => {
    const ast = parseRuby("add(1, 2)");

    const methodCalls = findNodes(ast, "method_call");
    expect(methodCalls).toHaveLength(1);

    const tokens = findTokens(methodCalls[0]);
    const numbers = tokens.filter((t) => t.type === "NUMBER");
    expect(numbers).toHaveLength(2);
  });
});

describe("multiple statements", () => {
  it("parses two assignments separated by newline", () => {
    const ast = parseRuby("x = 1\ny = 2");
    const assignments = findNodes(ast, "assignment");
    expect(assignments).toHaveLength(2);
  });
});

describe("operator precedence", () => {
  it("parses 1 + 2 * 3", () => {
    const ast = parseRuby("1 + 2 * 3");
    const expressions = findNodes(ast, "expression");
    expect(expressions.length).toBeGreaterThanOrEqual(1);
  });
});
