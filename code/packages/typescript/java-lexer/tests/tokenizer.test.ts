/**
 * Tests for the Java Lexer (TypeScript).
 *
 * These tests verify that the grammar-driven lexer correctly tokenizes
 * Java source code when loaded with the `java{version}.tokens` grammar file.
 *
 * Java has unique features compared to JavaScript, Python, and Ruby:
 * - `class` is the fundamental organizational unit
 * - Static typing with explicit type annotations (`int`, `String`, `boolean`)
 * - Access modifiers (`public`, `private`, `protected`)
 * - Semicolons terminate statements
 * - Curly braces for blocks
 * - `==` for equality (no `===` strict equality like JavaScript)
 *
 * Version-aware API
 * -----------------
 *
 * `tokenizeJava(source, version?)` and `createJavaLexer(source, version?)`
 * both accept an optional Java version string: `"1.0"`, `"1.1"`, `"1.4"`,
 * `"5"`, `"7"`, `"8"`, `"10"`, `"14"`, `"17"`, `"21"`. Omitting the version
 * uses Java 21 as the default.
 */

import { describe, it, expect } from "vitest";
import { tokenizeJava, createJavaLexer } from "../src/index.js";

function tokenTypes(source: string, version?: string): string[] {
  return tokenizeJava(source, version).map((t) => t.type);
}

function tokenValues(source: string, version?: string): string[] {
  return tokenizeJava(source, version).map((t) => t.value);
}

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

describe("Java keywords", () => {
  it("recognizes class as a keyword", () => {
    const tokens = tokenizeJava("class");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("class");
  });

  it("recognizes public as a keyword", () => {
    const tokens = tokenizeJava("public");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("public");
  });

  it("recognizes static as a keyword", () => {
    const tokens = tokenizeJava("static");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("static");
  });

  it("recognizes true, false, null", () => {
    const tokens = tokenizeJava("true false null");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual(["true", "false", "null"]);
  });

  it("does not treat regular names as keywords", () => {
    const tokens = tokenizeJava("foobar");
    expect(tokens[0].type).toBe("NAME");
  });
});

describe("Java operators", () => {
  it("tokenizes equality ==", () => {
    const tokens = tokenizeJava("x == 1");
    expect(tokens[1].value).toBe("==");
  });

  it("tokenizes inequality !=", () => {
    const tokens = tokenizeJava("x != 1");
    expect(tokens[1].value).toBe("!=");
  });

  it("tokenizes greater-than-or-equal >=", () => {
    const tokens = tokenizeJava("x >= 1");
    expect(tokens[1].value).toBe(">=");
  });

  it("tokenizes less-than-or-equal <=", () => {
    const tokens = tokenizeJava("x <= 1");
    expect(tokens[1].value).toBe("<=");
  });
});

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
    const tokens = tokenizeJava(";");
    expect(tokens[0].type).toBe("SEMICOLON");
  });
});

describe("literals", () => {
  it("tokenizes string literals", () => {
    const tokens = tokenizeJava('"hello"');
    expect(tokens[0].type).toBe("STRING");
    expect(tokens[0].value).toBe("hello");
  });

  it("tokenizes number literals", () => {
    const tokens = tokenizeJava("42");
    expect(tokens[0].type).toBe("NUMBER");
    expect(tokens[0].value).toBe("42");
  });
});

// ---------------------------------------------------------------------------
// Version-aware API tests
// ---------------------------------------------------------------------------

describe("version-aware tokenization", () => {
  it("tokenizes with no version (defaults to Java 21)", () => {
    const tokens = tokenizeJava("class Hello { }");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("class");
  });

  it("tokenizes with empty string version (same as no version)", () => {
    const tokens = tokenizeJava("class Hello { }", "");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("class");
  });

  it("tokenizes with Java 8 version", () => {
    const tokens = tokenizeJava("int x = 1;", "8");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("int");
    expect(tokens[tokens.length - 1].type).toBe("EOF");
  });

  it("tokenizes with Java 1.0 version", () => {
    const tokens = tokenizeJava("int x = 1;", "1.0");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("int");
  });

  it("tokenizes with Java 1.1 version", () => {
    const tokens = tokenizeJava("int x = 1;", "1.1");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("int");
  });

  it("tokenizes with Java 1.4 version", () => {
    const tokens = tokenizeJava("int x = 1;", "1.4");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("int");
  });

  it("tokenizes with Java 5 version", () => {
    const tokens = tokenizeJava("int x = 1;", "5");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("int");
  });

  it("tokenizes with Java 7 version", () => {
    const tokens = tokenizeJava("int x = 1;", "7");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("int");
  });

  it("tokenizes with Java 10 version", () => {
    const tokens = tokenizeJava("int x = 1;", "10");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("int");
  });

  it("tokenizes with Java 14 version", () => {
    const tokens = tokenizeJava("int x = 1;", "14");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("int");
  });

  it("tokenizes with Java 17 version", () => {
    const tokens = tokenizeJava("int x = 1;", "17");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("int");
  });

  it("tokenizes with Java 21 version", () => {
    const tokens = tokenizeJava("int x = 1;", "21");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("int");
  });

  it("throws for unknown version string", () => {
    expect(() => tokenizeJava("int x = 1;", "99")).toThrow(
      /Unknown Java version "99"/
    );
  });

  it("throws for completely invalid version string", () => {
    expect(() => tokenizeJava("int x = 1;", "latest")).toThrow(
      /Unknown Java version "latest"/
    );
  });
});

// ---------------------------------------------------------------------------
// createJavaLexer API tests
// ---------------------------------------------------------------------------

describe("createJavaLexer", () => {
  it("returns a GrammarLexer and produces tokens when tokenize() is called", () => {
    const lexer = createJavaLexer("class Hello { }");
    const tokens = lexer.tokenize();
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("class");
    expect(tokens[tokens.length - 1].type).toBe("EOF");
  });

  it("accepts a version string", () => {
    const lexer = createJavaLexer("int y = 2;", "8");
    const tokens = lexer.tokenize();
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("int");
  });

  it("throws for unknown version", () => {
    expect(() => createJavaLexer("int x = 1;", "99")).toThrow(
      /Unknown Java version "99"/
    );
  });
});
