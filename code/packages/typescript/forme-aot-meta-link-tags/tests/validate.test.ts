/**
 * validate.test.ts — URL, rel/as/crossorigin allowlists, escape helpers.
 */

import { describe, it, expect } from "vitest";
import {
  escapeHtmlAttr,
  stripAsciiControl,
  validateCrossOrigin,
  validateHintAs,
  validateHintRel,
  validateIconRel,
  validateOptionalString,
  validateUrl,
} from "../src/index.js";

describe("validateUrl — accept", () => {
  it("https URL", () => {
    expect(validateUrl("https://example.com/page", "f")).toBe("https://example.com/page");
  });
  it("http URL", () => {
    expect(validateUrl("http://example.com", "f")).toBe("http://example.com");
  });
  it("scheme is case-insensitive", () => {
    expect(validateUrl("HTTPS://example.com", "f")).toBe("HTTPS://example.com");
    expect(validateUrl("HtTp://example.com", "f")).toBe("HtTp://example.com");
  });
  it("root-relative /path", () => {
    expect(validateUrl("/about", "f")).toBe("/about");
  });
  it("bare /", () => {
    expect(validateUrl("/", "f")).toBe("/");
  });
  it("multi-segment root-relative", () => {
    expect(validateUrl("/posts/2026/may", "f")).toBe("/posts/2026/may");
  });
});

describe("validateUrl — reject", () => {
  it("javascript:", () => {
    expect(() => validateUrl("javascript:alert(1)", "canonical"))
      .toThrow(/canonical must be http\(s\)/);
  });
  it("data:", () => {
    expect(() => validateUrl("data:text/html,x", "f")).toThrow(/http\(s\)/);
  });
  it("file:", () => {
    expect(() => validateUrl("file:///etc/passwd", "f")).toThrow(/http\(s\)/);
  });
  it("vbscript:", () => {
    expect(() => validateUrl("vbscript:msgbox", "f")).toThrow(/http\(s\)/);
  });
  it("protocol-relative //host", () => {
    expect(() => validateUrl("//evil.com", "f")).toThrow(/http\(s\)/);
  });
  it("backslash-variant /\\host", () => {
    expect(() => validateUrl("/\\evil.com", "f")).toThrow(/http\(s\)/);
  });
  it("bare relative", () => {
    expect(() => validateUrl("about", "f")).toThrow(/http\(s\)/);
  });
  it("empty string", () => {
    expect(() => validateUrl("", "f")).toThrow(/non-empty string/);
  });
  it("non-string number", () => {
    expect(() => validateUrl(42 as unknown as string, "f")).toThrow(/non-empty string/);
  });
  it("null", () => {
    expect(() => validateUrl(null, "f")).toThrow(/got null/);
  });
  it("undefined", () => {
    expect(() => validateUrl(undefined, "f")).toThrow(/got undefined/);
  });
  it("long URL truncated in error message", () => {
    const longUrl = "bad://" + "a".repeat(500);
    try {
      validateUrl(longUrl, "f");
      expect.fail("expected throw");
    } catch (e) {
      const msg = (e as Error).message;
      expect(msg).toContain("…");
      expect(msg.length).toBeLessThan(longUrl.length + 100);
    }
  });
  it("error includes the field path", () => {
    expect(() => validateUrl("javascript:x", "icons[3].href")).toThrow(/icons\[3\]\.href/);
  });
});

describe("validateIconRel", () => {
  it("accepts 'icon'", () => { expect(validateIconRel("icon", "f")).toBe("icon"); });
  it("accepts 'shortcut icon'", () => { expect(validateIconRel("shortcut icon", "f")).toBe("shortcut icon"); });
  it("accepts 'apple-touch-icon'", () => { expect(validateIconRel("apple-touch-icon", "f")).toBe("apple-touch-icon"); });
  it("accepts 'mask-icon'", () => { expect(validateIconRel("mask-icon", "f")).toBe("mask-icon"); });
  it("rejects 'manifest'", () => { expect(() => validateIconRel("manifest", "f")).toThrow(/one of/); });
  it("case-sensitive: 'ICON' rejected", () => { expect(() => validateIconRel("ICON", "f")).toThrow(/one of/); });
  it("rejects non-string", () => { expect(() => validateIconRel(123 as unknown as string, "f")).toThrow(/must be a string/); });
  it("rejects null", () => { expect(() => validateIconRel(null, "f")).toThrow(/must be a string/); });
});

