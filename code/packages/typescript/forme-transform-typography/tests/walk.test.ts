/**
 * walk.test.ts — AST walker: DocumentNode → typography-transformed copy.
 */

import { describe, it, expect } from "vitest";
import type {
  BlockNode,
  DocumentNode,
  InlineNode,
} from "@coding-adventures/document-ast";
import { typography } from "../src/index.js";

function txt(value: string): InlineNode { return { type: "text", value }; }
function p(...children: InlineNode[]): BlockNode {
  return { type: "paragraph", children };
}
function h(level: 1|2|3|4|5|6, ...children: InlineNode[]): BlockNode {
  return { type: "heading", level, children };
}
function doc(...children: BlockNode[]): DocumentNode {
  return { type: "document", children };
}

describe("typography — basic prose text", () => {
  it("smart-quotes text inside a paragraph", () => {
    const out = typography(doc(p(txt(`He said "wait"`))));
    const para = out.children[0] as { children: InlineNode[] };
    expect(para.children[0]).toEqual({ type: "text", value: `He said “wait”` });
  });

  it("transforms text inside heading", () => {
    const out = typography(doc(h(2, txt(`don't`))));
    const heading = out.children[0] as { children: InlineNode[] };
    expect(heading.children[0]).toEqual({ type: "text", value: `don’t` });
  });

  it("empty document → fresh empty document", () => {
    const out = typography(doc());
    expect(out).toEqual({ type: "document", children: [] });
  });
});

describe("typography — recurses into formatting wrappers", () => {
  it("inside emphasis", () => {
    const out = typography(doc(p({
      type: "emphasis",
      children: [txt(`it's`)],
    })));
    const para = out.children[0] as { children: InlineNode[] };
    const em = para.children[0] as { children: InlineNode[] };
    expect(em.children[0]).toEqual({ type: "text", value: `it’s` });
  });

  it("inside strong", () => {
    const out = typography(doc(p({
      type: "strong",
      children: [txt(`"loud"`)],
    })));
    const para = out.children[0] as { children: InlineNode[] };
    const strong = para.children[0] as { children: InlineNode[] };
    expect(strong.children[0]).toEqual({ type: "text", value: `“loud”` });
  });

  it("inside strikethrough", () => {
    const out = typography(doc(p({
      type: "strikethrough",
      children: [txt(`-- gone --`)],
    })));
    const para = out.children[0] as { children: InlineNode[] };
    const strike = para.children[0] as { children: InlineNode[] };
    expect(strike.children[0]).toEqual({ type: "text", value: `– gone –` });
  });

  it("inside link label (but URL passes through)", () => {
    const out = typography(doc(p({
      type: "link",
      destination: `https://example.com/?q="raw"`,
      title: null,
      children: [txt(`don't`)],
    })));
    const para = out.children[0] as { children: InlineNode[] };
    const link = para.children[0] as { type: string; destination: string; children: InlineNode[] };
    expect(link.destination).toBe(`https://example.com/?q="raw"`);  // unchanged
    expect(link.children[0]).toEqual({ type: "text", value: `don’t` }); // typeset
  });
});

