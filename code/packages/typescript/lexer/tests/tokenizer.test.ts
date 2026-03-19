/**
 * Tests for the Hand-Written Tokenizer
 * ======================================
 *
 * These tests verify that the hand-written `tokenize` function correctly
 * breaks source code into tokens. We test each category of token (numbers,
 * names, strings, operators, delimiters) individually, then test combinations
 * and edge cases.
 *
 * The tests are organized in layers:
 *
 * 1. **Basic token types** — each kind of token in isolation
 * 2. **Keyword handling** — configurable keyword recognition
 * 3. **String handling** — escape sequences, unterminated strings
 * 4. **Position tracking** — line and column numbers
 * 5. **Error handling** — unexpected characters, edge cases
 * 6. **Combination tests** — multiple token types together
 */

import { describe, it, expect } from "vitest";
import { tokenize, LexerError } from "../src/tokenizer.js";
import type { Token } from "../src/token.js";
import type { LexerConfig } from "../src/tokenizer.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Extract just the token types from a list of tokens.
 * This makes assertions more readable when we only care about types.
 */
function types(tokens: Token[]): string[] {
  return tokens.map((t) => t.type);
}

/**
 * Extract just the token values from a list of tokens.
 */
function values(tokens: Token[]): string[] {
  return tokens.map((t) => t.value);
}

// ============================================================================
// Basic Token Types
// ============================================================================

describe("tokenize — basic token types", () => {
  it("should tokenize a single number", () => {
    const tokens = tokenize("42");
    expect(tokens[0]).toEqual({ type: "NUMBER", value: "42", line: 1, column: 1 });
    expect(tokens[1].type).toBe("EOF");
  });

  it("should tokenize multi-digit numbers", () => {
    const tokens = tokenize("1000");
    expect(tokens[0].type).toBe("NUMBER");
    expect(tokens[0].value).toBe("1000");
  });

  it("should tokenize a single-digit number", () => {
    const tokens = tokenize("0");
    expect(tokens[0]).toEqual({ type: "NUMBER", value: "0", line: 1, column: 1 });
  });

  it("should tokenize a simple name", () => {
    const tokens = tokenize("x");
    expect(tokens[0]).toEqual({ type: "NAME", value: "x", line: 1, column: 1 });
  });

  it("should tokenize multi-character names", () => {
    const tokens = tokenize("hello_world");
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("hello_world");
  });

  it("should tokenize names starting with underscore", () => {
    const tokens = tokenize("_foo");
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("_foo");
  });

  it("should tokenize a single underscore as a name", () => {
    const tokens = tokenize("_");
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("_");
  });

  it("should tokenize names with digits", () => {
    const tokens = tokenize("var1");
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("var1");
  });

  it("should tokenize names with mixed case and underscores", () => {
    const tokens = tokenize("hello_world_123");
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("hello_world_123");
  });

  it("should tokenize a string literal", () => {
    const tokens = tokenize('"Hello, World!"');
    expect(tokens[0].type).toBe("STRING");
    expect(tokens[0].value).toBe("Hello, World!");
  });

  it("should tokenize an empty string", () => {
    const tokens = tokenize('""');
    expect(tokens[0].type).toBe("STRING");
    expect(tokens[0].value).toBe("");
  });

  it("should always end with EOF", () => {
    const tokens = tokenize("x");
    expect(tokens[tokens.length - 1].type).toBe("EOF");
    expect(tokens[tokens.length - 1].value).toBe("");
  });

  it("should produce only EOF for empty input", () => {
    const tokens = tokenize("");
    expect(tokens).toHaveLength(1);
    expect(tokens[0].type).toBe("EOF");
  });

  it("should produce only EOF for whitespace-only input", () => {
    const tokens = tokenize("   \t  ");
    expect(tokens).toHaveLength(1);
    expect(tokens[0].type).toBe("EOF");
  });
});

// ============================================================================
// Operators and Delimiters
// ============================================================================

