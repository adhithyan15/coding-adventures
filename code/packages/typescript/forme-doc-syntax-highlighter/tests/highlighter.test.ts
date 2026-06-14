/**
 * highlighter.test.ts — stub walker tests.
 *
 * The tests check the v0 CONTRACT: every code block gets a
 * `highlighted` field whose spans tile the original `value`
 * byte-for-byte.  When v1 swaps in a real engine, these contract
 * tests stay valid; only the more-than-one-span expectations need
 * to be added.
 */

import { describe, it, expect } from "vitest";
import { highlightCodeBlocks, highlight } from "../src/index.js";
import type {
  DocumentNode,
  BlockNode,
  CodeBlockNode,
  ParagraphNode,
  BlockquoteNode,
  ListNode,
  ListItemNode,
  TaskItemNode,
  InlineNode,
} from "@coding-adventures/document-ast";
import type { HighlightedCodeBlockNode, HighlightSpan } from "../src/types.js";

// ─────────────────────────────────────────────────────────────────────
// AST builders
// ─────────────────────────────────────────────────────────────────────

function text(value: string): InlineNode {
  return { type: "text", value };
}
function paragraph(...children: InlineNode[]): ParagraphNode {
  return { type: "paragraph", children };
}
function codeBlock(language: string | null, value: string): CodeBlockNode {
  return { type: "code_block", language, value };
}
function blockquote(...children: BlockNode[]): BlockquoteNode {
  return { type: "blockquote", children };
}
function list(ordered: boolean, ...items: (ListItemNode | TaskItemNode)[]): ListNode {
  return {
    type: "list",
    ordered,
    start: ordered ? 1 : null,
    tight: true,
    children: items,
  };
}
function item(...children: BlockNode[]): ListItemNode {
  return { type: "list_item", children };
}
function taskItem(checked: boolean, ...children: BlockNode[]): TaskItemNode {
  return { type: "task_item", checked, children };
}
function doc(...children: BlockNode[]): DocumentNode {
  return { type: "document", children };
}

/** v0 invariant: concatenating span values reconstructs the original. */
function reconstruct(spans: readonly HighlightSpan[]): string {
  return spans.map((s) => s.value).join("");
}

// ─────────────────────────────────────────────────────────────────────
// `highlight` stand-alone helper
// ─────────────────────────────────────────────────────────────────────

describe("highlight — v0 stub", () => {
  it("returns one plain span for non-empty input", () => {
    expect(highlight("const x = 1;\n")).toEqual([{ type: "plain", value: "const x = 1;\n" }]);
  });
  it("returns [] for empty input", () => {
    expect(highlight("")).toEqual([]);
  });
  it("ignores language parameter (v0 stub)", () => {
    expect(highlight("x", "ts")).toEqual([{ type: "plain", value: "x" }]);
    expect(highlight("x", null)).toEqual([{ type: "plain", value: "x" }]);
    expect(highlight("x", "cobol")).toEqual([{ type: "plain", value: "x" }]);
  });
  it("preserves whitespace and newlines exactly", () => {
    const code = "  a\n\nb\t\n";
    expect(reconstruct(highlight(code))).toBe(code);
  });
});

// ─────────────────────────────────────────────────────────────────────
// `highlightCodeBlocks` walker
// ─────────────────────────────────────────────────────────────────────

