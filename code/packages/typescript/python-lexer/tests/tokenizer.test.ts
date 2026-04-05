/**
 * Tests for the Python Lexer (TypeScript).
 *
 * These tests verify that the grammar-driven lexer, when loaded with
 * versioned Python grammar files, correctly tokenizes Python source code.
 *
 * The key insight: **no new lexer code was written**. The same
 * `grammarTokenize` engine that handles any language handles Python —
 * only the grammar file differs.
 */

import { describe, it, expect } from "vitest";
import { tokenizePython, SUPPORTED_VERSIONS } from "../src/tokenizer.js";

// ---------------------------------------------------------------------------
// Helper — extract just the type names for cleaner assertions
// ---------------------------------------------------------------------------

function tokenTypes(source: string, version?: string): string[] {
  return tokenizePython(source, version).map((t) => t.type);
}

function tokenValues(source: string, version?: string): string[] {
  return tokenizePython(source, version).map((t) => t.value);
}

// ---------------------------------------------------------------------------
// Basic Expressions
// ---------------------------------------------------------------------------

describe("basic expressions", () => {
  it("tokenizes x = 1 + 2", () => {
    const types = tokenTypes("x = 1 + 2");
    expect(types).toEqual(["NAME", "EQUALS", "NUMBER", "PLUS", "NUMBER", "EOF"]);
  });

  it("captures correct values for x = 1 + 2", () => {
    const values = tokenValues("x = 1 + 2");
    expect(values).toEqual(["x", "=", "1", "+", "2", ""]);
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

  it("tokenizes equality operator ==", () => {
    const types = tokenTypes("x == 1");
    expect(types).toEqual(["NAME", "EQUALS_EQUALS", "NUMBER", "EOF"]);
  });
});

// ---------------------------------------------------------------------------
// Python Keywords
// ---------------------------------------------------------------------------

describe("Python keywords", () => {
  it("recognizes if as a keyword", () => {
    const tokens = tokenizePython("if");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("if");
  });

  it("recognizes def as a keyword", () => {
    const tokens = tokenizePython("def");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("def");
  });

  it("recognizes True, False, None as keywords", () => {
    const tokens = tokenizePython("True False None");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual(["True", "False", "None"]);
  });

  it("does not treat regular names as keywords", () => {
    const tokens = tokenizePython("foobar");
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("foobar");
  });
});

// ---------------------------------------------------------------------------
// Strings and Numbers
// ---------------------------------------------------------------------------

describe("literals", () => {
  it("tokenizes string literals", () => {
    const tokens = tokenizePython('"hello"');
    expect(tokens[0].type).toBe("STRING");
    expect(tokens[0].value).toBe("hello");
  });

  it("tokenizes multi-digit numbers", () => {
    const tokens = tokenizePython("12345");
    expect(tokens[0].type).toBe("NUMBER");
    expect(tokens[0].value).toBe("12345");
  });
});

// ---------------------------------------------------------------------------
// Multi-line Code
// ---------------------------------------------------------------------------

describe("multi-line code", () => {
  it("tokenizes two lines of assignments", () => {
    const types = tokenTypes("x = 1\ny = 2");
    expect(types).toEqual([
      "NAME", "EQUALS", "NUMBER", "NEWLINE",
      "NAME", "EQUALS", "NUMBER", "EOF",
    ]);
  });
});

// ---------------------------------------------------------------------------
// Function Definition Syntax
// ---------------------------------------------------------------------------

describe("function definition syntax", () => {
  it("tokenizes def foo(x):", () => {
    const types = tokenTypes("def foo(x):");
    expect(types).toEqual([
      "KEYWORD", "NAME", "LPAREN", "NAME", "RPAREN", "COLON", "EOF",
    ]);
  });
});

// ---------------------------------------------------------------------------
// Version Support
// ---------------------------------------------------------------------------

describe("version support", () => {
  it("exports all supported versions", () => {
    expect(SUPPORTED_VERSIONS).toEqual(["2.7", "3.0", "3.6", "3.8", "3.10", "3.12"]);
  });

  it("defaults to 3.12 when no version is specified", () => {
    // Both calls should produce the same output
    const withDefault = tokenTypes("x = 1");
    const explicit = tokenTypes("x = 1", "3.12");
    expect(withDefault).toEqual(explicit);
  });

  it("loads each supported version without error", () => {
    for (const version of SUPPORTED_VERSIONS) {
      // Smoke test: every version should be able to tokenize a simple expression
      const tokens = tokenizePython("x = 1", version);
      expect(tokens.length).toBeGreaterThan(0);
      expect(tokens[tokens.length - 1].type).toBe("EOF");
    }
  });

  it("tokenizes with Python 2.7 grammar", () => {
    const tokens = tokenizePython("x = 1", "2.7");
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("x");
  });

  it("caches grammars across calls", () => {
    // Calling twice with the same version should reuse the cached grammar.
    // We verify this indirectly by checking that both calls succeed and
    // produce identical results.
    const first = tokenTypes("a + b", "3.8");
    const second = tokenTypes("a + b", "3.8");
    expect(first).toEqual(second);
  });
});
