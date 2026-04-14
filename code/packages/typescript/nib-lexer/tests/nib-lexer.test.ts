import { describe, expect, it } from "vitest";

import { tokenizeNib } from "../src/index.js";

function tokenTypes(source: string): string[] {
  return tokenizeNib(source)
    .filter((token) => token.type !== "EOF")
    .map((token) => token.type);
}

describe("nib-lexer", () => {
  it("tokenizes a typed let declaration", () => {
    expect(tokenTypes("let x: u4 = 0xF;")).toEqual([
      "let",
      "NAME",
      "COLON",
      "NAME",
      "EQ",
      "HEX_LIT",
      "SEMICOLON",
    ]);
  });

  it("keeps Nib keywords case-sensitive", () => {
    expect(tokenTypes("fn FN return RETURN")).toEqual([
      "fn",
      "NAME",
      "return",
      "NAME",
    ]);
  });

  it("recognizes wrapping and saturating add operators", () => {
    expect(tokenTypes("a +% b +? c")).toEqual([
      "NAME",
      "WRAP_ADD",
      "NAME",
      "SAT_ADD",
      "NAME",
    ]);
  });

  it("skips line comments cleanly", () => {
    expect(tokenTypes("// comment only\nconst MAX: u4 = 10;")).toEqual([
      "const",
      "NAME",
      "COLON",
      "NAME",
      "EQ",
      "INT_LIT",
      "SEMICOLON",
    ]);
  });
});
