/**
 * Tests for document-ast-to-layout converter.
 *
 * We build minimal Document AST nodes by hand and verify the resulting
 * LayoutNode tree structure, ext fields, font properties, and colors.
 */

import { describe, it, expect } from "vitest";
import type {
  DocumentNode,
  BlockNode,
  InlineNode,
  ListChildNode,
  TableRowNode,
  TableCellNode,
} from "@coding-adventures/document-ast";
import type { LayoutNode, Color, FontSpec } from "@coding-adventures/layout-ir";
import {
  document_ast_to_layout,
  document_default_theme,
  type DocumentLayoutTheme,
} from "../src/index.js";

// ─── Helpers ─────────────────────────────────────────────────────────────────

const theme = document_default_theme();

function doc(...children: BlockNode[]): DocumentNode {
  return { type: "document", children };
}

function heading(level: 1 | 2 | 3 | 4 | 5 | 6, ...children: InlineNode[]): BlockNode {
  return { type: "heading", level, children };
}

function para(...children: InlineNode[]): BlockNode {
  return { type: "paragraph", children };
}

function codeBlock(value: string, language: string | null = null): BlockNode {
  return { type: "code_block", language, value };
}

function blockquote(...children: BlockNode[]): BlockNode {
  return { type: "blockquote", children };
}

function bulletList(...items: ListChildNode[]): BlockNode {
  return { type: "list", ordered: false, start: null, tight: true, children: items };
}

function orderedList(start: number, ...items: ListChildNode[]): BlockNode {
  return { type: "list", ordered: true, start, tight: false, children: items };
}

function listItem(...children: BlockNode[]): ListChildNode {
  return { type: "list_item", children };
}

function taskItem(checked: boolean, ...children: BlockNode[]): ListChildNode {
  return { type: "task_item", checked, children };
}

function tbreak(): BlockNode {
  return { type: "thematic_break" };
}

function rawBlock(format: string, value: string): BlockNode {
  return { type: "raw_block", format, value };
}

function tableNode(
  align: (null | "left" | "right" | "center")[],
  ...rows: TableRowNode[]
): BlockNode {
  return { type: "table", align, children: rows };
}

function tableRow(isHeader: boolean, ...cells: TableCellNode[]): TableRowNode {
  return { type: "table_row", isHeader, children: cells };
}

function tableCell(...children: InlineNode[]): TableCellNode {
  return { type: "table_cell", children };
}

// Inline helpers
function text(value: string): InlineNode { return { type: "text", value }; }
function em(...children: InlineNode[]): InlineNode { return { type: "emphasis", children }; }
function strong(...children: InlineNode[]): InlineNode { return { type: "strong", children }; }
function strike(...children: InlineNode[]): InlineNode { return { type: "strikethrough", children }; }
function codeSpan(value: string): InlineNode { return { type: "code_span", value }; }
function link(destination: string, title: string | null, ...children: InlineNode[]): InlineNode {
  return { type: "link", destination, title, children };
}
function autolink(destination: string, isEmail: boolean): InlineNode {
  return { type: "autolink", destination, isEmail };
}
function image(destination: string, alt: string, title: string | null = null): InlineNode {
  return { type: "image", destination, alt, title };
}
function softBreak(): InlineNode { return { type: "soft_break" }; }
function hardBreak(): InlineNode { return { type: "hard_break" }; }
function rawInline(format: string, value: string): InlineNode {
  return { type: "raw_inline", format, value };
}

// Tree walk helpers
function allLeaves(node: LayoutNode): LayoutNode[] {
  if (node.content !== null) return [node];
  return node.children.flatMap(allLeaves);
}

function firstLeaf(node: LayoutNode): LayoutNode {
  const leaves = allLeaves(node);
  if (leaves.length === 0) throw new Error("no leaves");
  return leaves[0];
}

function firstText(node: LayoutNode): string {
  const leaf = firstLeaf(node);
  if (leaf.content?.kind !== "text") throw new Error("first leaf is not text");
  return leaf.content.value;
}

