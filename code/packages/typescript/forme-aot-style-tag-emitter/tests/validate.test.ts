/**
 * validate.test.ts — URL, SRI integrity, crossorigin,
 * inline-CSS, escape helpers.
 */

import { describe, it, expect } from "vitest";
import {
  escapeHtmlAttr,
  stripAsciiControl,
  validateCrossOrigin,
  validateInlineCss,
  validateIntegrity,
  validateOptionalString,
  validateStyleHref,
} from "../src/index.js";

const SHA256_B64 = "A".repeat(43) + "=";
const SHA384_B64 = "A".repeat(64);
const SHA512_B64 = "A".repeat(86) + "==";

describe("validateStyleHref — accept", () => {
  it("https URL", () => {
    expect(validateStyleHref("https://example.com/x.css", "f")).toBe("https://example.com/x.css");
  });
  it("http URL", () => {
    expect(validateStyleHref("http://example.com/x.css", "f")).toBe("http://example.com/x.css");
  });
  it("scheme case-insensitive", () => {
    expect(validateStyleHref("HTTPS://example.com/x.css", "f")).toBe("HTTPS://example.com/x.css");
  });
  it("root-relative", () => { expect(validateStyleHref("/main.css", "f")).toBe("/main.css"); });
  it("bare /", () => { expect(validateStyleHref("/", "f")).toBe("/"); });
  it("multi-segment", () => { expect(validateStyleHref("/assets/css/main.css", "f")).toBe("/assets/css/main.css"); });
});

describe("validateStyleHref — reject", () => {
  it("javascript:", () => { expect(() => validateStyleHref("javascript:x", "href")).toThrow(/href must be http\(s\)/); });
  it("data:", () => { expect(() => validateStyleHref("data:text/css,x", "f")).toThrow(/http\(s\)/); });
  it("file:", () => { expect(() => validateStyleHref("file:///etc", "f")).toThrow(/http\(s\)/); });
  it("vbscript:", () => { expect(() => validateStyleHref("vbscript:x", "f")).toThrow(/http\(s\)/); });
  it("protocol-relative //host", () => { expect(() => validateStyleHref("//evil.com/x.css", "f")).toThrow(/http\(s\)/); });
  it("backslash-variant", () => { expect(() => validateStyleHref("/\\evil.com/x.css", "f")).toThrow(/http\(s\)/); });
  it("bare relative", () => { expect(() => validateStyleHref("main.css", "f")).toThrow(/http\(s\)/); });
  it("empty", () => { expect(() => validateStyleHref("", "f")).toThrow(/non-empty/); });
  it("non-string", () => { expect(() => validateStyleHref(42 as unknown as string, "f")).toThrow(/non-empty/); });
  it("null", () => { expect(() => validateStyleHref(null, "f")).toThrow(/got null/); });
  it("undefined", () => { expect(() => validateStyleHref(undefined, "f")).toThrow(/got undefined/); });
  it("NUL byte rejected", () => { expect(() => validateStyleHref("/x.css\x00", "f")).toThrow(/control bytes/); });
  it("tab rejected", () => { expect(() => validateStyleHref("/\tevil", "f")).toThrow(/control bytes/); });
  it("newline rejected", () => { expect(() => validateStyleHref("/x\nevil", "f")).toThrow(/control bytes/); });
  it("DEL rejected", () => { expect(() => validateStyleHref("/x\x7F", "f")).toThrow(/control bytes/); });
  it("error contains field name", () => {
    expect(() => validateStyleHref("javascript:x", "stylesheets[2].href"))
      .toThrow(/stylesheets\[2\]\.href/);
  });
  it("long URL truncated", () => {
    const long = "bad://" + "a".repeat(500);
    try {
      validateStyleHref(long, "f");
      expect.fail("expected throw");
    } catch (e) {
      const msg = (e as Error).message;
      expect(msg).toContain("…");
      expect(msg.length).toBeLessThan(long.length + 100);
    }
  });
});

describe("validateIntegrity — accept", () => {
  it("sha256", () => { expect(validateIntegrity(`sha256-${SHA256_B64}`, "f")).toBe(`sha256-${SHA256_B64}`); });
  it("sha384", () => { expect(validateIntegrity(`sha384-${SHA384_B64}`, "f")).toBe(`sha384-${SHA384_B64}`); });
  it("sha512", () => { expect(validateIntegrity(`sha512-${SHA512_B64}`, "f")).toBe(`sha512-${SHA512_B64}`); });
  it("two algos joined", () => {
    const in_ = `sha256-${SHA256_B64} sha384-${SHA384_B64}`;
    expect(validateIntegrity(in_, "f")).toBe(in_);
  });
  it("collapses whitespace", () => {
    expect(validateIntegrity(`sha256-${SHA256_B64}\t\tsha384-${SHA384_B64}`, "f"))
      .toBe(`sha256-${SHA256_B64} sha384-${SHA384_B64}`);
  });
  it("trims surrounding whitespace", () => {
    expect(validateIntegrity(`  sha256-${SHA256_B64}  `, "f")).toBe(`sha256-${SHA256_B64}`);
  });
});

