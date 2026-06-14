/**
 * twitter.test.ts — Twitter Card generator.
 */

import { describe, it, expect } from "vitest";
import { generateTwitterCardTags, type TwitterCardMeta } from "../src/index.js";

describe("generateTwitterCardTags — card types", () => {
  it.each(["summary", "summary_large_image", "player", "app"] as const)(
    "accepts card type %s",
    (card) => {
      const out = generateTwitterCardTags({ card });
      expect(out).toContain(`<meta name="twitter:card" content="${card}">`);
    },
  );

  it("rejects unknown card type", () => {
    expect(() => generateTwitterCardTags({ card: "wonky" as never }))
      .toThrow(/twitter:card must be one of/);
  });
});

describe("generateTwitterCardTags — optional fields", () => {
  it("includes title when supplied", () => {
    expect(generateTwitterCardTags({ card: "summary", title: "Hello" }))
      .toContain(`<meta name="twitter:title" content="Hello">`);
  });

  it("includes description when supplied", () => {
    expect(generateTwitterCardTags({ card: "summary", description: "Nice" }))
      .toContain(`<meta name="twitter:description" content="Nice">`);
  });

  it("includes image when supplied (URL-validated)", () => {
    expect(generateTwitterCardTags({ card: "summary_large_image", image: "https://example.com/img.png" }))
      .toContain(`<meta name="twitter:image" content="https://example.com/img.png">`);
  });

  it("includes site handle when supplied", () => {
    expect(generateTwitterCardTags({ card: "summary", site: "@acme" }))
      .toContain(`<meta name="twitter:site" content="@acme">`);
  });

  it("includes creator handle when supplied", () => {
    expect(generateTwitterCardTags({ card: "summary", creator: "@author" }))
      .toContain(`<meta name="twitter:creator" content="@author">`);
  });

  it("emits ONLY the supplied tags (no auto-fallbacks)", () => {
    const out = generateTwitterCardTags({ card: "summary" });
    // Just one tag — the card type.
    expect(out.split("\n").length).toBe(1);
  });

  it("fields appear in conventional order", () => {
    const lines = generateTwitterCardTags({
      card: "summary",
      title: "t",
      description: "d",
      image: "https://example.com/i.png",
      site: "@s",
      creator: "@c",
    }).split("\n");
    expect(lines.length).toBe(6);
    expect(lines[0]).toContain("twitter:card");
    expect(lines[1]).toContain("twitter:title");
    expect(lines[2]).toContain("twitter:description");
    expect(lines[3]).toContain("twitter:image");
    expect(lines[4]).toContain("twitter:site");
    expect(lines[5]).toContain("twitter:creator");
  });
});

describe("generateTwitterCardTags — URL validation", () => {
  it("throws on relative twitter:image", () => {
    expect(() => generateTwitterCardTags({ card: "summary", image: "/img.png" }))
      .toThrow(/twitter:image.*absolute/);
  });

  it("throws on javascript: twitter:image", () => {
    expect(() => generateTwitterCardTags({ card: "summary", image: "javascript:alert(1)" }))
      .toThrow(/twitter:image.*absolute/);
  });

  it("throws on data: twitter:image", () => {
    expect(() => generateTwitterCardTags({ card: "summary", image: "data:image/png;base64,xx" }))
      .toThrow(/twitter:image.*absolute/);
  });
});

describe("generateTwitterCardTags — HTML escaping", () => {
  it("escapes special chars in title / description / handles", () => {
    const out = generateTwitterCardTags({
      card: "summary",
      title: `AT&T "Blog"`,
      description: `<x>`,
      site: `@a"b`,
    });
    expect(out).toContain(`content="AT&amp;T &quot;Blog&quot;"`);
    expect(out).toContain(`content="&lt;x&gt;"`);
    expect(out).toContain(`content="@a&quot;b"`);
  });

  it("strips ASCII control bytes", () => {
    expect(generateTwitterCardTags({ card: "summary", title: "Hello\x00World" }))
      .toContain(`content="HelloWorld"`);
  });
});

describe("generateTwitterCardTags — reproducibility", () => {
  it("same input → byte-identical output", () => {
    const m: TwitterCardMeta = { card: "summary_large_image", title: "t", site: "@x" };
    expect(generateTwitterCardTags(m)).toBe(generateTwitterCardTags(m));
  });
});