// ─── document structure ───────────────────────────────────────────────────────

describe("document structure", () => {
  it("empty document produces root container with no children", () => {
    const result = document_ast_to_layout(doc(), theme);
    expect(result.content).toBeNull();
    expect(result.children).toHaveLength(0);
    expect(result.width).toEqual({ kind: "fill" });
    expect(result.ext["block"]).toMatchObject({ display: "block" });
  });

  it("two paragraphs → two block children", () => {
    const result = document_ast_to_layout(
      doc(para(text("A")), para(text("B"))),
      theme
    );
    expect(result.children).toHaveLength(2);
  });
});

// ─── default theme ────────────────────────────────────────────────────────────

describe("default theme", () => {
  it("returns a theme with body and code fonts", () => {
    const t = document_default_theme();
    expect(t.bodyFont.size).toBe(16);
    expect(t.codeFont.family).toBe("monospace");
    expect(t.headingScale).toHaveLength(6);
  });

  it("h1 scale is the largest heading scale", () => {
    const t = document_default_theme();
    expect(t.headingScale[0]).toBeGreaterThan(t.headingScale[5]);
  });
});

// ─── heading ─────────────────────────────────────────────────────────────────

describe("heading", () => {
  it("heading block has display:block", () => {
    const result = document_ast_to_layout(doc(heading(1, text("Hello"))), theme);
    const h = result.children[0];
    expect(h.ext["block"]).toMatchObject({ display: "block" });
  });

  it("h1 text has bold font", () => {
    const result = document_ast_to_layout(doc(heading(1, text("Title"))), theme);
    const leaf = firstLeaf(result);
    expect(leaf.content?.kind).toBe("text");
    if (leaf.content?.kind === "text") {
      expect(leaf.content.font.weight).toBe(700);
    }
  });

  it("h1 font is larger than body font", () => {
    const result = document_ast_to_layout(
      doc(heading(1, text("Big")), para(text("small"))),
      theme
    );
    const h1Leaf = allLeaves(result.children[0])[0];
    const paraLeaf = allLeaves(result.children[1])[0];
    if (h1Leaf.content?.kind === "text" && paraLeaf.content?.kind === "text") {
      expect(h1Leaf.content.font.size).toBeGreaterThan(paraLeaf.content.font.size);
    }
  });

  it("h6 font is smaller than h1 font", () => {
    const result = document_ast_to_layout(
      doc(heading(1, text("H1")), heading(6, text("H6"))),
      theme
    );
    const h1Leaf = allLeaves(result.children[0])[0];
    const h6Leaf = allLeaves(result.children[1])[0];
    if (h1Leaf.content?.kind === "text" && h6Leaf.content?.kind === "text") {
      expect(h1Leaf.content.font.size).toBeGreaterThan(h6Leaf.content.font.size);
    }
  });

  it("heading has margin-bottom", () => {
    const result = document_ast_to_layout(doc(heading(2, text("H2"))), theme);
    const h = result.children[0];
    expect(h.margin?.bottom).toBeGreaterThan(0);
  });

  it("heading with inline em still produces inline child leaves", () => {
    const result = document_ast_to_layout(
      doc(heading(2, text("Hello "), em(text("world")))),
      theme
    );
    const leaves = allLeaves(result.children[0]);
    expect(leaves.length).toBeGreaterThanOrEqual(2);
  });

  it("heading uses heading color", () => {
    const result = document_ast_to_layout(doc(heading(1, text("Hi"))), theme);
    const leaf = firstLeaf(result);
    if (leaf.content?.kind === "text") {
      expect(leaf.content.color).toEqual(theme.colors.heading);
    }
  });
});

// ─── paragraph ───────────────────────────────────────────────────────────────

