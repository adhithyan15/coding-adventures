/**
 * escape.test.ts — HTML escape + URL validation.
 */

import { describe, it, expect } from "vitest";
import { escapeHtmlAttr, escapeHtmlText, assertItemUrl } from "../src/index.js";

describe("escapeHtmlAttr", () => {
  it("escapes the five HTML entities", () => {
    expect(escapeHtmlAttr(`<a href="x">'AT&T'</a>`))
      .toBe(`&lt;a href=&quot;x&quot;&gt;&#39;AT&amp;T&#39;&lt;/a&gt;`);
  });

  it("strips ASCII control bytes", () => {
    expect(escapeHtmlAttr("Hello\x00\x1fWorld")).toBe("HelloWorld");
  });

  it("passes through Unicode unchanged", () => {
    expect(escapeHtmlAttr("café — résumé")).toBe("café — résumé");
  });

  it("escapeHtmlText is an alias", () => {
    expect(escapeHtmlText("<a>")).toBe("&lt;a&gt;");
  });
});

describe("assertItemUrl", () => {
  it("accepts absolute http(s)", () => {
    expect(() => assertItemUrl("https://example.com/x")).not.toThrow();
    expect(() => assertItemUrl("http://example.com/x")).not.toThrow();
  });

  it("accepts root-relative /path", () => {
    expect(() => assertItemUrl("/about")).not.toThrow();
    expect(() => assertItemUrl("/blog/post.html")).not.toThrow();
  });

  it("accepts bare root /", () => {
    expect(() => assertItemUrl("/")).not.toThrow();
  });

  it("rejects javascript: URL", () => {
    expect(() => assertItemUrl("javascript:alert(1)"))
      .toThrow(/absolute http\(s\) or root-relative/);
  });

  it("rejects data: URL", () => {
    expect(() => assertItemUrl("data:text/html,<script>alert(1)</script>"))
      .toThrow(/absolute http\(s\) or root-relative/);
  });

  it("rejects file: URL", () => {
    expect(() => assertItemUrl("file:///etc/passwd"))
      .toThrow(/absolute http\(s\) or root-relative/);
  });

  it("rejects protocol-relative //host", () => {
    expect(() => assertItemUrl("//example.com/x"))
      .toThrow(/absolute http\(s\) or root-relative/);
  });

  it("rejects bare relative path", () => {
    expect(() => assertItemUrl("about"))
      .toThrow(/absolute http\(s\) or root-relative/);
    expect(() => assertItemUrl("./about"))
      .toThrow(/absolute http\(s\) or root-relative/);
  });

  it("rejects empty / non-string", () => {
    expect(() => assertItemUrl("")).toThrow(/non-empty string/);
    expect(() => assertItemUrl(null)).toThrow(/non-empty string/);
    expect(() => assertItemUrl(42)).toThrow(/non-empty string/);
  });
});
