/**
 * slug.test.ts — GitHub-style slugifier unit tests.
 */

import { describe, it, expect } from "vitest";
import { slugify } from "../src/index.js";

describe("slugify — happy paths", () => {
  it("simple ASCII title", () => {
    expect(slugify("Getting Started")).toBe("getting-started");
  });
  it("multi-word, mixed case", () => {
    expect(slugify("API Reference")).toBe("api-reference");
  });
  it("preserves underscores", () => {
    expect(slugify("Snake_case_works")).toBe("snake_case_works");
  });
  it("preserves existing hyphens", () => {
    expect(slugify("Already-hyphenated")).toBe("already-hyphenated");
  });
  it("preserves digits", () => {
    expect(slugify("Section 42")).toBe("section-42");
  });
  it("preserves digit-led tokens", () => {
    expect(slugify("100% done")).toBe("100-done");
  });
});

describe("slugify — punctuation stripping", () => {
  it("strips trailing punctuation", () => {
    expect(slugify("Hello, World!")).toBe("hello-world");
  });
  it("strips question marks", () => {
    expect(slugify("What is it?")).toBe("what-is-it");
  });
  it("strips dots inside tokens", () => {
    expect(slugify("v2.0 — Release notes")).toBe("v20--release-notes");
  });
  it("strips parens", () => {
    expect(slugify("foo (bar)")).toBe("foo-bar");
  });
  it("strips brackets and slashes", () => {
    expect(slugify("path/to/[file]")).toBe("pathtofile");
  });
  it("strips emoji", () => {
    expect(slugify("Release 🚀 notes")).toBe("release--notes");
  });
});

describe("slugify — Unicode", () => {
  it("preserves Chinese characters", () => {
    expect(slugify("中文标题")).toBe("中文标题");
  });
  it("preserves accented Latin letters", () => {
    expect(slugify("Café résumé")).toBe("café-résumé");
  });
  it("preserves Cyrillic", () => {
    expect(slugify("Привет мир")).toBe("привет-мир");
  });
  it("lowercases Greek", () => {
    expect(slugify("Πρώτο Κεφάλαιο")).toBe("πρώτο-κεφάλαιο");
  });
});

describe("slugify — edge cases", () => {
  it("empty string", () => {
    expect(slugify("")).toBe("");
  });
  it("only punctuation collapses to empty", () => {
    expect(slugify("!@#$%^&*()")).toBe("");
  });
  it("only whitespace becomes only hyphens", () => {
    expect(slugify("   ")).toBe("---");
  });
  it("preserves leading and trailing spaces (as hyphens)", () => {
    expect(slugify("  Trim me  ")).toBe("--trim-me--");
  });
  it("preserves runs of spaces (as runs of hyphens)", () => {
    expect(slugify("A   B")).toBe("a---b");
  });
  it("Turkish capital I uses default case-folding, NOT tr-TR", () => {
    // Under tr-TR, 'I' → 'ı'; under default Unicode, 'I' → 'i'.
    // The slug MUST be stable across machines, so we use default.
    expect(slugify("INTRO")).toBe("intro");
  });
});