describe("tokenize — operators and delimiters", () => {
  it("should tokenize the + operator", () => {
    const tokens = tokenize("+");
    expect(tokens[0]).toEqual({ type: "PLUS", value: "+", line: 1, column: 1 });
  });

  it("should tokenize the - operator", () => {
    const tokens = tokenize("-");
    expect(tokens[0]).toEqual({ type: "MINUS", value: "-", line: 1, column: 1 });
  });

  it("should tokenize the * operator", () => {
    const tokens = tokenize("*");
    expect(tokens[0]).toEqual({ type: "STAR", value: "*", line: 1, column: 1 });
  });

  it("should tokenize the / operator", () => {
    const tokens = tokenize("/");
    expect(tokens[0]).toEqual({ type: "SLASH", value: "/", line: 1, column: 1 });
  });

  it("should tokenize = (assignment)", () => {
    const tokens = tokenize("=");
    expect(tokens[0]).toEqual({ type: "EQUALS", value: "=", line: 1, column: 1 });
  });

  it("should tokenize == (comparison)", () => {
    const tokens = tokenize("==");
    expect(tokens[0]).toEqual({ type: "EQUALS_EQUALS", value: "==", line: 1, column: 1 });
  });

  it("should distinguish = from ==", () => {
    const tokens = tokenize("a = b == c");
    expect(types(tokens)).toEqual([
      "NAME", "EQUALS", "NAME", "EQUALS_EQUALS", "NAME", "EOF",
    ]);
  });

  it("should tokenize ( and )", () => {
    const tokens = tokenize("()");
    expect(tokens[0].type).toBe("LPAREN");
    expect(tokens[1].type).toBe("RPAREN");
  });

  it("should tokenize comma", () => {
    const tokens = tokenize(",");
    expect(tokens[0]).toEqual({ type: "COMMA", value: ",", line: 1, column: 1 });
  });

  it("should tokenize colon", () => {
    const tokens = tokenize(":");
    expect(tokens[0]).toEqual({ type: "COLON", value: ":", line: 1, column: 1 });
  });

  it("should tokenize all operators without spaces", () => {
    const tokens = tokenize("+-*/");
    expect(types(tokens)).toEqual(["PLUS", "MINUS", "STAR", "SLASH", "EOF"]);
  });
});

// ============================================================================
// Keyword Handling
// ============================================================================

describe("tokenize — keyword handling", () => {
  const pythonConfig: LexerConfig = {
    keywords: [
      "if", "else", "elif", "while", "for", "def", "return",
      "class", "import", "from", "as", "True", "False", "None",
    ],
  };

  it("should classify keywords as KEYWORD", () => {
    const tokens = tokenize("if", pythonConfig);
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("if");
  });

  it("should classify def as KEYWORD", () => {
    const tokens = tokenize("def", pythonConfig);
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("def");
  });

  it("should classify return as KEYWORD", () => {
    const tokens = tokenize("return", pythonConfig);
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("return");
  });

  it("should not classify keyword-like names as KEYWORD", () => {
    const tokens = tokenize("iffy", pythonConfig);
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("iffy");
  });

  it("should not classify if_condition as KEYWORD", () => {
    const tokens = tokenize("if_condition", pythonConfig);
    expect(tokens[0].type).toBe("NAME");
  });

  it("should treat all words as NAME when no config given", () => {
    const tokens = tokenize("if else while");
    expect(types(tokens)).toEqual(["NAME", "NAME", "NAME", "EOF"]);
  });

  it("should handle keyword in expression context", () => {
    const tokens = tokenize("if x == 1", pythonConfig);
    expect(types(tokens)).toEqual([
      "KEYWORD", "NAME", "EQUALS_EQUALS", "NUMBER", "EOF",
    ]);
  });

  it("should handle return in expression context", () => {
    const tokens = tokenize("return x + 1", pythonConfig);
    expect(types(tokens)).toEqual([
      "KEYWORD", "NAME", "PLUS", "NUMBER", "EOF",
    ]);
  });
});

// ============================================================================
// String Handling
// ============================================================================

