/**
 * walk.test.ts — rewriteInternalLinks end-to-end.
 */

import { describe, it, expect } from "vitest";
import type {
  BlockNode,
  DocumentNode,
  InlineNode,
  LinkNode,
} from "@coding-adventures/document-ast";
import { rewriteInternalLinks } from "../src/index.js";

function txt(value: string): InlineNode { return { type: "text", value }; }
function link(destination: string, ...children: InlineNode[]): InlineNode {
  return { type: "link", destination, title: null, children };
}
function p(...children: InlineNode[]): BlockNode {
  return { type: "paragraph", children };
}
function doc(...children: BlockNode[]): DocumentNode {
  return { type: "document", children };
}

// Common resolver fixtures.
const knownResolver = (slug: string): string | null => {
  const map: Record<string, string> = {
    "/about": "https://example.com/about",
    "/blog/post": "https://example.com/blog/post",
    "/": "https://example.com/",
  };
  return map[slug] ?? null;
};
const nullResolver = (): null => null;

describe("rewriteInternalLinks — internal link resolution", () => {
  it("rewrites a known internal slug", () => {
    const out = rewriteInternalLinks(
      doc(p(link("/about", txt("About")))),
      knownResolver,
    );
    const para = out.children[0] as { children: InlineNode[] };
    expect(para.children[0]).toEqual({
      type: "link",
      destination: "https://example.com/about",
      title: null,
      children: [txt("About")],
    });
  });

  it("preserves link title", () => {
    const out = rewriteInternalLinks(
      doc(p({
        type: "link",
        destination: "/about",
        title: "About us",
        children: [txt("About")],
      })),
      knownResolver,
    );
    const para = out.children[0] as { children: LinkNode[] };
    expect(para.children[0]!.title).toBe("About us");
    expect(para.children[0]!.destination).toBe("https://example.com/about");
  });

  it("rewrites bare /", () => {
    const out = rewriteInternalLinks(doc(p(link("/", txt("Home")))), knownResolver);
    const para = out.children[0] as { children: LinkNode[] };
    expect(para.children[0]!.destination).toBe("https://example.com/");
  });
});

describe("rewriteInternalLinks — external links pass through", () => {
  it("absolute https unchanged", () => {
    const out = rewriteInternalLinks(
      doc(p(link("https://github.com", txt("GH")))),
      knownResolver,
    );
    const para = out.children[0] as { children: LinkNode[] };
    expect(para.children[0]!.destination).toBe("https://github.com");
  });

  it("mailto unchanged", () => {
    const out = rewriteInternalLinks(
      doc(p(link("mailto:x@y.com", txt("Mail")))),
      knownResolver,
    );
    const para = out.children[0] as { children: LinkNode[] };
    expect(para.children[0]!.destination).toBe("mailto:x@y.com");
  });

  it("resolver is NOT called for external links", () => {
    let called = false;
    rewriteInternalLinks(
      doc(p(link("https://x.com", txt("X")))),
      () => { called = true; return null; },
    );
    expect(called).toBe(false);
  });
});

describe("rewriteInternalLinks — unresolved policy", () => {
  it("default 'keep' preserves original /slug", () => {
    const out = rewriteInternalLinks(
      doc(p(link("/missing", txt("M")))),
      nullResolver,
    );
    const para = out.children[0] as { children: LinkNode[] };
    expect(para.children[0]!.destination).toBe("/missing");
  });

  it("'keep' explicit option also preserves", () => {
    const out = rewriteInternalLinks(
      doc(p(link("/missing", txt("M")))),
      nullResolver,
      { unresolved: "keep" },
    );
    const para = out.children[0] as { children: LinkNode[] };
    expect(para.children[0]!.destination).toBe("/missing");
  });

  it("'strip' drops the link wrapper but keeps children", () => {
    const out = rewriteInternalLinks(
      doc(p(
        txt("before "),
        link("/missing", txt("M")),
        txt(" after"),
      )),
      nullResolver,
      { unresolved: "strip" },
    );
    const para = out.children[0] as { children: InlineNode[] };
    // Original 3 inlines; link expanded inline to one text node
    // → still 3 total.
    expect(para.children.length).toBe(3);
    expect(para.children[1]).toEqual(txt("M"));
  });

  it("'strip' on a link with multiple children expands them all", () => {
    const out = rewriteInternalLinks(
      doc(p(link("/missing", txt("hello "), {
        type: "strong", children: [txt("world")],
      }))),
      nullResolver,
      { unresolved: "strip" },
    );
    const para = out.children[0] as { children: InlineNode[] };
    expect(para.children.length).toBe(2);
    expect(para.children[0]).toEqual(txt("hello "));
    expect(para.children[1]!.type).toBe("strong");
  });

  it("'throw' throws on first unresolved slug", () => {
    expect(() => rewriteInternalLinks(
      doc(p(link("/missing", txt("M")))),
      nullResolver,
      { unresolved: "throw" },
    )).toThrow(/unresolved internal slug.*\/missing/);
  });

  it("resolver returning undefined treated same as null", () => {
    const out = rewriteInternalLinks(
      doc(p(link("/x", txt("X")))),
      (() => undefined) as unknown as (s: string) => string | null,
      { unresolved: "keep" },
    );
    const para = out.children[0] as { children: LinkNode[] };
    expect(para.children[0]!.destination).toBe("/x");
  });
});

