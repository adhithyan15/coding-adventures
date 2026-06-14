/**
 * autolink.test.ts — end-to-end autolinkHeadings transform.
 */

import { describe, it, expect } from "vitest";
import type { BlockNode, DocumentNode, InlineNode } from "@coding-adventures/document-ast";
import { autolinkHeadings } from "../src/index.js";

function txt(value: string): InlineNode { return { type: "text", value }; }
function h(level: 1|2|3|4|5|6, text: string): BlockNode {
  return { type: "heading", level, children: [txt(text)] };
}
function doc(...children: BlockNode[]): DocumentNode {
  return { type: "document", children };
}

describe("autolinkHeadings — flat document", () => {
  it("one heading → one slug", () => {
    const out = autolinkHeadings(doc(h(1, "Hello World")));
    expect(out).toEqual([
      { level: 1, text: "Hello World", slug: "hello-world", anchorHref: "#hello-world" },
    ]);
  });

  it("preserves document order", () => {
    const out = autolinkHeadings(doc(
      h(1, "First"),
      h(2, "Second"),
      h(2, "Third"),
    ));
    expect(out.map((s) => s.text)).toEqual(["First", "Second", "Third"]);
  });

  it("preserves level on each annotation", () => {
    const out = autolinkHeadings(doc(
      h(1, "h1"),
      h(2, "h2"),
      h(3, "h3"),
      h(6, "h6"),
    ));
    expect(out.map((s) => s.level)).toEqual([1, 2, 3, 6]);
  });

  it("empty document → empty array", () => {
    expect(autolinkHeadings(doc())).toEqual([]);
  });

  it("document with no headings → empty array", () => {
    const out = autolinkHeadings(doc(
      { type: "paragraph", children: [txt("just a paragraph")] },
    ));
    expect(out).toEqual([]);
  });
});

describe("autolinkHeadings — anchorHref always equals #slug", () => {
  it("simple", () => {
    const [first] = autolinkHeadings(doc(h(1, "Hi")));
    expect(first!.anchorHref).toBe(`#${first!.slug}`);
  });

  it("after collision suffix", () => {
    const out = autolinkHeadings(doc(h(1, "Setup"), h(1, "Setup")));
    expect(out[0]!.anchorHref).toBe("#setup");
    expect(out[1]!.anchorHref).toBe("#setup-2");
  });
});

describe("autolinkHeadings — collision resolution applies globally", () => {
  it("two headings with the same text get -2 on the second", () => {
    const out = autolinkHeadings(doc(h(2, "Setup"), h(2, "Setup")));
    expect(out.map((s) => s.slug)).toEqual(["setup", "setup-2"]);
  });

  it("three same-text headings: -2, -3", () => {
    const out = autolinkHeadings(doc(h(2, "Setup"), h(2, "Setup"), h(2, "Setup")));
    expect(out.map((s) => s.slug)).toEqual(["setup", "setup-2", "setup-3"]);
  });

  it("different texts producing the same slug also collide", () => {
    // "Hello World" and "Hello, World!" both slugify to "hello-world"
    // (the comma + bang are stripped, multi-space collapses).
    const out = autolinkHeadings(doc(h(2, "Hello World"), h(2, "Hello, World!")));
    expect(out.map((s) => s.slug)).toEqual(["hello-world", "hello-world-2"]);
  });
});

describe("autolinkHeadings — extracts heading text correctly", () => {
  it("heading with mixed inline content", () => {
    const out = autolinkHeadings(doc({
      type: "heading", level: 2,
      children: [
        txt("Step "),
        { type: "strong", children: [txt("2")] },
        txt(": install "),
        { type: "code_span", value: "npm" },
        txt(" deps"),
      ],
    }));
    expect(out[0]!.text).toBe("Step 2: install npm deps");
    expect(out[0]!.slug).toBe("step-2-install-npm-deps");
  });

  it("heading whose flattened text is empty → 'section' fallback", () => {
    const out = autolinkHeadings(doc({
      type: "heading", level: 1,
      children: [{ type: "raw_inline", format: "html", value: "<!--x-->" }],
    }));
    expect(out[0]!.slug).toBe("section");
  });

  it("multiple empty-text headings get section, section-2, section-3", () => {
    const out = autolinkHeadings(doc(
      { type: "heading", level: 1, children: [] },
      { type: "heading", level: 2, children: [] },
      { type: "heading", level: 3, children: [] },
    ));
    expect(out.map((s) => s.slug)).toEqual(["section", "section-2", "section-3"]);
  });
});

