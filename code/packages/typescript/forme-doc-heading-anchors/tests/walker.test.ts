/**
 * walker.test.ts — AST walker tests.
 *
 * We build DocumentNode literals by hand here rather than going
 * through the full commonmark-parser pipeline.  That keeps these
 * tests fast, deterministic, and free of upstream-parser
 * coupling — if the parser's emphasis-node shape changes, these
 * tests still verify our walker's contract against the AST type.
 */

import { describe, it, expect } from "vitest";
import { generateHeadingAnchors } from "../src/index.js";
import type {
  DocumentNode,
  BlockNode,
  HeadingNode,
  InlineNode,
  ParagraphNode,
} from "@coding-adventures/document-ast";
import type { AnchoredHeadingNode } from "../src/types.js";

// ─────────────────────────────────────────────────────────────────────
// Tiny AST-builder helpers — keep tests readable.
// ─────────────────────────────────────────────────────────────────────

function text(value: string): InlineNode {
  return { type: "text", value };
}
function emphasis(...children: InlineNode[]): InlineNode {
  return { type: "emphasis", children };
}
function strong(...children: InlineNode[]): InlineNode {
  return { type: "strong", children };
}
function strikethrough(...children: InlineNode[]): InlineNode {
  return { type: "strikethrough", children };
}
function codeSpan(value: string): InlineNode {
  return { type: "code_span", value };
}
function link(destination: string, ...children: InlineNode[]): InlineNode {
  return { type: "link", destination, title: null, children };
}
function image(alt: string, destination = "img.png"): InlineNode {
  return { type: "image", destination, title: null, alt };
}
function autolink(destination: string, isEmail = false): InlineNode {
  return { type: "autolink", destination, isEmail };
}
function rawInline(value: string, format = "html"): InlineNode {
  return { type: "raw_inline", format, value };
}
function softBreak(): InlineNode {
  return { type: "soft_break" };
}
function hardBreak(): InlineNode {
  return { type: "hard_break" };
}
function heading(level: 1 | 2 | 3 | 4 | 5 | 6, ...children: InlineNode[]): HeadingNode {
  return { type: "heading", level, children };
}
function paragraph(...children: InlineNode[]): ParagraphNode {
  return { type: "paragraph", children };
}
function doc(...children: BlockNode[]): DocumentNode {
  return { type: "document", children };
}

// ─────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────

describe("generateHeadingAnchors — basic", () => {
  it("empty document returns empty document and empty anchors", () => {
    const result = generateHeadingAnchors(doc());
    expect(result.document.children).toEqual([]);
    expect(result.anchors).toEqual([]);
  });
  it("document with no headings passes blocks through by reference", () => {
    const p = paragraph(text("hello"));
    const input = doc(p);
    const result = generateHeadingAnchors(input);
    expect(result.document.children).toHaveLength(1);
    // Pass-through by reference — same object identity.
    expect(result.document.children[0]).toBe(p);
    expect(result.anchors).toEqual([]);
  });
  it("single heading gets a slug id", () => {
    const result = generateHeadingAnchors(doc(heading(1, text("Getting Started"))));
    const h = result.document.children[0] as AnchoredHeadingNode;
    expect(h.type).toBe("heading");
    expect(h.id).toBe("getting-started");
    expect(h.level).toBe(1);
    expect(result.anchors).toHaveLength(1);
    expect(result.anchors[0]).toEqual({
      text: "Getting Started",
      id: "getting-started",
      level: 1,
      heading: h,
    });
  });
  it("multiple headings in document order", () => {
    const result = generateHeadingAnchors(
      doc(
        heading(1, text("Intro")),
        paragraph(text("body")),
        heading(2, text("Details")),
        heading(3, text("Subsection")),
      ),
    );
    expect(result.anchors.map((a) => a.id)).toEqual(["intro", "details", "subsection"]);
    expect(result.anchors.map((a) => a.level)).toEqual([1, 2, 3]);
  });
});

