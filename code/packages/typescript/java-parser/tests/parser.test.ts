/**
 * Tests for the Java Parser (TypeScript).
 *
 * These tests verify that the grammar-driven parser correctly parses Java
 * source code when loaded with the `java{version}.grammar` file.
 *
 * Java grammar features:
 * - `var_declaration` rule: `int x = 1 + 2;`
 * - Semicolons terminate statements
 * - Class declarations as the fundamental unit
 *
 * Version-aware API
 * -----------------
 *
 * `parseJava(source, version?)` and `createJavaParser(source, version?)` accept
 * an optional Java version string (`"1.0"`, `"1.1"`, `"1.4"`, `"5"`, `"7"`,
 * `"8"`, `"10"`, `"14"`, `"17"`, `"21"`). Omitting the version uses Java 21
 * as the default.
 */

import { describe, it, expect } from "vitest";
import { parseJava, createJavaParser } from "../src/parser.js";
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
  it("parses int x = 1 + 2;", () => {
    const ast = parseJava("int x = 1 + 2;");
    expect(ast.ruleName).toBe("program");

    const varDecls = findNodes(ast, "var_declaration");
    expect(varDecls).toHaveLength(1);

    const tokens = findTokens(varDecls[0]);
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords[0].value).toBe("int");

    const names = tokens.filter((t) => t.type === "NAME");
    expect(names[0].value).toBe("x");
  });

  it("parses boolean flag = true;", () => {
    const ast = parseJava("boolean flag = true;");

    const varDecls = findNodes(ast, "var_declaration");
    expect(varDecls).toHaveLength(1);

    const tokens = findTokens(varDecls[0]);
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords[0].value).toBe("boolean");
  });
});

describe("expression statements", () => {
  it("parses 1 + 2;", () => {
    const ast = parseJava("1 + 2;");
    expect(ast.ruleName).toBe("program");

    const exprStmts = findNodes(ast, "expression_statement");
    expect(exprStmts).toHaveLength(1);
  });
});

describe("operator precedence", () => {
  it("parses 1 + 2 * 3; — multiplication before addition", () => {
    const ast = parseJava("int result = 1 + 2 * 3;");

    const expressions = findNodes(ast, "expression");
    expect(expressions.length).toBeGreaterThanOrEqual(1);

    const multiplicativeExpressions = findNodes(ast, "multiplicative_expression");
    expect(multiplicativeExpressions.length).toBeGreaterThanOrEqual(1);
  });
});

describe("multiple statements", () => {
  it("parses two variable declarations", () => {
    const ast = parseJava("int x = 1;int y = 2;");

    const varDecls = findNodes(ast, "var_declaration");
    expect(varDecls).toHaveLength(2);
  });
});

describe("assignments", () => {
  it("parses x = 5;", () => {
    const ast = parseJava("x = 5;");

    const assignments = findNodes(ast, "assignment_expression");
    expect(assignments.length).toBeGreaterThanOrEqual(1);
  });
});

// ---------------------------------------------------------------------------
// createJavaParser tests
// ---------------------------------------------------------------------------

describe("createJavaParser", () => {
  it("returns a GrammarParser that produces a valid AST", () => {
    const parser = createJavaParser("int x = 1;");
    const ast = parser.parse();
    expect(ast.ruleName).toBe("program");
  });

  it("accepts a version string", () => {
    const parser = createJavaParser("int x = 1;", "8");
    const ast = parser.parse();
    expect(ast.ruleName).toBe("program");
  });

  it("throws for unknown version", () => {
    expect(() => createJavaParser("int x = 1;", "99")).toThrow(
      /Unknown Java version "99"/
    );
  });
});

// ---------------------------------------------------------------------------
// Version-aware API tests
// ---------------------------------------------------------------------------

describe("version-aware parsing", () => {
  it("parses with no version (defaults to Java 21)", () => {
    const ast = parseJava("int x = 1;");
    expect(ast.ruleName).toBe("program");
    const varDecls = findNodes(ast, "var_declaration");
    expect(varDecls).toHaveLength(1);
  });

  it("parses with empty string version (same as no version)", () => {
    const ast = parseJava("int x = 1;", "");
    expect(ast.ruleName).toBe("program");
  });

  it("parses with Java 8 version", () => {
    const ast = parseJava("int x = 1 + 2;", "8");
    expect(ast.ruleName).toBe("program");
    const varDecls = findNodes(ast, "var_declaration");
    expect(varDecls).toHaveLength(1);
  });

  it("parses with Java 21 version", () => {
    const ast = parseJava("int x = 42;", "21");
    expect(ast.ruleName).toBe("program");
  });

  it("parses with Java 1.0 version", () => {
    const ast = parseJava("int x = 0;", "1.0");
    expect(ast.ruleName).toBe("program");
  });

  it("parses with Java 1.1 version", () => {
    const ast = parseJava("int x = 0;", "1.1");
    expect(ast.ruleName).toBe("program");
  });

  it("parses with Java 1.4 version", () => {
    const ast = parseJava("int x = 0;", "1.4");
    expect(ast.ruleName).toBe("program");
  });

  it("parses with Java 5 version", () => {
    const ast = parseJava("int x = 0;", "5");
    expect(ast.ruleName).toBe("program");
  });

  it("parses with Java 7 version", () => {
    const ast = parseJava("int x = 0;", "7");
    expect(ast.ruleName).toBe("program");
  });

  it("parses with Java 10 version", () => {
    const ast = parseJava("int x = 0;", "10");
    expect(ast.ruleName).toBe("program");
  });

  it("parses with Java 14 version", () => {
    const ast = parseJava("int x = 0;", "14");
    expect(ast.ruleName).toBe("program");
  });

  it("parses with Java 17 version", () => {
    const ast = parseJava("int x = 0;", "17");
    expect(ast.ruleName).toBe("program");
  });

  it("throws for unknown Java version", () => {
    expect(() => parseJava("int x = 1;", "99")).toThrow(
      /Unknown Java version "99"/
    );
  });
});
