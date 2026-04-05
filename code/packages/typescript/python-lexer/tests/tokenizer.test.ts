/**
 * Tests for the Python Lexer (TypeScript).
 *
 * These tests verify that the grammar-driven lexer, when loaded with
 * versioned Python grammar files, correctly tokenizes Python source code.
 *
 * The versioned grammars use "INT" and "FLOAT" instead of "NUMBER",
 * and produce NEWLINE tokens (indentation mode is active).
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
    const types = tokenTypes("x = 1 + 2\n");
    expect(types).toEqual(["NAME", "EQUALS", "INT", "PLUS", "INT", "NEWLINE", "EOF"]);
  });

  it("captures correct values for x = 1 + 2", () => {
    const values = tokenValues("x = 1 + 2\n");
    // Filter out synthetic tokens (NEWLINE, EOF) for value checking
    const meaningful = tokenizePython("x = 1 + 2\n").filter(
      (t) => t.type !== "NEWLINE" && t.type !== "EOF"
    );
    expect(meaningful.map((t) => t.value)).toEqual(["x", "=", "1", "+", "2"]);
  });

  it("tokenizes all arithmetic operators", () => {
    const types = tokenTypes("a + b - c * d / e\n");
    expect(types).toEqual([
      "NAME", "PLUS", "NAME", "MINUS", "NAME",
      "STAR", "NAME", "SLASH", "NAME", "NEWLINE", "EOF",
    ]);
  });

  it("tokenizes parenthesized expressions", () => {
    const types = tokenTypes("(1 + 2) * 3\n");
    expect(types).toEqual([
      "LPAREN", "INT", "PLUS", "INT", "RPAREN",
      "STAR", "INT", "NEWLINE", "EOF",
    ]);
  });

  it("tokenizes equality operator ==", () => {
    const types = tokenTypes("x == 1\n");
    expect(types).toEqual(["NAME", "EQUALS_EQUALS", "INT", "NEWLINE", "EOF"]);
  });
});

// ---------------------------------------------------------------------------
// Python Keywords
// ---------------------------------------------------------------------------

describe("Python keywords", () => {
  it("recognizes if as a keyword", () => {
    const tokens = tokenizePython("if x:\n    pass\n");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("if");
  });

  it("recognizes def as a keyword", () => {
    const tokens = tokenizePython("def f():\n    pass\n");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("def");
  });

  it("recognizes True, False, None as keywords", () => {
    const tokens = tokenizePython("True False None\n");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual(["True", "False", "None"]);
  });

  it("does not treat regular names as keywords", () => {
    const tokens = tokenizePython("foobar\n");
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("foobar");
  });
});

// ---------------------------------------------------------------------------
// Strings and Numbers
// ---------------------------------------------------------------------------

describe("literals", () => {
  it("tokenizes string literals", () => {
    const tokens = tokenizePython('"hello"\n');
    expect(tokens[0].type).toBe("STRING");
  });

  it("tokenizes multi-digit integers", () => {
    const tokens = tokenizePython("12345\n");
    expect(tokens[0].type).toBe("INT");
    expect(tokens[0].value).toBe("12345");
  });
});

// ---------------------------------------------------------------------------
// Multi-line Code
// ---------------------------------------------------------------------------

describe("multi-line code", () => {
  it("tokenizes two lines of assignments", () => {
    const types = tokenTypes("x = 1\ny = 2\n");
    expect(types).toEqual([
      "NAME", "EQUALS", "INT", "NEWLINE",
      "NAME", "EQUALS", "INT", "NEWLINE", "EOF",
    ]);
  });
});

// ---------------------------------------------------------------------------
// Function Definition Syntax
// ---------------------------------------------------------------------------

describe("function definition syntax", () => {
  it("tokenizes def foo(x):", () => {
    const types = tokenTypes("def foo(x):\n    pass\n");
    // def=KEYWORD, foo=NAME, (=LPAREN, x=NAME, )=RPAREN, :=COLON, NEWLINE,
    // INDENT, pass=KEYWORD, NEWLINE, DEDENT, EOF
    expect(types[0]).toBe("KEYWORD"); // def
    expect(types[1]).toBe("NAME");    // foo
    expect(types[2]).toBe("LPAREN");
    expect(types[3]).toBe("NAME");    // x
    expect(types[4]).toBe("RPAREN");
    expect(types[5]).toBe("COLON");
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
    const withDefault = tokenTypes("x = 1\n");
    const explicit = tokenTypes("x = 1\n", "3.12");
    expect(withDefault).toEqual(explicit);
  });

  it("loads each supported version without error", () => {
    for (const version of SUPPORTED_VERSIONS) {
      const tokens = tokenizePython("x = 1\n", version);
      expect(tokens.length).toBeGreaterThan(0);
      expect(tokens[tokens.length - 1].type).toBe("EOF");
    }
  });

  it("tokenizes with Python 2.7 grammar", () => {
    const tokens = tokenizePython("x = 1\n", "2.7");
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("x");
  });

  it("Python 2.7 treats print as keyword", () => {
    const tokens = tokenizePython("print x\n", "2.7");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("print");
  });

  it("caches grammars across calls", () => {
    const first = tokenTypes("a + b\n", "3.8");
    const second = tokenTypes("a + b\n", "3.8");
    expect(first).toEqual(second);
  });
});
