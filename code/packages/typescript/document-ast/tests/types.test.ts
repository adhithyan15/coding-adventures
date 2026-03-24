/**
 * Document AST — Type-level tests
 *
 * Because this is a types-only package, the "tests" are mostly compile-time
 * checks: build valid AST values and verify that the TypeScript compiler
 * accepts them. We also verify the discriminated-union structure at runtime
 * by inspecting `node.type`.
 *
 * These tests serve as living documentation — they show how to construct and
 * traverse Document AST values.
 */

import { describe, it, expect } from "vitest";
import type {
  DocumentNode, BlockNode, InlineNode,
  HeadingNode, ParagraphNode, CodeBlockNode,
  BlockquoteNode, ListNode, ListItemNode,
  ThematicBreakNode, RawBlockNode,
  TextNode, EmphasisNode, StrongNode, CodeSpanNode,
  LinkNode, ImageNode, AutolinkNode, RawInlineNode,
  HardBreakNode, SoftBreakNode,
} from "../src/types.js";

// ─── Helpers ──────────────────────────────────────────────────────────────────

/** Build a minimal DocumentNode for testing. */
function makeDoc(...children: BlockNode[]): DocumentNode {
  return { type: "document", children };
}

/** Build a ParagraphNode with the given inline children. */
function makePara(...children: InlineNode[]): ParagraphNode {
  return { type: "paragraph", children };
}

/** Build a TextNode. */
function text(value: string): TextNode {
  return { type: "text", value };
}

// ─── Block Node Tests ─────────────────────────────────────────────────────────

describe("DocumentNode", () => {
  it("has type 'document' and a children array", () => {
    const doc = makeDoc();
    expect(doc.type).toBe("document");
    expect(doc.children).toEqual([]);
  });

  it("accepts any BlockNode as a child", () => {
    const heading: HeadingNode = {
      type: "heading",
      level: 1,
      children: [text("Title")],
    };
    const doc = makeDoc(heading);
    expect(doc.children[0]?.type).toBe("heading");
  });
});

describe("HeadingNode", () => {
  it("supports all six levels", () => {
    const levels: Array<1 | 2 | 3 | 4 | 5 | 6> = [1, 2, 3, 4, 5, 6];
    for (const level of levels) {
      const node: HeadingNode = { type: "heading", level, children: [text("hi")] };
      expect(node.level).toBe(level);
    }
  });

  it("contains inline children", () => {
    const em: EmphasisNode = { type: "emphasis", children: [text("italic")] };
    const h: HeadingNode = { type: "heading", level: 2, children: [text("Hello "), em] };
    expect(h.children).toHaveLength(2);
    expect(h.children[1]?.type).toBe("emphasis");
  });
});

describe("ParagraphNode", () => {
  it("contains inline children", () => {
    const p = makePara(text("Hello"), { type: "soft_break" }, text("world"));
    expect(p.type).toBe("paragraph");
    expect(p.children).toHaveLength(3);
    expect(p.children[1]?.type).toBe("soft_break");
  });
});

describe("CodeBlockNode", () => {
  it("stores raw code and optional language", () => {
    const cb: CodeBlockNode = {
      type: "code_block",
      language: "typescript",
      value: "const x = 1;\n",
    };
    expect(cb.language).toBe("typescript");
    expect(cb.value).toBe("const x = 1;\n");
  });

  it("accepts null language for unlabelled blocks", () => {
    const cb: CodeBlockNode = {
      type: "code_block",
      language: null,
      value: "plain text\n",
    };
    expect(cb.language).toBeNull();
  });
});

describe("BlockquoteNode", () => {
  it("can contain block children including nested blockquotes", () => {
    const inner: BlockquoteNode = {
      type: "blockquote",
      children: [makePara(text("nested"))],
    };
    const outer: BlockquoteNode = {
      type: "blockquote",
      children: [makePara(text("outer")), inner],
    };
    expect(outer.children).toHaveLength(2);
    expect(outer.children[1]?.type).toBe("blockquote");
  });
});

describe("ListNode and ListItemNode", () => {
  it("builds an unordered tight list", () => {
    const item1: ListItemNode = { type: "list_item", children: [makePara(text("a"))] };
    const item2: ListItemNode = { type: "list_item", children: [makePara(text("b"))] };
    const list: ListNode = {
      type: "list",
      ordered: false,
      start: null,
      tight: true,
      children: [item1, item2],
    };
    expect(list.ordered).toBe(false);
    expect(list.tight).toBe(true);
    expect(list.start).toBeNull();
    expect(list.children).toHaveLength(2);
  });

  it("builds an ordered list with a custom start number", () => {
    const item: ListItemNode = { type: "list_item", children: [makePara(text("x"))] };
    const list: ListNode = {
      type: "list",
      ordered: true,
      start: 42,
      tight: false,
      children: [item],
    };
    expect(list.ordered).toBe(true);
    expect(list.start).toBe(42);
  });
});

describe("ThematicBreakNode", () => {
  it("is a leaf node with only a type field", () => {
    const hr: ThematicBreakNode = { type: "thematic_break" };
    expect(hr.type).toBe("thematic_break");
    // @ts-expect-error — thematic_break has no children
    void hr.children;
  });
});

describe("RawBlockNode", () => {
  it("stores format and verbatim value", () => {
    const raw: RawBlockNode = {
      type: "raw_block",
      format: "html",
      value: "<div>raw</div>\n",
    };
    expect(raw.format).toBe("html");
    expect(raw.value).toBe("<div>raw</div>\n");
  });

  it("accepts arbitrary format strings for non-HTML back-ends", () => {
    const latex: RawBlockNode = {
      type: "raw_block",
      format: "latex",
      value: "\\textbf{bold}\n",
    };
    expect(latex.format).toBe("latex");
  });
});

