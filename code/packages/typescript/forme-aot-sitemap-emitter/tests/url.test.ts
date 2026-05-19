/**
 * url.test.ts — normaliseBaseUrl + resolveEntryUrl.
 */

import { describe, it, expect } from "vitest";
import { normaliseBaseUrl, resolveEntryUrl } from "../src/index.js";

describe("normaliseBaseUrl — accepts", () => {
  it("https://example.com", () => {
    expect(normaliseBaseUrl("https://example.com")).toBe("https://example.com");
  });

  it("strips single trailing slash", () => {
    expect(normaliseBaseUrl("https://example.com/")).toBe("https://example.com");
  });

  it("http://", () => {
    expect(normaliseBaseUrl("http://example.com")).toBe("http://example.com");
  });

  it("preserves path and port", () => {
    expect(normaliseBaseUrl("https://example.com:8443/sub")).toBe("https://example.com:8443/sub");
  });

  it("scheme case-insensitive", () => {
    expect(normaliseBaseUrl("HTTPS://example.com")).toBe("HTTPS://example.com");
  });
});

describe("normaliseBaseUrl — rejects", () => {
  it("non-string", () => {
    // @ts-expect-error
    expect(() => normaliseBaseUrl(42)).toThrow(/non-empty string/);
  });

  it("null", () => {
    // @ts-expect-error
    expect(() => normaliseBaseUrl(null)).toThrow(/null/);
  });

  it("empty string", () => {
    expect(() => normaliseBaseUrl("")).toThrow(/non-empty string/);
  });

  it("javascript:", () => {
    expect(() => normaliseBaseUrl("javascript:alert(1)")).toThrow(/http\(s\)/);
  });

  it("file:", () => {
    expect(() => normaliseBaseUrl("file:///etc/passwd")).toThrow(/http\(s\)/);
  });

  it("protocol-relative", () => {
    expect(() => normaliseBaseUrl("//example.com")).toThrow(/http\(s\)/);
  });

  it("bare relative", () => {
    expect(() => normaliseBaseUrl("example.com")).toThrow(/http\(s\)/);
  });

  it("very long invalid URL truncated in error message", () => {
    const long = "ftp://" + "x".repeat(500);
    try {
      normaliseBaseUrl(long);
      expect.fail("expected throw");
    } catch (e) {
      const msg = (e as Error).message;
      expect(msg).toMatch(/…/);
      expect(msg.length).toBeLessThan(400);
    }
  });
});

describe("resolveEntryUrl — absolute http(s)", () => {
  it("https://other.example/x returned verbatim", () => {
    expect(resolveEntryUrl("https://other.example/x", "https://base.example"))
      .toBe("https://other.example/x");
  });

  it("http://", () => {
    expect(resolveEntryUrl("http://other.example", "https://base.example"))
      .toBe("http://other.example");
  });

  it("case-insensitive scheme", () => {
    expect(resolveEntryUrl("HTTPS://other.example", "https://base.example"))
      .toBe("HTTPS://other.example");
  });
});

describe("resolveEntryUrl — root-relative", () => {
  it("/about joined with base", () => {
    expect(resolveEntryUrl("/about", "https://base.example"))
      .toBe("https://base.example/about");
  });

  it("/ joined with base", () => {
    expect(resolveEntryUrl("/", "https://base.example"))
      .toBe("https://base.example/");
  });

  it("multi-segment path joined", () => {
    expect(resolveEntryUrl("/blog/2026/post", "https://base.example"))
      .toBe("https://base.example/blog/2026/post");
  });
});

describe("resolveEntryUrl — rejects", () => {
  it("javascript:", () => {
    expect(() => resolveEntryUrl("javascript:alert(1)", "https://b"))
      .toThrow(/must be http\(s\)/);
  });

  it("data:", () => {
    expect(() => resolveEntryUrl("data:text/html,<script>", "https://b"))
      .toThrow(/must be http\(s\)/);
  });

  it("file:", () => {
    expect(() => resolveEntryUrl("file:///etc/passwd", "https://b"))
      .toThrow(/must be http\(s\)/);
  });

  it("protocol-relative //host", () => {
    expect(() => resolveEntryUrl("//evil.com", "https://b"))
      .toThrow(/must be http\(s\)/);
  });

  it("/\\host (backslash variant)", () => {
    expect(() => resolveEntryUrl("/\\evil.com", "https://b"))
      .toThrow(/must be http\(s\)/);
  });

  it("bare relative", () => {
    expect(() => resolveEntryUrl("about", "https://b"))
      .toThrow(/must be http\(s\)/);
  });

  it("mailto:", () => {
    expect(() => resolveEntryUrl("mailto:x@y.com", "https://b"))
      .toThrow(/must be http\(s\)/);
  });

  it("empty string", () => {
    expect(() => resolveEntryUrl("", "https://b"))
      .toThrow(/non-empty/);
  });

  it("non-string", () => {
    // @ts-expect-error
    expect(() => resolveEntryUrl(42, "https://b"))
      .toThrow(/non-empty/);
  });

  it("null", () => {
    // @ts-expect-error
    expect(() => resolveEntryUrl(null, "https://b"))
      .toThrow(/null/);
  });

  it("long unsafe URL truncated in error", () => {
    const long = "javascript:" + "a".repeat(500);
    try {
      resolveEntryUrl(long, "https://b");
      expect.fail("expected throw");
    } catch (e) {
      const msg = (e as Error).message;
      expect(msg).toMatch(/…/);
      expect(msg.length).toBeLessThan(400);
    }
  });
});
