/**
 * slugify.test.ts — GitHub-flavoured slugification rules.
 */

import { describe, it, expect } from "vitest";
import { slugify } from "../src/index.js";

describe("slugify — basic shape", () => {
  it("lowercases", () => {
    expect(slugify("Hello")).toBe("hello");
  });

  it("replaces spaces with hyphens", () => {
    expect(slugify("hello world")).toBe("hello-world");
  });

  it("collapses multi-space runs into one hyphen", () => {
    expect(slugify("hello   world")).toBe("hello-world");
  });

  it("strips punctuation", () => {
    expect(slugify("Hello, World!")).toBe("hello-world");
  });

  it("keeps digits", () => {
    expect(slugify("Step 2: Install")).toBe("step-2-install");
  });

  it("trims leading and trailing hyphens", () => {
    expect(slugify("--hello--")).toBe("hello");
  });

  it("trims hyphens after punctuation stripping", () => {
    expect(slugify("!! hello !!")).toBe("hello");
  });

  it("collapses adjacent hyphens", () => {
    expect(slugify("hello---world")).toBe("hello-world");
  });

  it("matches GitHub idiom: 'Step 2: Install dependencies'", () => {
    expect(slugify("Step 2: Install dependencies")).toBe("step-2-install-dependencies");
  });
});

describe("slugify — fallback for empty / collapsed input", () => {
  it("empty string → 'section'", () => {
    expect(slugify("")).toBe("section");
  });

  it("whitespace-only → 'section'", () => {
    expect(slugify("   ")).toBe("section");
  });

  it("punctuation-only → 'section'", () => {
    expect(slugify("!!!")).toBe("section");
  });

  it("hyphens-only → 'section'", () => {
    expect(slugify("---")).toBe("section");
  });

  it("non-ASCII-only (CJK) → 'section'", () => {
    expect(slugify("日本語")).toBe("section");
  });
});

describe("slugify — security / hostile inputs", () => {
  it("strips ASCII control bytes (NUL, ESC, DEL)", () => {
    expect(slugify("hel\x00lo\x1bwor\x7fld")).toBe("helloworld");
  });

  it("strips HTML-injection attempts", () => {
    expect(slugify("<script>alert(1)</script>")).toBe("scriptalert1script");
  });

  it("strips quotes (cannot break out of HTML attribute)", () => {
    expect(slugify(`hello"world'`)).toBe("helloworld");
  });

  it("strips angle brackets", () => {
    expect(slugify("a<b>c")).toBe("abc");
  });

  it("strips ampersands (no entity injection)", () => {
    expect(slugify("a&amp;b")).toBe("aampb");
  });
});

describe("slugify — output guarantees", () => {
  it("output matches /^[a-z0-9-]+$/", () => {
    const inputs = ["Hello!", "Step 2", "<a>", "日本語", "  spaces  ", "a/b/c"];
    for (const input of inputs) {
      expect(slugify(input)).toMatch(/^[a-z0-9-]+$/);
    }
  });

  it("output is never empty", () => {
    const inputs = ["", "   ", "!!!", "日本語", "\x00"];
    for (const input of inputs) {
      expect(slugify(input).length).toBeGreaterThan(0);
    }
  });

  it("idempotent: slugify(slugify(x)) === slugify(x)", () => {
    const inputs = ["Hello World", "Step 2: Install", "  weird  "];
    for (const input of inputs) {
      const once = slugify(input);
      expect(slugify(once)).toBe(once);
    }
  });

  it("deterministic: same input → same output across calls", () => {
    expect(slugify("Hello World")).toBe(slugify("Hello World"));
  });
});

describe("slugify — non-string defensive coercion", () => {
  it("coerces non-strings via String(...)", () => {
    // @ts-expect-error — defensive runtime coercion
    expect(slugify(42)).toBe("42");
  });

  it("null coerces to 'null'", () => {
    // @ts-expect-error — defensive runtime coercion
    expect(slugify(null)).toBe("null");
  });
});
