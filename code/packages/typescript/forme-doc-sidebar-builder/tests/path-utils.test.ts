/**
 * path-utils.test.ts — normalisePath + stripRoot tests.
 */

import { describe, it, expect } from "vitest";
import { normalisePath, stripRoot } from "../src/index.js";

describe("normalisePath — extension stripping", () => {
  it(".md", () => {
    expect(normalisePath("guide/setup.md")).toEqual({ parts: ["guide", "setup"], isIndex: false });
  });
  it(".mdx", () => {
    expect(normalisePath("guide/setup.mdx")).toEqual({ parts: ["guide", "setup"], isIndex: false });
  });
  it(".html", () => {
    expect(normalisePath("guide/setup.html")).toEqual({ parts: ["guide", "setup"], isIndex: false });
  });
  it(".htm", () => {
    expect(normalisePath("guide/setup.htm")).toEqual({ parts: ["guide", "setup"], isIndex: false });
  });
  it(".MD (case-insensitive)", () => {
    expect(normalisePath("guide/setup.MD")).toEqual({ parts: ["guide", "setup"], isIndex: false });
  });
  it("no extension", () => {
    expect(normalisePath("guide/setup")).toEqual({ parts: ["guide", "setup"], isIndex: false });
  });
  it(".png NOT stripped (only doc extensions)", () => {
    expect(normalisePath("img.png")).toEqual({ parts: ["img.png"], isIndex: false });
  });
});

describe("normalisePath — slash handling", () => {
  it("leading slash stripped", () => {
    expect(normalisePath("/guide/setup")).toEqual({ parts: ["guide", "setup"], isIndex: false });
  });
  it("multiple leading slashes", () => {
    expect(normalisePath("///guide/setup")).toEqual({ parts: ["guide", "setup"], isIndex: false });
  });
  it("trailing slash stripped", () => {
    expect(normalisePath("guide/setup/")).toEqual({ parts: ["guide", "setup"], isIndex: false });
  });
  it("both leading and trailing slashes", () => {
    expect(normalisePath("/guide/setup/")).toEqual({ parts: ["guide", "setup"], isIndex: false });
  });
  it("empty segments filtered", () => {
    expect(normalisePath("guide//setup")).toEqual({ parts: ["guide", "setup"], isIndex: false });
  });
});

describe("normalisePath — index detection", () => {
  it("index.md at root", () => {
    expect(normalisePath("index.md")).toEqual({ parts: [], isIndex: true });
  });
  it("guide/index.md", () => {
    expect(normalisePath("guide/index.md")).toEqual({ parts: ["guide"], isIndex: true });
  });
  it("deeply nested index.md", () => {
    expect(normalisePath("a/b/c/index.md")).toEqual({ parts: ["a", "b", "c"], isIndex: true });
  });
  it("INDEX (case-insensitive)", () => {
    expect(normalisePath("guide/INDEX.md")).toEqual({ parts: ["guide"], isIndex: true });
  });
  it("Index.mdx", () => {
    expect(normalisePath("guide/Index.mdx")).toEqual({ parts: ["guide"], isIndex: true });
  });
  it("filename starting with 'index' but not exactly 'index' is NOT index", () => {
    expect(normalisePath("guide/indexes.md")).toEqual({ parts: ["guide", "indexes"], isIndex: false });
  });
  it("filename 'index-old.md' is NOT index", () => {
    expect(normalisePath("guide/index-old.md")).toEqual({ parts: ["guide", "index-old"], isIndex: false });
  });
});

describe("normalisePath — path that strips to empty parts", () => {
  it("'/' → [] (parts empty, not index)", () => {
    expect(normalisePath("/")).toEqual({ parts: [], isIndex: false });
  });
  it("'///' → [] (multiple slashes only)", () => {
    expect(normalisePath("///")).toEqual({ parts: [], isIndex: false });
  });
});

describe("normalisePath — errors", () => {
  it("empty string throws", () => {
    expect(() => normalisePath("")).toThrow(/empty path/);
  });
  it("whitespace-only throws", () => {
    expect(() => normalisePath("   ")).toThrow(/empty path/);
  });
  it("path deeper than 64 levels throws (stack-overflow defence)", () => {
    const deep = Array.from({ length: 65 }, (_, i) => `d${i}`).join("/") + ".md";
    expect(() => normalisePath(deep)).toThrow(/directory-depth cap/);
  });
  it("path at exactly 64 levels is accepted", () => {
    // 63 dirs + 1 file slug = 64 parts.
    const ok = Array.from({ length: 63 }, (_, i) => `d${i}`).join("/") + "/file.md";
    expect(() => normalisePath(ok)).not.toThrow();
  });
  it("raw path exceeding 8192-char length cap throws (regex-DoS defence)", () => {
    // E.g. 10,000 leading slashes — the old regex-based stripper
    // would have done linear-but-large work; the new explicit
    // loop is also bounded, but the upfront length cap fails fast
    // before any per-character processing runs.
    const huge = "/".repeat(10_000) + "x.md";
    expect(() => normalisePath(huge)).toThrow(/length cap/);
  });
  it("raw path at exactly 8192 chars is accepted", () => {
    const ok = "a".repeat(8189) + ".md"; // 8189 + 3 = 8192
    expect(() => normalisePath(ok)).not.toThrow();
  });
});

describe("stripRoot", () => {
  it("empty root → unchanged", () => {
    expect(stripRoot(["guide", "setup"], "")).toEqual(["guide", "setup"]);
  });
  it("matching root → stripped", () => {
    expect(stripRoot(["docs", "guide", "setup"], "docs")).toEqual(["guide", "setup"]);
  });
  it("multi-part root → stripped", () => {
    expect(stripRoot(["site", "docs", "guide"], "site/docs")).toEqual(["guide"]);
  });
  it("non-matching root → null", () => {
    expect(stripRoot(["other", "guide"], "docs")).toBeNull();
  });
  it("root longer than parts → null", () => {
    expect(stripRoot(["docs"], "docs/api")).toBeNull();
  });
  it("root with leading slash", () => {
    expect(stripRoot(["docs", "guide"], "/docs")).toEqual(["guide"]);
  });
  it("root with trailing slash", () => {
    expect(stripRoot(["docs", "guide"], "docs/")).toEqual(["guide"]);
  });
  it("whitespace root → no stripping", () => {
    expect(stripRoot(["guide"], "   ")).toEqual(["guide"]);
  });
});
