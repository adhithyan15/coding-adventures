/**
 * normalise.test.ts — tag-string normalisation rules.
 */

import assert from "node:assert/strict";
import { describe, test as it } from "node:test";
import { normaliseTag } from "../src/index.js";

describe("normaliseTag — basic", () => {
  it("lowercases", () => {
    assert.equal(normaliseTag("TypeScript"), "typescript");
  });

  it("replaces spaces with hyphens", () => {
    assert.equal(normaliseTag("type script"), "type-script");
  });

  it("collapses multi-space", () => {
    assert.equal(normaliseTag("type   script"), "type-script");
  });

  it("keeps digits", () => {
    assert.equal(normaliseTag("Node 22"), "node-22");
  });

  it("trims leading/trailing hyphens", () => {
    assert.equal(normaliseTag("-foo-"), "foo");
  });

  it("trims leading/trailing whitespace", () => {
    assert.equal(normaliseTag("  foo  "), "foo");
  });

  it("collapses adjacent hyphens", () => {
    assert.equal(normaliseTag("foo---bar"), "foo-bar");
  });

  it("idempotent: normaliseTag(normaliseTag(x)) === normaliseTag(x)", () => {
    for (const input of ["TypeScript", "Type Script", "Foo!!Bar", "   x   "]) {
      const once = normaliseTag(input);
      assert.equal(normaliseTag(once), once);
    }
  });
});

describe("normaliseTag — security hardening", () => {
  it("strips angle brackets", () => {
    assert.equal(normaliseTag("<script>"), "script");
  });

  it("strips quotes", () => {
    assert.equal(normaliseTag(`"x"`), "x");
    assert.equal(normaliseTag(`'x'`), "x");
  });

  it("strips ampersand", () => {
    assert.equal(normaliseTag("foo&bar"), "foobar");
  });

  it("strips control bytes", () => {
    assert.equal(normaliseTag("foo\x00bar\x1bbaz\x7f"), "foobarbaz");
  });

  it("strips underscores (so `__proto__` becomes `proto`)", () => {
    assert.equal(normaliseTag("__proto__"), "proto");
  });

  it("strips non-ASCII (CJK / emoji)", () => {
    assert.equal(normaliseTag("日本語"), "");
    assert.equal(normaliseTag("🎉"), "");
  });

  it("strips path-traversal sequences", () => {
    assert.equal(normaliseTag("../../etc/passwd"), "etcpasswd");
  });

  it("output always matches /^[a-z0-9-]*$/", () => {
    const cases = ["<x>", "a&b", `"q"`, "../x", "\x00\x01", "日本語"];
    for (const c of cases) {
      assert.match(normaliseTag(c), /^[a-z0-9-]*$/);
    }
  });
});

describe("normaliseTag — empty / collapsed input", () => {
  it("empty string → empty", () => {
    assert.equal(normaliseTag(""), "");
  });

  it("whitespace-only → empty", () => {
    assert.equal(normaliseTag("   "), "");
  });

  it("punctuation-only → empty", () => {
    assert.equal(normaliseTag("!!!"), "");
  });

  it("non-ASCII-only → empty", () => {
    assert.equal(normaliseTag("日本語"), "");
  });
});

describe("normaliseTag — defensive coercion", () => {
  it("non-string coerces via String(...)", () => {
    // @ts-expect-error — runtime coercion
    assert.equal(normaliseTag(42), "42");
  });

  it("null → 'null'", () => {
    // @ts-expect-error — runtime coercion
    assert.equal(normaliseTag(null), "null");
  });
});

describe("normaliseTag — deterministic", () => {
  it("same input → same output", () => {
    assert.equal(normaliseTag("Foo Bar"), normaliseTag("Foo Bar"));
  });
});
