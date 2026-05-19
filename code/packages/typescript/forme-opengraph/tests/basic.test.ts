/**
 * basic.test.ts — basic <title> / <meta description> / <link canonical> generator.
 */

import { describe, it, expect } from "vitest";
import { generateBasicTags } from "../src/index.js";

describe("generateBasicTags", () => {
  it("emits <title> when supplied", () => {
    expect(generateBasicTags({ title: "Hello" })).toBe(`<title>Hello</title>`);
  });

  it("emits <meta description> when supplied", () => {
    expect(generateBasicTags({ description: "A nice page" }))
      .toBe(`<meta name="description" content="A nice page">`);
  });

  it("emits <link rel=canonical> when supplied", () => {
    expect(generateBasicTags({ canonical: "https://example.com/x" }))
      .toBe(`<link rel="canonical" href="https://example.com/x">`);
  });

  it("emits all three in conventional order when all supplied", () => {
    const lines = generateBasicTags({
      title: "T",
      description: "D",
      canonical: "https://example.com/x",
    }).split("\n");
    expect(lines.length).toBe(3);
    expect(lines[0]).toContain("<title>");
    expect(lines[1]).toContain("description");
    expect(lines[2]).toContain("canonical");
  });

  it("empty meta → empty string (no spurious whitespace)", () => {
    expect(generateBasicTags({})).toBe("");
  });

  it("escapes special chars in <title>", () => {
    expect(generateBasicTags({ title: `AT&T "Blog"` }))
      .toBe(`<title>AT&amp;T &quot;Blog&quot;</title>`);
  });

  it("escapes special chars in description attribute", () => {
    expect(generateBasicTags({ description: `<script>` }))
      .toBe(`<meta name="description" content="&lt;script&gt;">`);
  });

  it("validates canonical URL is absolute http(s)", () => {
    expect(() => generateBasicTags({ canonical: "/relative" }))
      .toThrow(/canonical.*absolute/);
    expect(() => generateBasicTags({ canonical: "javascript:alert(1)" }))
      .toThrow(/canonical.*absolute/);
  });

  it("strips ASCII control bytes from title", () => {
    expect(generateBasicTags({ title: "Hello\x00World" }))
      .toBe(`<title>HelloWorld</title>`);
  });
});
