/**
 * Document AST → HTML Renderer Tests
 *
 * Comprehensive tests for the HTML rendering of every Document AST node type.
 * These tests verify:
 *   - Each block and inline node type renders to the correct HTML
 *   - Tight vs loose list rendering
 *   - RawBlockNode/RawInlineNode format matching
 *   - URL sanitization (XSS prevention)
 *   - HTML entity escaping in text content
 */

import { describe, it, expect } from "vitest";
import { toHtml } from "../src/index.js";
import type { DocumentNode, BlockNode, InlineNode } from "@coding-adventures/document-ast";

// ─── Helpers ──────────────────────────────────────────────────────────────────

function doc(...children: BlockNode[]): DocumentNode {
  return { type: "document", children };
}

function text(value: string): InlineNode {
  return { type: "text", value };
}

function para(...children: InlineNode[]): BlockNode {
  return { type: "paragraph", children };
}

// ─── Block node rendering ─────────────────────────────────────────────────────

describe("toHtml — DocumentNode", () => {
  it("renders an empty document to empty string", () => {
    expect(toHtml(doc())).toBe("");
  });

  it("renders multiple blocks in order", () => {
    const result = toHtml(doc(
      { type: "heading", level: 1, children: [text("Title")] },
      para(text("Body")),
    ));
    expect(result).toBe("<h1>Title</h1>\n<p>Body</p>\n");
  });
});

describe("toHtml — HeadingNode", () => {
  it("renders all six heading levels", () => {
    for (let level = 1; level <= 6; level++) {
      const result = toHtml(doc({
        type: "heading",
        level: level as 1 | 2 | 3 | 4 | 5 | 6,
        children: [text("Hi")],
      }));
      expect(result).toBe(`<h${level}>Hi</h${level}>\n`);
    }
  });
});

describe("toHtml — ParagraphNode", () => {
  it("wraps content in <p> tags", () => {
    expect(toHtml(doc(para(text("Hello"))))).toBe("<p>Hello</p>\n");
  });

  it("escapes HTML special chars in text", () => {
    expect(toHtml(doc(para(text("a & b < c > d"))))).toBe(
      "<p>a &amp; b &lt; c &gt; d</p>\n",
    );
  });
});

describe("toHtml — CodeBlockNode", () => {
  it("renders without language", () => {
    const result = toHtml(doc({
      type: "code_block",
      language: null,
      value: "hello\n",
    }));
    expect(result).toBe("<pre><code>hello\n</code></pre>\n");
  });

  it("renders with language class attribute", () => {
    const result = toHtml(doc({
      type: "code_block",
      language: "typescript",
      value: "const x = 1;\n",
    }));
    expect(result).toBe(
      '<pre><code class="language-typescript">const x = 1;\n</code></pre>\n',
    );
  });

  it("escapes HTML in code content", () => {
    const result = toHtml(doc({
      type: "code_block",
      language: null,
      value: "<script>alert('xss')</script>\n",
    }));
    // < and > are escaped; single quotes are safe in HTML text content and not encoded
    expect(result).toBe(
      "<pre><code>&lt;script&gt;alert('xss')&lt;/script&gt;\n</code></pre>\n",
    );
  });
});

describe("toHtml — BlockquoteNode", () => {
  it("wraps content in <blockquote> tags", () => {
    const result = toHtml(doc({
      type: "blockquote",
      children: [para(text("quoted"))],
    }));
    expect(result).toBe("<blockquote>\n<p>quoted</p>\n</blockquote>\n");
  });

  it("supports nested blockquotes", () => {
    const result = toHtml(doc({
      type: "blockquote",
      children: [{
        type: "blockquote",
        children: [para(text("nested"))],
      }],
    }));
    expect(result).toBe(
      "<blockquote>\n<blockquote>\n<p>nested</p>\n</blockquote>\n</blockquote>\n",
    );
  });
});

describe("toHtml — ThematicBreakNode", () => {
  it("renders as <hr />", () => {
    expect(toHtml(doc({ type: "thematic_break" }))).toBe("<hr />\n");
  });
});

describe("toHtml — ListNode (unordered, tight)", () => {
  it("renders tight unordered list without <p> tags", () => {
    const result = toHtml(doc({
      type: "list",
      ordered: false,
      start: null,
      tight: true,
      children: [
        { type: "list_item", children: [para(text("a"))] },
        { type: "list_item", children: [para(text("b"))] },
      ],
    }));
    expect(result).toBe("<ul>\n<li>a</li>\n<li>b</li>\n</ul>\n");
  });
});

describe("toHtml — ListNode (ordered, loose)", () => {
  it("renders loose ordered list with <p> tags", () => {
    const result = toHtml(doc({
      type: "list",
      ordered: true,
      start: 1,
      tight: false,
      children: [
        { type: "list_item", children: [para(text("first"))] },
      ],
    }));
    expect(result).toBe("<ol>\n<li>\n<p>first</p>\n</li>\n</ol>\n");
  });

  it("includes start attribute when not 1", () => {
    const result = toHtml(doc({
      type: "list",
      ordered: true,
      start: 42,
      tight: false,
      children: [
        { type: "list_item", children: [para(text("x"))] },
      ],
    }));
    expect(result).toContain('<ol start="42">');
  });

  it("omits start attribute when start is 1", () => {
    const result = toHtml(doc({
      type: "list",
      ordered: true,
      start: 1,
      tight: false,
      children: [
        { type: "list_item", children: [para(text("x"))] },
      ],
    }));
    expect(result).toContain("<ol>");
    expect(result).not.toContain("start=");
  });
});

