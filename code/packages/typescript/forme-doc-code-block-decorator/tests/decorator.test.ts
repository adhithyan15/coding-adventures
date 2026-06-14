/**
 * decorator.test.ts — AST walker integration tests.
 */

import { describe, it, expect } from "vitest";
import { decorateCodeBlocks } from "../src/index.js";
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
import type { DecoratedCodeBlockNode } from "../src/types.js";

// ─────────────────────────────────────────────────────────────────────
// AST builder helpers
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
  return { type: "list", ordered, start: ordered ? 1 : null, tight: true, children: items };
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

// ─────────────────────────────────────────────────────────────────────
// Top-level behaviour
// ─────────────────────────────────────────────────────────────────────

describe("decorateCodeBlocks — basic", () => {
  it("empty document → empty document", () => {
    const result = decorateCodeBlocks(doc());
    expect(result.children).toEqual([]);
  });
  it("no code blocks → non-code children pass through by reference", () => {
    const p = paragraph(text("hello"));
    const result = decorateCodeBlocks(doc(p));
    expect(result.children).toHaveLength(1);
    expect(result.children[0]).toBe(p);
  });
  it("single code block: gets all four decoration fields", () => {
    const result = decorateCodeBlocks(doc(codeBlock("ts", "x = 1\n")));
    const b = result.children[0] as DecoratedCodeBlockNode;
    expect(b.type).toBe("code_block");
    expect(b.language).toBe("ts");
    expect(b.value).toBe("x = 1\n");
    expect(b.copyable).toBe(true);
    expect(b.languageLabel).toBe("TypeScript");
    expect(b.filename).toBeNull();
    expect(b.lineNumbers).toBe(false);
  });
  it("lineNumbers: true is propagated", () => {
    const result = decorateCodeBlocks(doc(codeBlock("py", "x = 1\n")), { lineNumbers: true });
    const b = result.children[0] as DecoratedCodeBlockNode;
    expect(b.lineNumbers).toBe(true);
  });
  it("multiple code blocks all decorated", () => {
    const result = decorateCodeBlocks(
      doc(codeBlock("ts", "a"), codeBlock("py", "b"), codeBlock(null, "c")),
    );
    expect(result.children).toHaveLength(3);
    expect((result.children[0] as DecoratedCodeBlockNode).languageLabel).toBe("TypeScript");
    expect((result.children[1] as DecoratedCodeBlockNode).languageLabel).toBe("Python");
    expect((result.children[2] as DecoratedCodeBlockNode).languageLabel).toBeNull();
  });
});

describe("decorateCodeBlocks — filename extraction integration", () => {
  it("filename hint extracted and value stripped", () => {
    const result = decorateCodeBlocks(
      doc(codeBlock("ts", "// file: src/auth.ts\nexport function login() {}\n")),
    );
    const b = result.children[0] as DecoratedCodeBlockNode;
    expect(b.filename).toBe("src/auth.ts");
    expect(b.value).toBe("export function login() {}\n");
  });
  it("language unaffected by filename extraction", () => {
    const result = decorateCodeBlocks(
      doc(codeBlock("python", "# file: app.py\nprint('hi')\n")),
    );
    const b = result.children[0] as DecoratedCodeBlockNode;
    expect(b.language).toBe("python");
    expect(b.languageLabel).toBe("Python");
    expect(b.filename).toBe("app.py");
  });
  it("no filename hint → filename is null, value untouched", () => {
    const result = decorateCodeBlocks(doc(codeBlock("ts", "export function f() {}\n")));
    const b = result.children[0] as DecoratedCodeBlockNode;
    expect(b.filename).toBeNull();
    expect(b.value).toBe("export function f() {}\n");
  });
});

// ─────────────────────────────────────────────────────────────────────
// Recursion into containers
// ─────────────────────────────────────────────────────────────────────

