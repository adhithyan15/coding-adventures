/**
 * Tests for the ECMAScript 3 (1999) Lexer (TypeScript).
 *
 * These tests verify that the grammar-driven lexer correctly tokenizes
 * ES3 source code when loaded with the `es3.tokens` grammar file.
 *
 * ES3 added critical features over ES1:
 * - === and !== (strict equality)
 * - try/catch/finally/throw (error handling keywords)
 * - Regular expression literals (/pattern/flags)
 * - `instanceof` keyword
 */

import { describe, it, expect } from "vitest";
import { tokenizeEs3 } from "../src/tokenizer.js";

/**
 * Helper: extract just the token types from a source string.
 */
function tokenTypes(source: string): string[] {
  return tokenizeEs3(source).map((t) => t.type);
}

/**
 * Helper: extract just the token values from a source string.
 */
function tokenValues(source: string): string[] {
  return tokenizeEs3(source).map((t) => t.value);
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

  it("captures correct values", () => {
    const values = tokenValues("var x = 1 + 2;");
    expect(values).toEqual(["var", "x", "=", "1", "+", "2", ";", ""]);
  });
});

// ============================================================================
// ES3 Strict Equality (NEW in ES3)
// ============================================================================

describe("strict equality operators (new in ES3)", () => {
  it("tokenizes === (strict equals)", () => {
    const tokens = tokenizeEs3("x === 1");
    expect(tokens[1].type).toBe("STRICT_EQUALS");
    expect(tokens[1].value).toBe("===");
  });

  it("tokenizes !== (strict not equals)", () => {
    const tokens = tokenizeEs3("x !== 1");
    expect(tokens[1].type).toBe("STRICT_NOT_EQUALS");
    expect(tokens[1].value).toBe("!==");
  });

  it("still has == and != (abstract equality)", () => {
    const types1 = tokenTypes("x == 1");
    expect(types1).toEqual(["NAME", "EQUALS_EQUALS", "NUMBER", "EOF"]);

    const types2 = tokenTypes("x != 1");
    expect(types2).toEqual(["NAME", "NOT_EQUALS", "NUMBER", "EOF"]);
  });

  it("distinguishes === from == correctly", () => {
    // === should be one token, not == + =
    const tokens = tokenizeEs3("a === b == c");
    expect(tokens[1].type).toBe("STRICT_EQUALS");
    expect(tokens[3].type).toBe("EQUALS_EQUALS");
  });
});

// ============================================================================
// ES3 Error Handling Keywords (NEW in ES3)
// ============================================================================

describe("error handling keywords (new in ES3)", () => {
  it("recognizes try as a keyword", () => {
    const tokens = tokenizeEs3("try");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("try");
  });

  it("recognizes catch as a keyword", () => {
    const tokens = tokenizeEs3("catch");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("catch");
  });

  it("recognizes finally as a keyword", () => {
    const tokens = tokenizeEs3("finally");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("finally");
  });

  it("recognizes throw as a keyword", () => {
    const tokens = tokenizeEs3("throw");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("throw");
  });

  it("recognizes instanceof as a keyword", () => {
    const tokens = tokenizeEs3("instanceof");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("instanceof");
  });

  it("tokenizes a try/catch block", () => {
    const types = tokenTypes("try { } catch (e) { }");
    expect(types).toEqual([
      "KEYWORD", "LBRACE", "RBRACE",
      "KEYWORD", "LPAREN", "NAME", "RPAREN", "LBRACE", "RBRACE",
      "EOF",
    ]);
  });
});

// ============================================================================
// ES1 Features Still Present
// ============================================================================

describe("ES1 features still present", () => {
  it("recognizes var as a keyword", () => {
    const tokens = tokenizeEs3("var");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("var");
  });

  it("recognizes all ES1 control flow keywords", () => {
    const source = "if else while do for switch case break continue return";
    const tokens = tokenizeEs3(source);
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords).toHaveLength(10);
  });

  it("tokenizes all arithmetic operators", () => {
    const types = tokenTypes("a + b - c * d / e % f");
    expect(types).toEqual([
      "NAME", "PLUS", "NAME", "MINUS", "NAME",
      "STAR", "NAME", "SLASH", "NAME", "PERCENT", "NAME", "EOF",
    ]);
  });

  it("tokenizes shift operators", () => {
    const types = tokenTypes("a << b >> c >>> d");
    expect(types).toEqual([
      "NAME", "LEFT_SHIFT", "NAME", "RIGHT_SHIFT", "NAME",
      "UNSIGNED_RIGHT_SHIFT", "NAME", "EOF",
    ]);
  });
});

// ============================================================================
// Literals
// ============================================================================

describe("literals", () => {
  it("tokenizes string literals", () => {
    const tokens = tokenizeEs3('"hello"');
    expect(tokens[0].type).toBe("STRING");
  });

  it("tokenizes identifiers with $", () => {
    const tokens = tokenizeEs3("$foo");
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("$foo");
  });

  it("tokenizes hex numbers", () => {
    const tokens = tokenizeEs3("0xFF");
    expect(tokens[0].type).toBe("NUMBER");
  });

  it("tokenizes floating point numbers", () => {
    const tokens = tokenizeEs3("3.14");
    expect(tokens[0].type).toBe("NUMBER");
  });
});

// ============================================================================
// ES3 Reserved Words
// ============================================================================

describe("ES3 expanded reserved words", () => {
  it("rejects abstract as reserved", () => {
    // Reserved words in ES3 cannot be used as identifiers — the lexer throws
    expect(() => tokenizeEs3("abstract")).toThrow(/Reserved keyword/);
  });

  it("rejects interface as reserved", () => {
    expect(() => tokenizeEs3("interface")).toThrow(/Reserved keyword/);
  });

  it("rejects class as reserved", () => {
    expect(() => tokenizeEs3("class")).toThrow(/Reserved keyword/);
  });
});

// ============================================================================
// Comments
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
});

// ============================================================================
// Complete ES3 Statements
// ============================================================================

describe("complete ES3 statements", () => {
  it("tokenizes a try/catch/finally block", () => {
    const types = tokenTypes("try { x; } catch (e) { y; } finally { z; }");
    expect(types).toEqual([
      "KEYWORD", "LBRACE", "NAME", "SEMICOLON", "RBRACE",
      "KEYWORD", "LPAREN", "NAME", "RPAREN", "LBRACE", "NAME", "SEMICOLON", "RBRACE",
      "KEYWORD", "LBRACE", "NAME", "SEMICOLON", "RBRACE",
      "EOF",
    ]);
  });

  it("tokenizes instanceof expression", () => {
    const types = tokenTypes("x instanceof Array");
    expect(types).toEqual(["NAME", "KEYWORD", "NAME", "EOF"]);
  });
});