describe("validateIntegrity — reject", () => {
  it("non-string", () => { expect(() => validateIntegrity(42 as unknown as string, "f")).toThrow(/must be a string/); });
  it("empty", () => { expect(() => validateIntegrity("", "f")).toThrow(/non-empty/); });
  it("whitespace-only", () => { expect(() => validateIntegrity("   ", "f")).toThrow(/non-empty/); });
  it("md5 algo", () => { expect(() => validateIntegrity("md5-abc", "f")).toThrow(/algo must be one of/); });
  it("sha1 algo", () => { expect(() => validateIntegrity("sha1-abc", "f")).toThrow(/algo must be one of/); });
  it("no dash", () => { expect(() => validateIntegrity("sha256abc", "f")).toThrow(/"<algo>-<base64>"/); });
  it("dash at start", () => { expect(() => validateIntegrity("-abc", "f")).toThrow(/"<algo>-<base64>"/); });
  it("dash at end", () => { expect(() => validateIntegrity("sha256-", "f")).toThrow(/"<algo>-<base64>"/); });
  it("sha256 wrong length", () => { expect(() => validateIntegrity(`sha256-${"A".repeat(20)}=`, "f")).toThrow(/sha256 expects 44/); });
  it("sha384 wrong length", () => { expect(() => validateIntegrity(`sha384-${"A".repeat(60)}`, "f")).toThrow(/sha384 expects 64/); });
  it("sha512 wrong length", () => { expect(() => validateIntegrity(`sha512-${"A".repeat(80)}==`, "f")).toThrow(/sha512 expects 88/); });
  it("invalid base64 char @", () => { expect(() => validateIntegrity(`sha256-${"A".repeat(42)}@=`, "f")).toThrow(/invalid characters/); });
  it("URL-safe _ rejected", () => { expect(() => validateIntegrity(`sha256-${"A".repeat(42)}_=`, "f")).toThrow(/invalid characters/); });
  it("triple padding rejected", () => { expect(() => validateIntegrity(`sha256-${"A".repeat(41)}===`, "f")).toThrow(/invalid characters/); });
  it("sha256 wrong padding (== instead of =)", () => {
    expect(() => validateIntegrity(`sha256-${"A".repeat(42)}==`, "f"))
      .toThrow(/sha256 requires 1 '=' padding/);
  });
  it("sha384 with padding rejected", () => {
    expect(() => validateIntegrity(`sha384-${"A".repeat(62)}==`, "f"))
      .toThrow(/sha384 requires 0 '=' padding chars/);
  });
  it("sha512 wrong padding rejected", () => {
    expect(() => validateIntegrity(`sha512-${"A".repeat(87)}=`, "f"))
      .toThrow(/sha512 requires 2 '=' padding chars/);
  });
  it("__proto__ algo rejected (Object.prototype walk defence)", () => {
    expect(() => validateIntegrity("__proto__-AAAA", "f")).toThrow(/algo must be one of/);
  });
  it("toString algo rejected", () => {
    expect(() => validateIntegrity("toString-AAAA", "f")).toThrow(/algo must be one of/);
  });
  it("hasOwnProperty algo rejected", () => {
    expect(() => validateIntegrity("hasOwnProperty-AAAA", "f")).toThrow(/algo must be one of/);
  });
  it("second token bad", () => {
    expect(() => validateIntegrity(`sha256-${SHA256_B64} md5-xxx`, "f")).toThrow(/algo must be one of/);
  });
  it("error contains field name", () => {
    expect(() => validateIntegrity("md5-aaaa", "stylesheets[0].integrity"))
      .toThrow(/stylesheets\[0\]\.integrity/);
  });
});

