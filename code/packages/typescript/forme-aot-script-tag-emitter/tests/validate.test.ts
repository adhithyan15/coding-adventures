/**
 * validate.test.ts — URL, SRI integrity, type / crossorigin /
 * referrerpolicy allowlists, escape helpers.
 */

import { describe, it, expect } from "vitest";
import {
  escapeHtmlAttr,
  stripAsciiControl,
  validateCrossOrigin,
  validateIntegrity,
  validateReferrerPolicy,
  validateScriptSrc,
  validateScriptType,
} from "../src/index.js";

// Pre-computed valid base64 of exact length for each algo.
// 32 bytes (sha256) → 44 chars (43 'A' + '=')
const SHA256_B64 = "A".repeat(43) + "=";
// 48 bytes (sha384) → 64 chars (no padding needed since 48 % 3 == 0)
const SHA384_B64 = "A".repeat(64);
// 64 bytes (sha512) → 88 chars (86 'A' + '==')
const SHA512_B64 = "A".repeat(86) + "==";

describe("validateScriptSrc — accept", () => {
  it("https URL", () => {
    expect(validateScriptSrc("https://example.com/app.js")).toBe("https://example.com/app.js");
  });
  it("http URL", () => {
    expect(validateScriptSrc("http://example.com/x.js")).toBe("http://example.com/x.js");
  });
  it("scheme case-insensitive", () => {
    expect(validateScriptSrc("HTTPS://example.com/x.js")).toBe("HTTPS://example.com/x.js");
    expect(validateScriptSrc("HtTp://example.com/x.js")).toBe("HtTp://example.com/x.js");
  });
  it("root-relative /path", () => {
    expect(validateScriptSrc("/main.js")).toBe("/main.js");
  });
  it("bare /", () => {
    expect(validateScriptSrc("/")).toBe("/");
  });
  it("multi-segment root-relative", () => {
    expect(validateScriptSrc("/assets/js/main.js")).toBe("/assets/js/main.js");
  });
});

describe("validateScriptSrc — reject", () => {
  it("javascript:", () => {
    expect(() => validateScriptSrc("javascript:alert(1)")).toThrow(/src must be http\(s\)/);
  });
  it("data:", () => {
    expect(() => validateScriptSrc("data:application/javascript,alert(1)")).toThrow(/http\(s\)/);
  });
  it("file:", () => {
    expect(() => validateScriptSrc("file:///etc/passwd")).toThrow(/http\(s\)/);
  });
  it("vbscript:", () => {
    expect(() => validateScriptSrc("vbscript:msgbox")).toThrow(/http\(s\)/);
  });
  it("protocol-relative //host", () => {
    expect(() => validateScriptSrc("//evil.com/x.js")).toThrow(/http\(s\)/);
  });
  it("backslash-variant /\\host", () => {
    expect(() => validateScriptSrc("/\\evil.com/x.js")).toThrow(/http\(s\)/);
  });
  it("bare relative", () => {
    expect(() => validateScriptSrc("about.js")).toThrow(/http\(s\)/);
  });
  it("empty", () => {
    expect(() => validateScriptSrc("")).toThrow(/non-empty string/);
  });
  it("non-string number", () => {
    expect(() => validateScriptSrc(42 as unknown as string)).toThrow(/non-empty string/);
  });
  it("null", () => {
    expect(() => validateScriptSrc(null)).toThrow(/got null/);
  });
  it("undefined", () => {
    expect(() => validateScriptSrc(undefined)).toThrow(/got undefined/);
  });
  it("long URL truncated in error", () => {
    const longUrl = "bad://" + "a".repeat(500);
    try {
      validateScriptSrc(longUrl);
      expect.fail("expected throw");
    } catch (e) {
      const msg = (e as Error).message;
      expect(msg).toContain("…");
      expect(msg.length).toBeLessThan(longUrl.length + 100);
    }
  });
  it("NUL byte in src rejected", () => {
    expect(() => validateScriptSrc("/main.js\x00"))
      .toThrow(/must not contain ASCII control bytes/);
  });
  it("tab in src rejected (defends against /\\tevil bypass)", () => {
    expect(() => validateScriptSrc("/\tevil")).toThrow(/control bytes/);
  });
  it("newline in src rejected", () => {
    expect(() => validateScriptSrc("/x\nevil")).toThrow(/control bytes/);
  });
  it("DEL byte in src rejected", () => {
    expect(() => validateScriptSrc("/x\x7Fy")).toThrow(/control bytes/);
  });
  it("ESC byte in src rejected", () => {
    expect(() => validateScriptSrc("/x\x1By")).toThrow(/control bytes/);
  });
});

