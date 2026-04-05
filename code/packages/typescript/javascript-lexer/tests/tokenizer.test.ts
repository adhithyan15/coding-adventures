/**
 * Tests for the JavaScript Lexer (TypeScript).
 *
 * These tests verify that the grammar-driven lexer correctly tokenizes
 * JavaScript source code when loaded with the `javascript.tokens` grammar file.
 *
 * JavaScript has unique features compared to Python and Ruby:
 * - `let`, `const`, `var` keywords for variable declarations
 * - `===` and `!==` for strict equality/inequality
 * - Semicolons terminate statements
 * - Curly braces for blocks
 * - `$` is valid in identifiers
 *
 * Version-aware API (added in v0.2.0)
 * ------------------------------------
 *
 * `tokenizeJavascript(source, version?)` and `createJavascriptLexer(source, version?)`
 * both accept an optional ECMAScript version string: `"es1"`, `"es3"`, `"es5"`,
 * `"es2015"` … `"es2025"`. Omitting the version uses the generic `javascript.tokens`
 * grammar (backwards-compatible with v0.1.x).
 */

import { describe, it, expect } from "vitest";
import { tokenizeJavascript, createJavascriptLexer } from "../src/index.js";

function tokenTypes(source: string, version?: string): string[] {
  return tokenizeJavascript(source, version).map((t) => t.type);
}

function tokenValues(source: string, version?: string): string[] {
  return tokenizeJavascript(source, version).map((t) => t.value);
}

describe("basic expressions", () => {
  it("tokenizes let x = 1 + 2;", () => {
    const types = tokenTypes("let x = 1 + 2;");
    expect(types).toEqual([
      "KEYWORD", "NAME", "EQUALS", "NUMBER", "PLUS", "NUMBER",
      "SEMICOLON", "EOF",
    ]);
  });

  it("captures correct values for let x = 1 + 2;", () => {
    const values = tokenValues("let x = 1 + 2;");
    expect(values).toEqual(["let", "x", "=", "1", "+", "2", ";", ""]);
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

describe("JavaScript keywords", () => {
  it("recognizes let as a keyword", () => {
    const tokens = tokenizeJavascript("let");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("let");
  });

  it("recognizes const as a keyword", () => {
    const tokens = tokenizeJavascript("const");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("const");
  });

  it("recognizes function as a keyword", () => {
    const tokens = tokenizeJavascript("function");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("function");
  });

  it("recognizes true, false, null, undefined", () => {
    const tokens = tokenizeJavascript("true false null undefined");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual(["true", "false", "null", "undefined"]);
  });

  it("does not treat regular names as keywords", () => {
    const tokens = tokenizeJavascript("foobar");
    expect(tokens[0].type).toBe("NAME");
  });
});

describe("JavaScript-specific operators", () => {
  it("tokenizes strict equality ===", () => {
    const tokens = tokenizeJavascript("x === 1");
    expect(tokens[1].value).toBe("===");
  });

  it("tokenizes strict inequality !==", () => {
    const tokens = tokenizeJavascript("x !== 1");
    expect(tokens[1].value).toBe("!==");
  });

  it("tokenizes equality == (not strict)", () => {
    const types = tokenTypes("x == 1");
    expect(types).toEqual(["NAME", "EQUALS_EQUALS", "NUMBER", "EOF"]);
  });

  it("tokenizes arrow operator =>", () => {
    const tokens = tokenizeJavascript("x => x");
    expect(tokens[1].value).toBe("=>");
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
    const tokens = tokenizeJavascript(";");
    expect(tokens[0].type).toBe("SEMICOLON");
  });
});

describe("literals", () => {
  it("tokenizes string literals", () => {
    const tokens = tokenizeJavascript('"hello"');
    expect(tokens[0].type).toBe("STRING");
    expect(tokens[0].value).toBe("hello");
  });

  it("tokenizes identifiers with $", () => {
    const tokens = tokenizeJavascript("$foo");
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("$foo");
  });
});

// ---------------------------------------------------------------------------
// Version-aware API tests (v0.2.0)
// ---------------------------------------------------------------------------

describe("version-aware tokenization", () => {
  it("tokenizes with no version (generic grammar — backwards compatible)", () => {
    const tokens = tokenizeJavascript("let x = 1;");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("let");
  });

  it("tokenizes with empty string version (same as no version)", () => {
    const tokens = tokenizeJavascript("let x = 1;", "");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("let");
  });

  it("tokenizes with es5 version", () => {
    const tokens = tokenizeJavascript("var x = 1;", "es5");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("var");
    expect(tokens[tokens.length - 1].type).toBe("EOF");
  });

  it("tokenizes with es1 version", () => {
    // ES1 uses 'var'; basic declarations should work
    const tokens = tokenizeJavascript("var x = 1;", "es1");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("var");
  });

  it("tokenizes with es3 version", () => {
    const tokens = tokenizeJavascript("var x = 1;", "es3");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("var");
  });

  it("tokenizes with es2015 version", () => {
    const tokens = tokenizeJavascript("let x = 1;", "es2015");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("let");
  });

  it("tokenizes with es2020 version", () => {
    const tokens = tokenizeJavascript("const x = 1;", "es2020");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("const");
  });

  it("tokenizes with es2025 version", () => {
    const tokens = tokenizeJavascript("let x = 1;", "es2025");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("let");
  });

  it("throws for unknown version string", () => {
    expect(() => tokenizeJavascript("let x = 1;", "es2099")).toThrow(
      /Unknown JavaScript\/ECMAScript version "es2099"/
    );
  });

  it("throws for completely invalid version string", () => {
    expect(() => tokenizeJavascript("let x = 1;", "latest")).toThrow(
      /Unknown JavaScript\/ECMAScript version "latest"/
    );
  });
});

// ---------------------------------------------------------------------------
// createJavascriptLexer API tests (v0.2.0)
// ---------------------------------------------------------------------------

describe("createJavascriptLexer", () => {
  it("returns a GrammarLexer and produces tokens when tokenize() is called", () => {
    const lexer = createJavascriptLexer("let x = 1;");
    const tokens = lexer.tokenize();
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("let");
    expect(tokens[tokens.length - 1].type).toBe("EOF");
  });

  it("accepts a version string", () => {
    const lexer = createJavascriptLexer("var y = 2;", "es5");
    const tokens = lexer.tokenize();
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("var");
  });

  it("throws for unknown version", () => {
    expect(() => createJavascriptLexer("let x = 1;", "es99")).toThrow(
      /Unknown JavaScript\/ECMAScript version "es99"/
    );
  });
});