describe("generateHeadingAnchors — inline node plain-text extraction", () => {
  it("emphasis flattened", () => {
    const result = generateHeadingAnchors(
      doc(heading(2, text("Hello "), emphasis(text("world")))),
    );
    expect((result.document.children[0] as AnchoredHeadingNode).id).toBe("hello-world");
    expect(result.anchors[0].text).toBe("Hello world");
  });
  it("strong flattened", () => {
    const result = generateHeadingAnchors(
      doc(heading(2, strong(text("Bold")), text(" header"))),
    );
    expect((result.document.children[0] as AnchoredHeadingNode).id).toBe("bold-header");
  });
  it("strikethrough flattened", () => {
    const result = generateHeadingAnchors(
      doc(heading(2, strikethrough(text("deleted")), text(" text"))),
    );
    expect((result.document.children[0] as AnchoredHeadingNode).id).toBe("deleted-text");
  });
  it("code spans contribute their value", () => {
    const result = generateHeadingAnchors(
      doc(heading(2, codeSpan("foo()"), text(" rules"))),
    );
    expect((result.document.children[0] as AnchoredHeadingNode).id).toBe("foo-rules");
  });
  it("links contribute child text but not destination", () => {
    const result = generateHeadingAnchors(
      doc(heading(2, link("https://example.com/x", text("Click here")), text(" please"))),
    );
    expect((result.document.children[0] as AnchoredHeadingNode).id).toBe("click-here-please");
  });
  it("images contribute alt text", () => {
    const result = generateHeadingAnchors(
      doc(heading(2, image("Logo"), text(" caption"))),
    );
    expect((result.document.children[0] as AnchoredHeadingNode).id).toBe("logo-caption");
  });
  it("autolinks emit the destination", () => {
    const result = generateHeadingAnchors(
      doc(heading(2, text("See "), autolink("https://x.test"))),
    );
    // After slugify: "See https://x.test" → "see-httpsxtest" (dots/colons/slashes stripped)
    expect((result.document.children[0] as AnchoredHeadingNode).id).toBe("see-httpsxtest");
  });
  it("raw_inline skipped entirely", () => {
    const result = generateHeadingAnchors(
      doc(heading(2, text("alpha "), rawInline("<sub>x</sub>"), text(" beta"))),
    );
    expect((result.document.children[0] as AnchoredHeadingNode).id).toBe("alpha--beta");
  });
  it("soft_break becomes a space", () => {
    const result = generateHeadingAnchors(
      doc(heading(2, text("one"), softBreak(), text("two"))),
    );
    expect((result.document.children[0] as AnchoredHeadingNode).id).toBe("one-two");
  });
  it("hard_break becomes a space", () => {
    const result = generateHeadingAnchors(
      doc(heading(2, text("one"), hardBreak(), text("two"))),
    );
    expect((result.document.children[0] as AnchoredHeadingNode).id).toBe("one-two");
  });
  it("nested markup recurses", () => {
    const result = generateHeadingAnchors(
      doc(heading(2, emphasis(strong(text("Deep"))), text(" nesting"))),
    );
    expect((result.document.children[0] as AnchoredHeadingNode).id).toBe("deep-nesting");
  });
});

describe("generateHeadingAnchors — collision suffixing", () => {
  it("two identical headings: first bare, second gets -1", () => {
    const result = generateHeadingAnchors(
      doc(heading(2, text("Setup")), heading(2, text("Setup"))),
    );
    expect(result.anchors.map((a) => a.id)).toEqual(["setup", "setup-1"]);
  });
  it("three identical headings: setup, setup-1, setup-2", () => {
    const result = generateHeadingAnchors(
      doc(
        heading(1, text("Setup")),
        heading(2, text("Setup")),
        heading(3, text("Setup")),
      ),
    );
    expect(result.anchors.map((a) => a.id)).toEqual(["setup", "setup-1", "setup-2"]);
  });
  it("collisions are case-insensitive (slug is lowercased)", () => {
    const result = generateHeadingAnchors(
      doc(heading(1, text("Setup")), heading(2, text("SETUP")), heading(2, text("setup"))),
    );
    expect(result.anchors.map((a) => a.id)).toEqual(["setup", "setup-1", "setup-2"]);
  });
  it("empty slugs also collide on the empty string", () => {
    const result = generateHeadingAnchors(
      doc(heading(1, text("")), heading(2, text("!@#$"))),
    );
    expect(result.anchors.map((a) => a.id)).toEqual(["", "-1"]);
  });
  it("non-colliding headings keep their bare slugs", () => {
    const result = generateHeadingAnchors(
      doc(heading(1, text("Alpha")), heading(2, text("Beta")), heading(3, text("Gamma"))),
    );
    expect(result.anchors.map((a) => a.id)).toEqual(["alpha", "beta", "gamma"]);
  });
});