describe("autolinkHeadings — walks nested containers", () => {
  it("headings inside blockquote are found", () => {
    const out = autolinkHeadings(doc({
      type: "blockquote",
      children: [h(2, "Quoted Heading")],
    }));
    expect(out.map((s) => s.text)).toEqual(["Quoted Heading"]);
  });

  it("headings inside list items are found", () => {
    const out = autolinkHeadings(doc({
      type: "list", ordered: false, start: null, tight: true,
      children: [
        { type: "list_item", checked: null, children: [h(3, "Item Heading")] },
      ],
    }));
    expect(out.map((s) => s.text)).toEqual(["Item Heading"]);
  });

  it("headings inside task items are found", () => {
    const out = autolinkHeadings(doc({
      type: "list", ordered: false, start: null, tight: true,
      children: [
        { type: "task_item", checked: true, children: [h(3, "Task Heading")] },
      ],
    }));
    expect(out.map((s) => s.text)).toEqual(["Task Heading"]);
  });

  it("deeply nested headings (blockquote → list → heading)", () => {
    const out = autolinkHeadings(doc({
      type: "blockquote",
      children: [{
        type: "list", ordered: false, start: null, tight: true,
        children: [
          { type: "list_item", checked: null, children: [h(4, "Deep")] },
        ],
      }],
    }));
    expect(out.map((s) => s.slug)).toEqual(["deep"]);
  });

  it("blocks without nested-block content (paragraph, code_block, thematic_break, raw_block, table) don't break the walk", () => {
    const out = autolinkHeadings(doc(
      { type: "paragraph", children: [txt("p")] },
      { type: "code_block", language: null, value: "x\n" },
      { type: "thematic_break" },
      { type: "raw_block", format: "html", value: "<hr>" },
      { type: "table", align: [null], children: [
        { type: "table_row", children: [{ type: "table_cell", header: true, children: [txt("c")] }] },
      ]},
      h(1, "Real Heading"),
    ));
    expect(out.map((s) => s.text)).toEqual(["Real Heading"]);
  });
});

describe("autolinkHeadings — defensive: non-tree BlockNode variants ignored", () => {
  // The BlockNode union includes DocumentNode / ListItemNode /
  // TaskItemNode / TableRowNode / TableCellNode for type-system
  // simplicity, but they never appear as direct siblings of
  // other blocks in well-formed AST.  The walker silently
  // ignores them (defensive no-op).  These tests exercise the
  // defensive branches.

  it("DocumentNode in doc.children is ignored", () => {
    const out = autolinkHeadings(doc(
      h(1, "Real"),
      { type: "document", children: [h(1, "Nested-doc heading")] } as BlockNode,
    ));
    // Only the real heading is picked up; the nested DocumentNode
    // is treated as a defensive no-op and its inner heading is NOT
    // walked.
    expect(out.map((s) => s.text)).toEqual(["Real"]);
  });

  it("ListItemNode in doc.children is ignored", () => {
    const out = autolinkHeadings(doc(
      h(1, "Real"),
      { type: "list_item", checked: null, children: [h(2, "Stray")] } as BlockNode,
    ));
    expect(out.map((s) => s.text)).toEqual(["Real"]);
  });

  it("TaskItemNode in doc.children is ignored", () => {
    const out = autolinkHeadings(doc(
      h(1, "Real"),
      { type: "task_item", checked: false, children: [h(2, "Stray")] } as BlockNode,
    ));
    expect(out.map((s) => s.text)).toEqual(["Real"]);
  });

  it("TableRowNode in doc.children is ignored", () => {
    const out = autolinkHeadings(doc(
      h(1, "Real"),
      { type: "table_row", children: [] } as BlockNode,
    ));
    expect(out.map((s) => s.text)).toEqual(["Real"]);
  });

  it("TableCellNode in doc.children is ignored", () => {
    const out = autolinkHeadings(doc(
      h(1, "Real"),
      { type: "table_cell", header: false, children: [] } as BlockNode,
    ));
    expect(out.map((s) => s.text)).toEqual(["Real"]);
  });
});

describe("autolinkHeadings — reproducibility (FM03)", () => {
  it("same input → byte-identical output", () => {
    const d = doc(h(1, "A"), h(2, "B"), h(2, "B"));
    expect(autolinkHeadings(d)).toEqual(autolinkHeadings(d));
  });

  it("does not mutate the input document", () => {
    const d = doc(h(1, "A"), h(2, "B"));
    const snapshot = JSON.stringify(d);
    autolinkHeadings(d);
    expect(JSON.stringify(d)).toBe(snapshot);
  });
});

describe("autolinkHeadings — security / hostile inputs", () => {
  it("heading text with script tag → slug stripped, no markup leaks", () => {
    const out = autolinkHeadings(doc(h(2, "<script>alert(1)</script>")));
    expect(out[0]!.slug).toMatch(/^[a-z0-9-]+$/);
    expect(out[0]!.anchorHref).toMatch(/^#[a-z0-9-]+$/);
    expect(out[0]!.slug).not.toContain("<");
    expect(out[0]!.slug).not.toContain(">");
  });

  it("heading text with NUL byte → slug stripped clean", () => {
    const out = autolinkHeadings(doc(h(2, "hel\x00lo")));
    expect(out[0]!.slug).toBe("hello");
  });

  it("heading with attribute-breakout chars → all stripped", () => {
    const out = autolinkHeadings(doc(h(2, `evil"onload=alert(1)//`)));
    expect(out[0]!.slug).not.toContain('"');
    expect(out[0]!.slug).not.toContain("=");
    expect(out[0]!.slug).toMatch(/^[a-z0-9-]+$/);
  });
});
