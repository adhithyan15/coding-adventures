/**
 * Tests for the TOML Lexer (TypeScript).
 *
 * These tests verify that the grammar-driven lexer correctly tokenizes TOML
 * text when loaded with the `toml.tokens` grammar file.
 *
 * TOML (Tom's Obvious Minimal Language) is significantly more complex to
 * tokenize than JSON because of:
 *   - Four string types (basic, literal, multi-line basic, multi-line literal)
 *   - Date/time literals (offset datetime, local datetime, date, time)
 *   - Multiple integer formats (decimal, hex, octal, binary)
 *   - Special float values (inf, nan)
 *   - Bare keys (unquoted identifiers)
 *   - Newline sensitivity (key-value pairs are newline-delimited)
 *   - Comments (# to end of line)
 *   - Underscore separators in numbers (1_000)
 *
 * Test Categories
 * ---------------
 *
 *   1. **String tokens** -- all four string types
 *   2. **Number tokens** -- integers (all bases) and floats
 *   3. **Boolean tokens** -- true and false
 *   4. **Date/time tokens** -- all four datetime types
 *   5. **Bare key tokens** -- unquoted key names
 *   6. **Structural tokens** -- delimiters (=, ., ,, [], {})
 *   7. **Newline handling** -- NEWLINE tokens between expressions
 *   8. **Comment handling** -- comments are skipped
 *   9. **Complete TOML documents** -- key-value pairs, tables, arrays
 *  10. **Position tracking** -- line and column numbers
 *  11. **Error cases** -- invalid characters
 */

import { describe, it, expect } from "vitest";
import { tokenizeTOML } from "../src/tokenizer.js";

/**
 * Helper: extract just the token types from a TOML string.
 * This makes assertions concise -- we can compare arrays of type strings
 * instead of inspecting full Token objects.
 */
function tokenTypes(source: string): string[] {
  return tokenizeTOML(source).map((t) => t.type);
}

/**
 * Helper: extract just the token values from a TOML string.
 * Useful for verifying that the lexer captures the correct text for each token.
 */
function tokenValues(source: string): string[] {
  return tokenizeTOML(source).map((t) => t.value);
}

/**
 * Helper: extract non-NEWLINE, non-EOF token types for cleaner assertions
 * when we only care about the meaningful tokens.
 */
function meaningfulTypes(source: string): string[] {
  return tokenizeTOML(source)
    .filter((t) => t.type !== "NEWLINE" && t.type !== "EOF")
    .map((t) => t.type);
}

// =========================================================================
// String Tokens
// =========================================================================

describe("basic strings (double-quoted)", () => {
  it("tokenizes a simple basic string", () => {
    /**
     * Basic strings are surrounded by double quotes. They support
     * escape sequences like \n, \t, \\, \", \uXXXX, and \UXXXXXXXX.
     *
     * With escapes: none in toml.tokens, the lexer strips the quotes
     * but leaves escape sequences as raw text for the semantic layer.
     */
    const tokens = tokenizeTOML('"hello"');
    expect(tokens[0].type).toBe("BASIC_STRING");
    expect(tokens[0].value).toBe("hello");
  });

  it("tokenizes an empty basic string", () => {
    /**
     * An empty basic string: "" -- just two double quotes with nothing
     * between them. The lexer should produce a BASIC_STRING with empty value.
     */
    const tokens = tokenizeTOML('""');
    expect(tokens[0].type).toBe("BASIC_STRING");
    expect(tokens[0].value).toBe("");
  });

  it("tokenizes a basic string with escape sequences (raw)", () => {
    /**
     * Because escapes: none is set, the lexer strips quotes but leaves
     * the backslash sequences as-is. The parser/semantic layer is
     * responsible for interpreting \n as a newline character.
     */
    const tokens = tokenizeTOML('"hello\\nworld"');
    expect(tokens[0].type).toBe("BASIC_STRING");
    expect(tokens[0].value).toBe("hello\\nworld");
  });
});

