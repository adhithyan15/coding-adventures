/**
 * normalise.test.ts — text → tokens pipeline tests.
 */

import { describe, it, expect } from "vitest";
import { normaliseToTokens } from "../src/index.js";

describe("normaliseToTokens — basic", () => {
  it("simple ASCII words", () => {
    expect(normaliseToTokens("hello world")).toEqual(["hello", "world"]);
  });
  it("preserves digit-only tokens", () => {
    expect(normaliseToTokens("year 2026")).toEqual(["year", "2026"]);
  });
  it("preserves mixed alphanumeric tokens", () => {
    expect(normaliseToTokens("v1.2.3 release")).toEqual(["v1", "2", "3", "release"]);
  });
  it("preserves underscores inside tokens", () => {
    expect(normaliseToTokens("getting_started_guide")).toEqual(["getting_started_guide"]);
  });
});

describe("normaliseToTokens — lowercasing", () => {
  it("uppercase → lowercase", () => {
    expect(normaliseToTokens("HELLO WORLD")).toEqual(["hello", "world"]);
  });
  it("mixed case → lowercase", () => {
    expect(normaliseToTokens("Hello, World!")).toEqual(["hello", "world"]);
  });
  it("Turkish I uses locale-independent case-folding (NOT tr-TR)", () => {
    // Under tr-TR, 'I' would fold to 'ı'; default Unicode folds
    // to 'i'.  Search indexes must be stable across locales.
    expect(normaliseToTokens("INTRODUCTION")).toEqual(["introduction"]);
  });
});

describe("normaliseToTokens — punctuation stripping", () => {
  it("commas and periods", () => {
    expect(normaliseToTokens("Hello, world.")).toEqual(["hello", "world"]);
  });
  it("quotes and brackets", () => {
    expect(normaliseToTokens('She said "hi" (loudly).')).toEqual([
      "she", "said", "hi", "loudly",
    ]);
  });
  it("hyphens split tokens", () => {
    expect(normaliseToTokens("state-of-the-art")).toEqual(["state", "of", "the", "art"]);
  });
  it("URL gets split into letter runs", () => {
    expect(normaliseToTokens("https://example.com/path")).toEqual([
      "https", "example", "com", "path",
    ]);
  });
  it("collapses runs of punctuation/whitespace", () => {
    expect(normaliseToTokens("a !!! b???   c")).toEqual(["a", "b", "c"]);
  });
});

describe("normaliseToTokens — Unicode", () => {
  it("preserves accented Latin letters", () => {
    expect(normaliseToTokens("café résumé")).toEqual(["café", "résumé"]);
  });
  it("preserves Chinese characters", () => {
    expect(normaliseToTokens("中文 标题")).toEqual(["中文", "标题"]);
  });
  it("preserves Cyrillic", () => {
    expect(normaliseToTokens("Привет мир")).toEqual(["привет", "мир"]);
  });
  it("preserves Greek", () => {
    expect(normaliseToTokens("Καλημέρα κόσμε")).toEqual(["καλημέρα", "κόσμε"]);
  });
  it("emoji are non-alphanumeric → stripped", () => {
    expect(normaliseToTokens("rocket 🚀 launch")).toEqual(["rocket", "launch"]);
  });
});

describe("normaliseToTokens — token-length cap (DoS defence)", () => {
  it("caps individual token at 256 characters", () => {
    // 1000 letters, no separators → one 256-char token.
    const huge = "a".repeat(1000);
    const tokens = normaliseToTokens(huge);
    expect(tokens).toHaveLength(1);
    expect(tokens[0].length).toBe(256);
  });
  it("multiple long tokens cap independently", () => {
    const text = "a".repeat(500) + " " + "b".repeat(500);
    const tokens = normaliseToTokens(text);
    expect(tokens).toHaveLength(2);
    expect(tokens[0].length).toBe(256);
    expect(tokens[1].length).toBe(256);
  });
  it("normal-length tokens unaffected", () => {
    expect(normaliseToTokens("normal length words")).toEqual(["normal", "length", "words"]);
  });
});

describe("normaliseToTokens — edge cases", () => {
  it("empty string", () => {
    expect(normaliseToTokens("")).toEqual([]);
  });
  it("only punctuation", () => {
    expect(normaliseToTokens("!@#$%^&*()")).toEqual([]);
  });
  it("only whitespace", () => {
    expect(normaliseToTokens("   \t\n")).toEqual([]);
  });
  it("single character word", () => {
    expect(normaliseToTokens("a")).toEqual(["a"]);
  });
  it("trailing token (no separator after)", () => {
    expect(normaliseToTokens("foo bar")).toEqual(["foo", "bar"]);
  });
  it("leading separator", () => {
    expect(normaliseToTokens("  foo")).toEqual(["foo"]);
  });
});
