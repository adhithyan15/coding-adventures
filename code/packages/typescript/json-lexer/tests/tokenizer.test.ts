/**
 * Tests for the JSON Lexer (TypeScript).
 *
 * These tests verify that the grammar-driven lexer correctly tokenizes JSON
 * text when loaded with the `json.tokens` grammar file.
 *
 * JSON (RFC 8259) is the simplest practical grammar for the grammar-driven
 * infrastructure. It has no keywords, no comments, no identifiers, and no
 * significant whitespace. This makes it an ideal first test case -- if the
 * generic engine handles JSON correctly, the fundamentals work.
 *
 * Test Categories
 * ---------------
 *
 *   1. **Number tokens** -- integers, negatives, decimals, exponents
 *   2. **String tokens** -- simple strings, escape sequences
 *   3. **Literal tokens** -- true, false, null
 *   4. **Structural tokens** -- braces, brackets, colons, commas
 *   5. **Complete JSON objects** -- key-value pairs, nested structures
 *   6. **Complete JSON arrays** -- flat arrays, mixed-type arrays
 *   7. **Nested structures** -- objects in arrays, arrays in objects
 *   8. **Whitespace handling** -- spaces, tabs, newlines are skipped
 *   9. **Position tracking** -- line and column numbers
 *  10. **Error cases** -- invalid input that should cause lexer errors
 */

import { describe, it, expect } from "vitest";
import { tokenizeJSON } from "../src/tokenizer.js";

/**
 * Helper: extract just the token types from a JSON string.
 * This makes assertions concise -- we can compare arrays of type strings
 * instead of inspecting full Token objects.
 */
function tokenTypes(source: string): string[] {
  return tokenizeJSON(source).map((t) => t.type);
}

/**
 * Helper: extract just the token values from a JSON string.
 * Useful for verifying that the lexer captures the correct text for each token.
 */
function tokenValues(source: string): string[] {
  return tokenizeJSON(source).map((t) => t.value);
}

describe("number tokens", () => {
  it("tokenizes a simple integer", () => {
    /**
     * The simplest JSON number: a plain integer with no sign, no decimal,
     * no exponent. The json.tokens NUMBER regex matches this as one token.
     */
    const tokens = tokenizeJSON("42");
    expect(tokens[0].type).toBe("NUMBER");
    expect(tokens[0].value).toBe("42");
  });

  it("tokenizes zero", () => {
    /**
     * Zero is a special case in the NUMBER regex: the integer part is
     * (0|[1-9][0-9]*), so "0" matches the first alternative. Leading
     * zeros like "007" are NOT valid JSON numbers.
     */
    const tokens = tokenizeJSON("0");
    expect(tokens[0].type).toBe("NUMBER");
    expect(tokens[0].value).toBe("0");
  });

  it("tokenizes a negative integer", () => {
    /**
     * In JSON, the minus sign is part of the number literal, not a
     * separate operator. The NUMBER regex starts with -? to allow this.
     * This is different from programming languages where -42 would be
     * parsed as MINUS INT(42).
     */
    const tokens = tokenizeJSON("-42");
    expect(tokens[0].type).toBe("NUMBER");
    expect(tokens[0].value).toBe("-42");
  });

  it("tokenizes a decimal number", () => {
    /**
     * Decimal numbers have an optional fractional part: (\.[0-9]+)?
     * Note that JSON requires digits after the decimal point -- "3." is
     * not valid JSON, but "3.0" and "3.14" are.
     */
    const tokens = tokenizeJSON("3.14");
    expect(tokens[0].type).toBe("NUMBER");
    expect(tokens[0].value).toBe("3.14");
  });

  it("tokenizes a negative decimal number", () => {
    /**
     * Combining negative sign with decimal part.
     */
    const tokens = tokenizeJSON("-0.5");
    expect(tokens[0].type).toBe("NUMBER");
    expect(tokens[0].value).toBe("-0.5");
  });

  it("tokenizes a number with exponent", () => {
    /**
     * Scientific notation uses e or E followed by an optional sign and
     * digits: ([eE][+-]?[0-9]+)?
     *
     * 1e10 means 1 * 10^10 = 10000000000
     */
    const tokens = tokenizeJSON("1e10");
    expect(tokens[0].type).toBe("NUMBER");
    expect(tokens[0].value).toBe("1e10");
  });

  it("tokenizes a number with negative exponent", () => {
    /**
     * Negative exponents represent very small numbers:
     * 2.5e-3 = 2.5 * 10^(-3) = 0.0025
     */
    const tokens = tokenizeJSON("2.5e-3");
    expect(tokens[0].type).toBe("NUMBER");
    expect(tokens[0].value).toBe("2.5e-3");
  });

  it("tokenizes a number with positive exponent and capital E", () => {
    /**
     * Both 'e' and 'E' are valid for the exponent marker. The '+' sign
     * is optional but explicit: 1E+10 is the same as 1E10.
     */
    const tokens = tokenizeJSON("1E+10");
    expect(tokens[0].type).toBe("NUMBER");
    expect(tokens[0].value).toBe("1E+10");
  });
});