describe("literal strings (single-quoted)", () => {
  it("tokenizes a simple literal string", () => {
    /**
     * Literal strings are surrounded by single quotes. They do NOT
     * support escape sequences -- what you see is what you get.
     * This makes them ideal for Windows paths: 'C:\Users\name'
     */
    const tokens = tokenizeTOML("'hello'");
    expect(tokens[0].type).toBe("LITERAL_STRING");
    expect(tokens[0].value).toBe("hello");
  });

  it("tokenizes a literal string with backslashes (no escaping)", () => {
    /**
     * Backslashes in literal strings are literal -- they are NOT
     * escape characters. 'C:\Users' contains a real backslash.
     */
    const tokens = tokenizeTOML("'C:\\\\Users\\\\name'");
    expect(tokens[0].type).toBe("LITERAL_STRING");
    expect(tokens[0].value).toBe("C:\\\\Users\\\\name");
  });
});

describe("multi-line basic strings (triple double-quoted)", () => {
  it("tokenizes a multi-line basic string", () => {
    /**
     * Multi-line basic strings use triple double quotes: """..."""
     * They can span multiple lines. Escape sequences are supported
     * (but with escapes: none, they are left as raw text).
     */
    const tokens = tokenizeTOML('"""hello\nworld"""');
    expect(tokens[0].type).toBe("ML_BASIC_STRING");
    expect(tokens[0].value).toBe("hello\nworld");
  });

  it("tokenizes a multi-line basic string on one line", () => {
    /**
     * Multi-line basic strings don't have to actually span multiple lines.
     * """hello""" is valid -- it's just a basic string with triple-quote
     * delimiters.
     */
    const tokens = tokenizeTOML('"""hello"""');
    expect(tokens[0].type).toBe("ML_BASIC_STRING");
    expect(tokens[0].value).toBe("hello");
  });
});

describe("multi-line literal strings (triple single-quoted)", () => {
  it("tokenizes a multi-line literal string", () => {
    /**
     * Multi-line literal strings use triple single quotes: '''...'''
     * They can span multiple lines. NO escape processing occurs.
     */
    const tokens = tokenizeTOML("'''hello\nworld'''");
    expect(tokens[0].type).toBe("ML_LITERAL_STRING");
    expect(tokens[0].value).toBe("hello\nworld");
  });
});

// =========================================================================
// Number Tokens
// =========================================================================

describe("integer tokens", () => {
  it("tokenizes a decimal integer", () => {
    /**
     * Simple decimal integers: 42, 0, 1000
     * TOML integers are more flexible than JSON -- they allow leading +
     * signs and underscore separators.
     */
    const tokens = tokenizeTOML("42");
    expect(tokens[0].type).toBe("INTEGER");
    expect(tokens[0].value).toBe("42");
  });

  it("tokenizes zero", () => {
    const tokens = tokenizeTOML("0");
    expect(tokens[0].type).toBe("INTEGER");
    expect(tokens[0].value).toBe("0");
  });

  it("tokenizes a positive integer with explicit sign", () => {
    /**
     * TOML allows explicit + signs on integers, unlike JSON.
     */
    const tokens = tokenizeTOML("+42");
    expect(tokens[0].type).toBe("INTEGER");
    expect(tokens[0].value).toBe("+42");
  });

  it("tokenizes a negative integer", () => {
    const tokens = tokenizeTOML("-42");
    expect(tokens[0].type).toBe("INTEGER");
    expect(tokens[0].value).toBe("-42");
  });

  it("tokenizes an integer with underscore separators", () => {
    /**
     * TOML allows underscores between digits for readability:
     * 1_000_000 is the same as 1000000.
     */
    const tokens = tokenizeTOML("1_000_000");
    expect(tokens[0].type).toBe("INTEGER");
    expect(tokens[0].value).toBe("1_000_000");
  });

  it("tokenizes a hexadecimal integer", () => {
    /**
     * Hex integers start with 0x: 0xDEADBEEF
     * They are aliased to INTEGER in toml.tokens via "-> INTEGER".
     */
    const tokens = tokenizeTOML("0xDEADBEEF");
    expect(tokens[0].type).toBe("INTEGER");
    expect(tokens[0].value).toBe("0xDEADBEEF");
  });

  it("tokenizes an octal integer", () => {
    /**
     * Octal integers start with 0o: 0o755
     */
    const tokens = tokenizeTOML("0o755");
    expect(tokens[0].type).toBe("INTEGER");
    expect(tokens[0].value).toBe("0o755");
  });

  it("tokenizes a binary integer", () => {
    /**
     * Binary integers start with 0b: 0b11010110
     */
    const tokens = tokenizeTOML("0b11010110");
    expect(tokens[0].type).toBe("INTEGER");
    expect(tokens[0].value).toBe("0b11010110");
  });
});

