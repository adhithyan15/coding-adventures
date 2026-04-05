/**
 * Tests for the ECMAScript 5 (2009) Lexer (TypeScript).
 *
 * These tests verify that the grammar-driven lexer correctly tokenizes
 * ES5 source code when loaded with the `es5.tokens` grammar file.
 *
 * ES5 adds over ES3:
 * - `debugger` keyword (promoted from future-reserved)
 * - Getter/setter syntax (semantic, not lexical)
 * - Reduced future-reserved word list
 */

import { describe, it, expect } from "vitest";
import { tokenizeEs5 } from "../src/tokenizer.js";

/**
 * Helper: extract just the token types from a source string.
 */
function tokenTypes(source: string): string[] {
  return tokenizeEs5(source).map((t) => t.type);
}

/**
 * Helper: extract just the token values from a source string.
 */
function tokenValues(source: string): string[] {
  return tokenizeEs5(source).map((t) => t.value);
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
// ES5 debugger Keyword (NEW in ES5)
// ============================================================================

describe("debugger keyword (new in ES5)", () => {
  it("recognizes debugger as a keyword", () => {
    const tokens = tokenizeEs5("debugger");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("debugger");
  });

  it("tokenizes debugger statement", () => {
    const types = tokenTypes("debugger;");
    expect(types).toEqual(["KEYWORD", "SEMICOLON", "EOF"]);
  });
});

// ============================================================================
// ES3 Features Still Present
// ============================================================================

describe("ES3 features still present", () => {
  it("tokenizes strict equality ===", () => {
    const tokens = tokenizeEs5("x === 1");
    expect(tokens[1].type).toBe("STRICT_EQUALS");
    expect(tokens[1].value).toBe("===");
  });

  it("tokenizes strict inequality !==", () => {
    const tokens = tokenizeEs5("x !== 1");
    expect(tokens[1].type).toBe("STRICT_NOT_EQUALS");
    expect(tokens[1].value).toBe("!==");
  });

  it("recognizes try/catch/finally/throw keywords", () => {
    const source = "try catch finally throw";
    const tokens = tokenizeEs5(source);
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual([
      "try", "catch", "finally", "throw",
    ]);
  });

  it("recognizes instanceof keyword", () => {
    const tokens = tokenizeEs5("instanceof");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("instanceof");
  });
});

// ============================================================================
// ES5 Reduced Reserved Words
// ============================================================================

describe("ES5 reduced reserved words", () => {
  it("still reserves class, const, enum, export, extends, import, super", () => {
    // Reserved words throw errors — they cannot be used as identifiers
    expect(() => tokenizeEs5("class")).toThrow(/Reserved keyword/);
    expect(() => tokenizeEs5("const")).toThrow(/Reserved keyword/);
    expect(() => tokenizeEs5("enum")).toThrow(/Reserved keyword/);
    expect(() => tokenizeEs5("export")).toThrow(/Reserved keyword/);
    expect(() => tokenizeEs5("extends")).toThrow(/Reserved keyword/);
    expect(() => tokenizeEs5("import")).toThrow(/Reserved keyword/);
    expect(() => tokenizeEs5("super")).toThrow(/Reserved keyword/);
  });

  it("abstract is NOT reserved in ES5 (was in ES3)", () => {
    // In ES5, abstract is no longer a reserved word — it should be a NAME
    const tokens = tokenizeEs5("abstract");
    expect(tokens[0].type).toBe("NAME");
  });

  it("interface is NOT reserved in ES5 non-strict mode", () => {
    const tokens = tokenizeEs5("interface");
    expect(tokens[0].type).toBe("NAME");
  });
});

// ============================================================================
// Operators
// ============================================================================

describe("operators", () => {
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

  it("tokenizes logical operators", () => {
    const types = tokenTypes("a && b || c");
    expect(types).toEqual([
      "NAME", "AND_AND", "NAME", "OR_OR", "NAME", "EOF",
    ]);
  });

  it("tokenizes compound assignment operators", () => {
    const ops = ["+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>=", ">>>="];
    for (const op of ops) {
      const tokens = tokenizeEs5(`x ${op} 1`);
      expect(tokens[1].value).toBe(op);
    }
  });

  it("tokenizes increment and decrement", () => {
    const types = tokenTypes("x++ y--");
    expect(types).toEqual([
      "NAME", "PLUS_PLUS", "NAME", "MINUS_MINUS", "EOF",
    ]);
  });
});

// ============================================================================
// Literals
// ============================================================================

describe("literals", () => {
  it("tokenizes string literals", () => {
    const tokens = tokenizeEs5('"hello"');
    expect(tokens[0].type).toBe("STRING");
  });

  it("tokenizes single-quoted strings", () => {
    const tokens = tokenizeEs5("'world'");
    expect(tokens[0].type).toBe("STRING");
  });

  it("tokenizes hex numbers", () => {
    const tokens = tokenizeEs5("0xFF");
    expect(tokens[0].type).toBe("NUMBER");
  });

  it("tokenizes floating point numbers", () => {
    const tokens = tokenizeEs5("3.14");
    expect(tokens[0].type).toBe("NUMBER");
  });

  it("tokenizes identifiers with $", () => {
    const tokens = tokenizeEs5("$foo _bar");
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("$foo");
  });
});

// ============================================================================
// Delimiters
// ============================================================================

describe("delimiters", () => {
  it("tokenizes all delimiters", () => {
    const types = tokenTypes("( ) { } [ ] ; , : .");
    expect(types).toEqual([
      "LPAREN", "RPAREN", "LBRACE", "RBRACE", "LBRACKET", "RBRACKET",
      "SEMICOLON", "COMMA", "COLON", "DOT", "EOF",
    ]);
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
// Complete ES5 Statements
// ============================================================================

describe("complete ES5 statements", () => {
  it("tokenizes a getter/setter object literal", () => {
    // get and set are contextual — they are NAME tokens, not keywords
    const source = "var obj = { get x() { return 1; } };";
    const tokens = tokenizeEs5(source);
    const getToken = tokens.find((t) => t.value === "get");
    expect(getToken?.type).toBe("NAME"); // contextual, not keyword
  });

  it("tokenizes debugger in a function", () => {
    const types = tokenTypes("function f() { debugger; }");
    expect(types).toEqual([
      "KEYWORD", "NAME", "LPAREN", "RPAREN", "LBRACE",
      "KEYWORD", "SEMICOLON",
      "RBRACE", "EOF",
    ]);
  });
});