describe("string tokens", () => {
  it("tokenizes a simple double-quoted string", () => {
    /**
     * JSON strings must be double-quoted (single quotes are not allowed).
     * The STRING regex matches the entire string including the quotes.
     */
    const tokens = tokenizeJSON('"hello"');
    expect(tokens[0].type).toBe("STRING");
  });

  it("tokenizes an empty string", () => {
    /**
     * An empty string is just two double quotes with nothing between them.
     * The regex ([^"\\]|\\...)* matches zero characters, which is valid.
     */
    const tokens = tokenizeJSON('""');
    expect(tokens[0].type).toBe("STRING");
  });

  it("tokenizes a string with escape sequences", () => {
    /**
     * JSON supports these escape sequences: \" \\ \/ \b \f \n \r \t \uXXXX
     * The regex allows any of these after a backslash.
     */
    const tokens = tokenizeJSON('"hello\\nworld"');
    expect(tokens[0].type).toBe("STRING");
  });

  it("tokenizes a string with unicode escape", () => {
    /**
     * Unicode escapes use \\uXXXX where X is a hex digit.
     * For example, \\u0041 represents the letter 'A'.
     */
    const tokens = tokenizeJSON('"\\u0041"');
    expect(tokens[0].type).toBe("STRING");
  });
});

describe("literal tokens", () => {
  it("tokenizes true", () => {
    /**
     * In JSON, `true` is a literal value, not a keyword reclassified from
     * an identifier. It has its own token type: TRUE.
     */
    const tokens = tokenizeJSON("true");
    expect(tokens[0].type).toBe("TRUE");
    expect(tokens[0].value).toBe("true");
  });

  it("tokenizes false", () => {
    /**
     * Like `true`, `false` has its own dedicated token type: FALSE.
     */
    const tokens = tokenizeJSON("false");
    expect(tokens[0].type).toBe("FALSE");
    expect(tokens[0].value).toBe("false");
  });

  it("tokenizes null", () => {
    /**
     * `null` represents the absence of a value. It has its own token
     * type: NULL. In JSON, null is the only way to represent "nothing".
     */
    const tokens = tokenizeJSON("null");
    expect(tokens[0].type).toBe("NULL");
    expect(tokens[0].value).toBe("null");
  });
});

describe("structural tokens", () => {
  it("tokenizes object delimiters", () => {
    /**
     * Objects are wrapped in curly braces: { }
     * An empty object {} produces: LBRACE, RBRACE, EOF
     */
    const types = tokenTypes("{}");
    expect(types).toEqual(["LBRACE", "RBRACE", "EOF"]);
  });

  it("tokenizes array delimiters", () => {
    /**
     * Arrays are wrapped in square brackets: [ ]
     * An empty array [] produces: LBRACKET, RBRACKET, EOF
     */
    const types = tokenTypes("[]");
    expect(types).toEqual(["LBRACKET", "RBRACKET", "EOF"]);
  });

  it("tokenizes colon and comma", () => {
    /**
     * Colons separate keys from values in objects.
     * Commas separate elements in both objects and arrays.
     *
     * {"a": 1, "b": 2} uses both colons and commas.
     */
    const types = tokenTypes('{"a": 1, "b": 2}');
    expect(types).toContain("COLON");
    expect(types).toContain("COMMA");
  });
});