describe("paragraph", () => {
  it("paragraph block has display:block", () => {
    const result = document_ast_to_layout(doc(para(text("Hi"))), theme);
    expect(result.children[0].ext["block"]).toMatchObject({ display: "block" });
  });

  it("paragraph text leaf has display:inline", () => {
    const result = document_ast_to_layout(doc(para(text("Hi"))), theme);
    const leaf = firstLeaf(result);
    expect(leaf.ext["block"]).toMatchObject({ display: "inline" });
  });

  it("paragraph uses body color", () => {
    const result = document_ast_to_layout(doc(para(text("Hi"))), theme);
    const leaf = firstLeaf(result);
    if (leaf.content?.kind === "text") {
      expect(leaf.content.color).toEqual(theme.colors.text);
    }
  });

  it("paragraph has margin-bottom", () => {
    const result = document_ast_to_layout(doc(para(text("p"))), theme);
    expect(result.children[0].margin?.bottom).toBe(theme.paragraphSpacing);
  });

  it("paragraph width is fill", () => {
    const result = document_ast_to_layout(doc(para(text("p"))), theme);
    expect(result.children[0].width).toEqual({ kind: "fill" });
  });
});

// ─── inline nodes ─────────────────────────────────────────────────────────────

describe("inline nodes", () => {
  it("emphasis produces italic font", () => {
    const result = document_ast_to_layout(doc(para(em(text("hi")))), theme);
    const leaf = firstLeaf(result);
    if (leaf.content?.kind === "text") {
      expect(leaf.content.font.italic).toBe(true);
    }
  });

  it("strong produces bold font", () => {
    const result = document_ast_to_layout(doc(para(strong(text("bold")))), theme);
    const leaf = firstLeaf(result);
    if (leaf.content?.kind === "text") {
      expect(leaf.content.font.weight).toBe(700);
    }
  });

  it("nested strong > em produces bold italic font", () => {
    const result = document_ast_to_layout(doc(para(strong(em(text("both"))))), theme);
    const leaf = firstLeaf(result);
    if (leaf.content?.kind === "text") {
      expect(leaf.content.font.weight).toBe(700);
      expect(leaf.content.font.italic).toBe(true);
    }
  });

  it("strikethrough text is tagged", () => {
    const result = document_ast_to_layout(doc(para(strike(text("del")))), theme);
    const leaf = firstLeaf(result);
    expect(leaf.ext["strikethrough"]).toBe(true);
  });

  it("code_span uses code font and code color", () => {
    const result = document_ast_to_layout(doc(para(codeSpan("x"))), theme);
    const leaf = firstLeaf(result);
    if (leaf.content?.kind === "text") {
      expect(leaf.content.font.family).toBe("monospace");
      expect(leaf.content.color).toEqual(theme.colors.code);
    }
  });

  it("link text has link color and ext.link destination", () => {
    const result = document_ast_to_layout(
      doc(para(link("https://example.com", null, text("click")))),
      theme
    );
    const leaf = firstLeaf(result);
    if (leaf.content?.kind === "text") {
      expect(leaf.content.color).toEqual(theme.colors.link);
    }
    expect(leaf.ext["link"]).toBe("https://example.com");
  });

  it("URL autolink has link color and ext.link", () => {
    const result = document_ast_to_layout(
      doc(para(autolink("https://example.com", false))),
      theme
    );
    const leaf = firstLeaf(result);
    if (leaf.content?.kind === "text") {
      expect(leaf.content.color).toEqual(theme.colors.link);
    }
    expect(leaf.ext["link"]).toBe("https://example.com");
  });

  it("email autolink prepends mailto: in ext.link", () => {
    const result = document_ast_to_layout(
      doc(para(autolink("user@example.com", true))),
      theme
    );
    const leaf = firstLeaf(result);
    expect(leaf.ext["link"]).toBe("mailto:user@example.com");
  });

  it("inline image produces image leaf with display:inline", () => {
    const result = document_ast_to_layout(
      doc(para(image("cat.png", "a cat"))),
      theme
    );
    const leaf = firstLeaf(result);
    expect(leaf.content?.kind).toBe("image");
    if (leaf.content?.kind === "image") {
      expect(leaf.content.src).toBe("cat.png");
    }
    expect(leaf.ext["block"]).toMatchObject({ display: "inline" });
    expect(leaf.ext["imageAlt"]).toBe("a cat");
  });

  it("soft_break produces a space text node", () => {
    const result = document_ast_to_layout(
      doc(para(text("a"), softBreak(), text("b"))),
      theme
    );
    const leaves = allLeaves(result.children[0]);
    const spaceLeaf = leaves.find(
      (n) => n.content?.kind === "text" && n.content.value === " "
    );
    expect(spaceLeaf).toBeDefined();
  });

  it("hard_break produces a newline text node", () => {
    const result = document_ast_to_layout(
      doc(para(text("a"), hardBreak(), text("b"))),
      theme
    );
    const leaves = allLeaves(result.children[0]);
    const nlLeaf = leaves.find(
      (n) => n.content?.kind === "text" && n.content.value === "\n"
    );
    expect(nlLeaf).toBeDefined();
  });

  it("raw_inline is skipped", () => {
    const result = document_ast_to_layout(
      doc(para(text("a"), rawInline("html", "<em>x</em>"), text("b"))),
      theme
    );
    const leaves = allLeaves(result.children[0]);
    // Only 2 leaves: "a" and "b"
    expect(leaves).toHaveLength(2);
  });

  it("empty text node is skipped", () => {
    const result = document_ast_to_layout(doc(para(text(""))), theme);
    const leaves = allLeaves(result.children[0]);
    expect(leaves).toHaveLength(0);
  });
});