describe("float tokens", () => {
  it("tokenizes a decimal float", () => {
    /**
     * Decimal floats have a fractional part: 3.14
     * They are aliased to FLOAT in toml.tokens.
     */
    const tokens = tokenizeTOML("3.14");
    expect(tokens[0].type).toBe("FLOAT");
    expect(tokens[0].value).toBe("3.14");
  });

  it("tokenizes a float with exponent", () => {
    /**
     * Scientific notation: 1e10, 5e+22, 1e-2
     */
    const tokens = tokenizeTOML("1e10");
    expect(tokens[0].type).toBe("FLOAT");
    expect(tokens[0].value).toBe("1e10");
  });

  it("tokenizes a float with decimal and exponent", () => {
    const tokens = tokenizeTOML("6.626e-34");
    expect(tokens[0].type).toBe("FLOAT");
    expect(tokens[0].value).toBe("6.626e-34");
  });

  it("tokenizes positive infinity", () => {
    /**
     * TOML has special float values: inf and nan.
     * These are NOT bare keys because FLOAT_SPECIAL matches before BARE_KEY.
     */
    const tokens = tokenizeTOML("inf");
    expect(tokens[0].type).toBe("FLOAT");
    expect(tokens[0].value).toBe("inf");
  });

  it("tokenizes negative infinity", () => {
    const tokens = tokenizeTOML("-inf");
    expect(tokens[0].type).toBe("FLOAT");
    expect(tokens[0].value).toBe("-inf");
  });

  it("tokenizes nan", () => {
    const tokens = tokenizeTOML("nan");
    expect(tokens[0].type).toBe("FLOAT");
    expect(tokens[0].value).toBe("nan");
  });

  it("tokenizes a float with underscore separators", () => {
    const tokens = tokenizeTOML("1_000.000_1");
    expect(tokens[0].type).toBe("FLOAT");
    expect(tokens[0].value).toBe("1_000.000_1");
  });
});

// =========================================================================
// Boolean Tokens
// =========================================================================

describe("boolean tokens", () => {
  it("tokenizes true", () => {
    /**
     * In TOML, true is a literal value token. It is NOT a bare key.
     * The lexer matches TRUE = "true" before BARE_KEY = /[A-Za-z0-9_-]+/.
     */
    const tokens = tokenizeTOML("true");
    expect(tokens[0].type).toBe("TRUE");
    expect(tokens[0].value).toBe("true");
  });

  it("tokenizes false", () => {
    const tokens = tokenizeTOML("false");
    expect(tokens[0].type).toBe("FALSE");
    expect(tokens[0].value).toBe("false");
  });
});

// =========================================================================
// Date/Time Tokens
// =========================================================================

