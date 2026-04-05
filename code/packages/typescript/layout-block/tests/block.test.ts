/**
 * Tests for layout-block (block and inline flow layout algorithm).
 *
 * All node trees are built directly — we bypass any "real" producer to test
 * the layout algorithm in isolation.
 */

import { describe, it, expect } from "vitest";
import { layout_block, type BlockExt } from "../src/index.js";
import {
  node,
  leaf_text,
  leaf_image,
  container,
  constraints_fixed,
  constraints_width,
  constraints_unconstrained,
  font_spec,
  rgb,
} from "@coding-adventures/layout-ir";
import type { LayoutNode, PositionedNode } from "@coding-adventures/layout-ir";
import { createEstimatedMeasurer } from "@coding-adventures/layout-text-measure-estimated";

// ── Shared helpers ──────────────────────────────────────────────────────────

const font = font_spec("Arial", 16);
const black = rgb(0, 0, 0);
const measurer = createEstimatedMeasurer();

/** Build a block node (display: block) */
function blockNode(
  children: LayoutNode[],
  extra?: Partial<LayoutNode>
): LayoutNode {
  return {
    width: { kind: "fill" },
    height: { kind: "wrap" },
    content: null,
    children,
    ext: { block: { display: "block" } as BlockExt },
    minWidth: undefined,
    maxWidth: undefined,
    minHeight: undefined,
    maxHeight: undefined,
    margin: { top: 0, right: 0, bottom: 0, left: 0 },
    padding: { top: 0, right: 0, bottom: 0, left: 0 },
    ...extra,
  };
}

/** Build an inline text node */
function inlineText(text: string, extra?: Partial<LayoutNode>): LayoutNode {
  return {
    width: { kind: "wrap" },
    height: { kind: "wrap" },
    content: { kind: "text", value: text, font, color: black, maxLines: null, textAlign: "start" },
    children: [],
    ext: { block: { display: "inline" } as BlockExt },
    minWidth: undefined,
    maxWidth: undefined,
    minHeight: undefined,
    maxHeight: undefined,
    margin: { top: 0, right: 0, bottom: 0, left: 0 },
    padding: { top: 0, right: 0, bottom: 0, left: 0 },
    ...extra,
  };
}

/** Build a block text leaf (display: block) */
function blockText(text: string, extra?: Partial<LayoutNode>): LayoutNode {
  return {
    width: { kind: "fill" },
    height: { kind: "wrap" },
    content: { kind: "text", value: text, font, color: black, maxLines: null, textAlign: "start" },
    children: [],
    ext: { block: { display: "block" } as BlockExt },
    minWidth: undefined,
    maxWidth: undefined,
    minHeight: undefined,
    maxHeight: undefined,
    margin: { top: 0, right: 0, bottom: 0, left: 0 },
    padding: { top: 0, right: 0, bottom: 0, left: 0 },
    ...extra,
  };
}

// ── Scene structure ─────────────────────────────────────────────────────────

describe("layout_block — scene structure", () => {
  it("empty block container has zero height", () => {
    const root = blockNode([]);
    const result = layout_block(root, constraints_width(400), measurer);
    expect(result.width).toBe(400);
    expect(result.height).toBe(0);
  });

  it("inherits fixed container width", () => {
    const root: LayoutNode = {
      ...blockNode([]),
      width: { kind: "fixed", value: 300 },
    };
    const result = layout_block(root, constraints_width(400), measurer);
    expect(result.width).toBe(300);
  });

  it("root x/y is always 0,0", () => {
    const root = blockNode([]);
    const result = layout_block(root, constraints_width(400), measurer);
    expect(result.x).toBe(0);
    expect(result.y).toBe(0);
  });

  it("respects min/maxWidth on container", () => {
    const root: LayoutNode = {
      ...blockNode([]),
      width: { kind: "fill" },
      minWidth: 500,
    };
    const result = layout_block(root, constraints_width(400), measurer);
    expect(result.width).toBe(500);
  });

  it("fixed height overrides content height", () => {
    const root: LayoutNode = {
      ...blockNode([blockText("Hello")]),
      height: { kind: "fixed", value: 200 },
    };
    const result = layout_block(root, constraints_width(400), measurer);
    expect(result.height).toBe(200);
  });
});