// ─── code_block ───────────────────────────────────────────────────────────────

describe("code_block", () => {
  it("produces a leaf with monospace font", () => {
    const result = document_ast_to_layout(doc(codeBlock("x = 1\n")), theme);
    const leaf = firstLeaf(result);
    if (leaf.content?.kind === "text") {
      expect(leaf.content.font.family).toBe("monospace");
    }
  });

  it("has code background color in paint ext", () => {
    const result = document_ast_to_layout(doc(codeBlock("x\n")), theme);
    const node = result.children[0];
    const paint = node.ext["paint"] as { backgroundColor: Color } | undefined;
    expect(paint?.backgroundColor).toEqual(theme.colors.codeBackground);
  });

  it("has whiteSpace pre in block ext", () => {
    const result = document_ast_to_layout(doc(codeBlock("x\n")), theme);
    const node = result.children[0];
    expect(node.ext["block"]).toMatchObject({ whiteSpace: "pre" });
  });

  it("has padding", () => {
    const result = document_ast_to_layout(doc(codeBlock("x\n")), theme);
    const node = result.children[0];
    expect(node.padding?.top).toBe(theme.codePadding);
  });

  it("preserves code text verbatim", () => {
    const code = "function hello() {\n  return 42;\n}\n";
    const result = document_ast_to_layout(doc(codeBlock(code)), theme);
    const leaf = firstLeaf(result);
    if (leaf.content?.kind === "text") {
      expect(leaf.content.value).toBe(code);
    }
  });
});

// ─── blockquote ───────────────────────────────────────────────────────────────

