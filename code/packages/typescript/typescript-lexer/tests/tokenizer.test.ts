/**
 * Tests for the TypeScript Lexer (TypeScript).
 *
 * These tests verify that the grammar-driven lexer correctly tokenizes
 * TypeScript source code when loaded with the `typescript.tokens` grammar file.
 *
 * TypeScript extends JavaScript with additional features:
 * - `interface`, `type`, `enum` keywords for type system constructs
 * - Type annotation keywords like `number`, `string`, `boolean`
 * - `readonly`, `abstract`, `implements` keywords
 * - All JavaScript features (`let`, `const`, `===`, `!==`, etc.) carry over
 */

import { describe, it, expect } from "vitest";
import { tokenizeTypescript } from "../src/tokenizer.js";

function tokenTypes(source: string): string[] {
  return tokenizeTypescript(source).map((t) => t.type);
}

function tokenValues(source: string): string[] {
  return tokenizeTypescript(source).map((t) => t.value);
}

describe("basic expressions", () => {
  it("tokenizes let x = 1 + 2;", () => {
    const values = tokenValues("let x = 1 + 2;");
    expect(values).toEqual(["let", "x", "=", "1", "+", "2", ";", ""]);
    // Verify known token types
    const tokens = tokenizeTypescript("let x = 1 + 2;");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[1].type).toBe("NAME");
    expect(tokens[2].type).toBe("EQUALS");
    expect(tokens[3].type).toBe("NUMBER");
    expect(tokens[4].type).toBe("PLUS");
    expect(tokens[tokens.length - 1].type).toBe("EOF");
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

describe("TypeScript-specific keywords", () => {
  it("recognizes interface as a keyword", () => {
    const tokens = tokenizeTypescript("interface");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("interface");
  });

  it("recognizes type as a keyword", () => {
    const tokens = tokenizeTypescript("type");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("type");
  });

  it("recognizes number as a keyword", () => {
    const tokens = tokenizeTypescript("number");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("number");
  });

  it("recognizes string as a keyword", () => {
    const tokens = tokenizeTypescript("string");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("string");
  });

  it("recognizes boolean as a keyword", () => {
    const tokens = tokenizeTypescript("boolean");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("boolean");
  });
});

describe("JavaScript keywords (inherited)", () => {
  it("recognizes let as a keyword", () => {
    const tokens = tokenizeTypescript("let");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("let");
  });

  it("recognizes const as a keyword", () => {
    const tokens = tokenizeTypescript("const");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("const");
  });

  it("recognizes function as a keyword", () => {
    const tokens = tokenizeTypescript("function");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("function");
  });

  it("recognizes true, false, null, undefined", () => {
    const tokens = tokenizeTypescript("true false null undefined");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual(["true", "false", "null", "undefined"]);
  });

  it("does not treat regular names as keywords", () => {
    const tokens = tokenizeTypescript("foobar");
    expect(tokens[0].type).toBe("NAME");
  });
});

describe("JavaScript-specific operators", () => {
  it("tokenizes strict equality ===", () => {
    const tokens = tokenizeTypescript("x === 1");
    expect(tokens[1].value).toBe("===");
  });

  it("tokenizes strict inequality !==", () => {
    const tokens = tokenizeTypescript("x !== 1");
    expect(tokens[1].value).toBe("!==");
  });

  it("tokenizes equality == (not strict)", () => {
    const types = tokenTypes("x == 1");
    expect(types).toEqual(["NAME", "EQUALS_EQUALS", "NUMBER", "EOF"]);
  });

  it("tokenizes arrow operator =>", () => {
    const tokens = tokenizeTypescript("x => x");
    expect(tokens[1].value).toBe("=>");
  });
});

describe("delimiters", () => {
  it("tokenizes curly braces", () => {
    const values = tokenValues("{ }");
    expect(values).toEqual(["{", "}", ""]);
  });

  it("tokenizes square brackets", () => {
    const values = tokenValues("[ ]");
    expect(values).toEqual(["[", "]", ""]);
  });

  it("tokenizes semicolons", () => {
    const tokens = tokenizeTypescript(";");
    expect(tokens[0].value).toBe(";");
  });
});

describe("literals", () => {
  it("tokenizes string literals", () => {
    const tokens = tokenizeTypescript('"hello"');
    expect(tokens[0].type).toBe("STRING");
    expect(tokens[0].value).toBe("hello");
  });

  it("tokenizes identifiers with $", () => {
    const tokens = tokenizeTypescript("$foo");
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("$foo");
  });
});
