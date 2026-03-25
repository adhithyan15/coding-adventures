/**
 * @coding-adventures/commonmark — Pipeline Integration Tests
 *
 * Tests the `commonmark` package as a pipeline:
 *   parse(markdown) → DocumentNode → toHtml(doc) → string
 *
 * These tests verify:
 *   1. The full 652-example CommonMark 0.31.2 spec (end-to-end pipeline)
 *   2. Integration between the parser and the HTML renderer
 *   3. That the pipeline package correctly re-exports from its constituent
 *      packages (@coding-adventures/commonmark-parser and
 *      @coding-adventures/document-ast-to-html)
 *
 * Detailed unit tests for the parser and renderer live in their own packages:
 *   - @coding-adventures/commonmark-parser — parser unit tests
 *   - @coding-adventures/document-ast-to-html — renderer unit tests
 *   - @coding-adventures/document-ast — AST type tests
 */

import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { parse, toHtml, VERSION } from "../src/index.js";

// ─── Spec Fixture ─────────────────────────────────────────────────────────────

const __dir = dirname(fileURLToPath(import.meta.url));
const specPath = join(__dir, "fixtures", "spec.json");

interface SpecExample {
  markdown: string;
  html: string;
  example: number;
  start_line: number;
  end_line: number;
  section: string;
}

const ALL_EXAMPLES: SpecExample[] = JSON.parse(readFileSync(specPath, "utf8"));

// Group by section for organised test output
const bySection = new Map<string, SpecExample[]>();
for (const ex of ALL_EXAMPLES) {
  const group = bySection.get(ex.section) ?? [];
  group.push(ex);
  bySection.set(ex.section, group);
}

// ─── Package API ──────────────────────────────────────────────────────────────

describe("package", () => {
  it("exports a version string", () => {
    expect(VERSION).toBe("0.1.0");
  });

  it("exports parse function", () => {
    expect(typeof parse).toBe("function");
  });

  it("exports toHtml function", () => {
    expect(typeof toHtml).toBe("function");
  });

  it("pipeline: parse then toHtml", () => {
    const html = toHtml(parse("# Hello\n\nWorld\n"));
    expect(html).toBe("<h1>Hello</h1>\n<p>World</p>\n");
  });
});

// ─── CommonMark Spec Examples ─────────────────────────────────────────────────
//
// All 652 examples from the CommonMark 0.31.2 specification.
// Each section becomes a describe block; each example is a separate test.
//
// Failures report example number, section, and a diff for easy lookup at:
// https://spec.commonmark.org/0.31.2/#example-N

for (const [section, examples] of bySection) {
  describe(`CommonMark spec — ${section}`, () => {
    for (const ex of examples) {
      it(`example ${ex.example}`, () => {
        const ast = parse(ex.markdown);
        const actual = toHtml(ast);
        expect(actual).toBe(ex.html);
      });
    }
  });
}

// ─── Integration tests: parse + toHtml pipeline ───────────────────────────────
//
// These verify specific pipeline behaviours end-to-end.

describe("pipeline — Document AST node types", () => {
  it("parse returns a DocumentNode", () => {
    const doc = parse("");
    expect(doc.type).toBe("document");
    expect(doc.children).toEqual([]);
  });

  it("produces HeadingNode from ATX heading", () => {
    const doc = parse("# Hello\n");
    const h = doc.children[0]!;
    expect(h.type).toBe("heading");
    if (h.type === "heading") {
      expect(h.level).toBe(1);
    }
  });

  it("produces ParagraphNode", () => {
    expect(parse("Hello\n").children[0]?.type).toBe("paragraph");
  });

  it("produces CodeBlockNode with language", () => {
    const node = parse("```ts\nconst x = 1;\n```\n").children[0]!;
    expect(node.type).toBe("code_block");
    if (node.type === "code_block") {
      expect(node.language).toBe("ts");
    }
  });

  it("produces ThematicBreakNode from ---", () => {
    expect(parse("---\n").children[0]?.type).toBe("thematic_break");
  });

  it("produces BlockquoteNode from >", () => {
    expect(parse("> quote\n").children[0]?.type).toBe("blockquote");
  });

  it("produces ListNode (unordered)", () => {
    const list = parse("- a\n- b\n").children[0]!;
    expect(list.type).toBe("list");
    if (list.type === "list") {
      expect(list.ordered).toBe(false);
      expect(list.children).toHaveLength(2);
    }
  });

  it("produces ListNode (ordered)", () => {
    const list = parse("1. first\n").children[0]!;
    expect(list.type).toBe("list");
    if (list.type === "list") {
      expect(list.ordered).toBe(true);
      expect(list.start).toBe(1);
    }
  });

  it("produces RawBlockNode (format=html) for HTML blocks", () => {
    const node = parse("<div>\nhello\n</div>\n").children[0]!;
    expect(node.type).toBe("raw_block");
    if (node.type === "raw_block") {
      expect(node.format).toBe("html");
    }
  });

  it("link reference definitions are resolved and not emitted", () => {
    const html = toHtml(parse("[ref]: https://example.com\n"));
    expect(html).toBe("");
  });

  it("reference links are resolved to LinkNode", () => {
    const doc = parse("[link][ref]\n\n[ref]: https://example.com\n");
    const para = doc.children[0]!;
    expect(para.type).toBe("paragraph");
    if (para.type === "paragraph") {
      expect(para.children[0]?.type).toBe("link");
    }
  });
});

