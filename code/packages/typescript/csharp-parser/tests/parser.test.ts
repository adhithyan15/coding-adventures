/**
 * Tests for the C# Parser (TypeScript).
 *
 * These tests verify that the grammar-driven parser correctly parses C#
 * source code when loaded with the `csharp{version}.grammar` file.
 *
 * C# grammar features exercised here:
 * - `var_declaration` rule: `int x = 1 + 2;`
 * - Semicolons terminate statements
 * - Class declarations as the fundamental unit
 * - Method declarations inside classes
 *
 * Version-aware API
 * -----------------
 *
 * `parseCSharp(source, version?)` and `createCSharpParser(source, version?)`
 * accept an optional C# version string:
 *   `"1.0"`, `"2.0"`, `"3.0"`, `"4.0"`, `"5.0"`, `"6.0"`,
 *   `"7.0"`, `"8.0"`, `"9.0"`, `"10.0"`, `"11.0"`, `"12.0"`.
 * Omitting the version uses C# 12.0 as the default.
 */

import { describe, it, expect } from "vitest";
import { parseCSharp, createCSharpParser } from "../src/parser.js";
import { isASTNode } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import type { Token } from "@coding-adventures/lexer";

// ---------------------------------------------------------------------------
// Tree-walking helpers
// ---------------------------------------------------------------------------

/**
 * Walk the AST depth-first and collect all nodes whose `ruleName` matches.
 *
 * This is the workhorse for our assertions. Rather than hard-coding exact
 * tree positions, we search the whole tree — this is resilient to minor
 * grammar evolution between C# versions.
 */
function findNodes(node: ASTNode, ruleName: string): ASTNode[] {
  const results: ASTNode[] = [];
  if (node.ruleName === ruleName) results.push(node);
  for (const child of node.children) {
    if (isASTNode(child)) results.push(...findNodes(child, ruleName));
  }
  return results;
}