describe("tokenize — string handling", () => {
  it("should handle escape sequence \\n", () => {
    const tokens = tokenize('"hello\\nworld"');
    expect(tokens[0].value).toBe("hello\nworld");
  });

  it("should handle escape sequence \\t", () => {
    const tokens = tokenize('"col1\\tcol2"');
    expect(tokens[0].value).toBe("col1\tcol2");
  });

  it("should handle escape sequence \\\\", () => {
    const tokens = tokenize('"path\\\\to\\\\file"');
    expect(tokens[0].value).toBe("path\\to\\file");
  });

  it("should handle escape sequence \\\\ for quotes", () => {
    const tokens = tokenize('"He said \\"hi\\""');
    expect(tokens[0].value).toBe('He said "hi"');
  });

  it("should handle unknown escape sequences by passing through", () => {
    const tokens = tokenize('"hello\\xworld"');
    expect(tokens[0].value).toBe("helloxworld");
  });

  it("should handle string with spaces", () => {
    const tokens = tokenize('"abc 123"');
    expect(tokens[0].type).toBe("STRING");
    expect(tokens[0].value).toBe("abc 123");
  });

  it("should throw on unterminated string", () => {
    expect(() => tokenize('"hello')).toThrow(LexerError);
    expect(() => tokenize('"hello')).toThrow("Unterminated string literal");
  });

  it("should throw on string ending with backslash", () => {
    expect(() => tokenize('"hello\\')).toThrow(LexerError);
  });

  it("should tokenize two adjacent strings", () => {
    const tokens = tokenize('"a" "b"');
    expect(tokens[0].type).toBe("STRING");
    expect(tokens[0].value).toBe("a");
    expect(tokens[1].type).toBe("STRING");
    expect(tokens[1].value).toBe("b");
  });
});

// ============================================================================
// Position Tracking
// ============================================================================

describe("tokenize — position tracking", () => {
  it("should track column positions for simple expression", () => {
    const tokens = tokenize("x = 1");
    expect(tokens[0].line).toBe(1);
    expect(tokens[0].column).toBe(1); // x
    expect(tokens[1].column).toBe(3); // =
    expect(tokens[2].column).toBe(5); // 1
  });

  it("should track line numbers across newlines", () => {
    const tokens = tokenize("x = 1\ny = 2");
    // x: line 1, col 1
    expect(tokens[0]).toEqual({ type: "NAME", value: "x", line: 1, column: 1 });
    // NEWLINE
    expect(tokens[3].type).toBe("NEWLINE");
    // y: line 2, col 1
    const yToken = tokens.find((t) => t.value === "y");
    expect(yToken?.line).toBe(2);
    expect(yToken?.column).toBe(1);
  });

  it("should track position across multi-character tokens", () => {
    const tokens = tokenize("abc de");
    expect(tokens[0].column).toBe(1); // abc starts at col 1
    expect(tokens[1].column).toBe(5); // de starts at col 5
  });

  it("should place EOF at position after last character", () => {
    const tokens = tokenize("ab");
    const eof = tokens[tokens.length - 1];
    expect(eof.type).toBe("EOF");
    expect(eof.line).toBe(1);
    expect(eof.column).toBe(3);
  });

  it("should produce NEWLINE tokens", () => {
    const tokens = tokenize("x\ny");
    expect(types(tokens)).toEqual(["NAME", "NEWLINE", "NAME", "EOF"]);
    expect(tokens[1].value).toBe("\\n");
  });

  it("should handle consecutive newlines", () => {
    const tokens = tokenize("x\n\ny");
    expect(types(tokens)).toEqual([
      "NAME", "NEWLINE", "NEWLINE", "NAME", "EOF",
    ]);
  });

  it("should handle only newlines", () => {
    const tokens = tokenize("\n\n");
    expect(types(tokens)).toEqual(["NEWLINE", "NEWLINE", "EOF"]);
  });
});

// ============================================================================
// Error Handling
// ============================================================================

