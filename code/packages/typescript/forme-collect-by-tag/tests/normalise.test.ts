/**
 * normalise.test.ts — tag-string normalisation rules.
 */

import { describe, it, expect } from "vitest";
import { normaliseTag } from "../src/index.js";

describe("normaliseTag — basic", () => {
  it("lowercases", () => {
    expect(normaliseTag("TypeScript")).toBe("typescript");
  });

  it("replaces spaces with hyphens", () => {
    expect(normaliseTag("type script")).toBe("type-script");
  });

  it("collapses multi-space", () => {
    expect(normaliseTag("type   script")).toBe("type-script");
  });

  it("keeps digits", () => {
    expect(normaliseTag("Node 22")).toBe("node-22");
  });

  it("trims leading/trailing hyphens", () => {
    expect(normaliseTag("-foo-")).toBe("foo");
  });

  it("trims leading/trailing whitespace", () => {
    expect(normaliseTag("  foo  ")).toBe("foo");
  });

  it("collapses adjacent hyphens", () => {
    expect(normaliseTag("foo---bar")).toBe("foo-bar");
  });

  it("idempotent: normaliseTag(normaliseTag(x)) === normaliseTag(x)", () => {
    for (const input of ["TypeScript", "Type Script", "Foo!!Bar", "   x   "]) {
      const once = normaliseTag(input);
      expect(normaliseTag(once)).toBe(once);
    }
  });
});

describe("normaliseTag — security hardening", () => {
  it("strips angle brackets", () => {
    expect(normaliseTag("<script>")).toBe("script");
  });

  it("strips quotes", () => {
    expect(normaliseTag(`"x"`)).toBe("x");
    expect(normaliseTag(`'x'`)).toBe("x");
  });

  it("strips ampersand", () => {
    expect(normaliseTag("foo&bar")).toBe("foobar");
  });

  it("strips control bytes", () => {
    expect(normaliseTag("foo\x00bar\x1bbaz\x7f")).toBe("foobarbaz");
  });

  it("strips underscores (so `__proto__` becomes `proto`)", () => {
    expect(normaliseTag("__proto__")).toBe("proto");
  });

  it("strips non-ASCII (CJK / emoji)", () => {
    expect(normaliseTag("日本語")).toBe("");
    expect(normaliseTag("🎉")).toBe("");
  });

  it("strips path-traversal sequences", () => {
    expect(normaliseTag("../../etc/passwd")).toBe("etcpasswd");
  });

  it("output always matches /^[a-z0-9-]*$/", () => {
    const cases = ["<x>", "a&b", `"q"`, "../x", "\x00\x01", "日本語"];
    for (const c of cases) {
      expect(normaliseTag(c)).toMatch(/^[a-z0-9-]*$/);
    }
  });
});

describe("normaliseTag — empty / collapsed input", () => {
  it("empty string → empty", () => {
    expect(normaliseTag("")).toBe("");
  });

  it("whitespace-only → empty", () => {
    expect(normaliseTag("   ")).toBe("");
  });

  it("punctuation-only → empty", () => {
    expect(normaliseTag("!!!")).toBe("");
  });

  it("non-ASCII-only → empty", () => {
    expect(normaliseTag("日本語")).toBe("");
  });
});

describe("normaliseTag — defensive coercion", () => {
  it("non-string coerces via String(...)", () => {
    // @ts-expect-error — runtime coercion
    expect(normaliseTag(42)).toBe("42");
  });

  it("null → 'null'", () => {
    // @ts-expect-error — runtime coercion
    expect(normaliseTag(null)).toBe("null");
  });
});

describe("normaliseTag — deterministic", () => {
  it("same input → same output", () => {
    expect(normaliseTag("Foo Bar")).toBe(normaliseTag("Foo Bar"));
  });
});
