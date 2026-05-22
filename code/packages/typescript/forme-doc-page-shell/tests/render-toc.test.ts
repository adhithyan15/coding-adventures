/**
 * render-toc.test.ts — TOC tree → HTML tests.
 */

import { describe, it, expect } from "vitest";
import { renderToc } from "../src/index.js";
import type { TocEntry } from "../src/index.js";

function entry(text: string, id: string, level: 1 | 2 | 3 | 4 | 5 | 6, children: TocEntry[] = []): TocEntry {
  return { text, id, level, children };
}

describe("renderToc — degenerate", () => {
  it("empty → empty string (NOT empty aside — page-shell omits whole element)", () => {
    expect(renderToc([])).toBe("");
  });
});

describe("renderToc — basic", () => {
  it("single entry", () => {
    const html = renderToc([entry("Install", "install", 2)]);
    expect(html).toContain('<aside class="toc" aria-label="On this page">');
    expect(html).toContain('<a href="#install">Install</a>');
  });
  it("nested entries", () => {
    const html = renderToc([
      entry("Setup", "setup", 2, [entry("Prereq", "prereq", 3)]),
    ]);
    expect(html).toContain('<a href="#setup">Setup</a>');
    expect(html).toContain('<a href="#prereq">Prereq</a>');
  });
  it("multiple top-level entries", () => {
    const html = renderToc([entry("A", "a", 2), entry("B", "b", 2)]);
    expect(html).toContain('href="#a"');
    expect(html).toContain('href="#b"');
  });
});

describe("renderToc — XSS defence", () => {
  it("escapes text content", () => {
    const html = renderToc([entry("<script>", "x", 2)]);
    expect(html).toContain("&lt;script&gt;");
    expect(html).not.toContain("<script>");
  });
  it("escapes id attribute (defensive — slugs are normally safe)", () => {
    const html = renderToc([entry("X", '"><script>alert(1)</script>', 2)]);
    expect(html).toContain("&quot;");
    expect(html).not.toContain("><script>");
  });
});
