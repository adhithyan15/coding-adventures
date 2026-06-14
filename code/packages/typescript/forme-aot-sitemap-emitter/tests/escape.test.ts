/**
 * escape.test.ts — XML escape + invalid-character strip.
 */

import { describe, it, expect } from "vitest";
import { escapeXml, stripInvalidXml } from "../src/index.js";

describe("escapeXml — all five entities", () => {
  it("ampersand", () => expect(escapeXml("a&b")).toBe("a&amp;b"));
  it("less-than", () => expect(escapeXml("a<b")).toBe("a&lt;b"));
  it("greater-than", () => expect(escapeXml("a>b")).toBe("a&gt;b"));
  it("double quote", () => expect(escapeXml(`a"b`)).toBe("a&quot;b"));
  it("single quote", () => expect(escapeXml(`a'b`)).toBe("a&apos;b"));

  it("composite", () => {
    expect(escapeXml(`<a href="x?a=1&b=2">'y'</a>`))
      .toBe(`&lt;a href=&quot;x?a=1&amp;b=2&quot;&gt;&apos;y&apos;&lt;/a&gt;`);
  });

  it("ampersand-first ordering (no double-escape)", () => {
    expect(escapeXml("&lt;")).toBe("&amp;lt;");
  });
});

describe("escapeXml — passthrough", () => {
  it("empty string", () => expect(escapeXml("")).toBe(""));
  it("plain ASCII unchanged", () => expect(escapeXml("hello world")).toBe("hello world"));
  it("digits unchanged", () => expect(escapeXml("123")).toBe("123"));
  it("unicode > U+007F passes through", () => expect(escapeXml("日本語")).toBe("日本語"));
});

describe("stripInvalidXml — XML 1.0 forbidden C0", () => {
  it("strips NUL", () => expect(stripInvalidXml("a\x00b")).toBe("ab"));
  it("strips backspace", () => expect(stripInvalidXml("a\x08b")).toBe("ab"));
  it("strips vertical tab", () => expect(stripInvalidXml("a\x0Bb")).toBe("ab"));
  it("strips form feed", () => expect(stripInvalidXml("a\x0Cb")).toBe("ab"));
  it("strips SO (shift out)", () => expect(stripInvalidXml("a\x0Eb")).toBe("ab"));
  it("strips ESC", () => expect(stripInvalidXml("a\x1Bb")).toBe("ab"));

  it("preserves tab (\\t)", () => expect(stripInvalidXml("a\tb")).toBe("a\tb"));
  it("preserves newline (\\n)", () => expect(stripInvalidXml("a\nb")).toBe("a\nb"));
  it("preserves carriage return (\\r)", () => expect(stripInvalidXml("a\rb")).toBe("a\rb"));
});

describe("escapeXml — strips control AND escapes entities", () => {
  it("'<x>\\x00y' → '&lt;x&gt;y'", () => {
    expect(escapeXml("<x>\x00y")).toBe("&lt;x&gt;y");
  });
});

describe("escapeXml — defensive coercion", () => {
  it("non-string coerces via String(...)", () => {
    // @ts-expect-error — runtime coercion
    expect(escapeXml(42)).toBe("42");
  });
});