describe("highlightCodeBlocks — top-level", () => {
  it("empty document", () => {
    const r = highlightCodeBlocks(doc());
    expect(r.children).toEqual([]);
  });
  it("no code blocks: non-code children pass through by reference", () => {
    const p = paragraph(text("hi"));
    const r = highlightCodeBlocks(doc(p));
    expect(r.children).toHaveLength(1);
    expect(r.children[0]).toBe(p);
  });
  it("single code block gets a `highlighted` field", () => {
    const r = highlightCodeBlocks(doc(codeBlock("ts", "x = 1\n")));
    const b = r.children[0] as HighlightedCodeBlockNode;
    expect(b.type).toBe("code_block");
    expect(b.language).toBe("ts");
    expect(b.value).toBe("x = 1\n");
    expect(b.highlighted).toEqual([{ type: "plain", value: "x = 1\n" }]);
  });
  it("multiple code blocks all get highlighted", () => {
    const r = highlightCodeBlocks(doc(
      codeBlock("ts", "a"),
      codeBlock("py", "b"),
      codeBlock(null, "c"),
    ));
    expect((r.children[0] as HighlightedCodeBlockNode).highlighted).toEqual([{ type: "plain", value: "a" }]);
    expect((r.children[1] as HighlightedCodeBlockNode).highlighted).toEqual([{ type: "plain", value: "b" }]);
    expect((r.children[2] as HighlightedCodeBlockNode).highlighted).toEqual([{ type: "plain", value: "c" }]);
  });
  it("empty code block gets highlighted: []", () => {
    const r = highlightCodeBlocks(doc(codeBlock("ts", "")));
    const b = r.children[0] as HighlightedCodeBlockNode;
    expect(b.highlighted).toEqual([]);
  });
  it("options.theme is accepted but ignored", () => {
    const r = highlightCodeBlocks(doc(codeBlock("ts", "x")), { theme: "github-light" });
    const b = r.children[0] as HighlightedCodeBlockNode;
    expect(b.highlighted).toEqual([{ type: "plain", value: "x" }]);
  });
});

describe("highlightCodeBlocks — tiling invariant", () => {
  it("v0 invariant: span values concatenate to original value (any input)", () => {
    const samples = [
      "",
      "x",
      "const x = 1;\n",
      "  weird\n\nwhitespace\t\n",
      "🚀 emoji 中文 ünïcödé\n",
      "//\n".repeat(100),
    ];
    for (const code of samples) {
      const r = highlightCodeBlocks(doc(codeBlock("ts", code)));
      const b = r.children[0] as HighlightedCodeBlockNode;
      expect(reconstruct(b.highlighted)).toBe(code);
    }
  });
});

// ─────────────────────────────────────────────────────────────────────
// Container recursion
// ─────────────────────────────────────────────────────────────────────

describe("highlightCodeBlocks — container recursion", () => {
  it("blockquote-wrapped code block", () => {
    const r = highlightCodeBlocks(doc(blockquote(codeBlock("ts", "x\n"))));
    const bq = r.children[0] as BlockquoteNode;
    expect(bq.type).toBe("blockquote");
    const b = bq.children[0] as HighlightedCodeBlockNode;
    expect(b.highlighted).toEqual([{ type: "plain", value: "x\n" }]);
  });
  it("list-item-wrapped", () => {
    const r = highlightCodeBlocks(doc(list(false, item(codeBlock("py", "x")))));
    const lst = r.children[0] as ListNode;
    const li = lst.children[0] as ListItemNode;
    const b = li.children[0] as HighlightedCodeBlockNode;
    expect(b.highlighted).toEqual([{ type: "plain", value: "x" }]);
  });
  it("task-item-wrapped, ordered list metadata preserved", () => {
    const r = highlightCodeBlocks(doc({
      type: "list",
      ordered: true,
      start: 7,
      tight: false,
      children: [taskItem(true, codeBlock("rs", "fn"))],
    } as ListNode));
    const lst = r.children[0] as ListNode;
    expect(lst.ordered).toBe(true);
    expect(lst.start).toBe(7);
    expect(lst.tight).toBe(false);
    const ti = lst.children[0] as TaskItemNode;
    expect(ti.checked).toBe(true);
    const b = ti.children[0] as HighlightedCodeBlockNode;
    expect(b.highlighted).toEqual([{ type: "plain", value: "fn" }]);
  });
  it("deeply nested blockquote → list → list-item → code", () => {
    const r = highlightCodeBlocks(
      doc(blockquote(list(false, item(codeBlock("go", "package main\n"))))),
    );
    const bq = r.children[0] as BlockquoteNode;
    const lst = bq.children[0] as ListNode;
    const li = lst.children[0] as ListItemNode;
    const b = li.children[0] as HighlightedCodeBlockNode;
    expect(b.highlighted).toEqual([{ type: "plain", value: "package main\n" }]);
  });
  it("defensive: list_item at top level", () => {
    const r = highlightCodeBlocks(doc(item(codeBlock("ts", "x"))));
    const li = r.children[0] as ListItemNode;
    expect(li.type).toBe("list_item");
    const b = li.children[0] as HighlightedCodeBlockNode;
    expect(b.highlighted).toEqual([{ type: "plain", value: "x" }]);
  });
  it("defensive: task_item at top level", () => {
    const r = highlightCodeBlocks(doc(taskItem(false, codeBlock("py", "x"))));
    const ti = r.children[0] as TaskItemNode;
    expect(ti.type).toBe("task_item");
    expect(ti.checked).toBe(false);
    const b = ti.children[0] as HighlightedCodeBlockNode;
    expect(b.highlighted).toEqual([{ type: "plain", value: "x" }]);
  });
  it("non-code children inside containers pass through by reference", () => {
    const p = paragraph(text("hi"));
    const r = highlightCodeBlocks(doc(blockquote(p, codeBlock("ts", "x"))));
    const bq = r.children[0] as BlockquoteNode;
    expect(bq.children[0]).toBe(p);
  });
});