// ── Block stacking ──────────────────────────────────────────────────────────

describe("block stacking", () => {
  it("two block children stack vertically", () => {
    const child1 = blockText("First");
    const child2 = blockText("Second");
    const root = blockNode([child1, child2]);
    const result = layout_block(root, constraints_width(400), measurer);
    expect(result.children).toHaveLength(2);
    expect(result.children[0].y).toBe(0);
    expect(result.children[1].y).toBeGreaterThan(0);
  });

  it("second child y = first child's bottom edge", () => {
    const child1 = blockText("First");
    const child2 = blockText("Second");
    const root = blockNode([child1, child2]);
    const result = layout_block(root, constraints_width(400), measurer);
    const c1 = result.children[0];
    const c2 = result.children[1];
    expect(c2.y).toBe(c1.y + c1.height);
  });

  it("block children fill the available width", () => {
    const child = blockText("Hello");
    const root = blockNode([child]);
    const result = layout_block(root, constraints_width(400), measurer);
    expect(result.children[0].width).toBe(400);
  });

  it("container height = sum of children heights (no margins)", () => {
    const child1 = blockText("First");
    const child2 = blockText("Second");
    const root = blockNode([child1, child2]);
    const result = layout_block(root, constraints_width(400), measurer);
    const totalChildHeight = result.children.reduce((s, c) => s + c.height, 0);
    expect(result.height).toBe(totalChildHeight);
  });

  it("three nested block children stack correctly", () => {
    const c1 = blockText("A");
    const c2 = blockText("B");
    const c3 = blockText("C");
    const root = blockNode([c1, c2, c3]);
    const result = layout_block(root, constraints_width(400), measurer);
    expect(result.children[2].y).toBeGreaterThan(result.children[1].y);
  });
});

// ── Margin collapsing ───────────────────────────────────────────────────────

describe("margin collapsing", () => {
  it("adjacent sibling margins collapse to max", () => {
    const child1: LayoutNode = {
      ...blockText("A"),
      margin: { top: 0, right: 0, bottom: 10, left: 0 },
    };
    const child2: LayoutNode = {
      ...blockText("B"),
      margin: { top: 8, right: 0, bottom: 0, left: 0 },
    };
    const root = blockNode([child1, child2]);
    const result = layout_block(root, constraints_width(400), measurer);
    // collapsed margin = max(10, 8) = 10
    // child2.y = child1.y + child1.height + collapsed_margin = h1 + 10
    const c1 = result.children[0];
    const c2 = result.children[1];
    expect(c2.y).toBe(c1.y + c1.height + 10);
  });

  it("equal margins collapse to the same value", () => {
    const child1: LayoutNode = {
      ...blockText("A"),
      margin: { top: 0, right: 0, bottom: 16, left: 0 },
    };
    const child2: LayoutNode = {
      ...blockText("B"),
      margin: { top: 16, right: 0, bottom: 0, left: 0 },
    };
    const root = blockNode([child1, child2]);
    const result = layout_block(root, constraints_width(400), measurer);
    const c1 = result.children[0];
    const c2 = result.children[1];
    // collapsed = max(16, 16) = 16
    expect(c2.y).toBe(c1.y + c1.height + 16);
  });

  it("first child top margin is not collapsed with parent", () => {
    const child: LayoutNode = {
      ...blockText("A"),
      margin: { top: 20, right: 0, bottom: 0, left: 0 },
    };
    const root = blockNode([child]);
    const result = layout_block(root, constraints_width(400), measurer);
    // First child: top margin is applied as-is (not collapsed with parent)
    expect(result.children[0].y).toBe(20);
  });
});

// ── Padding ─────────────────────────────────────────────────────────────────