// ─── Inline Node Tests ────────────────────────────────────────────────────────

describe("TextNode", () => {
  it("stores decoded Unicode", () => {
    const t: TextNode = { type: "text", value: "Hello & world" };
    expect(t.value).toBe("Hello & world");
  });
});

describe("EmphasisNode", () => {
  it("nests inline children", () => {
    const em: EmphasisNode = {
      type: "emphasis",
      children: [text("hi"), { type: "code_span", value: "x" }],
    };
    expect(em.children).toHaveLength(2);
  });
});

describe("StrongNode", () => {
  it("can nest EmphasisNode (triple asterisks case)", () => {
    const em: EmphasisNode = { type: "emphasis", children: [text("x")] };
    const strong: StrongNode = { type: "strong", children: [em] };
    expect(strong.children[0]?.type).toBe("emphasis");
  });
});

describe("CodeSpanNode", () => {
  it("stores raw code, not decoded", () => {
    const cs: CodeSpanNode = { type: "code_span", value: "&amp;" };
    expect(cs.value).toBe("&amp;"); // NOT decoded — code spans are raw
  });
});

describe("LinkNode", () => {
  it("stores resolved destination and optional title", () => {
    const link: LinkNode = {
      type: "link",
      destination: "https://example.com",
      title: "Example",
      children: [text("click here")],
    };
    expect(link.destination).toBe("https://example.com");
    expect(link.title).toBe("Example");
    expect(link.children[0]?.type).toBe("text");
  });

  it("accepts null title", () => {
    const link: LinkNode = {
      type: "link",
      destination: "/relative",
      title: null,
      children: [],
    };
    expect(link.title).toBeNull();
  });
});

describe("ImageNode", () => {
  it("stores destination, plain-text alt, and optional title", () => {
    const img: ImageNode = {
      type: "image",
      destination: "cat.png",
      title: "A cat",
      alt: "a tabby cat",
    };
    expect(img.alt).toBe("a tabby cat");
    expect(img.destination).toBe("cat.png");
  });
});

describe("AutolinkNode", () => {
  it("distinguishes email and URL autolinks via isEmail", () => {
    const email: AutolinkNode = {
      type: "autolink",
      destination: "user@example.com",
      isEmail: true,
    };
    const url: AutolinkNode = {
      type: "autolink",
      destination: "https://example.com",
      isEmail: false,
    };
    expect(email.isEmail).toBe(true);
    expect(url.isEmail).toBe(false);
  });
});

describe("RawInlineNode", () => {
  it("stores format and verbatim value", () => {
    const raw: RawInlineNode = {
      type: "raw_inline",
      format: "html",
      value: "<em>raw</em>",
    };
    expect(raw.format).toBe("html");
    expect(raw.value).toBe("<em>raw</em>");
  });
});

describe("HardBreakNode", () => {
  it("is a leaf node", () => {
    const hb: HardBreakNode = { type: "hard_break" };
    expect(hb.type).toBe("hard_break");
  });
});

describe("SoftBreakNode", () => {
  it("is a leaf node", () => {
    const sb: SoftBreakNode = { type: "soft_break" };
    expect(sb.type).toBe("soft_break");
  });
});

// ─── Discriminated union dispatch ─────────────────────────────────────────────

describe("BlockNode discriminated union", () => {
  it("allows exhaustive switch over all block types", () => {
    const nodes: BlockNode[] = [
      makeDoc(),
      { type: "heading", level: 1, children: [] },
      { type: "paragraph", children: [] },
      { type: "code_block", language: null, value: "" },
      { type: "blockquote", children: [] },
      { type: "list", ordered: false, start: null, tight: true, children: [] },
      { type: "list_item", children: [] },
      { type: "thematic_break" },
      { type: "raw_block", format: "html", value: "" },
    ];

    const typeNames = nodes.map(n => n.type);
    expect(typeNames).toEqual([
      "document", "heading", "paragraph", "code_block", "blockquote",
      "list", "list_item", "thematic_break", "raw_block",
    ]);
  });
});

describe("InlineNode discriminated union", () => {
  it("allows exhaustive switch over all inline types", () => {
    const nodes: InlineNode[] = [
      { type: "text", value: "hi" },
      { type: "emphasis", children: [] },
      { type: "strong", children: [] },
      { type: "code_span", value: "x" },
      { type: "link", destination: "/", title: null, children: [] },
      { type: "image", destination: "/", title: null, alt: "" },
      { type: "autolink", destination: "x@y.com", isEmail: true },
      { type: "raw_inline", format: "html", value: "" },
      { type: "hard_break" },
      { type: "soft_break" },
    ];

    const typeNames = nodes.map(n => n.type);
    expect(typeNames).toEqual([
      "text", "emphasis", "strong", "code_span", "link",
      "image", "autolink", "raw_inline", "hard_break", "soft_break",
    ]);
  });
});

// ─── Tree traversal example ───────────────────────────────────────────────────

describe("tree traversal", () => {
  it("can walk a document and collect all text values", () => {
    const doc = makeDoc(
      {
        type: "heading",
        level: 1,
        children: [text("Hello")],
      },
      makePara(text("World"), { type: "soft_break" }, text("!")),
    );

    const texts: string[] = [];
    function collectText(node: BlockNode | InlineNode): void {
      if (node.type === "text") {
        texts.push(node.value);
        return;
      }
      if ("children" in node) {
        for (const child of node.children) {
          collectText(child as BlockNode | InlineNode);
        }
      }
    }
    collectText(doc);

    expect(texts).toEqual(["Hello", "World", "!"]);
  });
});
