/**
 * Tests for the TypeScript Parser (TypeScript).
 *
 * These tests verify that the grammar-driven parser correctly parses TypeScript
 * source code when loaded with the `typescript.grammar` file.
 *
 * TypeScript grammar features:
 * - `var_declaration` rule: `let x = 1 + 2;`
 * - Semicolons terminate statements
 * - The `factor` rule includes KEYWORD (for `true`, `false`, `null`, `undefined`)
 * - TypeScript-specific keywords like `interface`, `type`, `number` are recognized
 */

import { describe, it, expect } from "vitest";
import { parseTypescript } from "../src/parser.js";
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

describe("variable declarations", () => {
  it("parses let x = 1 + 2;", () => {
    const ast = parseTypescript("let x = 1 + 2;");
    expect(ast.ruleName).toBe("program");

    const varDecls = findNodes(ast, "var_declaration");
    expect(varDecls).toHaveLength(1);

    const tokens = findTokens(varDecls[0]);
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords[0].value).toBe("let");

    const names = tokens.filter((t) => t.type === "NAME");
    expect(names[0].value).toBe("x");
  });

  it("parses const y = 42;", () => {
    const ast = parseTypescript("const y = 42;");

    const varDecls = findNodes(ast, "var_declaration");
    expect(varDecls).toHaveLength(1);

    const tokens = findTokens(varDecls[0]);
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords[0].value).toBe("const");
  });
});

describe("expression statements", () => {
  it("parses 1 + 2;", () => {
    const ast = parseTypescript("1 + 2;");
    expect(ast.ruleName).toBe("program");

    const exprStmts = findNodes(ast, "expression_stmt");
    expect(exprStmts).toHaveLength(1);
  });
});

describe("operator precedence", () => {
  it("parses 1 + 2 * 3; — multiplication before addition", () => {
    const ast = parseTypescript("1 + 2 * 3;");

    const expressions = findNodes(ast, "expression");
    expect(expressions.length).toBeGreaterThanOrEqual(1);

    // STAR should be inside a term, not at expression level
    const terms = findNodes(ast, "term");
    const starTerms = terms.filter((t) =>
      t.children.some((c) => !isASTNode(c) && (c as Token).value === "*")
    );
    expect(starTerms).toHaveLength(1);
  });
});

describe("multiple statements", () => {
  it("parses two variable declarations", () => {
    const ast = parseTypescript("let x = 1;let y = 2;");

    const varDecls = findNodes(ast, "var_declaration");
    expect(varDecls).toHaveLength(2);
  });
});

describe("assignments", () => {
  it("parses x = 5;", () => {
    const ast = parseTypescript("x = 5;");

    const assignments = findNodes(ast, "assignment");
    expect(assignments).toHaveLength(1);
  });
});
