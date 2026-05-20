/**
 * validate.test.ts — route / baseUrl / string validators.
 */

import { describe, it, expect } from "vitest";
import {
  validateBaseUrl,
  validateRoute,
  validateString,
} from "../src/index.js";

describe("validateRoute — accept", () => {
  it("bare /", () => { expect(validateRoute("/", "f")).toBe("/"); });
  it("/about", () => { expect(validateRoute("/about", "f")).toBe("/about"); });
  it("/posts/x", () => { expect(validateRoute("/posts/x", "f")).toBe("/posts/x"); });
  it("/p/x.html (with extension)", () => { expect(validateRoute("/p/x.html", "f")).toBe("/p/x.html"); });
  it("/feed.xml", () => { expect(validateRoute("/feed.xml", "f")).toBe("/feed.xml"); });
  it("dashes + underscores", () => { expect(validateRoute("/my-post_x", "f")).toBe("/my-post_x"); });
  it("digits", () => { expect(validateRoute("/page-2026", "f")).toBe("/page-2026"); });
  it("mixed-case (case preserved)", () => { expect(validateRoute("/AboutUs", "f")).toBe("/AboutUs"); });
  it("colon segment (RFC 3986 sub-delim)", () => {
    expect(validateRoute("/a:b", "f")).toBe("/a:b");
  });
});

describe("validateRoute — reject", () => {
  it("non-string", () => { expect(() => validateRoute(42 as unknown as string, "f")).toThrow(/must be a string/); });
  it("null", () => { expect(() => validateRoute(null, "f")).toThrow(/got null/); });
  it("empty", () => { expect(() => validateRoute("", "f")).toThrow(/non-empty/); });
  it("over 2048 chars", () => {
    expect(() => validateRoute("/" + "a".repeat(2050), "f")).toThrow(/≤ 2048/);
  });
  it("no leading /", () => { expect(() => validateRoute("about", "f")).toThrow(/must start with "\/"/); });
  it("protocol-relative //evil.com", () => {
    expect(() => validateRoute("//evil.com", "f")).toThrow(/protocol-relative/);
  });
  it("backslash variant /\\evil", () => {
    expect(() => validateRoute("/\\evil", "f")).toThrow(/backslash variant/);
  });
  it("contains \\ later in path", () => {
    expect(() => validateRoute("/p/x\\y", "f")).toThrow(/must not contain/);
  });
  it("path traversal /..", () => {
    expect(() => validateRoute("/..", "f")).toThrow(/path traversal/);
  });
  it("path traversal /a/..", () => {
    expect(() => validateRoute("/a/..", "f")).toThrow(/path traversal/);
  });
  it("path traversal /a/../b", () => {
    expect(() => validateRoute("/a/../b", "f")).toThrow(/path traversal/);
  });
  it("dot segment /.", () => {
    expect(() => validateRoute("/.", "f")).toThrow(/"\." segment/);
  });
  it("dot segment /a/./b", () => {
    expect(() => validateRoute("/a/./b", "f")).toThrow(/"\." segment/);
  });
  it("empty mid-segment /a//b", () => {
    expect(() => validateRoute("/a//b", "f")).toThrow(/empty segments/);
  });
  it("trailing slash /a/", () => {
    expect(() => validateRoute("/a/", "f")).toThrow(/empty segments/);
  });
  it("query string /a?b=c (contains ?)", () => {
    expect(() => validateRoute("/a?b=c", "f")).toThrow(/disallowed characters/);
  });
  it("hash /a#b", () => {
    expect(() => validateRoute("/a#b", "f")).toThrow(/disallowed characters/);
  });
  it("whitespace /a b", () => {
    expect(() => validateRoute("/a b", "f")).toThrow(/disallowed characters/);
  });
  it("NUL /a\\x00b", () => {
    expect(() => validateRoute("/a\x00b", "f")).toThrow(/disallowed characters/);
  });
  it("unicode segment", () => {
    expect(() => validateRoute("/café", "f")).toThrow(/disallowed characters/);
  });
  it("percent-encoded traversal /%2e%2e rejected (no decode)", () => {
    expect(() => validateRoute("/%2e%2e", "f")).toThrow(/disallowed characters/);
  });
  it("percent-encoded slash /%2f rejected", () => {
    expect(() => validateRoute("/%2f", "f")).toThrow(/disallowed characters/);
  });
  it("percent-encoded NUL /%00 rejected", () => {
    expect(() => validateRoute("/%00", "f")).toThrow(/disallowed characters/);
  });
  it("error contains field name", () => {
    expect(() => validateRoute("../bad", "pages[3].route"))
      .toThrow(/pages\[3\]\.route/);
  });
  it("long route truncated in error", () => {
    const long = "/" + "a".repeat(200) + "\\evil";
    try {
      validateRoute(long, "f");
      expect.fail("expected throw");
    } catch (e) {
      expect((e as Error).message).toContain("…");
    }
  });
});

describe("validateBaseUrl — accept", () => {
  it("https", () => { expect(validateBaseUrl("https://example.com")).toBe("https://example.com"); });
  it("http", () => { expect(validateBaseUrl("http://example.com")).toBe("http://example.com"); });
  it("HTTPS uppercase", () => { expect(validateBaseUrl("HTTPS://example.com")).toBe("HTTPS://example.com"); });
  it("with path", () => { expect(validateBaseUrl("https://example.com/blog")).toBe("https://example.com/blog"); });
});

describe("validateBaseUrl — reject", () => {
  it("non-string", () => { expect(() => validateBaseUrl(42 as unknown as string)).toThrow(/non-empty/); });
  it("null", () => { expect(() => validateBaseUrl(null)).toThrow(/got null/); });
  it("empty", () => { expect(() => validateBaseUrl("")).toThrow(/non-empty/); });
  it("javascript:", () => { expect(() => validateBaseUrl("javascript:alert(1)")).toThrow(/http\(s\)/); });
  it("data:", () => { expect(() => validateBaseUrl("data:text/x,1")).toThrow(/http\(s\)/); });
  it("file:", () => { expect(() => validateBaseUrl("file:///etc")).toThrow(/http\(s\)/); });
  it("ftp:", () => { expect(() => validateBaseUrl("ftp://x")).toThrow(/http\(s\)/); });
  it("/relative", () => { expect(() => validateBaseUrl("/about")).toThrow(/http\(s\)/); });
  it("over 2048", () => {
    expect(() => validateBaseUrl("https://" + "a".repeat(2100) + ".com"))
      .toThrow(/≤ 2048/);
  });
});

describe("validateString", () => {
  it("string passes", () => { expect(validateString("hello", "f")).toBe("hello"); });
  it("empty allowed", () => { expect(validateString("", "f")).toBe(""); });
  it("non-string rejected", () => { expect(() => validateString(42 as unknown as string, "f")).toThrow(/must be a string/); });
  it("null rejected", () => { expect(() => validateString(null, "f")).toThrow(/got null/); });
});
