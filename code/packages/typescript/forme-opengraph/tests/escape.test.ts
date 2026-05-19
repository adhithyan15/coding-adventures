/**
 * escape.test.ts — HTML attribute escaping + URL validation.
 */

import { describe, it, expect } from "vitest";
import { escapeHtmlAttr, escapeHtmlText, assertAbsoluteUrl } from "../src/index.js";

describe("escapeHtmlAttr", () => {
  it("escapes the five HTML entities", () => {
    expect(escapeHtmlAttr("&")).toBe("&amp;");
    expect(escapeHtmlAttr("<")).toBe("&lt;");
    expect(escapeHtmlAttr(">")).toBe("&gt;");
    expect(escapeHtmlAttr(`"`)).toBe("&quot;");
    expect(escapeHtmlAttr("'")).toBe("&#39;");
  });

  it("escapes all five in one string (single-pass)", () => {
    expect(escapeHtmlAttr(`<a href="x">'AT&T'</a>`))
      .toBe("&lt;a href=&quot;x&quot;&gt;&#39;AT&amp;T&#39;&lt;/a&gt;");
  });

  it("passes through plain ASCII unchanged", () => {
    expect(escapeHtmlAttr("Hello world 123")).toBe("Hello world 123");
  });

  it("passes through Unicode > 0x7F unchanged (HTML5 UTF-8)", () => {
    expect(escapeHtmlAttr("café — résumé 你好")).toBe("café — résumé 你好");
  });

  it("strips ASCII control bytes (0x00-0x1F)", () => {
    expect(escapeHtmlAttr("a\x00b\x01c\x1fd")).toBe("abcd");
  });

  it("strips DEL (0x7F)", () => {
    expect(escapeHtmlAttr("a\x7fb")).toBe("ab");
  });

  it("escapeHtmlText is an alias of escapeHtmlAttr", () => {
    const samples = [`&`, `<`, `"`, `'`, `Hello`, `a\x00b`];
    for (const s of samples) {
      expect(escapeHtmlText(s)).toBe(escapeHtmlAttr(s));
    }
  });
});

describe("assertAbsoluteUrl", () => {
  it("accepts http:// URLs", () => {
    expect(() => assertAbsoluteUrl("test", "http://example.com/x")).not.toThrow();
  });

  it("accepts https:// URLs", () => {
    expect(() => assertAbsoluteUrl("test", "https://example.com/x")).not.toThrow();
  });

  it("accepts URLs with query / fragment / port", () => {
    expect(() => assertAbsoluteUrl("test", "https://example.com:8443/x?a=1&b=2#frag")).not.toThrow();
  });

  it("is case-insensitive on the scheme", () => {
    expect(() => assertAbsoluteUrl("test", "HTTPS://example.com")).not.toThrow();
  });

  it("rejects relative paths", () => {
    expect(() => assertAbsoluteUrl("test", "/path"))
      .toThrow(/must be an absolute http\(s\) URL/);
    expect(() => assertAbsoluteUrl("test", "path"))
      .toThrow(/must be an absolute http\(s\) URL/);
    expect(() => assertAbsoluteUrl("test", "./x"))
      .toThrow(/must be an absolute http\(s\) URL/);
  });

  it("rejects javascript: URLs (injection vector)", () => {
    expect(() => assertAbsoluteUrl("test", "javascript:alert(1)"))
      .toThrow(/must be an absolute http\(s\) URL/);
  });

  it("rejects data: URLs (injection vector)", () => {
    expect(() => assertAbsoluteUrl("test", "data:text/html,<script>alert(1)</script>"))
      .toThrow(/must be an absolute http\(s\) URL/);
  });

  it("rejects file: URLs", () => {
    expect(() => assertAbsoluteUrl("test", "file:///etc/passwd"))
      .toThrow(/must be an absolute http\(s\) URL/);
  });

  it("rejects protocol-relative URLs (no scheme)", () => {
    expect(() => assertAbsoluteUrl("test", "//example.com/x"))
      .toThrow(/must be an absolute http\(s\) URL/);
  });

  it("rejects empty string", () => {
    expect(() => assertAbsoluteUrl("test", ""))
      .toThrow(/must be a non-empty string/);
  });

  it("rejects non-string values", () => {
    expect(() => assertAbsoluteUrl("test", null))
      .toThrow(/must be a non-empty string/);
    expect(() => assertAbsoluteUrl("test", 42))
      .toThrow(/must be a non-empty string/);
    expect(() => assertAbsoluteUrl("test", undefined))
      .toThrow(/must be a non-empty string/);
  });

  it("includes the field name in the error message", () => {
    expect(() => assertAbsoluteUrl("og:image", "relative"))
      .toThrow(/og:image/);
  });
});
