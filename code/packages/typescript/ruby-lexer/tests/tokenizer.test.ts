/**
 * Tests for the Ruby Lexer (TypeScript).
 *
 * These tests verify that the grammar-driven lexer correctly tokenizes Ruby
 * source code when loaded with the `ruby.tokens` grammar file.
 */

import { describe, it, expect } from "vitest";
import { tokenizeRuby } from "../src/tokenizer.js";

function tokenTypes(source: string): string[] {
  return tokenizeRuby(source).map((t) => t.type);
}

function tokenValues(source: string): string[] {
  return tokenizeRuby(source).map((t) => t.value);
}

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
});

describe("Ruby keywords", () => {
  it("recognizes def as a keyword", () => {
    const tokens = tokenizeRuby("def");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("def");
  });

  it("recognizes end as a keyword", () => {
    const tokens = tokenizeRuby("end");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("end");
  });

  it("recognizes puts as a keyword", () => {
    const tokens = tokenizeRuby("puts");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("puts");
  });

  it("recognizes true, false, nil as keywords", () => {
    const tokens = tokenizeRuby("true false nil");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual(["true", "false", "nil"]);
  });
});

describe("Ruby-specific operators", () => {
  it("tokenizes range operator ..", () => {
    const tokens = tokenizeRuby("1..10");
    expect(tokens[1].value).toBe("..");
  });

  it("tokenizes hash rocket =>", () => {
    const tokens = tokenizeRuby("key => value");
    expect(tokens[1].value).toBe("=>");
  });

  it("tokenizes != operator", () => {
    const tokens = tokenizeRuby("x != 1");
    expect(tokens[1].value).toBe("!=");
  });
});

describe("multi-line code", () => {
  it("tokenizes two lines of assignments", () => {
    const types = tokenTypes("x = 1\ny = 2");
    expect(types).toEqual([
      "NAME", "EQUALS", "NUMBER", "NEWLINE",
      "NAME", "EQUALS", "NUMBER", "EOF",
    ]);
  });
});

describe("string literals", () => {
  it("tokenizes a simple string", () => {
    const tokens = tokenizeRuby('"hello"');
    expect(tokens[0].type).toBe("STRING");
    expect(tokens[0].value).toBe("hello");
  });
});