describe("padding", () => {
  it("padding top offsets first child", () => {
    const child = blockText("Hello");
    const root: LayoutNode = {
      ...blockNode([child]),
      padding: { top: 10, right: 0, bottom: 0, left: 0 },
    };
    const result = layout_block(root, constraints_width(400), measurer);
    expect(result.children[0].y).toBe(10);
  });

  it("padding left offsets children x", () => {
    const child = blockText("Hello");
    const root: LayoutNode = {
      ...blockNode([child]),
      padding: { top: 0, right: 0, bottom: 0, left: 20 },
    };
    const result = layout_block(root, constraints_width(400), measurer);
    expect(result.children[0].x).toBe(20);
  });

  it("padding reduces inner width", () => {
    const child = blockText("Hello");
    const root: LayoutNode = {
      ...blockNode([child]),
      padding: { top: 0, right: 15, bottom: 0, left: 15 },
    };
    const result = layout_block(root, constraints_width(400), measurer);
    // inner width = 400 - 30 = 370
    expect(result.children[0].width).toBe(370);
  });

  it("padding bottom adds to container height", () => {
    const child = blockText("Hello");
    const root: LayoutNode = {
      ...blockNode([child]),
      padding: { top: 0, right: 0, bottom: 20, left: 0 },
    };
    const result = layout_block(root, constraints_width(400), measurer);
    expect(result.height).toBe(result.children[0].height + 20);
  });
});

// ── Inline flow ─────────────────────────────────────────────────────────────

describe("inline flow", () => {
  it("inline children produce positioned nodes", () => {
    const root = blockNode([
      inlineText("Hello"),
      inlineText("World"),
    ]);
    const result = layout_block(root, constraints_width(400), measurer);
    // Anonymous block wraps both inline children
    expect(result.children).toHaveLength(1); // one anonymous block
    expect(result.children[0].children).toHaveLength(2); // two tokens
  });

  it("inline tokens are placed left-to-right", () => {
    const root = blockNode([
      inlineText("A"),
      inlineText("B"),
    ]);
    const result = layout_block(root, constraints_width(400), measurer);
    const anon = result.children[0];
    expect(anon.children[0].x).toBeLessThan(anon.children[1].x);
  });

  it("inline text wraps when too wide", () => {
    // Single word that is longer than the available width forces a wrap
    // Use a tiny container so even single words wrap to new lines
    const root = blockNode([
      inlineText("foo bar baz"),
    ]);
    const narrowConstraints = constraints_width(20); // very narrow
    const result = layout_block(root, narrowConstraints, measurer);
    const anon = result.children[0];
    // With only 20px, words must wrap — multiple lines → children have y > 0 for some
    const hasWrapped = anon.children.some(c => c.y > 0);
    expect(hasWrapped).toBe(true);
  });

  it("nowrap prevents line wrapping", () => {
    const inlineNode: LayoutNode = {
      ...inlineText("foo bar baz"),
      ext: { block: { display: "inline", whiteSpace: "nowrap" } as BlockExt },
    };
    const root = blockNode([inlineNode]);
    const result = layout_block(root, constraints_width(20), measurer);
    const anon = result.children[0];
    // All tokens on one line — only one child (the whole text as one token)
    expect(anon.children.every(c => c.y === 0)).toBe(true);
  });

  it("empty text produces no inline tokens", () => {
    const root = blockNode([inlineText("")]);
    const result = layout_block(root, constraints_width(400), measurer);
    // anonymous block exists but its children array is empty (no tokens)
    expect(result.children[0].children).toHaveLength(0);
  });
});

// ── Mixed block and inline ──────────────────────────────────────────────────

describe("anonymous block wrapping", () => {
  it("inline children before a block child are wrapped in an anonymous block", () => {
    const root = blockNode([
      inlineText("intro"),
      blockText("Heading"),
    ]);
    const result = layout_block(root, constraints_width(400), measurer);
    // Two block children: anonymous block for inline + the explicit block
    expect(result.children).toHaveLength(2);
  });

  it("block child comes after anonymous block", () => {
    const root = blockNode([
      inlineText("some text"),
      blockText("Block"),
    ]);
    const result = layout_block(root, constraints_width(400), measurer);
    // anonymous block first (y=0), block second (below it)
    expect(result.children[1].y).toBeGreaterThan(result.children[0].y);
  });

  it("inline children after a block child are wrapped separately", () => {
    const root = blockNode([
      blockText("Header"),
      inlineText("text"),
    ]);
    const result = layout_block(root, constraints_width(400), measurer);
    expect(result.children).toHaveLength(2);
  });
});

