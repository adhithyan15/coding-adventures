/**
 * semantic-tokens.test.ts -- Semantic token encoding tests
 *
 * Tests the delta encoding algorithm that converts SemanticToken objects
 * into the LSP compact integer array format.
 *
 * The encoding is a flat array of 5-tuples:
 *   [deltaLine, deltaStartChar, length, tokenTypeIndex, tokenModifierBitmask, ...]
 *
 * "Delta" means the difference from the PREVIOUS token's position.
 */

import { describe, it, expect } from "vitest";
import {
  encodeSemanticTokens,
  tokenTypeIndex,
  tokenModifierMask,
} from "../src/capabilities.js";
import type { SemanticToken } from "../src/types.js";

describe("encodeSemanticTokens", () => {
  it("empty input returns empty array", () => {
    expect(encodeSemanticTokens([])).toEqual([]);
  });

  it("single keyword token", () => {
    const tokens: SemanticToken[] = [
      { line: 0, character: 0, length: 5, tokenType: "keyword", modifiers: [] },
    ];
    const data = encodeSemanticTokens(tokens);

    // Expected: [deltaLine=0, deltaChar=0, length=5, keyword=15, mods=0]
    expect(data).toHaveLength(5);
    expect(data[0]).toBe(0); // deltaLine
    expect(data[1]).toBe(0); // deltaChar
    expect(data[2]).toBe(5); // length
    expect(data[3]).toBe(15); // keyword index
    expect(data[4]).toBe(0); // no modifiers
  });

  it("two tokens on same line", () => {
    const tokens: SemanticToken[] = [
      { line: 0, character: 0, length: 3, tokenType: "keyword", modifiers: [] },
      { line: 0, character: 4, length: 4, tokenType: "function", modifiers: ["declaration"] },
    ];
    const data = encodeSemanticTokens(tokens);

    expect(data).toHaveLength(10);

    // Token A: deltaLine=0, deltaChar=0, length=3, keyword(15), mods=0
    expect(data.slice(0, 5)).toEqual([0, 0, 3, 15, 0]);
    // Token B: deltaLine=0, deltaChar=4, length=4, function(12), mods=1 (declaration=bit0)
    expect(data.slice(5, 10)).toEqual([0, 4, 4, 12, 1]);
  });

  it("tokens on different lines", () => {
    const tokens: SemanticToken[] = [
      { line: 0, character: 0, length: 3, tokenType: "keyword", modifiers: [] },
      { line: 2, character: 4, length: 5, tokenType: "number", modifiers: [] },
    ];
    const data = encodeSemanticTokens(tokens);

    expect(data).toHaveLength(10);
    // Token B: deltaLine=2, deltaChar=4 (absolute on new line), number=19
    expect(data[5]).toBe(2);  // deltaLine
    expect(data[6]).toBe(4);  // deltaChar (absolute since different line)
    expect(data[8]).toBe(19); // number index
  });

  it("unsorted input is auto-sorted", () => {
    const tokens: SemanticToken[] = [
      { line: 1, character: 0, length: 2, tokenType: "number", modifiers: [] },
      { line: 0, character: 0, length: 3, tokenType: "keyword", modifiers: [] },
    ];
    const data = encodeSemanticTokens(tokens);

    expect(data).toHaveLength(10);
    // After sorting: keyword (15) first, number (19) second
    expect(data[3]).toBe(15); // first token is keyword
    expect(data[8]).toBe(19); // second token is number
  });

  it("unknown token type is skipped", () => {
    const tokens: SemanticToken[] = [
      { line: 0, character: 0, length: 3, tokenType: "unknownType", modifiers: [] },
      { line: 0, character: 4, length: 2, tokenType: "keyword", modifiers: [] },
    ];
    const data = encodeSemanticTokens(tokens);

    // unknownType skipped, only keyword remains
    expect(data).toHaveLength(5);
  });

  it("readonly modifier bitmask", () => {
    // "readonly" is at index 2 in the modifier list, value = 1 << 2 = 4
    const tokens: SemanticToken[] = [
      { line: 0, character: 0, length: 3, tokenType: "variable", modifiers: ["readonly"] },
    ];
    const data = encodeSemanticTokens(tokens);
    expect(data[4]).toBe(4); // readonly = bit 2 = value 4
  });

  it("multiple modifiers combine as bitmask", () => {
    // "declaration" = bit 0 = 1, "readonly" = bit 2 = 4, combined = 5
    const tokens: SemanticToken[] = [
      { line: 0, character: 0, length: 3, tokenType: "variable", modifiers: ["declaration", "readonly"] },
    ];
    const data = encodeSemanticTokens(tokens);
    expect(data[4]).toBe(5); // 1 | 4 = 5
  });

  it("three tokens across multiple lines", () => {
    const tokens: SemanticToken[] = [
      { line: 0, character: 0, length: 3, tokenType: "keyword", modifiers: [] },
      { line: 0, character: 4, length: 5, tokenType: "function", modifiers: ["declaration"] },
      { line: 1, character: 0, length: 8, tokenType: "variable", modifiers: [] },
    ];
    const data = encodeSemanticTokens(tokens);

    expect(data).toHaveLength(15);
    // Token C: deltaLine=1, deltaChar=0 (reset on new line), length=8, variable(8), mods=0
    expect(data.slice(10, 15)).toEqual([1, 0, 8, 8, 0]);
  });
});

describe("tokenTypeIndex", () => {
  it("keyword is 15", () => {
    expect(tokenTypeIndex("keyword")).toBe(15);
  });

  it("function is 12", () => {
    expect(tokenTypeIndex("function")).toBe(12);
  });

  it("variable is 8", () => {
    expect(tokenTypeIndex("variable")).toBe(8);
  });

  it("number is 19", () => {
    expect(tokenTypeIndex("number")).toBe(19);
  });

  it("unknown returns -1", () => {
    expect(tokenTypeIndex("nonexistent")).toBe(-1);
  });
});

describe("tokenModifierMask", () => {
  it("empty list returns 0", () => {
    expect(tokenModifierMask([])).toBe(0);
  });

  it("declaration is bit 0", () => {
    expect(tokenModifierMask(["declaration"])).toBe(1);
  });

  it("definition is bit 1", () => {
    expect(tokenModifierMask(["definition"])).toBe(2);
  });

  it("readonly is bit 2", () => {
    expect(tokenModifierMask(["readonly"])).toBe(4);
  });

  it("combined modifiers", () => {
    expect(tokenModifierMask(["declaration", "definition"])).toBe(3);
  });

  it("unknown modifier ignored", () => {
    expect(tokenModifierMask(["unknownMod"])).toBe(0);
  });
});