describe("complete JSON objects", () => {
  it("tokenizes an empty object", () => {
    /**
     * The simplest valid JSON object: {}
     */
    const types = tokenTypes("{}");
    expect(types).toEqual(["LBRACE", "RBRACE", "EOF"]);
  });

  it("tokenizes an object with one key-value pair", () => {
    /**
     * A JSON object with a single key-value pair:
     *   {"name": "Alice"}
     *
     * Token sequence: { STRING("name") : STRING("Alice") } EOF
     */
    const types = tokenTypes('{"name": "Alice"}');
    expect(types).toEqual([
      "LBRACE", "STRING", "COLON", "STRING", "RBRACE", "EOF",
    ]);
  });

  it("tokenizes an object with multiple key-value pairs", () => {
    /**
     * Multiple pairs separated by commas:
     *   {"name": "Alice", "age": 30}
     */
    const types = tokenTypes('{"name": "Alice", "age": 30}');
    expect(types).toEqual([
      "LBRACE", "STRING", "COLON", "STRING", "COMMA",
      "STRING", "COLON", "NUMBER", "RBRACE", "EOF",
    ]);
  });

  it("tokenizes an object with different value types", () => {
    /**
     * JSON objects can contain any value type: strings, numbers,
     * booleans, null, arrays, and nested objects.
     */
    const source = '{"s": "text", "n": 42, "b": true, "x": null}';
    const types = tokenTypes(source);
    expect(types).toContain("STRING");
    expect(types).toContain("NUMBER");
    expect(types).toContain("TRUE");
    expect(types).toContain("NULL");
  });
});

describe("complete JSON arrays", () => {
  it("tokenizes an empty array", () => {
    /**
     * The simplest valid JSON array: []
     */
    const types = tokenTypes("[]");
    expect(types).toEqual(["LBRACKET", "RBRACKET", "EOF"]);
  });

  it("tokenizes an array of numbers", () => {
    /**
     * A flat array of integers:
     *   [1, 2, 3]
     */
    const types = tokenTypes("[1, 2, 3]");
    expect(types).toEqual([
      "LBRACKET", "NUMBER", "COMMA", "NUMBER", "COMMA", "NUMBER",
      "RBRACKET", "EOF",
    ]);
  });

  it("tokenizes an array of mixed types", () => {
    /**
     * JSON arrays can contain elements of different types:
     *   [1, "two", true, null]
     */
    const types = tokenTypes('[1, "two", true, null]');
    expect(types).toEqual([
      "LBRACKET", "NUMBER", "COMMA", "STRING", "COMMA",
      "TRUE", "COMMA", "NULL", "RBRACKET", "EOF",
    ]);
  });
});

