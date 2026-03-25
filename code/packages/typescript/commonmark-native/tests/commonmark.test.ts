// commonmark.test.ts -- Comprehensive tests for the native CommonMark addon
// =========================================================================
//
// These tests verify that the Rust commonmark implementation is correctly
// exposed to JavaScript via the N-API node-bridge. Every public function
// is tested including CommonMark block elements, inline elements, lists,
// code blocks, raw HTML passthrough, and the safe XSS-prevention mode.

import { describe, it, expect } from "vitest";
import { markdownToHtml, markdownToHtmlSafe } from "../index.js";

// ---------------------------------------------------------------------------
// markdownToHtml -- block elements
// ---------------------------------------------------------------------------

describe("markdownToHtml: ATX headings", () => {
  it("renders H1", () => {
    expect(markdownToHtml("# Hello\n")).toBe("<h1>Hello</h1>\n");
  });

  it("renders H2", () => {
    expect(markdownToHtml("## Hello\n")).toBe("<h2>Hello</h2>\n");
  });

  it("renders H3", () => {
    expect(markdownToHtml("### Hello\n")).toBe("<h3>Hello</h3>\n");
  });

  it("renders H4", () => {
    expect(markdownToHtml("#### Hello\n")).toBe("<h4>Hello</h4>\n");
  });

  it("renders H5", () => {
    expect(markdownToHtml("##### Hello\n")).toBe("<h5>Hello</h5>\n");
  });

  it("renders H6", () => {
    expect(markdownToHtml("###### Hello\n")).toBe("<h6>Hello</h6>\n");
  });
});

describe("markdownToHtml: paragraphs", () => {
  it("renders a simple paragraph", () => {
    expect(markdownToHtml("Hello world\n")).toBe("<p>Hello world</p>\n");
  });

  it("renders multiple paragraphs", () => {
    const result = markdownToHtml("First\n\nSecond\n");
    expect(result).toContain("<p>First</p>");
    expect(result).toContain("<p>Second</p>");
  });

  it("returns empty string for empty input", () => {
    expect(markdownToHtml("")).toBe("");
  });

  it("returns empty string for blank lines only", () => {
    expect(markdownToHtml("\n\n\n")).toBe("");
  });
});

describe("markdownToHtml: other block elements", () => {
  it("renders blockquotes", () => {
    const result = markdownToHtml("> A quote\n");
    expect(result).toContain("<blockquote>");
    expect(result).toContain("A quote");
  });

  it("renders thematic breaks", () => {
    expect(markdownToHtml("---\n")).toContain("<hr");
  });
});

// ---------------------------------------------------------------------------
// markdownToHtml -- inline elements
// ---------------------------------------------------------------------------

describe("markdownToHtml: inline elements", () => {
  it("renders emphasis with asterisks", () => {
    expect(markdownToHtml("Hello *world*\n")).toBe("<p>Hello <em>world</em></p>\n");
  });

  it("renders emphasis with underscores", () => {
    expect(markdownToHtml("Hello _world_\n")).toBe("<p>Hello <em>world</em></p>\n");
  });

  it("renders strong with asterisks", () => {
    expect(markdownToHtml("Hello **world**\n")).toBe(
      "<p>Hello <strong>world</strong></p>\n"
    );
  });

  it("renders strong with underscores", () => {
    expect(markdownToHtml("Hello __world__\n")).toBe(
      "<p>Hello <strong>world</strong></p>\n"
    );
  });

  it("renders inline code", () => {
    const result = markdownToHtml("Use `print()` to output\n");
    expect(result).toContain("<code>print()</code>");
  });

  it("renders inline links", () => {
    const result = markdownToHtml("[GitHub](https://github.com)\n");
    expect(result).toContain('<a href="https://github.com">GitHub</a>');
  });

  it("renders inline images", () => {
    const result = markdownToHtml("![Alt text](image.png)\n");
    expect(result).toContain("<img");
    expect(result).toContain('alt="Alt text"');
    expect(result).toContain('src="image.png"');
  });

  it("renders hard line breaks (two trailing spaces)", () => {
    const result = markdownToHtml("line one  \nline two\n");
    expect(result).toContain("<br");
  });
});

// ---------------------------------------------------------------------------
// markdownToHtml -- code blocks
// ---------------------------------------------------------------------------

describe("markdownToHtml: code blocks", () => {
  it("renders fenced code blocks", () => {
    const result = markdownToHtml("```\nhello world\n```\n");
    expect(result).toContain("<code>");
    expect(result).toContain("hello world");
  });

  it("renders fenced code blocks with a language tag", () => {
    const result = markdownToHtml("```python\nprint('hello')\n```\n");
    expect(result).toContain("python");
    expect(result).toContain("print");
  });

  it("renders indented code blocks", () => {
    const result = markdownToHtml("    code here\n");
    expect(result).toContain("<code>");
    expect(result).toContain("code here");
  });
});

// ---------------------------------------------------------------------------
// markdownToHtml -- lists
// ---------------------------------------------------------------------------