describe("date/time tokens", () => {
  it("tokenizes an offset datetime", () => {
    /**
     * Offset datetimes include a timezone offset:
     *   1979-05-27T07:32:00Z (UTC)
     *   1979-05-27T07:32:00+09:00 (JST)
     *
     * They must match before LOCAL_DATETIME, LOCAL_DATE, and INTEGER
     * because 1979-05-27 would match as three integers with minus signs.
     */
    const tokens = tokenizeTOML("1979-05-27T07:32:00Z");
    expect(tokens[0].type).toBe("OFFSET_DATETIME");
    expect(tokens[0].value).toBe("1979-05-27T07:32:00Z");
  });

  it("tokenizes an offset datetime with numeric offset", () => {
    const tokens = tokenizeTOML("1979-05-27T07:32:00+09:00");
    expect(tokens[0].type).toBe("OFFSET_DATETIME");
    expect(tokens[0].value).toBe("1979-05-27T07:32:00+09:00");
  });

  it("tokenizes a local datetime", () => {
    /**
     * Local datetimes have both date and time but no timezone:
     *   1979-05-27T07:32:00
     */
    const tokens = tokenizeTOML("1979-05-27T07:32:00");
    expect(tokens[0].type).toBe("LOCAL_DATETIME");
    expect(tokens[0].value).toBe("1979-05-27T07:32:00");
  });

  it("tokenizes a local date", () => {
    /**
     * Local dates have only the date part:
     *   1979-05-27
     */
    const tokens = tokenizeTOML("1979-05-27");
    expect(tokens[0].type).toBe("LOCAL_DATE");
    expect(tokens[0].value).toBe("1979-05-27");
  });

  it("tokenizes a local time", () => {
    /**
     * Local times have only the time part:
     *   07:32:00
     *   07:32:00.999999
     */
    const tokens = tokenizeTOML("07:32:00");
    expect(tokens[0].type).toBe("LOCAL_TIME");
    expect(tokens[0].value).toBe("07:32:00");
  });

  it("tokenizes a local time with fractional seconds", () => {
    const tokens = tokenizeTOML("07:32:00.999999");
    expect(tokens[0].type).toBe("LOCAL_TIME");
    expect(tokens[0].value).toBe("07:32:00.999999");
  });
});

// =========================================================================
// Bare Key Tokens
// =========================================================================

describe("bare key tokens", () => {
  it("tokenizes a simple bare key", () => {
    /**
     * Bare keys consist of ASCII letters, digits, dashes, and underscores.
     * They match only when no other pattern matches first.
     */
    const tokens = tokenizeTOML("server");
    expect(tokens[0].type).toBe("BARE_KEY");
    expect(tokens[0].value).toBe("server");
  });

  it("tokenizes a bare key with dashes", () => {
    const tokens = tokenizeTOML("my-key");
    expect(tokens[0].type).toBe("BARE_KEY");
    expect(tokens[0].value).toBe("my-key");
  });

  it("tokenizes a bare key with underscores", () => {
    const tokens = tokenizeTOML("my_key");
    expect(tokens[0].type).toBe("BARE_KEY");
    expect(tokens[0].value).toBe("my_key");
  });

  it("tokenizes a bare key with digits", () => {
    const tokens = tokenizeTOML("key123");
    expect(tokens[0].type).toBe("BARE_KEY");
    expect(tokens[0].value).toBe("key123");
  });
});

// =========================================================================
// Structural Tokens
// =========================================================================

describe("structural tokens", () => {
  it("tokenizes equals sign", () => {
    const types = meaningfulTypes("key = value");
    expect(types).toContain("EQUALS");
  });

  it("tokenizes dot", () => {
    const types = meaningfulTypes("a.b");
    expect(types).toContain("DOT");
  });

  it("tokenizes comma", () => {
    const types = meaningfulTypes("[1, 2]");
    expect(types).toContain("COMMA");
  });

  it("tokenizes square brackets", () => {
    const types = meaningfulTypes("[table]");
    expect(types).toContain("LBRACKET");
    expect(types).toContain("RBRACKET");
  });

  it("tokenizes curly braces", () => {
    const types = meaningfulTypes("{ key = 1 }");
    expect(types).toContain("LBRACE");
    expect(types).toContain("RBRACE");
  });
});

// =========================================================================
// Newline Handling
// =========================================================================