describe("decorateCodeBlocks — container recursion", () => {
  it("blockquote-wrapped code block is decorated", () => {
    const result = decorateCodeBlocks(doc(blockquote(codeBlock("ts", "x\n"))));
    const bq = result.children[0] as BlockquoteNode;
    expect(bq.type).toBe("blockquote");
    const b = bq.children[0] as DecoratedCodeBlockNode;
    expect(b.copyable).toBe(true);
    expect(b.languageLabel).toBe("TypeScript");
  });
  it("list-item-wrapped code block is decorated", () => {
    const result = decorateCodeBlocks(
      doc(list(false, item(codeBlock("py", "x = 1\n")))),
    );
    const lst = result.children[0] as ListNode;
    const li = lst.children[0] as ListItemNode;
    const b = li.children[0] as DecoratedCodeBlockNode;
    expect(b.languageLabel).toBe("Python");
  });
  it("task-item-wrapped code block is decorated", () => {
    const result = decorateCodeBlocks(
      doc(list(false, taskItem(false, codeBlock("rs", "fn main() {}\n")))),
    );
    const lst = result.children[0] as ListNode;
    const ti = lst.children[0] as TaskItemNode;
    expect(ti.checked).toBe(false);
    const b = ti.children[0] as DecoratedCodeBlockNode;
    expect(b.languageLabel).toBe("Rust");
  });
  it("ordered list preserves ordered/start/tight fields", () => {
    const result = decorateCodeBlocks(
      doc({
        type: "list",
        ordered: true,
        start: 5,
        tight: false,
        children: [item(codeBlock("ts", "x"))],
      } as ListNode),
    );
    const lst = result.children[0] as ListNode;
    expect(lst.ordered).toBe(true);
    expect(lst.start).toBe(5);
    expect(lst.tight).toBe(false);
  });
  it("blockquote → list → list-item → code-block deeply nested", () => {
    const result = decorateCodeBlocks(
      doc(blockquote(list(false, item(codeBlock("go", "package main\n"))))),
    );
    const bq = result.children[0] as BlockquoteNode;
    const lst = bq.children[0] as ListNode;
    const li = lst.children[0] as ListItemNode;
    const b = li.children[0] as DecoratedCodeBlockNode;
    expect(b.languageLabel).toBe("Go");
  });
  it("non-code children inside containers pass through by reference", () => {
    const p = paragraph(text("hi"));
    const result = decorateCodeBlocks(doc(blockquote(p, codeBlock("ts", "x"))));
    const bq = result.children[0] as BlockquoteNode;
    expect(bq.children[0]).toBe(p);
  });
  it("list_item appearing as a direct document child (defensive — BlockNode union allows it)", () => {
    // commonmark-parser only ever produces list_item INSIDE a list, but
    // document-ast's BlockNode union includes it directly, so a
    // hand-crafted AST or an alternative parser could legitimately
    // emit one at the top level.  Defensive case: still recurses and
    // decorates any nested code blocks.
    const result = decorateCodeBlocks(doc(item(codeBlock("ts", "x"))));
    const li = result.children[0] as ListItemNode;
    expect(li.type).toBe("list_item");
    const b = li.children[0] as DecoratedCodeBlockNode;
    expect(b.languageLabel).toBe("TypeScript");
  });
  it("task_item appearing as a direct document child (defensive)", () => {
    const result = decorateCodeBlocks(doc(taskItem(true, codeBlock("py", "x"))));
    const ti = result.children[0] as TaskItemNode;
    expect(ti.type).toBe("task_item");
    expect(ti.checked).toBe(true);
    const b = ti.children[0] as DecoratedCodeBlockNode;
    expect(b.languageLabel).toBe("Python");
  });
});

// ─────────────────────────────────────────────────────────────────────
// Pass-through types (headings, thematic break, raw block, tables)
// ─────────────────────────────────────────────────────────────────────