describe("validateIntegrity — accept", () => {
  it("sha256 single hash", () => {
    expect(validateIntegrity(`sha256-${SHA256_B64}`)).toBe(`sha256-${SHA256_B64}`);
  });
  it("sha384 single hash", () => {
    expect(validateIntegrity(`sha384-${SHA384_B64}`)).toBe(`sha384-${SHA384_B64}`);
  });
  it("sha512 single hash", () => {
    expect(validateIntegrity(`sha512-${SHA512_B64}`)).toBe(`sha512-${SHA512_B64}`);
  });
  it("two algos space-separated", () => {
    const in_ = `sha256-${SHA256_B64} sha384-${SHA384_B64}`;
    expect(validateIntegrity(in_)).toBe(in_);
  });
  it("three algos", () => {
    const in_ = `sha256-${SHA256_B64} sha384-${SHA384_B64} sha512-${SHA512_B64}`;
    expect(validateIntegrity(in_)).toBe(in_);
  });
  it("collapses multiple whitespace between tokens", () => {
    const in_ = `sha256-${SHA256_B64}   sha384-${SHA384_B64}`;
    expect(validateIntegrity(in_)).toBe(`sha256-${SHA256_B64} sha384-${SHA384_B64}`);
  });
  it("collapses tab/newline whitespace", () => {
    const in_ = `sha256-${SHA256_B64}\tsha384-${SHA384_B64}`;
    expect(validateIntegrity(in_)).toBe(`sha256-${SHA256_B64} sha384-${SHA384_B64}`);
  });
  it("trims surrounding whitespace", () => {
    expect(validateIntegrity(`  sha256-${SHA256_B64}  `)).toBe(`sha256-${SHA256_B64}`);
  });
  it("base64 with + and /", () => {
    // Valid 44-char base64 with + and /
    const b64 = "+".repeat(20) + "/".repeat(23) + "=";
    expect(validateIntegrity(`sha256-${b64}`)).toBe(`sha256-${b64}`);
  });
  it("base64 with all alphabet classes", () => {
    // 44 chars mixing alpha/num/+//
    const b64 = "AaBbCcDdEeFfGgHhIiJjKkLlMmNnOoPpQqRrSsTtUu0=";
    expect(b64.length).toBe(44);
    expect(validateIntegrity(`sha256-${b64}`)).toBe(`sha256-${b64}`);
  });
});

describe("validateIntegrity — reject", () => {
  it("non-string", () => {
    expect(() => validateIntegrity(42 as unknown as string)).toThrow(/must be a string/);
  });
  it("null", () => {
    expect(() => validateIntegrity(null)).toThrow(/must be a string/);
  });
  it("empty", () => {
    expect(() => validateIntegrity("")).toThrow(/non-empty/);
  });
  it("whitespace-only", () => {
    expect(() => validateIntegrity("   ")).toThrow(/non-empty/);
  });
  it("unknown algo md5", () => {
    expect(() => validateIntegrity("md5-abc")).toThrow(/algo must be one of/);
  });
  it("unknown algo sha1", () => {
    expect(() => validateIntegrity("sha1-abc")).toThrow(/algo must be one of/);
  });
  it("no dash at all", () => {
    expect(() => validateIntegrity("sha256abc")).toThrow(/"<algo>-<base64>"/);
  });
  it("dash at start (empty algo)", () => {
    expect(() => validateIntegrity("-abc")).toThrow(/"<algo>-<base64>"/);
  });
  it("dash at end (empty b64)", () => {
    expect(() => validateIntegrity("sha256-")).toThrow(/"<algo>-<base64>"/);
  });
  it("sha256 wrong length (too short)", () => {
    expect(() => validateIntegrity(`sha256-${"A".repeat(20)}=`))
      .toThrow(/sha256 expects 44-char base64; got 21/);
  });
  it("sha256 wrong length (too long)", () => {
    expect(() => validateIntegrity(`sha256-${"A".repeat(60)}`))
      .toThrow(/sha256 expects 44-char base64; got 60/);
  });
  it("sha384 wrong length", () => {
    expect(() => validateIntegrity(`sha384-${"A".repeat(60)}`))
      .toThrow(/sha384 expects 64-char base64; got 60/);
  });
  it("sha512 wrong length", () => {
    expect(() => validateIntegrity(`sha512-${"A".repeat(80)}==`))
      .toThrow(/sha512 expects 88-char base64; got 82/);
  });
  it("base64 with invalid char (space inside)", () => {
    const bad = "A".repeat(20) + " " + "A".repeat(22) + "=";
    // After split by /\s+/ this becomes two tokens; the second will fail "<algo>-<base64>".
    expect(() => validateIntegrity(`sha256-${bad}`)).toThrow();
  });
  it("base64 with invalid char (@)", () => {
    const bad = "A".repeat(42) + "@" + "=";
    expect(() => validateIntegrity(`sha256-${bad}`)).toThrow(/invalid characters/);
  });
  it("base64 with URL-safe chars (- _) rejected", () => {
    // sha256 b64 with `_` replacing `/` (not allowed by SRI spec).
    const bad = "A".repeat(42) + "_" + "=";
    expect(() => validateIntegrity(`sha256-${bad}`)).toThrow(/invalid characters/);
  });
  it("base64 with three padding =", () => {
    // 41 A + '===' = 44, but BASE64_RE allows {0,2} = padding.
    const bad = "A".repeat(41) + "===";
    expect(() => validateIntegrity(`sha256-${bad}`)).toThrow(/invalid characters/);
  });
  it("sha256 with wrong padding (== instead of =) rejected", () => {
    // 42 A + '==' = 44 chars, passes length check but decodes to 31 bytes not 32.
    const bad = "A".repeat(42) + "==";
    expect(() => validateIntegrity(`sha256-${bad}`))
      .toThrow(/sha256 requires 1 '=' padding/);
  });
  it("sha384 with extra padding rejected", () => {
    // 62 A + '==' = 64 chars, passes length but pad expected = 0
    const bad = "A".repeat(62) + "==";
    expect(() => validateIntegrity(`sha384-${bad}`))
      .toThrow(/sha384 requires 0 '=' padding chars/);
  });
  it("sha512 with wrong padding rejected", () => {
    // 87 A + '=' = 88 chars, passes length but pad expected = 2
    const bad = "A".repeat(87) + "=";
    expect(() => validateIntegrity(`sha512-${bad}`))
      .toThrow(/sha512 requires 2 '=' padding chars/);
  });
  it("__proto__ algo rejected (Object.prototype walk defence)", () => {
    expect(() => validateIntegrity("__proto__-AAAA"))
      .toThrow(/algo must be one of/);
  });
  it("toString algo rejected (Object.prototype walk defence)", () => {
    expect(() => validateIntegrity("toString-AAAA"))
      .toThrow(/algo must be one of/);
  });
  it("hasOwnProperty algo rejected", () => {
    expect(() => validateIntegrity("hasOwnProperty-AAAA"))
      .toThrow(/algo must be one of/);
  });
  it("second-in-pair token bad", () => {
    expect(() => validateIntegrity(`sha256-${SHA256_B64} md5-xxx`))
      .toThrow(/algo must be one of/);
  });
  it("error message contains the bad algo", () => {
    try {
      validateIntegrity("md5-aaaa");
      expect.fail("expected throw");
    } catch (e) {
      expect((e as Error).message).toContain("md5");
    }
  });
});