describe("validateHintRel", () => {
  for (const v of ["preload", "prefetch", "preconnect", "dns-prefetch", "modulepreload"]) {
    it(`accepts '${v}'`, () => { expect(validateHintRel(v, "f")).toBe(v); });
  }
  it("rejects 'fetch'", () => { expect(() => validateHintRel("fetch", "f")).toThrow(/one of/); });
  it("rejects 'PRELOAD' (case-sensitive)", () => { expect(() => validateHintRel("PRELOAD", "f")).toThrow(/one of/); });
  it("rejects non-string", () => { expect(() => validateHintRel(0 as unknown as string, "f")).toThrow(/must be a string/); });
});

describe("validateHintAs", () => {
  for (const v of ["script", "style", "image", "font", "fetch", "document", "audio", "video", "track", "worker"]) {
    it(`accepts '${v}'`, () => { expect(validateHintAs(v, "f")).toBe(v); });
  }
  it("rejects 'iframe'", () => { expect(() => validateHintAs("iframe", "f")).toThrow(/one of/); });
  it("rejects 'object'", () => { expect(() => validateHintAs("object", "f")).toThrow(/one of/); });
  it("rejects non-string", () => { expect(() => validateHintAs(true as unknown as string, "f")).toThrow(/must be a string/); });
});

describe("validateCrossOrigin", () => {
  it("accepts 'anonymous'", () => { expect(validateCrossOrigin("anonymous", "f")).toBe("anonymous"); });
  it("accepts 'use-credentials'", () => { expect(validateCrossOrigin("use-credentials", "f")).toBe("use-credentials"); });
  it("rejects 'true'", () => { expect(() => validateCrossOrigin("true", "f")).toThrow(/one of/); });
  it("rejects empty", () => { expect(() => validateCrossOrigin("", "f")).toThrow(/one of/); });
  it("rejects non-string", () => { expect(() => validateCrossOrigin(1 as unknown as string, "f")).toThrow(/must be a string/); });
});

describe("validateOptionalString", () => {
  it("undefined passes through as undefined", () => {
    expect(validateOptionalString(undefined, "f")).toBeUndefined();
  });
  it("string passes through", () => {
    expect(validateOptionalString("hello", "f")).toBe("hello");
  });
  it("empty string allowed", () => {
    expect(validateOptionalString("", "f")).toBe("");
  });
  it("non-string number rejected", () => {
    expect(() => validateOptionalString(7 as unknown as string, "f")).toThrow(/must be a string/);
  });
  it("null rejected", () => {
    expect(() => validateOptionalString(null, "f")).toThrow(/must be a string/);
  });
  it("error includes field name", () => {
    expect(() => validateOptionalString(7 as unknown as string, "myField"))
      .toThrow(/myField must be a string/);
  });
});

describe("escapeHtmlAttr", () => {
  it("ampersand", () => { expect(escapeHtmlAttr("a&b")).toBe("a&amp;b"); });
  it("less-than", () => { expect(escapeHtmlAttr("a<b")).toBe("a&lt;b"); });
  it("greater-than", () => { expect(escapeHtmlAttr("a>b")).toBe("a&gt;b"); });
  it("double quote", () => { expect(escapeHtmlAttr(`a"b`)).toBe("a&quot;b"); });
  it("single quote", () => { expect(escapeHtmlAttr(`a'b`)).toBe("a&#39;b"); });
  it("composite", () => {
    expect(escapeHtmlAttr(`<a href="?x&y='z'">`))
      .toBe("&lt;a href=&quot;?x&amp;y=&#39;z&#39;&quot;&gt;");
  });
  it("ampersand ordered first (no double-escape)", () => {
    expect(escapeHtmlAttr("&amp;")).toBe("&amp;amp;");
  });
  it("strips NUL", () => { expect(escapeHtmlAttr("a\x00b")).toBe("ab"); });
  it("strips DEL", () => { expect(escapeHtmlAttr("a\x7Fb")).toBe("ab"); });
  it("strips ESC", () => { expect(escapeHtmlAttr("a\x1Bb")).toBe("ab"); });
  it("non-string coerced via String()", () => {
    expect(escapeHtmlAttr(42 as unknown as string)).toBe("42");
  });
});

describe("stripAsciiControl", () => {
  it("removes NUL/DEL/ESC", () => {
    expect(stripAsciiControl("a\x00b\x1Bc\x7Fd")).toBe("abcd");
  });
  it("preserves printable ASCII", () => {
    expect(stripAsciiControl("Hello, World!")).toBe("Hello, World!");
  });
  it("strips tab/newline/cr (these are <0x20)", () => {
    // tab=0x09, lf=0x0A, cr=0x0D — all in [0x00..0x1F] so they're stripped.
    expect(stripAsciiControl("a\tb\nc\rd")).toBe("abcd");
  });
});