describe("generateHeadingAnchors — prototype-pollution defence", () => {
  it("heading literally titled '__proto__' does not pollute Object.prototype", () => {
    const result = generateHeadingAnchors(
      doc(heading(1, text("__proto__")), heading(2, text("__proto__"))),
    );
    // First: id="__proto__", second: id="__proto__-1".  No throw, no
    // weirdness — we use Object.create(null) for the counter map so
    // `counts.__proto__` lookups go straight through, not via the
    // inherited Object.prototype getter.
    expect(result.anchors.map((a) => a.id)).toEqual(["__proto__", "__proto__-1"]);
    // And our innocent bystander object's prototype is intact:
    expect(({}).hasOwnProperty).toBe(Object.prototype.hasOwnProperty);
  });
  it("heading titled 'constructor' is handled normally", () => {
    const result = generateHeadingAnchors(
      doc(heading(1, text("constructor")), heading(2, text("constructor"))),
    );
    expect(result.anchors.map((a) => a.id)).toEqual(["constructor", "constructor-1"]);
  });
  it("heading titled 'toString' (would-be Object.prototype.toString clash)", () => {
    const result = generateHeadingAnchors(
      doc(heading(1, text("toString")), heading(2, text("toString"))),
    );
    expect(result.anchors.map((a) => a.id)).toEqual(["tostring", "tostring-1"]);
  });
});

describe("generateHeadingAnchors — immutability", () => {
  it("does not mutate the input DocumentNode", () => {
    const input = doc(heading(1, text("Hello")));
    const snapshot = JSON.stringify(input);
    generateHeadingAnchors(input);
    expect(JSON.stringify(input)).toBe(snapshot);
  });
  it("does not mutate input HeadingNode children references", () => {
    const headingChildren = [text("Hello")];
    const h = heading(1, ...headingChildren);
    generateHeadingAnchors(doc(h));
    // Original heading still has no `id` field.
    expect((h as unknown as { id?: string }).id).toBeUndefined();
  });
  it("produces a NEW DocumentNode object, even with no headings", () => {
    const input = doc(paragraph(text("body")));
    const result = generateHeadingAnchors(input);
    expect(result.document).not.toBe(input);
  });
  it("anchors list is in source order even with collisions", () => {
    const result = generateHeadingAnchors(
      doc(
        heading(1, text("A")),
        heading(2, text("B")),
        heading(3, text("A")),
        heading(4, text("B")),
        heading(5, text("A")),
      ),
    );
    expect(result.anchors.map((a) => a.id)).toEqual(["a", "b", "a-1", "b-1", "a-2"]);
  });
});

describe("generateHeadingAnchors — determinism", () => {
  it("same input → identical output", () => {
    const input = doc(
      heading(1, text("Hello")),
      heading(2, text("Hello")),
      heading(3, emphasis(text("World"))),
    );
    const a = generateHeadingAnchors(input);
    const b = generateHeadingAnchors(input);
    expect(JSON.stringify(a.anchors.map((x) => ({ id: x.id, text: x.text, level: x.level }))))
      .toBe(JSON.stringify(b.anchors.map((x) => ({ id: x.id, text: x.text, level: x.level }))));
  });
});