describe("validateScriptType", () => {
  it("accepts module", () => { expect(validateScriptType("module")).toBe("module"); });
  it("accepts importmap", () => { expect(validateScriptType("importmap")).toBe("importmap"); });
  it("rejects text/javascript", () => { expect(() => validateScriptType("text/javascript")).toThrow(/one of/); });
  it("rejects application/javascript", () => { expect(() => validateScriptType("application/javascript")).toThrow(/one of/); });
  it("rejects 'MODULE' (case-sensitive)", () => { expect(() => validateScriptType("MODULE")).toThrow(/one of/); });
  it("rejects empty", () => { expect(() => validateScriptType("")).toThrow(/one of/); });
  it("rejects non-string", () => { expect(() => validateScriptType(true as unknown as string)).toThrow(/must be a string/); });
});

describe("validateCrossOrigin", () => {
  it("accepts anonymous", () => { expect(validateCrossOrigin("anonymous")).toBe("anonymous"); });
  it("accepts use-credentials", () => { expect(validateCrossOrigin("use-credentials")).toBe("use-credentials"); });
  it("rejects 'true'", () => { expect(() => validateCrossOrigin("true")).toThrow(/one of/); });
  it("rejects empty", () => { expect(() => validateCrossOrigin("")).toThrow(/one of/); });
  it("rejects non-string", () => { expect(() => validateCrossOrigin(1 as unknown as string)).toThrow(/must be a string/); });
});

describe("validateReferrerPolicy", () => {
  const values = [
    "no-referrer", "no-referrer-when-downgrade", "origin",
    "origin-when-cross-origin", "same-origin", "strict-origin",
    "strict-origin-when-cross-origin", "unsafe-url",
  ];
  for (const v of values) {
    it(`accepts '${v}'`, () => { expect(validateReferrerPolicy(v)).toBe(v); });
  }
  it("rejects 'NO-REFERRER' (case-sensitive)", () => { expect(() => validateReferrerPolicy("NO-REFERRER")).toThrow(/one of/); });
  it("rejects 'never' (deprecated value)", () => { expect(() => validateReferrerPolicy("never")).toThrow(/one of/); });
  it("rejects empty", () => { expect(() => validateReferrerPolicy("")).toThrow(/one of/); });
  it("rejects non-string", () => { expect(() => validateReferrerPolicy(0 as unknown as string)).toThrow(/must be a string/); });
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
  it("strips NUL", () => { expect(escapeHtmlAttr("a\x00b")).toBe("ab"); });
  it("strips DEL", () => { expect(escapeHtmlAttr("a\x7Fb")).toBe("ab"); });
  it("non-string coerced", () => { expect(escapeHtmlAttr(7 as unknown as string)).toBe("7"); });
});

describe("stripAsciiControl", () => {
  it("removes NUL/DEL/ESC", () => {
    expect(stripAsciiControl("a\x00b\x1Bc\x7Fd")).toBe("abcd");
  });
  it("preserves printable ASCII", () => {
    expect(stripAsciiControl("Hello, World!")).toBe("Hello, World!");
  });
});