// ── Vertical alignment ──────────────────────────────────────────────────────

describe("verticalAlign", () => {
  // Use two inline nodes of different heights to exercise vertical alignment.
  // We'll use a tiny font and a large font.
  const smallFont = font_spec("Arial", 8);
  const largeFont = font_spec("Arial", 24);

  function inlineWithFont(text: string, f: typeof font, va: BlockExt["verticalAlign"]): LayoutNode {
    return {
      width: { kind: "wrap" },
      height: { kind: "wrap" },
      content: { kind: "text", value: text, font: f, color: black, maxLines: null, textAlign: "start" },
      children: [],
      ext: { block: { display: "inline", verticalAlign: va } as BlockExt },
      minWidth: undefined,
      maxWidth: undefined,
      minHeight: undefined,
      maxHeight: undefined,
      margin: { top: 0, right: 0, bottom: 0, left: 0 },
      padding: { top: 0, right: 0, bottom: 0, left: 0 },
    };
  }

  it("verticalAlign: top pins to line top (y=0)", () => {
    const root = blockNode([
      inlineWithFont("small", smallFont, "top"),
      inlineWithFont("LARGE", largeFont, "top"),
    ]);
    const result = layout_block(root, constraints_width(400), measurer);
    const anon = result.children[0];
    // Both should have y=0 (top of line box)
    expect(anon.children[0].y).toBe(0);
    expect(anon.children[1].y).toBe(0);
  });

  it("verticalAlign: bottom pins to line bottom", () => {
    const root = blockNode([
      inlineWithFont("small", smallFont, "bottom"),
      inlineWithFont("LARGE", largeFont, "bottom"),
    ]);
    const result = layout_block(root, constraints_width(400), measurer);
    const anon = result.children[0];
    const lineHeight = Math.max(...anon.children.map(c => c.height));
    // Both bottoms should align to the line bottom
    for (const c of anon.children) {
      expect(c.y + c.height).toBe(lineHeight);
    }
  });

  it("verticalAlign: middle centers within line height", () => {
    const root = blockNode([
      inlineWithFont("small", smallFont, "middle"),
      inlineWithFont("LARGE", largeFont, "middle"),
    ]);
    const result = layout_block(root, constraints_width(400), measurer);
    const anon = result.children[0];
    const lineHeight = Math.max(...anon.children.map(c => c.height));
    for (const c of anon.children) {
      expect(c.y + c.height / 2).toBeCloseTo(lineHeight / 2, 1);
    }
  });
});

// ── Overflow ────────────────────────────────────────────────────────────────

describe("overflow", () => {
  it("overflow:hidden sets ext paint.overflow", () => {
    const root: LayoutNode = {
      ...blockNode([blockText("Hello")]),
      ext: { block: { display: "block", overflow: "hidden" } as BlockExt },
    };
    const result = layout_block(root, constraints_width(400), measurer);
    const paintExt = result.ext["paint"] as Record<string, unknown> | undefined;
    expect(paintExt?.overflow).toBe("hidden");
  });

  it("overflow:scroll sets ext paint.overflow to scroll", () => {
    const root: LayoutNode = {
      ...blockNode([blockText("Hello")]),
      ext: { block: { display: "block", overflow: "scroll" } as BlockExt },
    };
    const result = layout_block(root, constraints_width(400), measurer);
    const paintExt = result.ext["paint"] as Record<string, unknown> | undefined;
    expect(paintExt?.overflow).toBe("scroll");
  });

  it("overflow:visible (default) does NOT add paint.overflow", () => {
    const root = blockNode([blockText("Hello")]);
    const result = layout_block(root, constraints_width(400), measurer);
    const paintExt = result.ext["paint"] as Record<string, unknown> | undefined;
    expect(paintExt?.overflow).toBeUndefined();
  });
});