describe("nested structures", () => {
  it("tokenizes nested objects", () => {
    /**
     * Objects can contain other objects as values:
     *   {"outer": {"inner": 1}}
     */
    const types = tokenTypes('{"outer": {"inner": 1}}');
    expect(types).toEqual([
      "LBRACE", "STRING", "COLON",
      "LBRACE", "STRING", "COLON", "NUMBER", "RBRACE",
      "RBRACE", "EOF",
    ]);
  });

  it("tokenizes nested arrays", () => {
    /**
     * Arrays can contain other arrays:
     *   [[1, 2], [3, 4]]
     */
    const types = tokenTypes("[[1, 2], [3, 4]]");
    expect(types).toEqual([
      "LBRACKET",
      "LBRACKET", "NUMBER", "COMMA", "NUMBER", "RBRACKET",
      "COMMA",
      "LBRACKET", "NUMBER", "COMMA", "NUMBER", "RBRACKET",
      "RBRACKET", "EOF",
    ]);
  });

  it("tokenizes an array of objects", () => {
    /**
     * A common pattern in APIs: an array of objects.
     *   [{"id": 1}, {"id": 2}]
     */
    const types = tokenTypes('[{"id": 1}, {"id": 2}]');
    expect(types).toEqual([
      "LBRACKET",
      "LBRACE", "STRING", "COLON", "NUMBER", "RBRACE",
      "COMMA",
      "LBRACE", "STRING", "COLON", "NUMBER", "RBRACE",
      "RBRACKET", "EOF",
    ]);
  });

  it("tokenizes an object with an array value", () => {
    /**
     * Objects can contain arrays as values:
     *   {"items": [1, 2, 3]}
     */
    const types = tokenTypes('{"items": [1, 2, 3]}');
    expect(types).toEqual([
      "LBRACE", "STRING", "COLON",
      "LBRACKET", "NUMBER", "COMMA", "NUMBER", "COMMA", "NUMBER", "RBRACKET",
      "RBRACE", "EOF",
    ]);
  });
});

describe("whitespace handling", () => {
  it("skips spaces between tokens", () => {
    /**
     * JSON whitespace (spaces, tabs, newlines, carriage returns) is
     * insignificant between tokens. The skip pattern in json.tokens
     * matches /[ \\t\\r\\n]+/ and discards it silently.
     */
    const compact = tokenTypes('{"a":1}');
    const spaced = tokenTypes('{ "a" : 1 }');
    expect(compact).toEqual(spaced);
  });

  it("skips newlines and tabs", () => {
    /**
     * Multi-line JSON is common for readability. The lexer engine may
     * emit NEWLINE tokens for line breaks (depending on the engine's
     * default behavior), but the meaningful tokens should be the same
     * as compact JSON. We filter out NEWLINE tokens to verify that the
     * structural and value tokens are identical.
     */
    const source = '{\n\t"key":\n\t\t"value"\n}';
    const types = tokenTypes(source).filter((t) => t !== "NEWLINE");
    const compactTypes = tokenTypes('{"key":"value"}').filter((t) => t !== "NEWLINE");
    expect(types).toEqual(compactTypes);
  });
});

describe("position tracking", () => {
  it("tracks line and column for each token", () => {
    /**
     * Every token includes position information: the line number and
     * column number where it starts. This is essential for error messages
     * that point to the exact location of a problem.
     *
     * Both line and column are 1-indexed (first line is line 1, first
     * column is column 1), following the Token interface convention.
     */
    const tokens = tokenizeJSON('{\n  "key": 1\n}');

    // First token: '{' is at line 1, column 1
    expect(tokens[0].line).toBe(1);
    expect(tokens[0].column).toBe(1);

    // "key" should be on line 2
    const keyToken = tokens.find((t) => t.type === "STRING");
    expect(keyToken).toBeDefined();
    expect(keyToken!.line).toBe(2);
  });

  it("tracks column positions on the same line", () => {
    /**
     * For tokens on the same line, the column number increases
     * as we move right through the source text.
     */
    const tokens = tokenizeJSON("[1, 2]");

    // '[' at column 1
    expect(tokens[0].column).toBe(1);

    // '1' at column 2
    expect(tokens[1].column).toBe(2);
  });
});

describe("error cases", () => {
  it("rejects bare identifiers", () => {
    /**
     * JSON has no identifier/NAME tokens. Bare words like `undefined`
     * or `NaN` are not valid JSON and should cause a lexer error.
     * Only `true`, `false`, and `null` are recognized as literal tokens.
     */
    expect(() => tokenizeJSON("undefined")).toThrow();
  });

  it("rejects single-quoted strings", () => {
    /**
     * JSON requires double quotes for strings. Single-quoted strings
     * are a common mistake when coming from JavaScript or Python.
     */
    expect(() => tokenizeJSON("'hello'")).toThrow();
  });
});