describe("markdownToHtml: lists", () => {
  it("renders unordered lists", () => {
    const result = markdownToHtml("- Item 1\n- Item 2\n- Item 3\n");
    expect(result).toContain("<ul>");
    expect(result).toContain("<li>Item 1</li>");
    expect(result).toContain("<li>Item 2</li>");
  });

  it("renders ordered lists", () => {
    const result = markdownToHtml("1. First\n2. Second\n3. Third\n");
    expect(result).toContain("<ol>");
    expect(result).toContain("<li>First</li>");
    expect(result).toContain("<li>Second</li>");
  });

  it("renders nested lists", () => {
    const result = markdownToHtml("- Item 1\n  - Sub-item\n");
    expect(result).toContain("<ul>");
    expect(result).toContain("Sub-item");
  });
});

// ---------------------------------------------------------------------------
// markdownToHtml -- raw HTML passthrough
// ---------------------------------------------------------------------------

describe("markdownToHtml: raw HTML passthrough", () => {
  it("passes through raw HTML blocks", () => {
    const result = markdownToHtml("<div>raw content</div>\n\nparagraph\n");
    expect(result).toContain("<div>raw content</div>");
    expect(result).toContain("<p>paragraph</p>");
  });

  it("passes through HTML comments", () => {
    const result = markdownToHtml("<!-- a comment -->\n\nparagraph\n");
    expect(result).toContain("<!-- a comment -->");
  });
});

// ---------------------------------------------------------------------------
// markdownToHtml -- combined / integration
// ---------------------------------------------------------------------------

describe("markdownToHtml: combined documents", () => {
  it("renders a heading and paragraph together", () => {
    const result = markdownToHtml("# Title\n\nSome text.\n");
    expect(result).toContain("<h1>Title</h1>");
    expect(result).toContain("<p>Some text.</p>");
  });

  it("renders a full document correctly", () => {
    const doc = [
      "# My Document",
      "",
      "A paragraph with **bold** and *emphasis*.",
      "",
      "## Section",
      "",
      "- Bullet one",
      "- Bullet two",
      "",
      "```python",
      "print('hello')",
      "```",
      "",
    ].join("\n");

    const result = markdownToHtml(doc);
    expect(result).toContain("<h1>My Document</h1>");
    expect(result).toContain("<strong>bold</strong>");
    expect(result).toContain("<em>emphasis</em>");
    expect(result).toContain("<h2>Section</h2>");
    expect(result).toContain("<li>Bullet one</li>");
    expect(result).toContain("python");
  });

  it("handles unicode correctly", () => {
    const result = markdownToHtml("# こんにちは\n\nHello 世界\n");
    expect(result).toContain("こんにちは");
    expect(result).toContain("世界");
  });

  it("returns a string", () => {
    const result = markdownToHtml("# Hello\n");
    expect(typeof result).toBe("string");
  });
});

// ---------------------------------------------------------------------------
// markdownToHtml -- error handling
// ---------------------------------------------------------------------------

describe("markdownToHtml: error handling", () => {
  it("throws when no argument is provided", () => {
    expect(() => (markdownToHtml as any)()).toThrow();
  });

  it("throws when argument is a number", () => {
    expect(() => (markdownToHtml as any)(42)).toThrow();
  });

  it("throws when argument is null", () => {
    expect(() => (markdownToHtml as any)(null)).toThrow();
  });
});

// ---------------------------------------------------------------------------
// markdownToHtmlSafe -- XSS prevention
// ---------------------------------------------------------------------------

describe("markdownToHtmlSafe: XSS prevention", () => {
  it("strips script tags", () => {
    const result = markdownToHtmlSafe("<script>alert(1)</script>\n\n**bold**\n");
    expect(result).not.toContain("<script>");
    expect(result).toContain("<strong>bold</strong>");
  });

  it("strips raw HTML blocks", () => {
    const result = markdownToHtmlSafe("<div class='evil'>content</div>\n\nparagraph\n");
    expect(result).not.toContain("<div");
    expect(result).toContain("<p>paragraph</p>");
  });

  it("preserves all Markdown formatting", () => {
    const result = markdownToHtmlSafe("# Heading\n\n**bold** and *em*\n");
    expect(result).toContain("<h1>Heading</h1>");
    expect(result).toContain("<strong>bold</strong>");
    expect(result).toContain("<em>em</em>");
  });

  it("returns empty string for empty input", () => {
    expect(markdownToHtmlSafe("")).toBe("");
  });

  it("renders regular Markdown links", () => {
    const result = markdownToHtmlSafe("[GitHub](https://github.com)\n");
    expect(result).toContain('<a href="https://github.com">');
  });

  it("throws when argument is not a string", () => {
    expect(() => (markdownToHtmlSafe as any)(42)).toThrow();
  });
});

// ---------------------------------------------------------------------------
// Contrast: safe vs unsafe
// ---------------------------------------------------------------------------

describe("markdownToHtml vs markdownToHtmlSafe", () => {
  it("raw HTML is present in unsafe mode, stripped in safe mode", () => {
    const md = "<div>content</div>\n\nparagraph\n";
    expect(markdownToHtml(md)).toContain("<div>");
    expect(markdownToHtmlSafe(md)).not.toContain("<div>");
  });

  it("script tags are present in unsafe mode, stripped in safe mode", () => {
    const md = "<script>evil()</script>\n\ntext\n";
    expect(markdownToHtml(md)).toContain("<script>");
    expect(markdownToHtmlSafe(md)).not.toContain("<script>");
  });

  it("output is identical when Markdown has no raw HTML", () => {
    const md = "# Title\n\n**Bold** and *italic*.\n";
    expect(markdownToHtml(md)).toBe(markdownToHtmlSafe(md));
  });
});