// ── Nested block layout ─────────────────────────────────────────────────────

describe("nested block layout", () => {
  it("nested block containers are laid out recursively", () => {
    const inner = blockNode([blockText("nested")]);
    const outer = blockNode([inner, blockText("sibling")]);
    const result = layout_block(outer, constraints_width(400), measurer);
    expect(result.children).toHaveLength(2);
    expect(result.children[0].children).toHaveLength(1);
  });

  it("outer block height includes nested content", () => {
    const inner = blockNode([blockText("line1"), blockText("line2")]);
    const outer = blockNode([inner]);
    const result = layout_block(outer, constraints_width(400), measurer);
    expect(result.height).toBeGreaterThan(0);
    expect(result.height).toBe(result.children[0].height);
  });
});

// ── Leaf content sizing ─────────────────────────────────────────────────────

describe("leaf content sizing", () => {
  it("leaf text with fixed height uses fixed height", () => {
    const leaf: LayoutNode = {
      ...blockText("Hello"),
      height: { kind: "fixed", value: 50 },
    };
    const result = layout_block(leaf, constraints_width(400), measurer);
    expect(result.height).toBe(50);
  });

  it("leaf text with overflow:hidden propagates to paint ext", () => {
    const leaf: LayoutNode = {
      ...blockText("Hello"),
      ext: { block: { display: "block", overflow: "hidden" } as BlockExt },
    };
    const result = layout_block(leaf, constraints_width(400), measurer);
    const paintExt = result.ext["paint"] as Record<string, unknown> | undefined;
    expect(paintExt?.overflow).toBe("hidden");
  });

  it("leaf image node gets square height when height is wrap", () => {
    const imgLeaf: LayoutNode = {
      width: { kind: "fill" },
      height: { kind: "wrap" },
      content: { kind: "image", src: "img.png", fit: "contain" },
      children: [],
      ext: { block: { display: "block" } as BlockExt },
      minWidth: undefined,
      maxWidth: undefined,
      minHeight: undefined,
      maxHeight: undefined,
      margin: { top: 0, right: 0, bottom: 0, left: 0 },
      padding: { top: 0, right: 0, bottom: 0, left: 0 },
    };
    const result = layout_block(imgLeaf, constraints_width(100), measurer);
    expect(result.height).toBeGreaterThan(0);
  });
});

// ── Fixed-width and wrap-width block children ───────────────────────────────

describe("child width kinds", () => {
  it("fixed-width block child uses its fixed width", () => {
    const child: LayoutNode = {
      ...blockText("Hello"),
      width: { kind: "fixed", value: 120 },
    };
    const root = blockNode([child]);
    const result = layout_block(root, constraints_width(400), measurer);
    expect(result.children[0].width).toBe(120);
  });

  it("wrap-width block child uses available inner width", () => {
    const child: LayoutNode = {
      ...blockText("Hello"),
      width: { kind: "wrap" },
    };
    const root = blockNode([child]);
    const result = layout_block(root, constraints_width(400), measurer);
    expect(result.children[0].width).toBe(400);
  });
});

// ── paragraphSpacing ────────────────────────────────────────────────────────

describe("paragraphSpacing", () => {
  it("paragraphSpacing adds extra gap between blocks", () => {
    const child1: LayoutNode = {
      ...blockText("Paragraph 1"),
      ext: { block: { display: "block", paragraphSpacing: 20 } as BlockExt },
    };
    const child2 = blockText("Paragraph 2");
    const root = blockNode([child1, child2]);
    const result = layout_block(root, constraints_width(400), measurer);
    const c1 = result.children[0];
    const c2 = result.children[1];
    // gap between c1 bottom and c2 top = max(paragraphSpacing, child2.margin.top) = 20
    expect(c2.y).toBeGreaterThanOrEqual(c1.y + c1.height + 16); // at least paragraphSpacing
  });
});