describe("newline handling", () => {
  it("emits NEWLINE tokens for line breaks", () => {
    /**
     * TOML is newline-sensitive: key-value pairs are terminated by newlines.
     * The lexer emits NEWLINE tokens so the parser can use them as
     * delimiters between expressions.
     */
    const types = tokenTypes("a = 1\nb = 2");
    expect(types).toContain("NEWLINE");
  });

  it("emits multiple NEWLINE tokens for blank lines", () => {
    /**
     * Blank lines produce consecutive NEWLINE tokens. The parser's
     * document rule handles this: { NEWLINE | expression }.
     */
    const tokens = tokenizeTOML("a = 1\n\nb = 2");
    const newlineCount = tokens.filter((t) => t.type === "NEWLINE").length;
    expect(newlineCount).toBe(2);
  });
});

// =========================================================================
// Comment Handling
// =========================================================================

describe("comment handling", () => {
  it("skips comments", () => {
    /**
     * Comments start with # and extend to the end of the line.
     * The skip: section in toml.tokens defines COMMENT = /#[^\n]* /
     * which consumes the comment text but NOT the newline.
     */
    const types = meaningfulTypes("# this is a comment\nkey = 1");
    expect(types).not.toContain("COMMENT");
    expect(types).toContain("BARE_KEY");
  });

  it("skips inline comments", () => {
    /**
     * Comments can appear at the end of a line after whitespace:
     *   key = "value" # inline comment
     */
    const types = meaningfulTypes('key = "value" # inline comment');
    expect(types).toContain("BARE_KEY");
    expect(types).toContain("EQUALS");
    expect(types).toContain("BASIC_STRING");
  });
});

// =========================================================================
// Complete TOML Documents
// =========================================================================

describe("key-value pairs", () => {
  it("tokenizes a simple key-value pair", () => {
    /**
     * The fundamental TOML construct: key = value
     *   title = "TOML Example"
     *
     * Token sequence: BARE_KEY EQUALS BASIC_STRING EOF
     */
    const types = meaningfulTypes('title = "TOML Example"');
    expect(types).toEqual(["BARE_KEY", "EQUALS", "BASIC_STRING"]);
  });

  it("tokenizes a dotted key-value pair", () => {
    /**
     * Dotted keys create nested tables implicitly:
     *   a.b.c = 1 is equivalent to [a.b]\n c = 1
     */
    const types = meaningfulTypes("a.b.c = 1");
    expect(types).toEqual([
      "BARE_KEY", "DOT", "BARE_KEY", "DOT", "BARE_KEY", "EQUALS", "INTEGER",
    ]);
  });

  it("tokenizes a quoted key", () => {
    /**
     * Keys can be quoted strings:
     *   "key with spaces" = "value"
     */
    const types = meaningfulTypes('"key with spaces" = "value"');
    expect(types).toEqual(["BASIC_STRING", "EQUALS", "BASIC_STRING"]);
  });
});

describe("table headers", () => {
  it("tokenizes a table header", () => {
    /**
     * Table headers switch the current table:
     *   [server]
     *
     * Token sequence: LBRACKET BARE_KEY RBRACKET
     */
    const types = meaningfulTypes("[server]");
    expect(types).toEqual(["LBRACKET", "BARE_KEY", "RBRACKET"]);
  });

  it("tokenizes a dotted table header", () => {
    /**
     * Dotted table headers create nested tables:
     *   [a.b.c]
     */
    const types = meaningfulTypes("[a.b.c]");
    expect(types).toEqual([
      "LBRACKET", "BARE_KEY", "DOT", "BARE_KEY", "DOT", "BARE_KEY", "RBRACKET",
    ]);
  });

  it("tokenizes an array-of-tables header", () => {
    /**
     * Array-of-tables headers use double brackets:
     *   [[products]]
     *
     * This is tokenized as LBRACKET LBRACKET BARE_KEY RBRACKET RBRACKET --
     * four bracket tokens, not two DOUBLE_BRACKET tokens.
     */
    const types = meaningfulTypes("[[products]]");
    expect(types).toEqual([
      "LBRACKET", "LBRACKET", "BARE_KEY", "RBRACKET", "RBRACKET",
    ]);
  });
});