describe("validateCrossOrigin", () => {
  it("anonymous", () => { expect(validateCrossOrigin("anonymous", "f")).toBe("anonymous"); });
  it("use-credentials", () => { expect(validateCrossOrigin("use-credentials", "f")).toBe("use-credentials"); });
  it("rejects 'true'", () => { expect(() => validateCrossOrigin("true", "f")).toThrow(/one of/); });
  it("rejects empty", () => { expect(() => validateCrossOrigin("", "f")).toThrow(/one of/); });
  it("rejects non-string", () => { expect(() => validateCrossOrigin(1 as unknown as string, "f")).toThrow(/must be a string/); });
});

describe("validateInlineCss", () => {
  it("benign CSS passes through", () => {
    expect(validateInlineCss(":root { --c: blue; }", "f")).toBe(":root { --c: blue; }");
  });
  it("empty string allowed (renders empty <style>)", () => {
    expect(validateInlineCss("", "f")).toBe("");
  });
  it("CSS with < and > but not </style> passes", () => {
    expect(validateInlineCss("a > b { color: red; }", "f")).toBe("a > b { color: red; }");
  });
  it("CSS with `style` substring but no closing tag passes", () => {
    expect(validateInlineCss(".style { color: red; }", "f")).toBe(".style { color: red; }");
  });
  it("rejects literal </style>", () => {
    expect(() => validateInlineCss("body{} </style><script>alert(1)</script>", "f"))
      .toThrow(/literal <\/style> sequence/);
  });
  it("rejects case-variant </STYLE>", () => {
    expect(() => validateInlineCss("</STYLE>", "f")).toThrow(/literal <\/style>/);
  });
  it("rejects mixed-case </StYlE>", () => {
    expect(() => validateInlineCss("</StYlE>", "f")).toThrow(/literal <\/style>/);
  });
  it("rejects </style with whitespace", () => {
    expect(() => validateInlineCss("</style >", "f")).toThrow(/literal <\/style>/);
  });
  it("rejects </style with slash", () => {
    expect(() => validateInlineCss("</style/x", "f")).toThrow(/literal <\/style>/);
  });
  it("permits </styles> (different tag name, not a close)", () => {
    // The regex requires </style followed by whitespace/>//, so </styles
    // would only match if 's' were in that set.  Actually </styles starts
    // with </style followed by 's' which is NOT in [\s>/], so it's NOT
    // matched — passes through.
    expect(validateInlineCss(".x { content: '</styles'; }", "f"))
      .toBe(".x { content: '</styles'; }");
  });
  it("rejects non-string", () => {
    expect(() => validateInlineCss(42 as unknown as string, "f"))
      .toThrow(/must be a string/);
  });
  it("rejects null", () => {
    expect(() => validateInlineCss(null, "f")).toThrow(/must be a string/);
  });
  it("error contains field path", () => {
    expect(() => validateInlineCss("</style>", "inline[2].css"))
      .toThrow(/inline\[2\]\.css/);
  });
});

describe("validateOptionalString", () => {
  it("undefined → undefined", () => { expect(validateOptionalString(undefined, "f")).toBeUndefined(); });
  it("string → string", () => { expect(validateOptionalString("x", "f")).toBe("x"); });
  it("empty string allowed", () => { expect(validateOptionalString("", "f")).toBe(""); });
  it("non-string rejected", () => { expect(() => validateOptionalString(7 as unknown as string, "f")).toThrow(/must be a string/); });
  it("null rejected", () => { expect(() => validateOptionalString(null, "f")).toThrow(/must be a string/); });
});

describe("escapeHtmlAttr", () => {
  it("ampersand", () => { expect(escapeHtmlAttr("a&b")).toBe("a&amp;b"); });
  it("less-than", () => { expect(escapeHtmlAttr("a<b")).toBe("a&lt;b"); });
  it("greater-than", () => { expect(escapeHtmlAttr("a>b")).toBe("a&gt;b"); });
  it("double quote", () => { expect(escapeHtmlAttr(`"`)).toBe("&quot;"); });
  it("single quote", () => { expect(escapeHtmlAttr(`'`)).toBe("&#39;"); });
  it("composite", () => { expect(escapeHtmlAttr(`<&>"'`)).toBe("&lt;&amp;&gt;&quot;&#39;"); });
  it("strips NUL", () => { expect(escapeHtmlAttr("a\x00b")).toBe("ab"); });
  it("non-string coerced", () => { expect(escapeHtmlAttr(7 as unknown as string)).toBe("7"); });
});

describe("stripAsciiControl", () => {
  it("removes control bytes", () => {
    expect(stripAsciiControl("a\x00b\x1Bc\x7Fd")).toBe("abcd");
  });
  it("preserves printable", () => {
    expect(stripAsciiControl("Hello, World!")).toBe("Hello, World!");
  });
});