describe("toHtml — RawBlockNode", () => {
  it("passes through HTML format verbatim", () => {
    const result = toHtml(doc({
      type: "raw_block",
      format: "html",
      value: "<div class='raw'>content</div>\n",
    }));
    expect(result).toBe("<div class='raw'>content</div>\n");
  });

  it("skips non-HTML formats silently", () => {
    const result = toHtml(doc({
      type: "raw_block",
      format: "latex",
      value: "\\textbf{bold}\n",
    }));
    expect(result).toBe("");
  });

  it("skips rtf format silently", () => {
    const result = toHtml(doc({
      type: "raw_block",
      format: "rtf",
      value: "{\\rtf1\\ansi}",
    }));
    expect(result).toBe("");
  });
});

// ─── Inline node rendering ────────────────────────────────────────────────────

describe("toHtml — inline nodes", () => {
  it("renders EmphasisNode", () => {
    const result = toHtml(doc(para({
      type: "emphasis",
      children: [text("italic")],
    })));
    expect(result).toBe("<p><em>italic</em></p>\n");
  });

  it("renders StrongNode", () => {
    const result = toHtml(doc(para({
      type: "strong",
      children: [text("bold")],
    })));
    expect(result).toBe("<p><strong>bold</strong></p>\n");
  });

  it("renders CodeSpanNode", () => {
    const result = toHtml(doc(para({
      type: "code_span",
      value: "x = 1",
    })));
    expect(result).toBe("<p><code>x = 1</code></p>\n");
  });

  it("renders SoftBreakNode as newline", () => {
    const result = toHtml(doc(para(text("a"), { type: "soft_break" }, text("b"))));
    expect(result).toBe("<p>a\nb</p>\n");
  });

  it("renders HardBreakNode as <br />", () => {
    const result = toHtml(doc(para(text("a"), { type: "hard_break" }, text("b"))));
    expect(result).toBe("<p>a<br />\nb</p>\n");
  });
});

describe("toHtml — LinkNode", () => {
  it("renders with href", () => {
    const result = toHtml(doc(para({
      type: "link",
      destination: "https://example.com",
      title: null,
      children: [text("click")],
    })));
    expect(result).toBe('<p><a href="https://example.com">click</a></p>\n');
  });

  it("renders with title attribute", () => {
    const result = toHtml(doc(para({
      type: "link",
      destination: "https://x.com",
      title: "X Site",
      children: [text("X")],
    })));
    expect(result).toContain('title="X Site"');
  });

  it("blocks javascript: scheme (XSS prevention)", () => {
    const result = toHtml(doc(para({
      type: "link",
      destination: "javascript:alert(1)",
      title: null,
      children: [text("click")],
    })));
    expect(result).toBe('<p><a href="">click</a></p>\n');
  });

  it("blocks data: scheme", () => {
    const result = toHtml(doc(para({
      type: "link",
      destination: "data:text/html,<script>alert(1)</script>",
      title: null,
      children: [text("x")],
    })));
    expect(result).toContain('href=""');
  });

  it("allows relative URLs", () => {
    const result = toHtml(doc(para({
      type: "link",
      destination: "/about",
      title: null,
      children: [text("About")],
    })));
    expect(result).toContain('href="/about"');
  });
});

describe("toHtml — ImageNode", () => {
  it("renders <img> with src and alt", () => {
    const result = toHtml(doc(para({
      type: "image",
      destination: "cat.png",
      title: null,
      alt: "a cat",
    })));
    expect(result).toBe('<p><img src="cat.png" alt="a cat" /></p>\n');
  });

  it("renders title attribute when present", () => {
    const result = toHtml(doc(para({
      type: "image",
      destination: "x.png",
      title: "A picture",
      alt: "pic",
    })));
    expect(result).toContain('title="A picture"');
  });
});

describe("toHtml — AutolinkNode", () => {
  it("renders URL autolink", () => {
    const result = toHtml(doc(para({
      type: "autolink",
      destination: "https://example.com",
      isEmail: false,
    })));
    expect(result).toBe(
      '<p><a href="https://example.com">https://example.com</a></p>\n',
    );
  });

  it("renders email autolink with mailto: prefix", () => {
    const result = toHtml(doc(para({
      type: "autolink",
      destination: "user@example.com",
      isEmail: true,
    })));
    expect(result).toBe(
      '<p><a href="mailto:user@example.com">user@example.com</a></p>\n',
    );
  });
});

describe("toHtml — RawInlineNode", () => {
  it("passes through HTML format verbatim", () => {
    const result = toHtml(doc(para({
      type: "raw_inline",
      format: "html",
      value: "<em>raw</em>",
    })));
    expect(result).toBe("<p><em>raw</em></p>\n");
  });

  it("skips non-HTML formats silently", () => {
    const result = toHtml(doc(para({
      type: "raw_inline",
      format: "latex",
      value: "\\emph{x}",
    })));
    expect(result).toBe("<p></p>\n");
  });
});

// ─── Integration: full document ───────────────────────────────────────────────

describe("toHtml — integration", () => {
  it("renders a complete document with multiple block types", () => {
    const result = toHtml(doc(
      { type: "heading", level: 1, children: [text("Hello")] },
      para(text("A "), { type: "emphasis", children: [text("world")] }, text(".")),
      { type: "thematic_break" },
      {
        type: "list",
        ordered: false,
        start: null,
        tight: true,
        children: [
          { type: "list_item", children: [para(text("item 1"))] },
          { type: "list_item", children: [para(text("item 2"))] },
        ],
      },
    ));

    expect(result).toBe(
      "<h1>Hello</h1>\n" +
      "<p>A <em>world</em>.</p>\n" +
      "<hr />\n" +
      "<ul>\n<li>item 1</li>\n<li>item 2</li>\n</ul>\n",
    );
  });
});