describe("blockquote", () => {
  it("has blockquote ext tag", () => {
    const result = document_ast_to_layout(doc(blockquote(para(text("q")))), theme);
    const bq = result.children[0];
    expect(bq.ext["blockquote"]).toBe(true);
  });

  it("has left border color in paint ext", () => {
    const result = document_ast_to_layout(doc(blockquote(para(text("q")))), theme);
    const bq = result.children[0];
    const paint = bq.ext["paint"] as { borderColor: Color } | undefined;
    expect(paint?.borderColor).toEqual(theme.colors.blockquoteBorder);
  });

  it("has background color in paint ext", () => {
    const result = document_ast_to_layout(doc(blockquote(para(text("q")))), theme);
    const bq = result.children[0];
    const paint = bq.ext["paint"] as { backgroundColor: Color } | undefined;
    expect(paint?.backgroundColor).toEqual(theme.colors.blockquoteBackground);
  });

  it("has left padding for indent", () => {
    const result = document_ast_to_layout(doc(blockquote(para(text("q")))), theme);
    const bq = result.children[0];
    expect(bq.padding?.left).toBe(theme.blockquoteIndent);
  });

  it("blockquote content is nested inside", () => {
    const result = document_ast_to_layout(
      doc(blockquote(para(text("inner")))),
      theme
    );
    const bq = result.children[0];
    expect(bq.children).toHaveLength(1); // the paragraph
    expect(firstText(bq)).toBe("inner");
  });

  it("nested blockquote works", () => {
    const result = document_ast_to_layout(
      doc(blockquote(blockquote(para(text("deep"))))),
      theme
    );
    const outer = result.children[0];
    const inner = outer.children[0];
    expect(inner.ext["blockquote"]).toBe(true);
  });
});

// ─── thematic_break ──────────────────────────────────────────────────────────

describe("thematic_break", () => {
  it("produces a block container with height 1", () => {
    const result = document_ast_to_layout(doc(tbreak()), theme);
    const hr = result.children[0];
    expect(hr.height).toEqual({ kind: "fixed", value: 1 });
  });

  it("has hr background color in paint ext", () => {
    const result = document_ast_to_layout(doc(tbreak()), theme);
    const hr = result.children[0];
    const paint = hr.ext["paint"] as { backgroundColor: Color } | undefined;
    expect(paint?.backgroundColor).toEqual(theme.colors.hrColor);
  });

  it("has vertical margin", () => {
    const result = document_ast_to_layout(doc(tbreak()), theme);
    const hr = result.children[0];
    expect(hr.margin?.top).toBeGreaterThan(0);
    expect(hr.margin?.bottom).toBeGreaterThan(0);
  });

  it("hr has fill width", () => {
    const result = document_ast_to_layout(doc(tbreak()), theme);
    expect(result.children[0].width).toEqual({ kind: "fill" });
  });
});

// ─── raw_block ────────────────────────────────────────────────────────────────

describe("raw_block", () => {
  it("html raw_block is skipped", () => {
    const result = document_ast_to_layout(
      doc(rawBlock("html", "<div>raw</div>")),
      theme
    );
    expect(result.children).toHaveLength(0);
  });

  it("latex raw_block is skipped", () => {
    const result = document_ast_to_layout(
      doc(rawBlock("latex", "\\textbf{x}")),
      theme
    );
    expect(result.children).toHaveLength(0);
  });
});

// ─── bullet list ─────────────────────────────────────────────────────────────

describe("bullet list", () => {
  it("list container has left padding", () => {
    const result = document_ast_to_layout(
      doc(bulletList(listItem(para(text("a"))))),
      theme
    );
    const list = result.children[0];
    expect(list.padding?.left).toBe(theme.listIndent);
  });

  it("produces one row per list item", () => {
    const result = document_ast_to_layout(
      doc(bulletList(listItem(para(text("a"))), listItem(para(text("b"))))),
      theme
    );
    const list = result.children[0];
    expect(list.children).toHaveLength(2);
  });

  it("each row has a bullet text child", () => {
    const result = document_ast_to_layout(
      doc(bulletList(listItem(para(text("x"))))),
      theme
    );
    const row = result.children[0].children[0];
    // bullet is first child, body is second
    const bullet = row.children[0];
    expect(bullet.content?.kind).toBe("text");
    if (bullet.content?.kind === "text") {
      expect(bullet.content.value).toBe("• ");
    }
  });

  it("bullet text child has fixed width", () => {
    const result = document_ast_to_layout(
      doc(bulletList(listItem(para(text("x"))))),
      theme
    );
    const row = result.children[0].children[0];
    const bullet = row.children[0];
    expect(bullet.width).toEqual({ kind: "fixed", value: 24 });
  });

  it("item row uses flex direction row", () => {
    const result = document_ast_to_layout(
      doc(bulletList(listItem(para(text("a"))))),
      theme
    );
    const row = result.children[0].children[0];
    expect(row.ext["flex"]).toMatchObject({ direction: "row" });
  });

  it("item body has block children", () => {
    const result = document_ast_to_layout(
      doc(bulletList(listItem(para(text("body"))))),
      theme
    );
    const row = result.children[0].children[0];
    const body = row.children[1];
    expect(body.content).toBeNull();
    expect(firstText(body)).toBe("body");
  });
});

