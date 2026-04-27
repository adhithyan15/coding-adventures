/**
 * Tests for the C# Lexer (TypeScript).
 *
 * These tests verify that the grammar-driven lexer correctly tokenizes
 * C# source code when loaded with the `csharp{version}.tokens` grammar file.
 *
 * C# has unique features compared to Java, JavaScript, Python, and Ruby:
 * - `class` and `namespace` are fundamental organizational units
 * - Static typing with explicit type annotations (`int`, `string`, `bool`)
 * - Access modifiers (`public`, `private`, `protected`, `internal`)
 * - Semicolons terminate statements
 * - Curly braces for blocks
 * - `null` (and nullable types: `int?`)
 * - Null-coalescing operator `??`
 * - Null-conditional operator `?.`
 * - Lambda expressions with fat arrow `=>`
 * - Properties with `get` / `set` accessors
 * - `using` directives for namespace imports
 *
 * Version-aware API
 * -----------------
 *
 * `tokenizeCSharp(source, version?)` and `createCSharpLexer(source, version?)`
 * both accept an optional C# version string:
 *   `"1.0"`, `"2.0"`, `"3.0"`, `"4.0"`, `"5.0"`, `"6.0"`,
 *   `"7.0"`, `"8.0"`, `"9.0"`, `"10.0"`, `"11.0"`, `"12.0"`.
 * Omitting the version uses C# 12.0 as the default.
 */

import { describe, it, expect } from "vitest";
import { tokenizeCSharp, createCSharpLexer } from "../src/index.js";

function tokenTypes(source: string, version?: string): string[] {
  return tokenizeCSharp(source, version).map((t) => t.type);
}

function tokenValues(source: string, version?: string): string[] {
  return tokenizeCSharp(source, version).map((t) => t.value);
}

// ---------------------------------------------------------------------------
// Basic class declaration
// ---------------------------------------------------------------------------

describe("basic class declaration", () => {
  it("tokenizes class Hello { }", () => {
    const types = tokenTypes("class Hello { }");
    // Should have at least: KEYWORD("class"), NAME("Hello"), LBRACE, RBRACE, EOF
    expect(types.length).toBeGreaterThanOrEqual(5);
    expect(types[0]).toBe("KEYWORD");
    expect(types[types.length - 1]).toBe("EOF");
  });

  it("captures correct values for class Hello { }", () => {
    const values = tokenValues("class Hello { }");
    expect(values[0]).toBe("class");
    expect(values[1]).toBe("Hello");
  });

  it("tokenizes all arithmetic operators", () => {
    const types = tokenTypes("a + b - c * d / e");
    expect(types).toEqual([
      "NAME", "PLUS", "NAME", "MINUS", "NAME",
      "STAR", "NAME", "SLASH", "NAME", "EOF",
    ]);
  });

  it("tokenizes parenthesized expressions", () => {
    const types = tokenTypes("(1 + 2) * 3");
    expect(types).toEqual([
      "LPAREN", "NUMBER", "PLUS", "NUMBER", "RPAREN",
      "STAR", "NUMBER", "EOF",
    ]);
  });
});

// ---------------------------------------------------------------------------
// C# keywords
// ---------------------------------------------------------------------------

describe("C# keywords", () => {
  it("recognizes class as a keyword", () => {
    const tokens = tokenizeCSharp("class");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("class");
  });

  it("recognizes namespace as a keyword", () => {
    const tokens = tokenizeCSharp("namespace");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("namespace");
  });

  it("recognizes using as a keyword", () => {
    const tokens = tokenizeCSharp("using");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("using");
  });

  it("recognizes public as a keyword", () => {
    const tokens = tokenizeCSharp("public");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("public");
  });

  it("recognizes private as a keyword", () => {
    const tokens = tokenizeCSharp("private");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("private");
  });

  it("recognizes static as a keyword", () => {
    const tokens = tokenizeCSharp("static");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("static");
  });

  it("recognizes void as a keyword", () => {
    const tokens = tokenizeCSharp("void");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("void");
  });

  it("recognizes int, string, bool as keywords", () => {
    const tokens = tokenizeCSharp("int string bool");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual(["int", "string", "bool"]);
  });

  it("recognizes true, false, null", () => {
    const tokens = tokenizeCSharp("true false null");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual(["true", "false", "null"]);
  });

  it("recognizes return as a keyword", () => {
    const tokens = tokenizeCSharp("return");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("return");
  });

  it("does not treat regular names as keywords", () => {
    const tokens = tokenizeCSharp("foobar");
    expect(tokens[0].type).toBe("NAME");
  });
});

// ---------------------------------------------------------------------------
// C#-specific operators: ??, ?., =>
// ---------------------------------------------------------------------------

