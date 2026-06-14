/**
 * combined.test.ts — `generateMetaTags` convenience wrapper.
 */

import { describe, it, expect } from "vitest";
import { generateMetaTags } from "../src/index.js";

describe("generateMetaTags", () => {
  it("combines all three blocks in basic → og → twitter order", () => {
    const out = generateMetaTags({
      basic: { title: "T", canonical: "https://example.com/" },
      og: {
        title: "OG-T", type: "article",
        image: "https://example.com/og.png",
        url: "https://example.com/x",
      },
      twitter: { card: "summary_large_image", site: "@acme" },
    });
    const idxBasic   = out.indexOf("<title>T</title>");
    const idxOg      = out.indexOf("og:title");
    const idxTwitter = out.indexOf("twitter:card");
    expect(idxBasic).toBeGreaterThanOrEqual(0);
    expect(idxOg).toBeGreaterThan(idxBasic);
    expect(idxTwitter).toBeGreaterThan(idxOg);
  });

  it("skips a block when not supplied", () => {
    const out = generateMetaTags({
      basic: { title: "T" },
    });
    expect(out).toBe(`<title>T</title>`);
    expect(out).not.toContain("og:");
    expect(out).not.toContain("twitter:");
  });

  it("empty combined input → empty string", () => {
    expect(generateMetaTags({})).toBe("");
  });

  it("propagates URL-validation errors from any block", () => {
    expect(() => generateMetaTags({
      og: {
        title: "x", type: "article",
        image: "/relative",
        url: "https://example.com/x",
      },
    })).toThrow(/og:image.*absolute/);
  });

  it("filters empty meta blocks (e.g. basic with no fields) without spurious blank lines", () => {
    const out = generateMetaTags({
      basic: {},
      og: {
        title: "x", type: "article",
        image: "https://example.com/i.png",
        url: "https://example.com/p",
      },
    });
    // Block separator is single \n; no double-blank lines.
    expect(out).not.toMatch(/\n\n/);
  });

  it("reproducibility — same input → byte-identical output", () => {
    const input = {
      basic: { title: "T" },
      og: {
        title: "x", type: "article",
        image: "https://example.com/i.png",
        url: "https://example.com/p",
      },
      twitter: { card: "summary" as const },
    };
    expect(generateMetaTags(input)).toBe(generateMetaTags(input));
  });
});