// ─── ordered list ─────────────────────────────────────────────────────────────

describe("ordered list", () => {
  it("first item bullet is '1. '", () => {
    const result = document_ast_to_layout(
      doc(orderedList(1, listItem(para(text("first"))))),
      theme
    );
    const row = result.children[0].children[0];
    const bullet = row.children[0];
    if (bullet.content?.kind === "text") {
      expect(bullet.content.value).toBe("1. ");
    }
  });

  it("start=3 second item bullet is '4. '", () => {
    const result = document_ast_to_layout(
      doc(orderedList(3,
        listItem(para(text("a"))),
        listItem(para(text("b")))
      )),
      theme
    );
    const list = result.children[0];
    const row2 = list.children[1];
    const bullet2 = row2.children[0];
    if (bullet2.content?.kind === "text") {
      expect(bullet2.content.value).toBe("4. ");
    }
  });
});

// ─── task list ────────────────────────────────────────────────────────────────

describe("task list", () => {
  it("unchecked task bullet is '☐ '", () => {
    const result = document_ast_to_layout(
      doc(bulletList(taskItem(false, para(text("todo"))))),
      theme
    );
    const row = result.children[0].children[0];
    const bullet = row.children[0];
    if (bullet.content?.kind === "text") {
      expect(bullet.content.value).toBe("☐ ");
    }
  });

  it("checked task bullet is '☑ '", () => {
    const result = document_ast_to_layout(
      doc(bulletList(taskItem(true, para(text("done"))))),
      theme
    );
    const row = result.children[0].children[0];
    const bullet = row.children[0];
    if (bullet.content?.kind === "text") {
      expect(bullet.content.value).toBe("☑ ");
    }
  });
});

// ─── table ────────────────────────────────────────────────────────────────────

describe("table", () => {
  it("produces a node with grid ext templateColumns", () => {
    const result = document_ast_to_layout(
      doc(tableNode(
        ["left", "right"],
        tableRow(true, tableCell(text("A")), tableCell(text("B"))),
        tableRow(false, tableCell(text("1")), tableCell(text("2"))),
      )),
      theme
    );
    const table = result.children[0];
    const grid = table.ext["grid"] as { templateColumns: string } | undefined;
    expect(grid?.templateColumns).toBe("1fr 1fr");
  });

  it("header cells have background color in paint", () => {
    const result = document_ast_to_layout(
      doc(tableNode(
        ["left"],
        tableRow(true, tableCell(text("H"))),
      )),
      theme
    );
    const table = result.children[0];
    const headerCell = table.children[0];
    const paint = headerCell.ext["paint"] as { backgroundColor: Color } | undefined;
    expect(paint?.backgroundColor).toEqual(theme.colors.tableHeaderBackground);
  });

  it("body cells have no paint ext", () => {
    const result = document_ast_to_layout(
      doc(tableNode(
        ["left"],
        tableRow(false, tableCell(text("B"))),
      )),
      theme
    );
    const table = result.children[0];
    const bodyCell = table.children[0];
    // paint ext may be undefined for body cells
    const paint = bodyCell.ext["paint"];
    expect(paint).toBeUndefined();
  });

  it("cells have explicit grid columnStart and rowStart", () => {
    const result = document_ast_to_layout(
      doc(tableNode(
        ["left", "right"],
        tableRow(true, tableCell(text("A")), tableCell(text("B"))),
      )),
      theme
    );
    const table = result.children[0];
    const cell1 = table.children[0];
    const cell2 = table.children[1];
    expect(cell1.ext["grid"]).toMatchObject({ columnStart: 1, rowStart: 1 });
    expect(cell2.ext["grid"]).toMatchObject({ columnStart: 2, rowStart: 1 });
  });

  it("header text is bold", () => {
    const result = document_ast_to_layout(
      doc(tableNode(
        ["left"],
        tableRow(true, tableCell(text("Head"))),
      )),
      theme
    );
    const table = result.children[0];
    const headerCell = table.children[0];
    const leaf = firstLeaf(headerCell);
    if (leaf.content?.kind === "text") {
      expect(leaf.content.font.weight).toBe(700);
    }
  });

  it("cells have padding", () => {
    const result = document_ast_to_layout(
      doc(tableNode(
        ["left"],
        tableRow(false, tableCell(text("x"))),
      )),
      theme
    );
    const table = result.children[0];
    const cell = table.children[0];
    expect(cell.padding?.top).toBe(theme.tableCellPadding);
  });

  it("table has bottom margin", () => {
    const result = document_ast_to_layout(
      doc(tableNode(["left"], tableRow(false, tableCell(text("x"))))),
      theme
    );
    expect(result.children[0].margin?.bottom).toBe(theme.paragraphSpacing);
  });
});

