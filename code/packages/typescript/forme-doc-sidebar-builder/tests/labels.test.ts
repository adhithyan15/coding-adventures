/**
 * labels.test.ts — humanise() tests.
 */

import { describe, it, expect } from "vitest";
import { humanise } from "../src/index.js";

describe("humanise — basic", () => {
  it("kebab-case → Title Case", () => {
    expect(humanise("getting-started")).toBe("Getting Started");
  });
  it("snake_case → Title Case", () => {
    expect(humanise("api_reference")).toBe("API Reference");
  });
  it("mixed separators", () => {
    expect(humanise("my_doc-page")).toBe("My Doc Page");
  });
  it("already spaced input", () => {
    expect(humanise("Hello World")).toBe("Hello World");
  });
  it("single word", () => {
    expect(humanise("intro")).toBe("Intro");
  });
  it("preserves Unicode (locale-independent toLowerCase)", () => {
    expect(humanise("café-résumé")).toBe("Café Résumé");
  });
});

describe("humanise — acronyms", () => {
  it("api → API", () => expect(humanise("api")).toBe("API"));
  it("sdk → SDK", () => expect(humanise("sdk")).toBe("SDK"));
  it("url → URL", () => expect(humanise("url")).toBe("URL"));
  it("http → HTTP", () => expect(humanise("http")).toBe("HTTP"));
  it("html → HTML", () => expect(humanise("html")).toBe("HTML"));
  it("json → JSON", () => expect(humanise("json")).toBe("JSON"));
  it("css → CSS", () => expect(humanise("css")).toBe("CSS"));
  it("cli → CLI", () => expect(humanise("cli")).toBe("CLI"));
  it("api in mixed phrase", () => {
    expect(humanise("api-reference")).toBe("API Reference");
  });
  it("multiple acronyms", () => {
    expect(humanise("http-api-cli")).toBe("HTTP API CLI");
  });
  it("io renders as I/O (special case)", () => {
    expect(humanise("io")).toBe("I/O");
  });
  it("case-insensitive acronym match", () => {
    expect(humanise("API")).toBe("API");
    expect(humanise("Api")).toBe("API");
  });
});

describe("humanise — edge cases", () => {
  it("empty string", () => expect(humanise("")).toBe(""));
  it("only separators", () => expect(humanise("---")).toBe(""));
  it("only whitespace", () => expect(humanise("   ")).toBe(""));
  it("collapses runs of separators", () => {
    expect(humanise("foo---bar___baz")).toBe("Foo Bar Baz");
  });
  it("__proto__ falls through to default capitalisation (not Object.prototype's __proto__)", () => {
    expect(humanise("__proto__")).toBe("Proto");
  });
});