// ─────────────────────────────────────────────────────────────────────
// Pass-through types
// ─────────────────────────────────────────────────────────────────────

describe("highlightCodeBlocks — non-container, non-code blocks pass through", () => {
  it("heading", () => {
    const h: BlockNode = { type: "heading", level: 1, children: [text("Title")] };
    const r = highlightCodeBlocks(doc(h));
    expect(r.children[0]).toBe(h);
  });
  it("thematic break", () => {
    const hr: BlockNode = { type: "thematic_break" };
    const r = highlightCodeBlocks(doc(hr));
    expect(r.children[0]).toBe(hr);
  });
  it("raw block", () => {
    const rb: BlockNode = { type: "raw_block", format: "html", value: "<x/>" };
    const r = highlightCodeBlocks(doc(rb));
    expect(r.children[0]).toBe(rb);
  });
  it("table", () => {
    const t: BlockNode = { type: "table", alignments: ["left"], children: [] };
    const r = highlightCodeBlocks(doc(t));
    expect(r.children[0]).toBe(t);
  });
});

// ─────────────────────────────────────────────────────────────────────
// Composability: decorator → highlighter preserves decoration fields
// ─────────────────────────────────────────────────────────────────────

describe("highlightCodeBlocks — composability with decorator", () => {
  it("decorator fields ride through (object spread preserves them)", () => {
    // Simulate a DecoratedCodeBlockNode by attaching arbitrary fields.
    // The type system says CodeBlockNode, but the runtime spread
    // copies every own enumerable property.
    const decorated = {
      type: "code_block" as const,
      language: "ts",
      value: "x\n",
      copyable: true,
      languageLabel: "TypeScript",
      filename: "src/auth.ts",
      lineNumbers: true,
    };
    const r = highlightCodeBlocks(doc(decorated));
    const b = r.children[0] as HighlightedCodeBlockNode & {
      copyable: boolean;
      languageLabel: string;
      filename: string;
      lineNumbers: boolean;
    };
    expect(b.copyable).toBe(true);
    expect(b.languageLabel).toBe("TypeScript");
    expect(b.filename).toBe("src/auth.ts");
    expect(b.lineNumbers).toBe(true);
    expect(b.highlighted).toEqual([{ type: "plain", value: "x\n" }]);
  });
});

// ─────────────────────────────────────────────────────────────────────
// Immutability + determinism
// ─────────────────────────────────────────────────────────────────────

describe("highlightCodeBlocks — immutability", () => {
  it("does not mutate input document", () => {
    const input = doc(codeBlock("ts", "x"), blockquote(codeBlock("py", "y")));
    const snapshot = JSON.stringify(input);
    highlightCodeBlocks(input);
    expect(JSON.stringify(input)).toBe(snapshot);
  });
  it("produces a NEW DocumentNode object", () => {
    const input = doc();
    const result = highlightCodeBlocks(input);
    expect(result).not.toBe(input);
  });
});

describe("highlightCodeBlocks — determinism", () => {
  it("same input → identical output", () => {
    const input = doc(
      codeBlock("ts", "a"),
      blockquote(codeBlock("py", "b")),
      codeBlock(null, "c"),
    );
    expect(JSON.stringify(highlightCodeBlocks(input))).toBe(
      JSON.stringify(highlightCodeBlocks(input)),
    );
  });
});
