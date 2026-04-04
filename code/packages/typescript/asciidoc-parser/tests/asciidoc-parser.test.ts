/**
 * AsciiDoc Parser — Test Suite
 *
 * Covers all block types and inline forms specified in the AsciiDoc parsing
 * algorithm. 30+ test cases ensuring every code path is exercised.
 */

import { describe, it, expect } from "vitest";
import { parse, parseInline } from "../src/index.js";
import type {
  DocumentNode,
  HeadingNode,
  ParagraphNode,
  CodeBlockNode,
  BlockquoteNode,
  ListNode,
  ThematicBreakNode,
  RawBlockNode,
  TextNode,
  StrongNode,
  EmphasisNode,
  CodeSpanNode,
  LinkNode,
  ImageNode,
  AutolinkNode,
  HardBreakNode,
  SoftBreakNode,
} from "../src/index.js";

// ─── Block parser tests ───────────────────────────────────────────────────────

describe("block parser — headings", () => {
  it("parses level-1 heading (=)", () => {
    const doc = parse("= Hello World\n");
    expect(doc.children).toHaveLength(1);
    const h = doc.children[0] as HeadingNode;
    expect(h.type).toBe("heading");
    expect(h.level).toBe(1);
    const text = h.children[0] as TextNode;
    expect(text.value).toBe("Hello World");
  });

  it("parses level-2 heading (==)", () => {
    const doc = parse("== Section Title\n");
    const h = doc.children[0] as HeadingNode;
    expect(h.level).toBe(2);
  });

  it("parses level-3 heading (===)", () => {
    const doc = parse("=== Sub-section\n");
    const h = doc.children[0] as HeadingNode;
    expect(h.level).toBe(3);
  });

  it("parses level-6 heading (======)", () => {
    const doc = parse("====== Deepest\n");
    const h = doc.children[0] as HeadingNode;
    expect(h.level).toBe(6);
  });

  it("parses heading with inline markup", () => {
    const doc = parse("== Hello *world*\n");
    const h = doc.children[0] as HeadingNode;
    expect(h.children).toHaveLength(2);
    expect(h.children[0]).toMatchObject({ type: "text", value: "Hello " });
    expect(h.children[1]).toMatchObject({ type: "strong" });
  });
});

describe("block parser — paragraphs", () => {
  it("parses a single-line paragraph", () => {
    const doc = parse("Hello world.\n");
    expect(doc.children).toHaveLength(1);
    const p = doc.children[0] as ParagraphNode;
    expect(p.type).toBe("paragraph");
  });

  it("parses a multi-line paragraph", () => {
    const doc = parse("Line one\nLine two\nLine three\n");
    const p = doc.children[0] as ParagraphNode;
    expect(p.type).toBe("paragraph");
    // The three lines are joined with \n, producing soft breaks
    expect(p.children.some(n => n.type === "soft_break")).toBe(true);
  });

  it("separates two paragraphs with a blank line", () => {
    const doc = parse("First paragraph.\n\nSecond paragraph.\n");
    expect(doc.children).toHaveLength(2);
    expect(doc.children[0].type).toBe("paragraph");
    expect(doc.children[1].type).toBe("paragraph");
  });

  it("returns empty document for empty string", () => {
    const doc = parse("");
    expect(doc.type).toBe("document");
    expect(doc.children).toHaveLength(0);
  });
});

describe("block parser — thematic break", () => {
  it("parses three apostrophes as thematic break", () => {
    const doc = parse("'''\n");
    expect(doc.children).toHaveLength(1);
    const tb = doc.children[0] as ThematicBreakNode;
    expect(tb.type).toBe("thematic_break");
  });

  it("parses five apostrophes as thematic break", () => {
    const doc = parse("'''''\n");
    const tb = doc.children[0] as ThematicBreakNode;
    expect(tb.type).toBe("thematic_break");
  });
});

describe("block parser — code blocks", () => {
  it("parses a delimited code block with language hint", () => {
    const src = "[source,typescript]\n----\nconst x = 1;\n----\n";
    const doc = parse(src);
    expect(doc.children).toHaveLength(1);
    const cb = doc.children[0] as CodeBlockNode;
    expect(cb.type).toBe("code_block");
    expect(cb.language).toBe("typescript");
    expect(cb.value).toContain("const x = 1;");
  });

  it("parses a code block without language hint", () => {
    const src = "----\necho hello\n----\n";
    const doc = parse(src);
    const cb = doc.children[0] as CodeBlockNode;
    expect(cb.language).toBeNull();
    expect(cb.value).toContain("echo hello");
  });

  it("parses a literal block (....)", () => {
    const src = "....\nraw text here\n....\n";
    const doc = parse(src);
    const cb = doc.children[0] as CodeBlockNode;
    expect(cb.type).toBe("code_block");
    expect(cb.language).toBeNull();
    expect(cb.value).toContain("raw text here");
  });

  it("preserves multi-line code content", () => {
    const src = "----\nline 1\nline 2\nline 3\n----\n";
    const doc = parse(src);
    const cb = doc.children[0] as CodeBlockNode;
    expect(cb.value).toBe("line 1\nline 2\nline 3\n");
  });

  it("handles unterminated code block gracefully (EOF)", () => {
    const src = "----\nsome code\n";
    const doc = parse(src);
    expect(doc.children).toHaveLength(1);
    const cb = doc.children[0] as CodeBlockNode;
    expect(cb.type).toBe("code_block");
  });
});