describe("pipeline — inline node types", () => {
  it("produces EmphasisNode from *text*", () => {
    const para = parse("*em*\n").children[0]!;
    if (para.type === "paragraph") {
      expect(para.children[0]?.type).toBe("emphasis");
    }
  });

  it("produces StrongNode from **text**", () => {
    const para = parse("**bold**\n").children[0]!;
    if (para.type === "paragraph") {
      expect(para.children[0]?.type).toBe("strong");
    }
  });

  it("produces CodeSpanNode from `code`", () => {
    const para = parse("`code`\n").children[0]!;
    if (para.type === "paragraph") {
      expect(para.children[0]?.type).toBe("code_span");
    }
  });

  it("produces LinkNode from [text](url)", () => {
    const para = parse("[text](https://x.com)\n").children[0]!;
    if (para.type === "paragraph") {
      const link = para.children[0]!;
      expect(link.type).toBe("link");
      if (link.type === "link") {
        expect(link.destination).toBe("https://x.com");
      }
    }
  });

  it("produces ImageNode from ![alt](url)", () => {
    const para = parse("![cat](cat.png)\n").children[0]!;
    if (para.type === "paragraph") {
      const img = para.children[0]!;
      expect(img.type).toBe("image");
      if (img.type === "image") {
        expect(img.alt).toBe("cat");
        expect(img.destination).toBe("cat.png");
      }
    }
  });

  it("produces AutolinkNode from <url>", () => {
    const para = parse("<https://example.com>\n").children[0]!;
    if (para.type === "paragraph") {
      const al = para.children[0]!;
      expect(al.type).toBe("autolink");
      if (al.type === "autolink") {
        expect(al.isEmail).toBe(false);
      }
    }
  });

  it("produces RawInlineNode (format=html) for inline HTML", () => {
    const para = parse("a <em>b</em> c\n").children[0]!;
    if (para.type === "paragraph") {
      const raw = para.children.find(c => c.type === "raw_inline");
      expect(raw).toBeDefined();
      if (raw?.type === "raw_inline") {
        expect(raw.format).toBe("html");
      }
    }
  });

  it("produces HardBreakNode from trailing spaces", () => {
    const para = parse("a  \nb\n").children[0]!;
    if (para.type === "paragraph") {
      expect(para.children.some(c => c.type === "hard_break")).toBe(true);
    }
  });

  it("produces SoftBreakNode from single newline", () => {
    const para = parse("a\nb\n").children[0]!;
    if (para.type === "paragraph") {
      expect(para.children.some(c => c.type === "soft_break")).toBe(true);
    }
  });
});

describe("pipeline — toHtml output", () => {
  it("renders heading", () => {
    expect(toHtml(parse("# Hello\n"))).toBe("<h1>Hello</h1>\n");
  });

  it("renders paragraph", () => {
    expect(toHtml(parse("Hello world\n"))).toBe("<p>Hello world</p>\n");
  });

  it("renders emphasis", () => {
    expect(toHtml(parse("*em*\n"))).toBe("<p><em>em</em></p>\n");
  });

  it("renders strong", () => {
    expect(toHtml(parse("**strong**\n"))).toBe("<p><strong>strong</strong></p>\n");
  });

  it("renders code block", () => {
    expect(toHtml(parse("    code\n"))).toBe("<pre><code>code\n</code></pre>\n");
  });

  it("renders thematic break", () => {
    expect(toHtml(parse("---\n"))).toBe("<hr />\n");
  });

  it("renders blockquote", () => {
    expect(toHtml(parse("> quote\n"))).toBe(
      "<blockquote>\n<p>quote</p>\n</blockquote>\n",
    );
  });

  it("renders tight unordered list", () => {
    expect(toHtml(parse("- a\n- b\n"))).toBe(
      "<ul>\n<li>a</li>\n<li>b</li>\n</ul>\n",
    );
  });

  it("renders ordered list", () => {
    expect(toHtml(parse("1. first\n2. second\n"))).toBe(
      "<ol>\n<li>first</li>\n<li>second</li>\n</ol>\n",
    );
  });

  it("renders inline link", () => {
    expect(toHtml(parse("[click](https://example.com)\n"))).toBe(
      '<p><a href="https://example.com">click</a></p>\n',
    );
  });

  it("renders image", () => {
    expect(toHtml(parse("![alt](img.png)\n"))).toBe(
      '<p><img src="img.png" alt="alt" /></p>\n',
    );
  });

  it("renders hard break", () => {
    expect(toHtml(parse("a  \nb\n"))).toContain("<br />");
  });

  it("renders soft break as newline", () => {
    expect(toHtml(parse("a\nb\n"))).toBe("<p>a\nb</p>\n");
  });

  it("renders autolink email with mailto:", () => {
    const html = toHtml(parse("<me@example.com>\n"));
    expect(html).toContain("mailto:");
  });
});