describe("decorateCodeBlocks — non-container, non-code blocks pass through", () => {
  it("headings", () => {
    const h: BlockNode = { type: "heading", level: 1, children: [text("Title")] };
    const result = decorateCodeBlocks(doc(h));
    expect(result.children[0]).toBe(h);
  });
  it("thematic break", () => {
    const hr: BlockNode = { type: "thematic_break" };
    const result = decorateCodeBlocks(doc(hr));
    expect(result.children[0]).toBe(hr);
  });
  it("raw block", () => {
    const rb: BlockNode = { type: "raw_block", format: "html", value: "<custom/>" };
    const result = decorateCodeBlocks(doc(rb));
    expect(result.children[0]).toBe(rb);
  });
  it("table", () => {
    const t: BlockNode = {
      type: "table",
      alignments: ["left", "right"],
      children: [],
    };
    const result = decorateCodeBlocks(doc(t));
    expect(result.children[0]).toBe(t);
  });
});

// ─────────────────────────────────────────────────────────────────────
// Immutability + determinism
// ─────────────────────────────────────────────────────────────────────

describe("decorateCodeBlocks — immutability", () => {
  it("does not mutate input document", () => {
    const input = doc(codeBlock("ts", "// file: f.ts\nx\n"));
    const snapshot = JSON.stringify(input);
    decorateCodeBlocks(input);
    expect(JSON.stringify(input)).toBe(snapshot);
  });
  it("does not mutate input code-block (original .value unchanged after filename strip)", () => {
    const cb = codeBlock("ts", "// file: f.ts\nx\n");
    decorateCodeBlocks(doc(cb));
    expect(cb.value).toBe("// file: f.ts\nx\n");
  });
  it("produces a NEW DocumentNode object", () => {
    const input = doc();
    const result = decorateCodeBlocks(input);
    expect(result).not.toBe(input);
  });
});

describe("decorateCodeBlocks — determinism", () => {
  it("same input → identical output", () => {
    const input = doc(
      codeBlock("ts", "// file: a.ts\na\n"),
      blockquote(codeBlock("py", "# file: b.py\nb\n")),
      codeBlock(null, "no language\n"),
    );
    const a = JSON.stringify(decorateCodeBlocks(input, { lineNumbers: true }));
    const b = JSON.stringify(decorateCodeBlocks(input, { lineNumbers: true }));
    expect(a).toBe(b);
  });
});

// ─────────────────────────────────────────────────────────────────────
// Realistic doc
// ─────────────────────────────────────────────────────────────────────

describe("decorateCodeBlocks — realistic doc", () => {
  it("interleaved headings, paragraphs, code blocks, nested containers", () => {
    const result = decorateCodeBlocks(
      doc(
        { type: "heading", level: 1, children: [text("Setup")] },
        paragraph(text("Install:")),
        codeBlock("bash", "npm install\n"),
        { type: "heading", level: 2, children: [text("Example")] },
        codeBlock("ts", "// file: example.ts\nconst x: number = 1;\n"),
        blockquote(
          paragraph(text("Tip:")),
          codeBlock(null, "tip code\n"),
        ),
        list(false, item(codeBlock("rs", "fn main() {}\n"))),
      ),
      { lineNumbers: true },
    );

    // Top-level code blocks
    const top1 = result.children[2] as DecoratedCodeBlockNode;
    expect(top1.languageLabel).toBe("Bash");
    expect(top1.filename).toBeNull();
    expect(top1.lineNumbers).toBe(true);

    const top2 = result.children[4] as DecoratedCodeBlockNode;
    expect(top2.languageLabel).toBe("TypeScript");
    expect(top2.filename).toBe("example.ts");
    expect(top2.value).toBe("const x: number = 1;\n");

    // Blockquote-nested
    const bq = result.children[5] as BlockquoteNode;
    const bqCode = bq.children[1] as DecoratedCodeBlockNode;
    expect(bqCode.copyable).toBe(true);
    expect(bqCode.languageLabel).toBeNull();
    expect(bqCode.lineNumbers).toBe(true);

    // List-nested
    const lst = result.children[6] as ListNode;
    const liCode = (lst.children[0] as ListItemNode).children[0] as DecoratedCodeBlockNode;
    expect(liCode.languageLabel).toBe("Rust");
  });
});
