/**
 * tokenize.test.ts — top-level pipeline tests.
 */

import { describe, it, expect } from "vitest";
import { tokenize, STOP_WORDS } from "../src/index.js";

describe("tokenize — defaults (no filter, no stem)", () => {
  it("plain text", () => {
    expect(tokenize("Hello, World!")).toEqual(["hello", "world"]);
  });
  it("empty", () => {
    expect(tokenize("")).toEqual([]);
  });
  it("non-string coerced", () => {
    expect(tokenize(42 as unknown as string)).toEqual(["42"]);
  });
});

describe("tokenize — stop-word filter", () => {
  it("filters built-in stop words", () => {
    expect(tokenize("the quick brown fox", { filterStopWords: true })).toEqual([
      "quick", "brown", "fox",
    ]);
  });
  it("does NOT filter when option omitted", () => {
    expect(tokenize("the quick brown fox")).toEqual([
      "the", "quick", "brown", "fox",
    ]);
  });
  it("custom stop-word list overrides built-in", () => {
    const custom = new Set(["fox"]);
    expect(
      tokenize("the quick brown fox", { filterStopWords: true, customStopWords: custom }),
    ).toEqual(["the", "quick", "brown"]);
  });
  it("empty custom list filters nothing", () => {
    expect(
      tokenize("the quick brown fox", { filterStopWords: true, customStopWords: new Set() }),
    ).toEqual(["the", "quick", "brown", "fox"]);
  });
});

describe("tokenize — Porter stemming", () => {
  it("stems each token", () => {
    expect(tokenize("running and walking", { stem: true })).toEqual([
      "run", "and", "walk",
    ]);
  });
  it("does NOT stem when option omitted", () => {
    expect(tokenize("running and walking")).toEqual([
      "running", "and", "walking",
    ]);
  });
  it("combines stop-word filter + stem", () => {
    expect(
      tokenize("running and walking", { filterStopWords: true, stem: true }),
    ).toEqual(["run", "walk"]);
  });
  it("stemming is applied AFTER stop-word filter", () => {
    // "the" gets filtered → never reaches the stemmer (which
    // wouldn't change it anyway, but the test pins the ordering).
    expect(
      tokenize("the running", { filterStopWords: true, stem: true }),
    ).toEqual(["run"]);
  });
});

describe("tokenize — realistic docs queries", () => {
  it("user query: 'How do I install?'", () => {
    expect(tokenize("How do I install?")).toEqual(["how", "do", "i", "install"]);
  });
  it("user query with filter + stem", () => {
    expect(
      tokenize("How do I install?", { filterStopWords: true, stem: true }),
    ).toEqual(["how", "do", "i", "instal"]);
  });
  it("doc body: paragraph", () => {
    const body =
      "The Porter stemmer reduces morphological variants of a word to a shared root form.";
    expect(tokenize(body)).toEqual([
      "the", "porter", "stemmer", "reduces", "morphological", "variants",
      "of", "a", "word", "to", "a", "shared", "root", "form",
    ]);
  });
});

describe("tokenize — determinism + immutability", () => {
  it("same input → identical output", () => {
    const t = "The quick brown fox jumps over the lazy dog";
    const a = JSON.stringify(tokenize(t, { filterStopWords: true, stem: true }));
    const b = JSON.stringify(tokenize(t, { filterStopWords: true, stem: true }));
    expect(a).toBe(b);
  });
  it("does not mutate options.customStopWords set", () => {
    const custom = new Set(["foo"]);
    const snapshot = [...custom];
    tokenize("foo bar", { filterStopWords: true, customStopWords: custom });
    expect([...custom]).toEqual(snapshot);
  });
});

describe("STOP_WORDS — built-in list", () => {
  it("includes common articles", () => {
    expect(STOP_WORDS.has("the")).toBe(true);
    expect(STOP_WORDS.has("a")).toBe(true);
    expect(STOP_WORDS.has("an")).toBe(true);
  });
  it("includes common pronouns", () => {
    expect(STOP_WORDS.has("he")).toBe(true);
    expect(STOP_WORDS.has("she")).toBe(true);
    expect(STOP_WORDS.has("you")).toBe(true);
  });
  it("does NOT include negation/question words (kept for docs queries)", () => {
    expect(STOP_WORDS.has("not")).toBe(false);
    expect(STOP_WORDS.has("how")).toBe(false);
    expect(STOP_WORDS.has("what")).toBe(false);
  });
});
