/**
 * normalise.test.ts — author-string normalisation rules.
 */

import { describe, it, expect } from "vitest";
import { normaliseAuthor } from "../src/index.js";

describe("normaliseAuthor — basic", () => {
  it("lowercases", () => {
    expect(normaliseAuthor("Ada Lovelace")).toBe("ada-lovelace");
  });

  it("collapses multi-space", () => {
    expect(normaliseAuthor("Ada   Lovelace")).toBe("ada-lovelace");
  });

  it("keeps digits (e.g. 'John Doe III' loses III to numerals)", () => {
    expect(normaliseAuthor("John 2nd")).toBe("john-2nd");
  });

  it("trims leading/trailing hyphens", () => {
    expect(normaliseAuthor("-Ada-")).toBe("ada");
  });

  it("trims leading/trailing whitespace", () => {
    expect(normaliseAuthor("  Ada  ")).toBe("ada");
  });

  it("collapses adjacent hyphens", () => {
    expect(normaliseAuthor("Ada---Lovelace")).toBe("ada-lovelace");
  });

  it("idempotent: normaliseAuthor(normaliseAuthor(x)) === normaliseAuthor(x)", () => {
    for (const input of ["Ada Lovelace", "Foo!!Bar", "  x  "]) {
      const once = normaliseAuthor(input);
      expect(normaliseAuthor(once)).toBe(once);
    }
  });
});

describe("normaliseAuthor — security hardening", () => {
  it("strips angle brackets", () => {
    expect(normaliseAuthor("<script>")).toBe("script");
  });

  it("strips quotes", () => {
    expect(normaliseAuthor(`Ada "Bytes" Lovelace`)).toBe("ada-bytes-lovelace");
  });

  it("strips ampersand", () => {
    expect(normaliseAuthor("Ada & Charles")).toBe("ada-charles");
  });

  it("strips control bytes", () => {
    expect(normaliseAuthor("Ada\x00\x1b\x7fLovelace")).toBe("adalovelace");
  });

  it("strips underscores (so '__proto__' becomes 'proto')", () => {
    expect(normaliseAuthor("__proto__")).toBe("proto");
  });

  it("strips non-ASCII (CJK / emoji)", () => {
    expect(normaliseAuthor("夏目漱石")).toBe("");
    expect(normaliseAuthor("🎉")).toBe("");
  });

  it("strips path-traversal sequences", () => {
    expect(normaliseAuthor("../../etc")).toBe("etc");
  });

  it("output always matches /^[a-z0-9-]*$/", () => {
    const cases = ["<x>", "Ada & Charles", `"q"`, "../x", "\x00\x01", "夏目漱石"];
    for (const c of cases) {
      expect(normaliseAuthor(c)).toMatch(/^[a-z0-9-]*$/);
    }
  });
});

describe("normaliseAuthor — empty / collapsed input", () => {
  it("empty string → empty", () => {
    expect(normaliseAuthor("")).toBe("");
  });

  it("whitespace-only → empty", () => {
    expect(normaliseAuthor("   ")).toBe("");
  });

  it("punctuation-only → empty", () => {
    expect(normaliseAuthor("!!!")).toBe("");
  });

  it("non-ASCII-only → empty", () => {
    expect(normaliseAuthor("夏目漱石")).toBe("");
  });
});

describe("normaliseAuthor — defensive coercion", () => {
  it("non-string coerces via String(...)", () => {
    // @ts-expect-error — runtime coercion
    expect(normaliseAuthor(42)).toBe("42");
  });

  it("null → 'null'", () => {
    // @ts-expect-error — runtime coercion
    expect(normaliseAuthor(null)).toBe("null");
  });
});

describe("normaliseAuthor — deterministic", () => {
  it("same input → same output", () => {
    expect(normaliseAuthor("Ada Lovelace")).toBe(normaliseAuthor("Ada Lovelace"));
  });
});