describe("typography — pass-through nodes (no typeset)", () => {
  it("code_block value unchanged", () => {
    const out = typography(doc({
      type: "code_block",
      language: "ts",
      value: `const x = "hello"; // don't break\n`,
    }));
    expect(out.children[0]).toEqual({
      type: "code_block",
      language: "ts",
      value: `const x = "hello"; // don't break\n`,
    });
  });

  it("code_span value unchanged inside paragraph", () => {
    const out = typography(doc(p(
      txt(`see `),
      { type: "code_span", value: `"raw"` },
      txt(` -- works`),
    )));
    const para = out.children[0] as { children: InlineNode[] };
    expect(para.children[1]).toEqual({ type: "code_span", value: `"raw"` }); // unchanged
    expect(para.children[2]).toEqual({ type: "text", value: ` – works` }); // typeset
  });

  it("raw_block value unchanged", () => {
    const out = typography(doc({
      type: "raw_block",
      format: "html",
      value: `<aside data-q="don't">x</aside>`,
    }));
    expect(out.children[0]).toEqual({
      type: "raw_block",
      format: "html",
      value: `<aside data-q="don't">x</aside>`,
    });
  });

  it("raw_inline value unchanged inside paragraph", () => {
    const out = typography(doc(p(
      txt(`text `),
      { type: "raw_inline", format: "html", value: `<sup data-q="x">x</sup>` },
    )));
    const para = out.children[0] as { children: InlineNode[] };
    expect(para.children[1]).toEqual({
      type: "raw_inline", format: "html", value: `<sup data-q="x">x</sup>`,
    });
  });

  it("image alt text unchanged (v0 chose safety)", () => {
    const out = typography(doc(p({
      type: "image", destination: "/cat.png", title: null, alt: `don't pet`,
    })));
    const para = out.children[0] as { children: InlineNode[] };
    expect(para.children[0]).toEqual({
      type: "image", destination: "/cat.png", title: null, alt: `don't pet`,
    });
  });

  it("image destination URL unchanged", () => {
    const out = typography(doc(p({
      type: "image",
      destination: `https://x.com/?q="hi"`,
      title: null,
      alt: "alt",
    })));
    const para = out.children[0] as { children: InlineNode[] };
    expect((para.children[0] as { destination: string }).destination)
      .toBe(`https://x.com/?q="hi"`);
  });

  it("autolink URL unchanged", () => {
    const out = typography(doc(p({
      type: "autolink", destination: `https://example.com`, isEmail: false,
    })));
    const para = out.children[0] as { children: InlineNode[] };
    expect(para.children[0]).toEqual({
      type: "autolink", destination: `https://example.com`, isEmail: false,
    });
  });

  it("hard_break + soft_break pass through unchanged", () => {
    const out = typography(doc(p(
      txt(`a`),
      { type: "hard_break" },
      txt(`b`),
      { type: "soft_break" },
      txt(`c`),
    )));
    const para = out.children[0] as { children: InlineNode[] };
    expect(para.children[1]).toEqual({ type: "hard_break" });
    expect(para.children[3]).toEqual({ type: "soft_break" });
  });

  it("thematic_break passes through", () => {
    const out = typography(doc({ type: "thematic_break" }));
    expect(out.children[0]).toEqual({ type: "thematic_break" });
  });
});

describe("typography — block containers", () => {
  it("recurses into blockquote", () => {
    const out = typography(doc({
      type: "blockquote",
      children: [p(txt(`said "x"`))],
    }));
    const bq = out.children[0] as { children: BlockNode[] };
    const para = bq.children[0] as { children: InlineNode[] };
    expect(para.children[0]).toEqual({ type: "text", value: `said “x”` });
  });

  it("recurses into list / list_item", () => {
    const out = typography(doc({
      type: "list", ordered: false, start: null, tight: true,
      children: [
        { type: "list_item", checked: null, children: [p(txt(`it's`))] },
      ],
    }));
    const list = out.children[0] as { children: { children: BlockNode[] }[] };
    const item = list.children[0]!;
    const para = item.children[0] as { children: InlineNode[] };
    expect(para.children[0]).toEqual({ type: "text", value: `it’s` });
  });

  it("recurses into task_item", () => {
    const out = typography(doc({
      type: "list", ordered: false, start: null, tight: true,
      children: [
        { type: "task_item", checked: true, children: [p(txt(`don't`))] },
      ],
    }));
    const list = out.children[0] as { children: { children: BlockNode[] }[] };
    const item = list.children[0]!;
    const para = item.children[0] as { children: InlineNode[] };
    expect(para.children[0]).toEqual({ type: "text", value: `don’t` });
  });

  it("recurses into table cells (header + body)", () => {
    const out = typography(doc({
      type: "table", align: [null],
      children: [
        { type: "table_row", children: [
          { type: "table_cell", header: true, children: [txt(`"head"`)] },
        ]},
        { type: "table_row", children: [
          { type: "table_cell", header: false, children: [txt(`don't`)] },
        ]},
      ],
    }));
    const table = out.children[0] as { children: { children: { children: InlineNode[] }[] }[] };
    const headCell = table.children[0]!.children[0]!;
    const bodyCell = table.children[1]!.children[0]!;
    expect(headCell.children[0]).toEqual({ type: "text", value: `“head”` });
    expect(bodyCell.children[0]).toEqual({ type: "text", value: `don’t` });
  });

  it("nested DocumentNode in children is also walked", () => {
    const out = typography(doc({
      type: "document",
      children: [p(txt(`"nested"`))],
    } as BlockNode));
    const inner = out.children[0] as { type: string; children: BlockNode[] };
    expect(inner.type).toBe("document");
    const para = inner.children[0] as { children: InlineNode[] };
    expect(para.children[0]).toEqual({ type: "text", value: `“nested”` });
  });
});