describe("block parser — passthrough block", () => {
  it("parses a ++++ block as raw HTML", () => {
    const src = "++++\n<div>raw HTML</div>\n++++\n";
    const doc = parse(src);
    const rb = doc.children[0] as RawBlockNode;
    expect(rb.type).toBe("raw_block");
    expect(rb.format).toBe("html");
    expect(rb.value).toContain("<div>raw HTML</div>");
  });
});

describe("block parser — quote block", () => {
  it("parses ____ block as blockquote", () => {
    const src = "____\nThis is a quote.\n____\n";
    const doc = parse(src);
    expect(doc.children).toHaveLength(1);
    const bq = doc.children[0] as BlockquoteNode;
    expect(bq.type).toBe("blockquote");
    expect(bq.children).toHaveLength(1);
    expect(bq.children[0].type).toBe("paragraph");
  });

  it("recursively parses quote block content", () => {
    const src = "____\n== Section\n\nParagraph text.\n____\n";
    const doc = parse(src);
    const bq = doc.children[0] as BlockquoteNode;
    expect(bq.children[0].type).toBe("heading");
    expect(bq.children[1].type).toBe("paragraph");
  });
});

describe("block parser — lists", () => {
  it("parses an unordered list", () => {
    const src = "* Alpha\n* Beta\n* Gamma\n";
    const doc = parse(src);
    expect(doc.children).toHaveLength(1);
    const list = doc.children[0] as ListNode;
    expect(list.type).toBe("list");
    expect(list.ordered).toBe(false);
    expect(list.children).toHaveLength(3);
  });

  it("parses an ordered list", () => {
    const src = ". First\n. Second\n. Third\n";
    const doc = parse(src);
    const list = doc.children[0] as ListNode;
    expect(list.type).toBe("list");
    expect(list.ordered).toBe(true);
    expect(list.children).toHaveLength(3);
  });

  it("parses nested unordered list (two levels)", () => {
    const src = "* Top\n** Nested\n* Top2\n";
    const doc = parse(src);
    const list = doc.children[0] as ListNode;
    // The nested item should appear under the first top-level item
    expect(list.children).toHaveLength(2);
    const firstItem = list.children[0];
    // First item should have a sub-list as its second child
    expect(firstItem.children.length).toBeGreaterThan(1);
    expect(firstItem.children[1].type).toBe("list");
  });

  it("terminates list on blank line", () => {
    const src = "* Item\n\nNext paragraph.\n";
    const doc = parse(src);
    expect(doc.children).toHaveLength(2);
    expect(doc.children[0].type).toBe("list");
    expect(doc.children[1].type).toBe("paragraph");
  });
});

describe("block parser — comments", () => {
  it("skips single-line // comments", () => {
    const src = "// This is a comment\nVisible text.\n";
    const doc = parse(src);
    expect(doc.children).toHaveLength(1);
    const p = doc.children[0] as ParagraphNode;
    const text = p.children[0] as TextNode;
    expect(text.value).toContain("Visible");
  });
});

describe("block parser — mixed content", () => {
  it("parses heading + paragraph + code block", () => {
    const src = [
      "= Document Title",
      "",
      "Introduction paragraph.",
      "",
      "[source,js]",
      "----",
      "console.log('hi');",
      "----",
      "",
    ].join("\n");
    const doc = parse(src);
    expect(doc.children).toHaveLength(3);
    expect(doc.children[0].type).toBe("heading");
    expect(doc.children[1].type).toBe("paragraph");
    expect(doc.children[2].type).toBe("code_block");
  });
});

// ─── Inline parser tests ──────────────────────────────────────────────────────

describe("inline parser — plain text", () => {
  it("returns a single TextNode for plain text", () => {
    const nodes = parseInline("Hello world");
    expect(nodes).toHaveLength(1);
    expect(nodes[0]).toMatchObject({ type: "text", value: "Hello world" });
  });
});

describe("inline parser — strong (asterisk)", () => {
  it("parses constrained *bold*", () => {
    const nodes = parseInline("Hello *bold* world");
    expect(nodes).toHaveLength(3);
    expect(nodes[1].type).toBe("strong");
    const strong = nodes[1] as StrongNode;
    expect(strong.children[0]).toMatchObject({ type: "text", value: "bold" });
  });

  it("parses unconstrained **bold**", () => {
    const nodes = parseInline("Hello **bold** world");
    const strong = nodes.find(n => n.type === "strong") as StrongNode;
    expect(strong).toBeDefined();
    expect(strong.children[0]).toMatchObject({ type: "text", value: "bold" });
  });
});