describe("rewriteInternalLinks — resolver-returned-URL validation", () => {
  it("throws if resolver returns javascript: URL", () => {
    expect(() => rewriteInternalLinks(
      doc(p(link("/about", txt("X")))),
      () => "javascript:alert(1)",
    )).toThrow(/unsafe URL/);
  });

  it("throws if resolver returns data: URL", () => {
    expect(() => rewriteInternalLinks(
      doc(p(link("/about", txt("X")))),
      () => "data:text/html,<script>",
    )).toThrow(/unsafe URL/);
  });

  it("throws if resolver returns protocol-relative URL", () => {
    expect(() => rewriteInternalLinks(
      doc(p(link("/about", txt("X")))),
      () => "//evil.com",
    )).toThrow(/unsafe URL/);
  });

  it("throws if resolver returns a non-URL string (bare relative)", () => {
    expect(() => rewriteInternalLinks(
      doc(p(link("/about", txt("X")))),
      () => "about",
    )).toThrow(/unsafe URL/);
  });

  it("accepts http://", () => {
    const out = rewriteInternalLinks(
      doc(p(link("/about", txt("X")))),
      () => "http://example.com/about",
    );
    const para = out.children[0] as { children: LinkNode[] };
    expect(para.children[0]!.destination).toBe("http://example.com/about");
  });

  it("accepts root-relative (resolver chose not to fully resolve)", () => {
    const out = rewriteInternalLinks(
      doc(p(link("/about", txt("X")))),
      () => "/canonical/about",
    );
    const para = out.children[0] as { children: LinkNode[] };
    expect(para.children[0]!.destination).toBe("/canonical/about");
  });
});