describe("C#-specific operators", () => {
  it("tokenizes null-coalescing operator ??", () => {
    const tokens = tokenizeCSharp("x ?? y");
    // We care that ?? appears as a single token somewhere in the stream
    const op = tokens.find((t) => t.value === "??");
    expect(op).toBeDefined();
  });

  it("tokenizes null-conditional operator ?.", () => {
    const tokens = tokenizeCSharp("obj?.Method()");
    const op = tokens.find((t) => t.value === "?.");
    expect(op).toBeDefined();
  });

  it("tokenizes lambda fat arrow =>", () => {
    const tokens = tokenizeCSharp("x => x + 1");
    const op = tokens.find((t) => t.value === "=>");
    expect(op).toBeDefined();
  });

  it("tokenizes equality ==", () => {
    const tokens = tokenizeCSharp("x == 1");
    expect(tokens[1].value).toBe("==");
  });

  it("tokenizes inequality !=", () => {
    const tokens = tokenizeCSharp("x != 1");
    expect(tokens[1].value).toBe("!=");
  });

  it("tokenizes greater-than-or-equal >=", () => {
    const tokens = tokenizeCSharp("x >= 1");
    expect(tokens[1].value).toBe(">=");
  });

  it("tokenizes less-than-or-equal <=", () => {
    const tokens = tokenizeCSharp("x <= 1");
    expect(tokens[1].value).toBe("<=");
  });
});

// ---------------------------------------------------------------------------
// Delimiters
// ---------------------------------------------------------------------------

describe("delimiters", () => {
  it("tokenizes curly braces", () => {
    const types = tokenTypes("{ }");
    expect(types).toEqual(["LBRACE", "RBRACE", "EOF"]);
  });

  it("tokenizes square brackets", () => {
    const types = tokenTypes("[ ]");
    expect(types).toEqual(["LBRACKET", "RBRACKET", "EOF"]);
  });

  it("tokenizes semicolons", () => {
    const tokens = tokenizeCSharp(";");
    expect(tokens[0].type).toBe("SEMICOLON");
  });
});

// ---------------------------------------------------------------------------
// Literals
// ---------------------------------------------------------------------------

describe("literals", () => {
  it("tokenizes string literals", () => {
    const tokens = tokenizeCSharp('"hello"');
    expect(tokens[0].type).toBe("STRING");
    expect(tokens[0].value).toBe("hello");
  });

  it("tokenizes number literals", () => {
    const tokens = tokenizeCSharp("42");
    expect(tokens[0].type).toBe("NUMBER");
    expect(tokens[0].value).toBe("42");
  });
});

// ---------------------------------------------------------------------------
// Version-aware API tests
// ---------------------------------------------------------------------------

describe("version-aware tokenization", () => {
  it("tokenizes with no version (defaults to C# 12.0)", () => {
    const tokens = tokenizeCSharp("class Hello { }");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("class");
  });

  it("tokenizes with empty string version (same as no version)", () => {
    const tokens = tokenizeCSharp("class Hello { }", "");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("class");
  });

  it("tokenizes with C# 1.0 version", () => {
    const tokens = tokenizeCSharp("int x = 1;", "1.0");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("int");
  });

  it("tokenizes with C# 2.0 version", () => {
    const tokens = tokenizeCSharp("int x = 1;", "2.0");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("int");
  });

  it("tokenizes with C# 3.0 version", () => {
    const tokens = tokenizeCSharp("int x = 1;", "3.0");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("int");
  });

  it("tokenizes with C# 4.0 version", () => {
    const tokens = tokenizeCSharp("int x = 1;", "4.0");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("int");
  });

  it("tokenizes with C# 5.0 version", () => {
    const tokens = tokenizeCSharp("int x = 1;", "5.0");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("int");
  });

  it("tokenizes with C# 6.0 version", () => {
    const tokens = tokenizeCSharp("int x = 1;", "6.0");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("int");
  });

  it("tokenizes with C# 7.0 version", () => {
    const tokens = tokenizeCSharp("int x = 1;", "7.0");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("int");
  });

  it("tokenizes with C# 8.0 version", () => {
    const tokens = tokenizeCSharp("int x = 1;", "8.0");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("int");
    expect(tokens[tokens.length - 1].type).toBe("EOF");
  });

  it("tokenizes with C# 9.0 version", () => {
    const tokens = tokenizeCSharp("int x = 1;", "9.0");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("int");
  });

  it("tokenizes with C# 10.0 version", () => {
    const tokens = tokenizeCSharp("int x = 1;", "10.0");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("int");
  });

  it("tokenizes with C# 11.0 version", () => {
    const tokens = tokenizeCSharp("int x = 1;", "11.0");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("int");
  });

  it("tokenizes with C# 12.0 version", () => {
    const tokens = tokenizeCSharp("int x = 1;", "12.0");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("int");
  });

  it("throws for unknown version string", () => {
    expect(() => tokenizeCSharp("int x = 1;", "99")).toThrow(
      /Unknown C# version "99"/
    );
  });

  it("throws for completely invalid version string", () => {
    expect(() => tokenizeCSharp("int x = 1;", "latest")).toThrow(
      /Unknown C# version "latest"/
    );
  });
});

// ---------------------------------------------------------------------------
// createCSharpLexer API tests
// ---------------------------------------------------------------------------

describe("createCSharpLexer", () => {
  it("returns a GrammarLexer and produces tokens when tokenize() is called", () => {
    const lexer = createCSharpLexer("class Hello { }");
    const tokens = lexer.tokenize();
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("class");
    expect(tokens[tokens.length - 1].type).toBe("EOF");
  });

  it("accepts a version string", () => {
    const lexer = createCSharpLexer("int y = 2;", "8.0");
    const tokens = lexer.tokenize();
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("int");
  });

  it("throws for unknown version", () => {
    expect(() => createCSharpLexer("int x = 1;", "99")).toThrow(
      /Unknown C# version "99"/
    );
  });
});
