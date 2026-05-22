/**
 * extractor.test.ts — TOC extractor tests.
 *
 * Two layers: `buildTocTree` (pure list-to-tree, easy unit tests
 * against handcrafted HeadingAnchor[]) and `extractToc` (the full
 * doc → tree pipeline, integration with heading-anchors).
 */

import { describe, it, expect } from "vitest";
import { buildTocTree, extractToc } from "../src/index.js";
import type { TocEntry } from "../src/types.js";
import type {
  HeadingAnchor,
  AnchoredHeadingNode,
} from "@coding-adventures/forme-doc-heading-anchors";
import type {
  DocumentNode,
  BlockNode,
  HeadingNode,
  InlineNode,
  ParagraphNode,
} from "@coding-adventures/document-ast";

// ─────────────────────────────────────────────────────────────────────
// Tiny builders — mirror heading-anchors' test helpers.
// ─────────────────────────────────────────────────────────────────────

function text(value: string): InlineNode {
  return { type: "text", value };
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
function anchor(text: string, id: string, level: 1 | 2 | 3 | 4 | 5 | 6): HeadingAnchor {
  return {
    text,
    id,
    level,
    // The tests below only ever inspect `text`, `id`, `level` — but
    // HeadingAnchor requires a `heading` reference, so we synthesise
    // a minimal AnchoredHeadingNode.
    heading: { type: "heading", level, children: [{ type: "text", value: text }], id } as AnchoredHeadingNode,
  };
}

/**
 * Project a TOC tree to just the structural skeleton — text + level +
 * recursive children.  Drops `id` and any future fields so test
 * expectations stay focused on the nesting algorithm.
 */
function shape(entries: readonly TocEntry[]): unknown {
  return entries.map((e) => ({
    text: e.text,
    level: e.level,
    children: shape(e.children),
  }));
}

// ─────────────────────────────────────────────────────────────────────
// buildTocTree — pure list-to-tree algorithm
// ─────────────────────────────────────────────────────────────────────

describe("buildTocTree — degenerate inputs", () => {
  it("empty input → empty tree", () => {
    expect(buildTocTree([])).toEqual([]);
  });
  it("single heading → single top-level entry", () => {
    const result = buildTocTree([anchor("Hello", "hello", 1)]);
    expect(shape(result)).toEqual([{ text: "Hello", level: 1, children: [] }]);
  });
});

describe("buildTocTree — straightforward nesting", () => {
  it("h1 → h2 → h3 nests three deep", () => {
    const result = buildTocTree([
      anchor("Intro", "intro", 1),
      anchor("Setup", "setup", 2),
      anchor("Prereq", "prereq", 3),
    ]);
    expect(shape(result)).toEqual([
      {
        text: "Intro",
        level: 1,
        children: [
          { text: "Setup", level: 2, children: [{ text: "Prereq", level: 3, children: [] }] },
        ],
      },
    ]);
  });
  it("two h2 siblings under one h1", () => {
    const result = buildTocTree([
      anchor("Intro", "intro", 1),
      anchor("A", "a", 2),
      anchor("B", "b", 2),
    ]);
    expect(shape(result)).toEqual([
      {
        text: "Intro",
        level: 1,
        children: [
          { text: "A", level: 2, children: [] },
          { text: "B", level: 2, children: [] },
        ],
      },
    ]);
  });
  it("multiple h1s become multiple top-level entries", () => {
    const result = buildTocTree([
      anchor("Part 1", "part-1", 1),
      anchor("Part 2", "part-2", 1),
      anchor("Part 3", "part-3", 1),
    ]);
    expect(shape(result)).toEqual([
      { text: "Part 1", level: 1, children: [] },
      { text: "Part 2", level: 1, children: [] },
      { text: "Part 3", level: 1, children: [] },
    ]);
  });
});

describe("buildTocTree — level jumps", () => {
  it("h1 → h3 (skipping h2) nests h3 directly under h1", () => {
    const result = buildTocTree([
      anchor("Top", "top", 1),
      anchor("Deep", "deep", 3),
    ]);
    expect(shape(result)).toEqual([
      {
        text: "Top",
        level: 1,
        children: [{ text: "Deep", level: 3, children: [] }],
      },
    ]);
  });
  it("h2 → h4 → h6 skipping odd levels still produces a chain", () => {
    const result = buildTocTree([
      anchor("A", "a", 2),
      anchor("B", "b", 4),
      anchor("C", "c", 6),
    ]);
    expect(shape(result)).toEqual([
      {
        text: "A",
        level: 2,
        children: [
          {
            text: "B",
            level: 4,
            children: [{ text: "C", level: 6, children: [] }],
          },
        ],
      },
    ]);
  });
  it("first heading at h3 produces a top-level h3 (no synthetic h1/h2 wrappers)", () => {
    const result = buildTocTree([
      anchor("Deep first", "deep-first", 3),
      anchor("Then top", "then-top", 1),
    ]);
    expect(shape(result)).toEqual([
      { text: "Deep first", level: 3, children: [] },
      { text: "Then top", level: 1, children: [] },
    ]);
  });
});

describe("buildTocTree — level drops", () => {
  it("h3 → h1 pops the stack all the way to root", () => {
    const result = buildTocTree([
      anchor("Top A", "top-a", 1),
      anchor("Mid", "mid", 2),
      anchor("Deep", "deep", 3),
      anchor("Top B", "top-b", 1),
    ]);
    expect(shape(result)).toEqual([
      {
        text: "Top A",
        level: 1,
        children: [
          {
            text: "Mid",
            level: 2,
            children: [{ text: "Deep", level: 3, children: [] }],
          },
        ],
      },
      { text: "Top B", level: 1, children: [] },
    ]);
  });
  it("h6 → h2 produces correct sibling at the right depth", () => {
    const result = buildTocTree([
      anchor("Section", "section", 2),
      anchor("Very deep", "very-deep", 6),
      anchor("Next section", "next-section", 2),
    ]);
    expect(shape(result)).toEqual([
      {
        text: "Section",
        level: 2,
        children: [{ text: "Very deep", level: 6, children: [] }],
      },
      { text: "Next section", level: 2, children: [] },
    ]);
  });
});

describe("buildTocTree — id preservation", () => {
  it("preserves the id field exactly (no re-slugging)", () => {
    const result = buildTocTree([
      anchor("Hello World", "hello-world", 1),
      anchor("Hello World", "hello-world-1", 2),
    ]);
    expect(result[0].id).toBe("hello-world");
    expect(result[0].children[0].id).toBe("hello-world-1");
  });
});

describe("buildTocTree — null-prototype output", () => {
  it("entries have no prototype chain (defensive for `for...in` consumers)", () => {
    const result = buildTocTree([anchor("Hi", "hi", 1)]);
    expect(Object.getPrototypeOf(result[0])).toBeNull();
  });
});

// ─────────────────────────────────────────────────────────────────────
// extractToc — full pipeline with heading-anchors
// ─────────────────────────────────────────────────────────────────────

describe("extractToc — integration with heading-anchors", () => {
  it("empty document → empty toc, empty anchors", () => {
    const result = extractToc(doc());
    expect(result.toc).toEqual([]);
    expect(result.anchors).toEqual([]);
    expect(result.document.children).toEqual([]);
  });
  it("document with only a paragraph (no headings) → empty toc, paragraph preserved", () => {
    const p = paragraph(text("body only"));
    const result = extractToc(doc(p));
    expect(result.toc).toEqual([]);
    expect(result.document.children).toHaveLength(1);
    expect(result.document.children[0]).toBe(p);
  });
  it("realistic doc: headings + paragraphs interleaved", () => {
    const result = extractToc(
      doc(
        heading(1, text("Introduction")),
        paragraph(text("intro body")),
        heading(2, text("Setup")),
        heading(3, text("Prerequisites")),
        heading(3, text("Install")),
        paragraph(text("install body")),
        heading(2, text("Quick start")),
        heading(1, text("Reference")),
        heading(2, text("API")),
      ),
    );
    expect(shape(result.toc)).toEqual([
      {
        text: "Introduction",
        level: 1,
        children: [
          {
            text: "Setup",
            level: 2,
            children: [
              { text: "Prerequisites", level: 3, children: [] },
              { text: "Install", level: 3, children: [] },
            ],
          },
          { text: "Quick start", level: 2, children: [] },
        ],
      },
      {
        text: "Reference",
        level: 1,
        children: [{ text: "API", level: 2, children: [] }],
      },
    ]);
  });
  it("slug ids match between toc tree and anchored document", () => {
    const result = extractToc(
      doc(heading(1, text("Getting Started")), heading(2, text("Install Now"))),
    );
    // Toc ids:
    expect(result.toc[0].id).toBe("getting-started");
    expect(result.toc[0].children[0].id).toBe("install-now");
    // Anchored AST ids (same):
    expect((result.document.children[0] as AnchoredHeadingNode).id).toBe("getting-started");
    expect((result.document.children[1] as AnchoredHeadingNode).id).toBe("install-now");
    // Flat anchors list ids (also same):
    expect(result.anchors.map((a) => a.id)).toEqual(["getting-started", "install-now"]);
  });
  it("collision suffixes propagate consistently across all three projections", () => {
    const result = extractToc(
      doc(heading(2, text("Setup")), heading(2, text("Setup")), heading(2, text("Setup"))),
    );
    expect(result.anchors.map((a) => a.id)).toEqual(["setup", "setup-1", "setup-2"]);
    expect(result.toc.map((e) => e.id)).toEqual(["setup", "setup-1", "setup-2"]);
  });
  it("does not mutate input document", () => {
    const input = doc(heading(1, text("Hi")), paragraph(text("body")));
    const snapshot = JSON.stringify(input);
    extractToc(input);
    expect(JSON.stringify(input)).toBe(snapshot);
  });
});

describe("extractToc — large input scalability", () => {
  it("10,000 headings — no stack overflow, O(N) time", () => {
    // 10k h2 headings.  The while-pop loop runs at most O(1) per
    // heading on average for a flat sequence; total work O(N).
    const children: BlockNode[] = Array.from({ length: 10000 }, (_, i) =>
      heading(2, text(`Section ${i}`)),
    );
    const result = extractToc(doc(...children));
    expect(result.toc).toHaveLength(10000);
    expect(result.toc[0].id).toBe("section-0");
    expect(result.toc[9999].id).toBe("section-9999");
  });
  it("alternating deep/shallow — exercises stack push/pop heavily", () => {
    // [h1, h6, h1, h6, …] — each iteration pushes then pops 5 levels.
    const children: BlockNode[] = [];
    for (let i = 0; i < 500; i++) {
      children.push(heading(1, text(`Top ${i}`)));
      children.push(heading(6, text(`Deep ${i}`)));
    }
    const result = extractToc(doc(...children));
    // 500 top-level h1 entries; each has a single h6 child.
    expect(result.toc).toHaveLength(500);
    expect(result.toc[0].children).toHaveLength(1);
    expect(result.toc[0].children[0].level).toBe(6);
    expect(result.toc[499].children[0].id).toBe("deep-499");
  });
});

describe("extractToc — determinism", () => {
  it("same input → identical output structure", () => {
    const input = doc(
      heading(1, text("A")),
      heading(2, text("B")),
      heading(2, text("C")),
      heading(1, text("D")),
    );
    const a = extractToc(input);
    const b = extractToc(input);
    expect(shape(a.toc)).toEqual(shape(b.toc));
    expect(a.anchors.map((x) => x.id)).toEqual(b.anchors.map((x) => x.id));
  });
});
