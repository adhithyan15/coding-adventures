/**
 * supported-languages.test.ts — language-set membership tests.
 */

import { describe, it, expect } from "vitest";
import { SUPPORTED_LANGUAGES, isSupportedLanguage } from "../src/index.js";

describe("SUPPORTED_LANGUAGES — coverage", () => {
  it("contains the eleven DOC00 v0 spec languages and aliases", () => {
    const required = [
      // TypeScript
      "ts", "tsx", "typescript",
      // JavaScript
      "js", "jsx", "javascript", "mjs", "cjs",
      // Python
      "py", "python",
      // Ruby
      "rb", "ruby",
      // Go
      "go", "golang",
      // Rust
      "rs", "rust",
      // Bash
      "sh", "bash", "shell", "zsh",
      // JSON
      "json",
      // HTML
      "html", "htm",
      // CSS
      "css",
      // Markdown
      "md", "markdown",
    ];
    for (const lang of required) {
      expect(SUPPORTED_LANGUAGES.has(lang), `expected '${lang}' in SUPPORTED_LANGUAGES`).toBe(true);
    }
  });
});

describe("isSupportedLanguage", () => {
  it("known language → true", () => {
    expect(isSupportedLanguage("ts")).toBe(true);
  });
  it("known alias → true", () => {
    expect(isSupportedLanguage("typescript")).toBe(true);
    expect(isSupportedLanguage("javascript")).toBe(true);
    expect(isSupportedLanguage("python")).toBe(true);
  });
  it("case-insensitive lookup", () => {
    expect(isSupportedLanguage("TS")).toBe(true);
    expect(isSupportedLanguage("Python")).toBe(true);
    expect(isSupportedLanguage("RUST")).toBe(true);
  });
  it("leading/trailing whitespace tolerated", () => {
    expect(isSupportedLanguage("  ts  ")).toBe(true);
  });
  it("unknown language → false", () => {
    expect(isSupportedLanguage("cobol")).toBe(false);
    expect(isSupportedLanguage("fortran")).toBe(false);
  });
  it("empty string → false", () => {
    expect(isSupportedLanguage("")).toBe(false);
  });
  it("null → false", () => {
    expect(isSupportedLanguage(null)).toBe(false);
  });
});