describe("tokenize — error handling", () => {
  it("should throw on unexpected character @", () => {
    expect(() => tokenize("@")).toThrow(LexerError);
    expect(() => tokenize("@")).toThrow("Unexpected character");
  });

  it("should throw on unexpected character #", () => {
    expect(() => tokenize("#")).toThrow(LexerError);
  });

  it("should report correct position for error", () => {
    try {
      tokenize("x = @");
      expect.unreachable("Should have thrown");
    } catch (e) {
      expect(e).toBeInstanceOf(LexerError);
      expect((e as LexerError).line).toBe(1);
      expect((e as LexerError).column).toBe(5);
    }
  });

  it("should report correct position for error on second line", () => {
    try {
      tokenize("x = 1\n@");
      expect.unreachable("Should have thrown");
    } catch (e) {
      expect(e).toBeInstanceOf(LexerError);
      expect((e as LexerError).line).toBe(2);
      expect((e as LexerError).column).toBe(1);
    }
  });
});

// ============================================================================
// Combination Tests
// ============================================================================

describe("tokenize — combinations", () => {
  it("should tokenize a simple assignment: x = 1 + 2", () => {
    const tokens = tokenize("x = 1 + 2");
    expect(types(tokens)).toEqual([
      "NAME", "EQUALS", "NUMBER", "PLUS", "NUMBER", "EOF",
    ]);
    expect(values(tokens)).toEqual(["x", "=", "1", "+", "2", ""]);
  });

  it("should tokenize arithmetic: 1 + 2 * 3", () => {
    const tokens = tokenize("1 + 2 * 3");
    expect(types(tokens)).toEqual([
      "NUMBER", "PLUS", "NUMBER", "STAR", "NUMBER", "EOF",
    ]);
  });

  it("should tokenize function call style: print(x, y)", () => {
    const tokens = tokenize("print(x, y)");
    expect(types(tokens)).toEqual([
      "NAME", "LPAREN", "NAME", "COMMA", "NAME", "RPAREN", "EOF",
    ]);
  });

  it("should tokenize tokens without spaces: x=1+2", () => {
    const tokens = tokenize("x=1+2");
    expect(types(tokens)).toEqual([
      "NAME", "EQUALS", "NUMBER", "PLUS", "NUMBER", "EOF",
    ]);
  });

  it("should tokenize multi-line code", () => {
    const tokens = tokenize("x = 1\ny = 2");
    expect(types(tokens)).toEqual([
      "NAME", "EQUALS", "NUMBER", "NEWLINE",
      "NAME", "EQUALS", "NUMBER", "EOF",
    ]);
  });

  it("should tokenize three-line code", () => {
    const tokens = tokenize("a = 1\nb = 2\nc = a + b");
    expect(types(tokens)).toEqual([
      "NAME", "EQUALS", "NUMBER", "NEWLINE",
      "NAME", "EQUALS", "NUMBER", "NEWLINE",
      "NAME", "EQUALS", "NAME", "PLUS", "NAME", "EOF",
    ]);
  });

  it("should tokenize string in expression: x = \"hello\"", () => {
    const tokens = tokenize('x = "hello"');
    expect(types(tokens)).toEqual(["NAME", "EQUALS", "STRING", "EOF"]);
    expect(tokens[2].value).toBe("hello");
  });

  it("should tokenize key: value", () => {
    const tokens = tokenize("key: value");
    expect(types(tokens)).toEqual(["NAME", "COLON", "NAME", "EOF"]);
  });

  it("should handle leading and trailing whitespace", () => {
    const tokens = tokenize("  x   =   1  ");
    expect(types(tokens)).toEqual(["NAME", "EQUALS", "NUMBER", "EOF"]);
  });

  it("should handle tabs", () => {
    const tokens = tokenize("\tx");
    expect(types(tokens)).toEqual(["NAME", "EOF"]);
  });

  it("should handle carriage returns", () => {
    const tokens = tokenize("x\r= 1");
    expect(types(tokens)).toEqual(["NAME", "EQUALS", "NUMBER", "EOF"]);
  });

  it("should tokenize parenthesized expression", () => {
    const tokens = tokenize("(1 + 2)");
    expect(types(tokens)).toEqual([
      "LPAREN", "NUMBER", "PLUS", "NUMBER", "RPAREN", "EOF",
    ]);
  });
});