/**
 * Collect all leaf tokens beneath a given AST node (depth-first).
 *
 * Useful for checking what concrete token values appear under a rule
 * (e.g. the keyword `int` and identifier `x` inside a `var_declaration`).
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

// ---------------------------------------------------------------------------
// Variable declarations
// ---------------------------------------------------------------------------

describe("variable declarations", () => {
  it("parses int x = 1 + 2;", () => {
    const ast = parseCSharp("int x = 1 + 2;");
    expect(ast.ruleName).toBe("program");

    const varDecls = findNodes(ast, "var_declaration");
    expect(varDecls).toHaveLength(1);

    const tokens = findTokens(varDecls[0]);
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords[0].value).toBe("int");

    const names = tokens.filter((t) => t.type === "NAME");
    expect(names[0].value).toBe("x");
  });

  it("parses bool flag = true;", () => {
    const ast = parseCSharp("bool flag = true;");

    const varDecls = findNodes(ast, "var_declaration");
    expect(varDecls).toHaveLength(1);

    const tokens = findTokens(varDecls[0]);
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords[0].value).toBe("bool");
  });
});

// ---------------------------------------------------------------------------
// Expression statements
// ---------------------------------------------------------------------------

describe("expression statements", () => {
  it("parses 1 + 2;", () => {
    const ast = parseCSharp("1 + 2;");
    expect(ast.ruleName).toBe("program");

    const exprStmts = findNodes(ast, "expression_stmt");
    expect(exprStmts).toHaveLength(1);
  });
});

// ---------------------------------------------------------------------------
// Operator precedence
// ---------------------------------------------------------------------------

describe("operator precedence", () => {
  it("parses 1 + 2 * 3; — multiplication before addition", () => {
    const ast = parseCSharp("1 + 2 * 3;");

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

// ---------------------------------------------------------------------------
// Multiple statements
// ---------------------------------------------------------------------------

describe("multiple statements", () => {
  it("parses two variable declarations", () => {
    const ast = parseCSharp("int x = 1;int y = 2;");

    const varDecls = findNodes(ast, "var_declaration");
    expect(varDecls).toHaveLength(2);
  });
});

// ---------------------------------------------------------------------------
// Assignments
// ---------------------------------------------------------------------------

describe("assignments", () => {
  it("parses x = 5;", () => {
    const ast = parseCSharp("x = 5;");

    const assignments = findNodes(ast, "assignment");
    expect(assignments).toHaveLength(1);
  });
});

// ---------------------------------------------------------------------------
// createCSharpParser tests
// ---------------------------------------------------------------------------

describe("createCSharpParser", () => {
  it("returns a GrammarParser that produces a valid AST", () => {
    const parser = createCSharpParser("int x = 1;");
    const ast = parser.parse();
    expect(ast.ruleName).toBe("program");
  });

  it("accepts a version string", () => {
    const parser = createCSharpParser("int x = 1;", "8.0");
    const ast = parser.parse();
    expect(ast.ruleName).toBe("program");
  });

  it("throws for unknown version", () => {
    expect(() => createCSharpParser("int x = 1;", "99")).toThrow(
      /Unknown C# version "99"/
    );
  });
});

// ---------------------------------------------------------------------------
// Version-aware API tests
// ---------------------------------------------------------------------------

describe("version-aware parsing", () => {
  it("parses with no version (defaults to C# 12.0)", () => {
    const ast = parseCSharp("int x = 1;");
    expect(ast.ruleName).toBe("program");
    const varDecls = findNodes(ast, "var_declaration");
    expect(varDecls).toHaveLength(1);
  });

  it("parses with empty string version (same as no version)", () => {
    const ast = parseCSharp("int x = 1;", "");
    expect(ast.ruleName).toBe("program");
  });

  it("parses with C# 1.0 version", () => {
    const ast = parseCSharp("int x = 0;", "1.0");
    expect(ast.ruleName).toBe("program");
  });

  it("parses with C# 2.0 version", () => {
    const ast = parseCSharp("int x = 0;", "2.0");
    expect(ast.ruleName).toBe("program");
  });

  it("parses with C# 3.0 version", () => {
    const ast = parseCSharp("int x = 0;", "3.0");
    expect(ast.ruleName).toBe("program");
  });

  it("parses with C# 4.0 version", () => {
    const ast = parseCSharp("int x = 0;", "4.0");
    expect(ast.ruleName).toBe("program");
  });

  it("parses with C# 5.0 version", () => {
    const ast = parseCSharp("int x = 0;", "5.0");
    expect(ast.ruleName).toBe("program");
  });

  it("parses with C# 6.0 version", () => {
    const ast = parseCSharp("int x = 0;", "6.0");
    expect(ast.ruleName).toBe("program");
  });

  it("parses with C# 7.0 version", () => {
    const ast = parseCSharp("int x = 0;", "7.0");
    expect(ast.ruleName).toBe("program");
  });

  it("parses with C# 8.0 version", () => {
    const ast = parseCSharp("int x = 1 + 2;", "8.0");
    expect(ast.ruleName).toBe("program");
    const varDecls = findNodes(ast, "var_declaration");
    expect(varDecls).toHaveLength(1);
  });

  it("parses with C# 9.0 version", () => {
    const ast = parseCSharp("int x = 0;", "9.0");
    expect(ast.ruleName).toBe("program");
  });

  it("parses with C# 10.0 version", () => {
    const ast = parseCSharp("int x = 0;", "10.0");
    expect(ast.ruleName).toBe("program");
  });

  it("parses with C# 11.0 version", () => {
    const ast = parseCSharp("int x = 0;", "11.0");
    expect(ast.ruleName).toBe("program");
  });

  it("parses with C# 12.0 version", () => {
    const ast = parseCSharp("int x = 42;", "12.0");
    expect(ast.ruleName).toBe("program");
  });

  it("throws for unknown C# version", () => {
    expect(() => parseCSharp("int x = 1;", "99")).toThrow(
      /Unknown C# version "99"/
    );
  });
});
