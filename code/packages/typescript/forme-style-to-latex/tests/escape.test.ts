/**
 * escape.test.ts — LaTeX escaping pins.
 *
 * One test per LaTeX-special character + control-char stripping +
 * identifier sanitisation.
 */

import { describe, it, expect } from "vitest";
import { escapeLatexText, latexIdent } from "../src/escape.js";

describe("escapeLatexText — all ten LaTeX specials", () => {
  it("escapes backslash first (so subsequent escapes don't collide)", () => {
    expect(escapeLatexText("\\")).toBe("\\textbackslash{}");
  });

  it("escapes %", () => {
    expect(escapeLatexText("50%")).toBe("50\\%");
  });

  it("escapes $", () => {
    expect(escapeLatexText("$5")).toBe("\\$5");
  });

  it("escapes &", () => {
    expect(escapeLatexText("a & b")).toBe("a \\& b");
  });

  it("escapes _", () => {
    expect(escapeLatexText("snake_case")).toBe("snake\\_case");
  });

  it("escapes #", () => {
    expect(escapeLatexText("#tag")).toBe("\\#tag");
  });

  it("escapes {", () => {
    expect(escapeLatexText("a{b")).toBe("a\\{b");
  });

  it("escapes }", () => {
    expect(escapeLatexText("a}b")).toBe("a\\}b");
  });

  it("escapes ^ as \\textasciicircum{}", () => {
    expect(escapeLatexText("a^b")).toBe("a\\textasciicircum{}b");
  });

  it("escapes ~ as \\textasciitilde{}", () => {
    expect(escapeLatexText("a~b")).toBe("a\\textasciitilde{}b");
  });

  it("escapes all ten in one string in the right order", () => {
    // The big composite test — backslash escaping uses placeholders so
    // its synthetic `\textbackslash{}` braces don't get double-escaped
    // on later passes.
    const input = `\\%$&_#{}^~`;
    const out = escapeLatexText(input);
    // Every literal in the input has been transformed to its
    // canonical LaTeX-escape form, in input order:
    //   \  → \textbackslash{}
    //   %  → \%
    //   $  → \$
    //   &  → \&
    //   _  → \_
    //   #  → \#
    //   {  → \{
    //   }  → \}
    //   ^  → \textasciicircum{}
    //   ~  → \textasciitilde{}
    expect(out).toBe(
      "\\textbackslash{}\\%\\$\\&\\_\\#\\{\\}\\textasciicircum{}\\textasciitilde{}",
    );
  });

  it("strips ASCII control characters (0x00-0x1F, 0x7F)", () => {
    const input = "a\x00b\x01c\x1Fd\x7Fe\nf"; // \n is 0x0A — control
    const out = escapeLatexText(input);
    // Newline IS in 0x00-0x1F so it gets stripped.
    expect(out).toBe("abcdef");
  });

  it("passes through plain ASCII letters unchanged", () => {
    expect(escapeLatexText("Hello world 123")).toBe("Hello world 123");
  });

  it("idempotent for already-safe input", () => {
    const safe = "Hello world 123";
    expect(escapeLatexText(escapeLatexText(safe))).toBe(safe);
  });

  it("does NOT escape Unicode (only LaTeX specials are ASCII)", () => {
    expect(escapeLatexText("café — résumé")).toBe("café — résumé");
  });
});

describe("latexIdent — LaTeX command-name sanitisation", () => {
  it("passes through all-letter input verbatim", () => {
    expect(latexIdent("Paragraph")).toBe("Paragraph");
  });

  it("encodes digits as Z<hex>Z", () => {
    expect(latexIdent("h1")).toBe(`hZ${"1".codePointAt(0)!.toString(16)}Z`);
  });

  it("encodes hyphens and other punctuation", () => {
    const out = latexIdent("foo-bar");
    // The hyphen (0x2d) becomes Z2dZ; rest passes through.
    expect(out).toBe(`fooZ${(0x2d).toString(16)}Zbar`);
  });

  it("encodes LaTeX-special characters defensively", () => {
    const out = latexIdent("a%b");
    expect(out).toContain("Z25Z");                  // % is 0x25
    expect(out.startsWith("a")).toBe(true);
    expect(out.endsWith("b")).toBe(true);
  });

  it("rejects empty input as Zempty", () => {
    expect(latexIdent("")).toBe("Zempty");
  });

  it("strips control characters first, then encodes", () => {
    expect(latexIdent("a\x00b")).toBe("ab");
  });

  it("an all-control-character input becomes Zempty after stripping", () => {
    expect(latexIdent("\x00\x01\x02")).toBe("Zempty");
  });
});