// ─── nested document ──────────────────────────────────────────────────────────

describe("nested document node", () => {
  it("nested document flattens its children", () => {
    const inner: BlockNode = {
      type: "document",
      children: [para(text("inner"))],
    };
    const result = document_ast_to_layout(doc(inner), theme);
    // The inner document is flattened — paragraph is a direct child
    expect(result.children).toHaveLength(1);
  });
});

// ─── list_item outside list ───────────────────────────────────────────────────

describe("list_item outside list", () => {
  it("standalone list_item is converted as item index 0", () => {
    const standalone: BlockNode = { type: "list_item", children: [para(text("x"))] };
    const result = document_ast_to_layout(doc(standalone), theme);
    expect(result.children).toHaveLength(1);
  });

  it("standalone task_item is converted as item index 0", () => {
    const standalone: BlockNode = { type: "task_item", checked: false, children: [para(text("y"))] };
    const result = document_ast_to_layout(doc(standalone), theme);
    expect(result.children).toHaveLength(1);
  });
});

// ─── table_row / table_cell outside table ─────────────────────────────────────

describe("table_row / table_cell outside table", () => {
  it("standalone table_row is skipped", () => {
    const row: BlockNode = { type: "table_row", isHeader: false, children: [] };
    const result = document_ast_to_layout(doc(row), theme);
    expect(result.children).toHaveLength(0);
  });

  it("standalone table_cell is skipped", () => {
    const cell: BlockNode = { type: "table_cell", children: [] };
    const result = document_ast_to_layout(doc(cell), theme);
    expect(result.children).toHaveLength(0);
  });
});

// ─── mixed content ────────────────────────────────────────────────────────────

describe("mixed content", () => {
  it("paragraph → heading → code_block produces 3 children", () => {
    const result = document_ast_to_layout(
      doc(
        para(text("intro")),
        heading(2, text("section")),
        codeBlock("x = 1\n", "python")
      ),
      theme
    );
    expect(result.children).toHaveLength(3);
  });

  it("list inside blockquote is rendered", () => {
    const result = document_ast_to_layout(
      doc(blockquote(bulletList(listItem(para(text("item")))))),
      theme
    );
    const bq = result.children[0];
    // blockquote → list → item row → bullet + body → paragraph → "item"
    // firstText returns "• " (bullet), so check all leaves for "item"
    const leaves = allLeaves(bq);
    const textValues = leaves
      .filter((n) => n.content?.kind === "text")
      .map((n) => (n.content as { kind: "text"; value: string }).value);
    expect(textValues).toContain("item");
  });
});