describe("inline parser — emphasis (underscore)", () => {
  it("parses constrained _italic_", () => {
    const nodes = parseInline("Hello _italic_ world");
    const em = nodes.find(n => n.type === "emphasis") as EmphasisNode;
    expect(em).toBeDefined();
    expect(em.children[0]).toMatchObject({ type: "text", value: "italic" });
  });

  it("parses unconstrained __italic__", () => {
    const nodes = parseInline("Hello __italic__ world");
    const em = nodes.find(n => n.type === "emphasis") as EmphasisNode;
    expect(em).toBeDefined();
  });
});

describe("inline parser — code span", () => {
  it("parses backtick code spans", () => {
    const nodes = parseInline("Use `npm install` to install.");
    const cs = nodes.find(n => n.type === "code_span") as CodeSpanNode;
    expect(cs).toBeDefined();
    expect(cs.value).toBe("npm install");
  });
});

describe("inline parser — links", () => {
  it("parses link: macro", () => {
    const nodes = parseInline("See link:https://example.com[Example Site].");
    const link = nodes.find(n => n.type === "link") as LinkNode;
    expect(link).toBeDefined();
    expect(link.url).toBe("https://example.com");
    expect(link.children[0]).toMatchObject({ type: "text", value: "Example Site" });
  });

  it("parses bare https:// URL as autolink", () => {
    const nodes = parseInline("Visit https://example.com today.");
    const al = nodes.find(n => n.type === "autolink") as AutolinkNode;
    expect(al).toBeDefined();
    expect(al.url).toBe("https://example.com");
  });

  it("parses https:// URL with [text] as link", () => {
    const nodes = parseInline("Visit https://example.com[click here] now.");
    const link = nodes.find(n => n.type === "link") as LinkNode;
    expect(link).toBeDefined();
    expect(link.url).toBe("https://example.com");
  });

  it("parses cross-reference <<anchor,text>>", () => {
    const nodes = parseInline("See <<introduction,Introduction>>.");
    const link = nodes.find(n => n.type === "link") as LinkNode;
    expect(link).toBeDefined();
    expect(link.url).toBe("#introduction");
    expect(link.children[0]).toMatchObject({ type: "text", value: "Introduction" });
  });

  it("parses cross-reference <<anchor>> without text", () => {
    const nodes = parseInline("See <<intro>>.");
    const link = nodes.find(n => n.type === "link") as LinkNode;
    expect(link).toBeDefined();
    expect(link.url).toBe("#intro");
  });
});

describe("inline parser — images", () => {
  it("parses image: macro", () => {
    const nodes = parseInline("Here: image:photo.png[A photo].");
    const img = nodes.find(n => n.type === "image") as ImageNode;
    expect(img).toBeDefined();
    expect(img.url).toBe("photo.png");
    expect(img.alt).toBe("A photo");
  });
});

describe("inline parser — line breaks", () => {
  it("parses two trailing spaces + newline as hard break", () => {
    const nodes = parseInline("Line one  \nLine two");
    const hb = nodes.find(n => n.type === "hard_break") as HardBreakNode;
    expect(hb).toBeDefined();
  });

  it("parses backslash + newline as hard break", () => {
    const nodes = parseInline("Line one\\\nLine two");
    const hb = nodes.find(n => n.type === "hard_break") as HardBreakNode;
    expect(hb).toBeDefined();
  });

  it("parses plain newline as soft break", () => {
    const nodes = parseInline("Line one\nLine two");
    const sb = nodes.find(n => n.type === "soft_break") as SoftBreakNode;
    expect(sb).toBeDefined();
  });
});

describe("inline parser — nested inline", () => {
  it("handles strong inside a paragraph", () => {
    const doc = parse("This is *bold text* in a paragraph.\n");
    const p = doc.children[0] as ParagraphNode;
    const strong = p.children.find(n => n.type === "strong") as StrongNode;
    expect(strong).toBeDefined();
    expect(strong.children[0]).toMatchObject({ type: "text", value: "bold text" });
  });

  it("handles emphasis inside a heading", () => {
    const doc = parse("== Hello _italic_ world\n");
    const h = doc.children[0] as HeadingNode;
    const em = h.children.find(n => n.type === "emphasis") as EmphasisNode;
    expect(em).toBeDefined();
  });
});

describe("document structure", () => {
  it("returns a DocumentNode with type 'document'", () => {
    const doc: DocumentNode = parse("Hello\n");
    expect(doc.type).toBe("document");
    expect(Array.isArray(doc.children)).toBe(true);
  });

  it("handles blank-only input", () => {
    const doc = parse("\n\n\n");
    expect(doc.children).toHaveLength(0);
  });
});
