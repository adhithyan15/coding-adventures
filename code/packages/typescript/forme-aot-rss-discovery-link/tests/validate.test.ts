/**
 * validate.test.ts — href URL + type allowlist.
 */

import { describe, it, expect } from "vitest";
import {
  escapeHtmlAttr,
  validateFeedHref,
  validateFeedType,
} from "../src/index.js";

describe("validateFeedHref — accepts", () => {
  it("https://", () => {
    expect(validateFeedHref("https://example.com/feed.xml"))
      .toBe("https://example.com/feed.xml");
  });
  it("http://", () => {
    expect(validateFeedHref("http://example.com/feed.xml"))
      .toBe("http://example.com/feed.xml");
  });
  it("case-insensitive scheme", () => {
    expect(validateFeedHref("HTTPS://example.com/x")).toBe("HTTPS://example.com/x");
  });
  it("root-relative", () => {
    expect(validateFeedHref("/feed.xml")).toBe("/feed.xml");
  });
  it("bare /", () => {
    expect(validateFeedHref("/")).toBe("/");
  });
});

describe("validateFeedHref — rejects", () => {
  it("javascript:", () => {
    expect(() => validateFeedHref("javascript:alert(1)")).toThrow(/http\(s\)/);
  });
  it("data:", () => expect(() => validateFeedHref("data:text/xml,x")).toThrow(/http\(s\)/));
  it("file:", () => expect(() => validateFeedHref("file:///etc")).toThrow(/http\(s\)/));
  it("protocol-relative", () => expect(() => validateFeedHref("//evil.com")).toThrow(/http\(s\)/));
  it("backslash-variant", () => expect(() => validateFeedHref("/\\evil.com")).toThrow(/http\(s\)/));
  it("bare relative", () => expect(() => validateFeedHref("feed.xml")).toThrow(/http\(s\)/));
  it("empty string", () => expect(() => validateFeedHref("")).toThrow(/non-empty/));
  it("non-string", () => expect(() => validateFeedHref(42)).toThrow(/non-empty/));
  it("null", () => expect(() => validateFeedHref(null)).toThrow(/null/));
  it("long URL truncated", () => {
    const long = "ftp://" + "x".repeat(500);
    try {
      validateFeedHref(long);
      expect.fail("expected throw");
    } catch (e) {
      const msg = (e as Error).message;
      expect(msg).toMatch(/…/);
      expect(msg.length).toBeLessThan(400);
    }
  });
});

describe("validateFeedType — accepts allowlist", () => {
  it("rss+xml", () => expect(validateFeedType("application/rss+xml")).toBe("application/rss+xml"));
  it("atom+xml", () => expect(validateFeedType("application/atom+xml")).toBe("application/atom+xml"));
  it("json", () => expect(validateFeedType("application/json")).toBe("application/json"));
});

describe("validateFeedType — rejects", () => {
  it("rdf+xml (RSS 1.0; deprecated)", () => {
    expect(() => validateFeedType("application/rdf+xml")).toThrow(/one of/);
  });
  it("text/xml", () => expect(() => validateFeedType("text/xml")).toThrow(/one of/));
  it("case-sensitive: 'APPLICATION/RSS+XML'", () => {
    expect(() => validateFeedType("APPLICATION/RSS+XML")).toThrow(/one of/);
  });
  it("empty string", () => expect(() => validateFeedType("")).toThrow(/one of/));
  it("non-string", () => {
    // @ts-expect-error
    expect(() => validateFeedType(42)).toThrow(/string/);
  });
});

describe("escapeHtmlAttr — all five HTML entities", () => {
  it("ampersand", () => expect(escapeHtmlAttr("a&b")).toBe("a&amp;b"));
  it("less-than", () => expect(escapeHtmlAttr("a<b")).toBe("a&lt;b"));
  it("greater-than", () => expect(escapeHtmlAttr("a>b")).toBe("a&gt;b"));
  it("double quote", () => expect(escapeHtmlAttr(`a"b`)).toBe("a&quot;b"));
  it("single quote", () => expect(escapeHtmlAttr(`a'b`)).toBe("a&#39;b"));
  it("composite", () => {
    expect(escapeHtmlAttr(`<a href="x?a=1&b=2">'y'</a>`))
      .toBe("&lt;a href=&quot;x?a=1&amp;b=2&quot;&gt;&#39;y&#39;&lt;/a&gt;");
  });
});

describe("escapeHtmlAttr — control byte stripping", () => {
  it("NUL stripped", () => expect(escapeHtmlAttr("a\x00b")).toBe("ab"));
  it("DEL stripped", () => expect(escapeHtmlAttr("a\x7Fb")).toBe("ab"));
  it("ESC stripped", () => expect(escapeHtmlAttr("a\x1Bb")).toBe("ab"));
});

describe("escapeHtmlAttr — defensive coercion", () => {
  it("non-string coerces via String(...)", () => {
    // @ts-expect-error
    expect(escapeHtmlAttr(42)).toBe("42");
  });
});
