/**
 * Tests for the ECMAScript 1 (1997) Lexer (TypeScript).
 *
 * These tests verify that the grammar-driven lexer correctly tokenizes
 * ES1 source code when loaded with the `es1.tokens` grammar file.
 *
 * ES1 is the first standardized version of JavaScript. It has:
 * - `var` for variable declarations (no `let` or `const`)
 * - == and != (no === or !==)
 * - No try/catch/finally/throw
 * - No regex literals
 * - $ is valid in identifiers
 */

import { describe, it, expect } from "vitest";
import { tokenizeEs1 } from "../src/tokenizer.js";

/**
 * Helper: extract just the token types from a source string.
 * This makes assertions more readable when we only care about
 * the structure (types) and not the values.
 */
function tokenTypes(source: string): string[] {
  return tokenizeEs1(source).map((t) => t.type);
}

/**
 * Helper: extract just the token values from a source string.
 */
function tokenValues(source: string): string[] {
  return tokenizeEs1(source).map((t) => t.value);
}

// ============================================================================
// Basic Expressions
// ============================================================================

describe("basic expressions", () => {
  it("tokenizes var x = 1 + 2;", () => {
    const types = tokenTypes("var x = 1 + 2;");
    expect(types).toEqual([
      "KEYWORD", "NAME", "EQUALS", "NUMBER", "PLUS", "NUMBER",
      "SEMICOLON", "EOF",
    ]);
  });

  it("captures correct values for var x = 1 + 2;", () => {
    const values = tokenValues("var x = 1 + 2;");
    expect(values).toEqual(["var", "x", "=", "1", "+", "2", ";", ""]);
  });

  it("tokenizes all arithmetic operators", () => {
    const types = tokenTypes("a + b - c * d / e % f");
    expect(types).toEqual([
      "NAME", "PLUS", "NAME", "MINUS", "NAME",
      "STAR", "NAME", "SLASH", "NAME", "PERCENT", "NAME", "EOF",
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

// ============================================================================
// ES1 Keywords
// ============================================================================

describe("ES1 keywords", () => {
  it("recognizes var as a keyword", () => {
    const tokens = tokenizeEs1("var");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("var");
  });

  it("recognizes function as a keyword", () => {
    const tokens = tokenizeEs1("function");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("function");
  });

  it("recognizes all control flow keywords", () => {
    const source = "if else while do for switch case break continue return";
    const tokens = tokenizeEs1(source);
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual([
      "if", "else", "while", "do", "for", "switch", "case",
      "break", "continue", "return",
    ]);
  });

  it("recognizes true, false, null as keywords", () => {
    const tokens = tokenizeEs1("true false null");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual(["true", "false", "null"]);
  });

  it("recognizes delete, typeof, void, new, this, with, default, in", () => {
    const source = "delete typeof void new this with default in";
    const tokens = tokenizeEs1(source);
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual([
      "delete", "typeof", "void", "new", "this", "with", "default", "in",
    ]);
  });

  it("does not treat regular names as keywords", () => {
    const tokens = tokenizeEs1("foobar");
    expect(tokens[0].type).toBe("NAME");
  });

  it("rejects future reserved words as identifiers", () => {
    // In ES1, future reserved words (class, const, enum, etc.) cannot be used
    // as identifiers — the lexer throws an error when it encounters them.
    expect(() => tokenizeEs1("class")).toThrow(/Reserved keyword/);
    expect(() => tokenizeEs1("const")).toThrow(/Reserved keyword/);
    expect(() => tokenizeEs1("enum")).toThrow(/Reserved keyword/);
    expect(() => tokenizeEs1("export")).toThrow(/Reserved keyword/);
    expect(() => tokenizeEs1("extends")).toThrow(/Reserved keyword/);
    expect(() => tokenizeEs1("import")).toThrow(/Reserved keyword/);
    expect(() => tokenizeEs1("super")).toThrow(/Reserved keyword/);
  });
});

// ============================================================================
// ES1 Operators
// ============================================================================

describe("ES1 operators", () => {
  it("tokenizes == (abstract equality)", () => {
    const types = tokenTypes("x == 1");
    expect(types).toEqual(["NAME", "EQUALS_EQUALS", "NUMBER", "EOF"]);
  });

  it("tokenizes != (abstract inequality)", () => {
    const types = tokenTypes("x != 1");
    expect(types).toEqual(["NAME", "NOT_EQUALS", "NUMBER", "EOF"]);
  });

  it("does NOT have === (strict equality is ES3)", () => {
    // In ES1, === should be tokenized as == followed by =
    const types = tokenTypes("x === 1");
    expect(types).toEqual(["NAME", "EQUALS_EQUALS", "EQUALS", "NUMBER", "EOF"]);
  });

  it("tokenizes compound assignment operators", () => {
    const tokens = tokenizeEs1("x += 1");
    expect(tokens[1].type).toBe("PLUS_EQUALS");
    expect(tokens[1].value).toBe("+=");
  });

  it("tokenizes bitwise operators", () => {
    const types = tokenTypes("a & b | c ^ d ~ e");
    expect(types).toEqual([
      "NAME", "AMPERSAND", "NAME", "PIPE", "NAME",
      "CARET", "NAME", "TILDE", "NAME", "EOF",
    ]);
  });

  it("tokenizes logical operators", () => {
    const types = tokenTypes("a && b || c");
    expect(types).toEqual([
      "NAME", "AND_AND", "NAME", "OR_OR", "NAME", "EOF",
    ]);
  });

  it("tokenizes shift operators including unsigned right shift", () => {
    const types = tokenTypes("a << b >> c >>> d");
    expect(types).toEqual([
      "NAME", "LEFT_SHIFT", "NAME", "RIGHT_SHIFT", "NAME",
      "UNSIGNED_RIGHT_SHIFT", "NAME", "EOF",
    ]);
  });

  it("tokenizes increment and decrement", () => {
    const types = tokenTypes("x++ y--");
    expect(types).toEqual([
      "NAME", "PLUS_PLUS", "NAME", "MINUS_MINUS", "EOF",
    ]);
  });

  it("tokenizes comparison operators", () => {
    const types = tokenTypes("a < b > c <= d >= e");
    expect(types).toEqual([
      "NAME", "LESS_THAN", "NAME", "GREATER_THAN", "NAME",
      "LESS_EQUALS", "NAME", "GREATER_EQUALS", "NAME", "EOF",
    ]);
  });

  it("tokenizes ternary operator", () => {
    const types = tokenTypes("a ? b : c");
    expect(types).toEqual([
      "NAME", "QUESTION", "NAME", "COLON", "NAME", "EOF",
    ]);
  });
});

// ============================================================================
// Delimiters
// ============================================================================

describe("delimiters", () => {
  it("tokenizes curly braces", () => {
    const types = tokenTypes("{ }");
    expect(types).toEqual(["LBRACE", "RBRACE", "EOF"]);
  });

  it("tokenizes square brackets", () => {
    const types = tokenTypes("[ ]");
    expect(types).toEqual(["LBRACKET", "RBRACKET", "EOF"]);
  });

  it("tokenizes semicolons and commas", () => {
    const types = tokenTypes("; ,");
    expect(types).toEqual(["SEMICOLON", "COMMA", "EOF"]);
  });

  it("tokenizes dot operator", () => {
    const types = tokenTypes("a.b");
    expect(types).toEqual(["NAME", "DOT", "NAME", "EOF"]);
  });
});

// ============================================================================
// Literals
// ============================================================================

describe("literals", () => {
  it("tokenizes double-quoted string literals", () => {
    const tokens = tokenizeEs1('"hello"');
    expect(tokens[0].type).toBe("STRING");
    expect(tokens[0].value).toBe("hello");
  });

  it("tokenizes single-quoted string literals", () => {
    const tokens = tokenizeEs1("'world'");
    expect(tokens[0].type).toBe("STRING");
    expect(tokens[0].value).toBe("world");
  });

  it("tokenizes integer numbers", () => {
    const tokens = tokenizeEs1("42");
    expect(tokens[0].type).toBe("NUMBER");
    expect(tokens[0].value).toBe("42");
  });

  it("tokenizes floating point numbers", () => {
    const tokens = tokenizeEs1("3.14");
    expect(tokens[0].type).toBe("NUMBER");
    expect(tokens[0].value).toBe("3.14");
  });

  it("tokenizes hex numbers", () => {
    const tokens = tokenizeEs1("0xFF");
    expect(tokens[0].type).toBe("NUMBER");
    expect(tokens[0].value).toBe("0xFF");
  });

  it("tokenizes leading-dot floats", () => {
    const tokens = tokenizeEs1(".5");
    expect(tokens[0].type).toBe("NUMBER");
  });

  it("tokenizes scientific notation", () => {
    const tokens = tokenizeEs1("1e10");
    expect(tokens[0].type).toBe("NUMBER");
  });

  it("tokenizes identifiers with $", () => {
    const tokens = tokenizeEs1("$foo _bar");
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("$foo");
    expect(tokens[1].type).toBe("NAME");
    expect(tokens[1].value).toBe("_bar");
  });
});

// ============================================================================
// Comments and Whitespace
// ============================================================================

describe("comments and whitespace", () => {
  it("skips single-line comments", () => {
    const types = tokenTypes("var x; // comment");
    expect(types).toEqual(["KEYWORD", "NAME", "SEMICOLON", "EOF"]);
  });

  it("skips block comments", () => {
    const types = tokenTypes("var /* block */ x;");
    expect(types).toEqual(["KEYWORD", "NAME", "SEMICOLON", "EOF"]);
  });

  it("skips whitespace and tabs", () => {
    const types = tokenTypes("  var\tx  ");
    expect(types).toEqual(["KEYWORD", "NAME", "EOF"]);
  });
});

// ============================================================================
// Complete ES1 Statements
// ============================================================================

describe("complete ES1 statements", () => {
  it("tokenizes a function declaration", () => {
    const types = tokenTypes("function add(a, b) { return a + b; }");
    expect(types).toEqual([
      "KEYWORD", "NAME", "LPAREN", "NAME", "COMMA", "NAME", "RPAREN",
      "LBRACE", "KEYWORD", "NAME", "PLUS", "NAME", "SEMICOLON", "RBRACE", "EOF",
    ]);
  });

  it("tokenizes a for loop", () => {
    const types = tokenTypes("for (var i = 0; i < 10; i++) { }");
    expect(types).toEqual([
      "KEYWORD", "LPAREN", "KEYWORD", "NAME", "EQUALS", "NUMBER",
      "SEMICOLON", "NAME", "LESS_THAN", "NUMBER", "SEMICOLON",
      "NAME", "PLUS_PLUS", "RPAREN", "LBRACE", "RBRACE", "EOF",
    ]);
  });

  it("tokenizes property access", () => {
    const types = tokenTypes("obj.prop");
    expect(types).toEqual(["NAME", "DOT", "NAME", "EOF"]);
  });

  it("tokenizes computed property access", () => {
    const types = tokenTypes("obj[0]");
    expect(types).toEqual(["NAME", "LBRACKET", "NUMBER", "RBRACKET", "EOF"]);
  });
});
