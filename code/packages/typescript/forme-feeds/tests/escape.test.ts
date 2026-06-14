/**
 * escape.test.ts — XML escaping + invalid-char stripping.
 */

import { describe, it, expect } from "vitest";
import { escapeXml, stripInvalidXml, wrapCdata } from "../src/index.js";

describe("escapeXml", () => {
  it("escapes the five XML predefined entities", () => {
    expect(escapeXml("&")).toBe("&amp;");
    expect(escapeXml("<")).toBe("&lt;");
    expect(escapeXml(">")).toBe("&gt;");
    expect(escapeXml(`"`)).toBe("&quot;");
    expect(escapeXml("'")).toBe("&apos;");
  });

  it("escapes all five in one string (single-pass)", () => {
    expect(escapeXml(`<a href="x">'AT&T'</a>`))
      .toBe("&lt;a href=&quot;x&quot;&gt;&apos;AT&amp;T&apos;&lt;/a&gt;");
  });

  it("passes through plain ASCII unchanged", () => {
    expect(escapeXml("Hello world 123")).toBe("Hello world 123");
  });

  it("passes through Unicode unchanged", () => {
    expect(escapeXml("café — résumé")).toBe("café — résumé");
  });

  it("strips invalid XML characters before escaping", () => {
    expect(escapeXml("a\x00b\x01c\x1fd")).toBe("abcd");
  });

  it("preserves the three allowed C0 controls (\\t, \\n, \\r)", () => {
    expect(escapeXml("a\tb\nc\rd")).toBe("a\tb\nc\rd");
  });
});

describe("stripInvalidXml", () => {
  it("strips NUL", () => {
    expect(stripInvalidXml("a\x00b")).toBe("ab");
  });

  it("strips vertical tab (0x0B) and form-feed (0x0C)", () => {
    expect(stripInvalidXml("a\x0bb\x0cc")).toBe("abc");
  });

  it("strips U+0001-U+0008", () => {
    const s = Array.from({ length: 8 }, (_, i) => String.fromCharCode(i + 1)).join("");
    expect(stripInvalidXml(`a${s}b`)).toBe("ab");
  });

  it("strips U+000E-U+001F", () => {
    const s = Array.from({ length: 18 }, (_, i) => String.fromCharCode(0x0e + i)).join("");
    expect(stripInvalidXml(`a${s}b`)).toBe("ab");
  });

  it("strips U+FFFE and U+FFFF", () => {
    expect(stripInvalidXml("a￾b￿c")).toBe("abc");
  });

  it("preserves tab, newline, carriage return", () => {
    expect(stripInvalidXml("a\tb\nc\rd")).toBe("a\tb\nc\rd");
  });
});

describe("wrapCdata", () => {
  it("wraps plain content in CDATA", () => {
    expect(wrapCdata("hello")).toBe("<![CDATA[hello]]>");
  });

  it("breaks `]]>` into safe form (prevents early CDATA termination)", () => {
    expect(wrapCdata("a]]>b")).toBe("<![CDATA[a]]]]><![CDATA[>b]]>");
  });

  it("strips invalid XML chars before wrapping", () => {
    expect(wrapCdata("a\x00b")).toBe("<![CDATA[ab]]>");
  });

  it("preserves HTML markup verbatim (CDATA bypasses XML parsing)", () => {
    expect(wrapCdata("<p>hello & world</p>")).toBe("<![CDATA[<p>hello & world</p>]]>");
  });
});