describe("rewriteInternalLinks — walks nested containers", () => {
  it("inside blockquote", () => {
    const out = rewriteInternalLinks(
      doc({ type: "blockquote", children: [p(link("/about", txt("A")))] }),
      knownResolver,
    );
    const bq = out.children[0] as { children: BlockNode[] };
    const para = bq.children[0] as { children: LinkNode[] };
    expect(para.children[0]!.destination).toBe("https://example.com/about");
  });

  it("inside list_item", () => {
    const out = rewriteInternalLinks(
      doc({
        type: "list", ordered: false, start: null, tight: true,
        children: [{ type: "list_item", children: [p(link("/about", txt("A")))] }],
      }),
      knownResolver,
    );
    const list = out.children[0] as { children: { children: BlockNode[] }[] };
    const item = list.children[0]!;
    const para = item.children[0] as { children: LinkNode[] };
    expect(para.children[0]!.destination).toBe("https://example.com/about");
  });

  it("inside task_item", () => {
    const out = rewriteInternalLinks(
      doc({
        type: "list", ordered: false, start: null, tight: true,
        children: [{ type: "task_item", checked: true, children: [p(link("/about", txt("A")))] }],
      }),
      knownResolver,
    );
    const list = out.children[0] as { children: { children: BlockNode[] }[] };
    const item = list.children[0]!;
    const para = item.children[0] as { children: LinkNode[] };
    expect(para.children[0]!.destination).toBe("https://example.com/about");
  });

  it("inside heading", () => {
    const out = rewriteInternalLinks(
      doc({ type: "heading", level: 2, children: [link("/about", txt("A"))] }),
      knownResolver,
    );
    const h = out.children[0] as { children: LinkNode[] };
    expect(h.children[0]!.destination).toBe("https://example.com/about");
  });

  it("inside table cells", () => {
    const out = rewriteInternalLinks(
      doc({
        type: "table", align: [null],
        children: [
          { type: "table_row", isHeader: true, children: [
            { type: "table_cell", children: [link("/about", txt("Header"))] },
          ]},
          { type: "table_row", isHeader: false, children: [
            { type: "table_cell", children: [link("/blog/post", txt("Body"))] },
          ]},
        ],
      }),
      knownResolver,
    );
    const table = out.children[0] as { children: { children: { children: LinkNode[] }[] }[] };
    expect(table.children[0]!.children[0]!.children[0]!.destination).toBe("https://example.com/about");
    expect(table.children[1]!.children[0]!.children[0]!.destination).toBe("https://example.com/blog/post");
  });

  it("inside emphasis", () => {
    const out = rewriteInternalLinks(
      doc(p({ type: "emphasis", children: [link("/about", txt("A"))] })),
      knownResolver,
    );
    const para = out.children[0] as { children: InlineNode[] };
    const em = para.children[0] as { type: string; children: LinkNode[] };
    expect(em.type).toBe("emphasis");
    expect(em.children[0]!.destination).toBe("https://example.com/about");
  });

  it("inside strong", () => {
    const out = rewriteInternalLinks(
      doc(p({ type: "strong", children: [link("/about", txt("A"))] })),
      knownResolver,
    );
    const para = out.children[0] as { children: InlineNode[] };
    const strong = para.children[0] as { type: string; children: LinkNode[] };
    expect(strong.type).toBe("strong");
    expect(strong.children[0]!.destination).toBe("https://example.com/about");
  });

  it("inside strikethrough", () => {
    const out = rewriteInternalLinks(
      doc(p({ type: "strikethrough", children: [link("/about", txt("A"))] })),
      knownResolver,
    );
    const para = out.children[0] as { children: InlineNode[] };
    const strike = para.children[0] as { type: string; children: LinkNode[] };
    expect(strike.type).toBe("strikethrough");
    expect(strike.children[0]!.destination).toBe("https://example.com/about");
  });

  it("nested DocumentNode in children", () => {
    const out = rewriteInternalLinks(
      doc({ type: "document", children: [p(link("/about", txt("A")))] } as BlockNode),
      knownResolver,
    );
    const inner = out.children[0] as { type: string; children: BlockNode[] };
    expect(inner.type).toBe("document");
    const para = inner.children[0] as { children: LinkNode[] };
    expect(para.children[0]!.destination).toBe("https://example.com/about");
  });
});

describe("rewriteInternalLinks — pass-through nodes", () => {
  it("ImageNode.destination unchanged even if internal-looking", () => {
    const img: InlineNode = {
      type: "image", destination: "/cat.png", title: null, alt: "cat",
    };
    const out = rewriteInternalLinks(doc(p(img)), knownResolver);
    const para = out.children[0] as { children: InlineNode[] };
    expect((para.children[0] as { destination: string }).destination).toBe("/cat.png");
  });

  it("AutolinkNode unchanged", () => {
    const out = rewriteInternalLinks(doc(p({
      type: "autolink", destination: "https://example.com", isEmail: false,
    })), knownResolver);
    const para = out.children[0] as { children: InlineNode[] };
    expect(para.children[0]).toEqual({
      type: "autolink", destination: "https://example.com", isEmail: false,
    });
  });

  it("CodeBlockNode unchanged", () => {
    const out = rewriteInternalLinks(doc({
      type: "code_block", language: "md", value: "[link](/about)\n",
    }), knownResolver);
    expect(out.children[0]).toEqual({
      type: "code_block", language: "md", value: "[link](/about)\n",
    });
  });

  it("CodeSpanNode unchanged", () => {
    const out = rewriteInternalLinks(doc(p(
      { type: "code_span", value: "[link](/about)" },
    )), knownResolver);
    const para = out.children[0] as { children: InlineNode[] };
    expect(para.children[0]).toEqual({ type: "code_span", value: "[link](/about)" });
  });

  it("RawBlockNode unchanged", () => {
    const out = rewriteInternalLinks(doc({
      type: "raw_block", format: "html", value: `<a href="/about">x</a>`,
    }), knownResolver);
    expect(out.children[0]).toEqual({
      type: "raw_block", format: "html", value: `<a href="/about">x</a>`,
    });
  });

  it("RawInlineNode unchanged", () => {
    const out = rewriteInternalLinks(doc(p({
      type: "raw_inline", format: "html", value: `<a href="/about">x</a>`,
    })), knownResolver);
    const para = out.children[0] as { children: InlineNode[] };
    expect(para.children[0]).toEqual({
      type: "raw_inline", format: "html", value: `<a href="/about">x</a>`,
    });
  });

  it("thematic_break / hard_break / soft_break unchanged", () => {
    const out = rewriteInternalLinks(doc(
      { type: "thematic_break" },
      p(txt("a"), { type: "hard_break" }, txt("b"), { type: "soft_break" }, txt("c")),
    ), knownResolver);
    expect(out.children[0]).toEqual({ type: "thematic_break" });
    const para = out.children[1] as { children: InlineNode[] };
    expect(para.children[1]).toEqual({ type: "hard_break" });
    expect(para.children[3]).toEqual({ type: "soft_break" });
  });
});