describe("typography — defensive: non-tree BlockNode variants as direct siblings", () => {
  // BlockNode union includes ListItem / TaskItem / TableRow /
  // TableCell which never appear as direct children of other
  // blocks in well-formed AST.  The walker handles them
  // defensively for type-system completeness.  Tests exercise
  // these branches.

  it("list_item as direct block child returns fresh copy", () => {
    const li: BlockNode = { type: "list_item", children: [p(txt(`"x"`))] };
    const out = typography(doc(li));
    expect(out.children[0]).not.toBe(li);
    const liOut = out.children[0] as { children: BlockNode[] };
    const para = liOut.children[0] as { children: InlineNode[] };
    expect(para.children[0]).toEqual({ type: "text", value: `“x”` });
  });

  it("task_item as direct block child returns fresh copy", () => {
    const ti: BlockNode = { type: "task_item", checked: true, children: [p(txt(`don't`))] };
    const out = typography(doc(ti));
    const tiOut = out.children[0] as { type: string; checked: boolean; children: BlockNode[] };
    expect(tiOut.type).toBe("task_item");
    expect(tiOut.checked).toBe(true);
  });

  it("table_row as direct block child returns fresh copy", () => {
    const tr: BlockNode = { type: "table_row", isHeader: false, children: [
      { type: "table_cell", children: [txt(`"x"`)] },
    ]};
    const out = typography(doc(tr));
    expect(out.children[0]).not.toBe(tr);
    expect((out.children[0] as { type: string }).type).toBe("table_row");
  });

  it("table_cell as direct block child returns fresh copy", () => {
    const tc: BlockNode = { type: "table_cell", children: [txt(`"x"`)] };
    const out = typography(doc(tc));
    expect(out.children[0]).not.toBe(tc);
    expect((out.children[0] as { type: string }).type).toBe("table_cell");
  });
});

describe("typography — options propagation", () => {
  it("smartQuotes: false reaches deep text", () => {
    const out = typography(doc(p(txt(`"x"`))), { smartQuotes: false });
    const para = out.children[0] as { children: InlineNode[] };
    expect(para.children[0]).toEqual({ type: "text", value: `"x"` });
  });

  it("ligatures: true applied to deep text", () => {
    const out = typography(doc(p({
      type: "strong",
      children: [txt(`(c) 2026`)],
    })), { ligatures: true });
    const para = out.children[0] as { children: InlineNode[] };
    const strong = para.children[0] as { children: InlineNode[] };
    expect(strong.children[0]).toEqual({ type: "text", value: `© 2026` });
  });
});

describe("typography — purity / determinism", () => {
  it("does not mutate input document", () => {
    const d = doc(p(txt(`"x"`)));
    const before = JSON.stringify(d);
    typography(d);
    expect(JSON.stringify(d)).toBe(before);
  });

  it("returns a fresh tree (no shared references with input)", () => {
    const d = doc(p(txt(`"x"`)));
    const out = typography(d);
    expect(out).not.toBe(d);
    expect(out.children).not.toBe(d.children);
    expect(out.children[0]).not.toBe(d.children[0]);
  });

  it("returns a fresh tree even for passthrough nodes", () => {
    // A code_block is not typeset, but its container should
    // still be a fresh object.
    const cb: BlockNode = { type: "code_block", language: null, value: "x\n" };
    const d = doc(cb);
    const out = typography(d);
    expect(out.children[0]).not.toBe(cb);
    expect(out.children[0]).toEqual(cb);
  });

  it("same input → byte-identical output", () => {
    const d = doc(p(txt(`"x" -- y...`)));
    expect(JSON.stringify(typography(d))).toBe(JSON.stringify(typography(d)));
  });

  it("default options match no-options call", () => {
    const d = doc(p(txt(`"x"`)));
    expect(JSON.stringify(typography(d))).toBe(JSON.stringify(typography(d, {})));
  });
});