describe("arrays", () => {
  it("tokenizes an inline array of integers", () => {
    const types = meaningfulTypes("[1, 2, 3]");
    expect(types).toEqual([
      "LBRACKET", "INTEGER", "COMMA", "INTEGER", "COMMA", "INTEGER", "RBRACKET",
    ]);
  });

  it("tokenizes an inline array of strings", () => {
    const types = meaningfulTypes('["red", "green", "blue"]');
    expect(types).toEqual([
      "LBRACKET", "BASIC_STRING", "COMMA", "BASIC_STRING", "COMMA",
      "BASIC_STRING", "RBRACKET",
    ]);
  });
});

describe("inline tables", () => {
  it("tokenizes an inline table", () => {
    /**
     * Inline tables are compact, single-line table definitions:
     *   point = { x = 1, y = 2 }
     */
    const types = meaningfulTypes("{ x = 1, y = 2 }");
    expect(types).toEqual([
      "LBRACE", "BARE_KEY", "EQUALS", "INTEGER", "COMMA",
      "BARE_KEY", "EQUALS", "INTEGER", "RBRACE",
    ]);
  });
});

describe("complete TOML documents", () => {
  it("tokenizes a multi-line document with tables and key-value pairs", () => {
    /**
     * A realistic TOML document with a table header and key-value pairs:
     *
     *   [server]
     *   host = "localhost"
     *   port = 8080
     */
    const source = '[server]\nhost = "localhost"\nport = 8080';
    const types = meaningfulTypes(source);
    expect(types).toEqual([
      "LBRACKET", "BARE_KEY", "RBRACKET",
      "BARE_KEY", "EQUALS", "BASIC_STRING",
      "BARE_KEY", "EQUALS", "INTEGER",
    ]);
  });

  it("tokenizes a document with mixed value types", () => {
    /**
     * TOML supports many value types in a single document:
     *   name = "TOML"
     *   version = 1
     *   pi = 3.14
     *   enabled = true
     */
    const source = 'name = "TOML"\nversion = 1\npi = 3.14\nenabled = true';
    const types = meaningfulTypes(source);
    expect(types).toEqual([
      "BARE_KEY", "EQUALS", "BASIC_STRING",
      "BARE_KEY", "EQUALS", "INTEGER",
      "BARE_KEY", "EQUALS", "FLOAT",
      "BARE_KEY", "EQUALS", "TRUE",
    ]);
  });
});

// =========================================================================
// Position Tracking
// =========================================================================

describe("position tracking", () => {
  it("tracks line and column for the first token", () => {
    /**
     * The first token should always be at line 1, column 1.
     */
    const tokens = tokenizeTOML("key = 1");
    expect(tokens[0].line).toBe(1);
    expect(tokens[0].column).toBe(1);
  });

  it("tracks line numbers across newlines", () => {
    /**
     * Tokens after a newline should report the correct line number.
     */
    const tokens = tokenizeTOML("a = 1\nb = 2");
    const bToken = tokens.find(
      (t) => t.type === "BARE_KEY" && t.value === "b",
    );
    expect(bToken).toBeDefined();
    expect(bToken!.line).toBe(2);
  });
});

// =========================================================================
// Edge Cases
// =========================================================================

describe("edge cases", () => {
  it("tokenizes an empty string as just EOF", () => {
    const tokens = tokenizeTOML("");
    expect(tokens).toHaveLength(1);
    expect(tokens[0].type).toBe("EOF");
  });

  it("tokenizes only whitespace as just EOF", () => {
    const tokens = tokenizeTOML("   \t  ");
    expect(tokens).toHaveLength(1);
    expect(tokens[0].type).toBe("EOF");
  });

  it("tokenizes only a comment as NEWLINE + EOF or just EOF", () => {
    /**
     * A line with only a comment should produce no meaningful tokens.
     * The comment is skipped. Whether a NEWLINE is emitted depends on
     * whether the comment is followed by a newline character.
     */
    const types = meaningfulTypes("# just a comment");
    expect(types).toHaveLength(0);
  });
});