describe("rewriteInternalLinks — defensive non-tree BlockNode variants", () => {
  it("list_item as direct block child rewrites internal links", () => {
    const out = rewriteInternalLinks(
      doc({ type: "list_item", children: [p(link("/about", txt("A")))] } as BlockNode),
      knownResolver,
    );
    const li = out.children[0] as { children: BlockNode[] };
    const para = li.children[0] as { children: LinkNode[] };
    expect(para.children[0]!.destination).toBe("https://example.com/about");
  });

  it("task_item as direct block child", () => {
    const out = rewriteInternalLinks(
      doc({ type: "task_item", checked: false, children: [p(link("/about", txt("A")))] } as BlockNode),
      knownResolver,
    );
    const ti = out.children[0] as { type: string; checked: boolean; children: BlockNode[] };
    expect(ti.type).toBe("task_item");
    const para = ti.children[0] as { children: LinkNode[] };
    expect(para.children[0]!.destination).toBe("https://example.com/about");
  });

  it("table_row / table_cell as direct block child", () => {
    const out = rewriteInternalLinks(
      doc(
        { type: "table_row", isHeader: false, children: [
          { type: "table_cell", children: [link("/about", txt("A"))] },
        ]} as BlockNode,
        { type: "table_cell", children: [link("/blog/post", txt("B"))] } as BlockNode,
      ),
      knownResolver,
    );
    const row = out.children[0] as { children: { children: LinkNode[] }[] };
    expect(row.children[0]!.children[0]!.destination).toBe("https://example.com/about");
    const cell = out.children[1] as { children: LinkNode[] };
    expect(cell.children[0]!.destination).toBe("https://example.com/blog/post");
  });
});

describe("rewriteInternalLinks — purity / determinism", () => {
  it("does not mutate input document", () => {
    const d = doc(p(link("/about", txt("A"))));
    const before = JSON.stringify(d);
    rewriteInternalLinks(d, knownResolver);
    expect(JSON.stringify(d)).toBe(before);
  });

  it("returns a fresh tree (no shared references)", () => {
    const d = doc(p(link("/about", txt("A"))));
    const out = rewriteInternalLinks(d, knownResolver);
    expect(out).not.toBe(d);
    expect(out.children).not.toBe(d.children);
    expect(out.children[0]).not.toBe(d.children[0]);
  });

  it("same input + resolver → byte-identical output", () => {
    const d = doc(p(link("/about", txt("A")), link("/blog/post", txt("B"))));
    expect(JSON.stringify(rewriteInternalLinks(d, knownResolver)))
      .toBe(JSON.stringify(rewriteInternalLinks(d, knownResolver)));
  });

  it("resolver invoked once per internal link (not per call)", () => {
    let calls = 0;
    rewriteInternalLinks(
      doc(p(link("/about", txt("A"))), p(link("/about", txt("B")))),
      (slug) => { calls++; return slug === "/about" ? "https://x/" : null; },
    );
    // Two internal links → resolver called twice.
    expect(calls).toBe(2);
  });
});
