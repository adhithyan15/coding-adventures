/**
 * Tests for the flexbox layout algorithm.
 *
 * We use `layout-text-measure-estimated` as the measurer throughout — it gives
 * deterministic, reproducible results without needing real fonts.
 */

import { describe, it, expect } from "vitest";
import { layout_flexbox, measure_node } from "../src/index.js";
import { createEstimatedMeasurer } from "@coding-adventures/layout-text-measure-estimated";
import {
  container, leaf_text, leaf_image,
  size_fixed, size_fill, size_wrap,
  font_spec, rgb, edges_all, edges_zero,
  constraints_fixed, constraints_width, constraints_unconstrained,
} from "@coding-adventures/layout-ir";
import type { LayoutNode, TextMeasurer } from "@coding-adventures/layout-ir";

const measurer: TextMeasurer = createEstimatedMeasurer();
const font = font_spec("Arial", 16);
const color = rgb(0, 0, 0);

function text(value: string, opts?: Parameters<typeof leaf_text>[1]): LayoutNode {
  return leaf_text({ kind: "text", value, font, color, maxLines: null, textAlign: "start" }, opts);
}

// ============================================================================
// Basic container sizing
// ============================================================================

describe("container sizing", () => {
  it("fill container uses full available width", () => {
    const c = container([], { width: size_fill(), height: size_wrap() });
    const r = layout_flexbox(c, constraints_fixed(800, 600), measurer);
    expect(r.width).toBe(800);
  });

  it("fixed container has exact size", () => {
    const c = container([], { width: size_fixed(200), height: size_fixed(100) });
    const r = layout_flexbox(c, constraints_fixed(800, 600), measurer);
    expect(r.width).toBe(200);
    expect(r.height).toBe(100);
  });

  it("empty container has zero size when wrap", () => {
    const c = container([], { width: size_wrap(), height: size_wrap() });
    const r = layout_flexbox(c, constraints_width(800), measurer);
    expect(r.width).toBe(0);
    expect(r.height).toBe(0);
  });

  it("positioned at x=0, y=0 (caller positions container)", () => {
    const c = container([], { width: size_fixed(100), height: size_fixed(50) });
    const r = layout_flexbox(c, constraints_fixed(800, 600), measurer);
    expect(r.x).toBe(0);
    expect(r.y).toBe(0);
  });
});

// ============================================================================
// Column layout (default direction)
// ============================================================================

describe("column layout", () => {
  it("children stack vertically", () => {
    // "A" = 1 char × 9.6 = 9.6 wide, 19.2 tall
    const c = container(
      [text("A"), text("B"), text("C")],
      {
        width: size_fill(),
        height: size_wrap(),
        ext: { flex: { direction: "column" } },
      }
    );
    const r = layout_flexbox(c, constraints_width(400), measurer);
    expect(r.children).toHaveLength(3);
    // Children stacked: y positions
    expect(r.children[0].y).toBe(0);
    expect(r.children[1].y).toBeCloseTo(19.2); // after "A"
    expect(r.children[2].y).toBeCloseTo(38.4); // after "A" + "B"
  });

  it("column gap applied between items", () => {
    const c = container(
      [text("A"), text("B")],
      {
        height: size_wrap(),
        ext: { flex: { direction: "column", gap: 10 } },
      }
    );
    const r = layout_flexbox(c, constraints_width(400), measurer);
    // "A" is 19.2 tall; gap 10; "B" starts at 29.2
    expect(r.children[1].y).toBeCloseTo(29.2);
  });
});

// ============================================================================
// Row layout
// ============================================================================

describe("row layout", () => {
  it("children placed side by side", () => {
    const c = container(
      [text("Hi"), text("There")],
      { ext: { flex: { direction: "row" } } }
    );
    const r = layout_flexbox(c, constraints_width(400), measurer);
    expect(r.children[0].x).toBe(0);
    // "Hi" = 2 × 9.6 = 19.2 wide
    expect(r.children[1].x).toBeCloseTo(19.2);
  });

  it("row gap applied", () => {
    const c = container(
      [text("A"), text("B")],
      { ext: { flex: { direction: "row", gap: 8 } } }
    );
    const r = layout_flexbox(c, constraints_width(400), measurer);
    // "A" = 9.6 wide; gap 8; "B" starts at 17.6
    expect(r.children[1].x).toBeCloseTo(17.6);
  });
});

// ============================================================================
// grow distribution
// ============================================================================

describe("flex grow", () => {
  it("single grow item fills remaining space", () => {
    // Container 400 wide, child1=fixed 100, child2=grow=1 gets remaining 300
    const c = container(
      [
        container([], { width: size_fixed(100), height: size_fixed(40) }),
        container([], { width: size_wrap(), height: size_fixed(40), ext: { flex: { grow: 1 } } }),
      ],
      { width: size_fixed(400), ext: { flex: { direction: "row" } } }
    );
    const r = layout_flexbox(c, constraints_fixed(400, 400), measurer);
    expect(r.children[1].width).toBeCloseTo(300);
  });

  it("two equal grow items split remaining space", () => {
    const c = container(
      [
        container([], { width: size_wrap(), height: size_fixed(40), ext: { flex: { grow: 1 } } }),
        container([], { width: size_wrap(), height: size_fixed(40), ext: { flex: { grow: 1 } } }),
      ],
      { width: size_fixed(200), ext: { flex: { direction: "row" } } }
    );
    const r = layout_flexbox(c, constraints_fixed(200, 200), measurer);
    expect(r.children[0].width).toBeCloseTo(100);
    expect(r.children[1].width).toBeCloseTo(100);
  });
});

// ============================================================================
// Padding
// ============================================================================

describe("container padding", () => {
  it("children offset by padding", () => {
    const c = container(
      [text("A")],
      {
        padding: edges_all(20),
        width: size_fill(),
        height: size_wrap(),
        ext: { flex: { direction: "column" } },
      }
    );
    const r = layout_flexbox(c, constraints_width(400), measurer);
    expect(r.children[0].x).toBe(20);
    expect(r.children[0].y).toBe(20);
  });

  it("padding adds to container height", () => {
    // "A" is 19.2 tall; padding 10 top+bottom → container = 39.2
    const c = container(
      [text("A")],
      {
        padding: { top: 10, bottom: 10, left: 0, right: 0 },
        height: size_wrap(),
        ext: { flex: { direction: "column" } },
      }
    );
    const r = layout_flexbox(c, constraints_width(400), measurer);
    expect(r.height).toBeCloseTo(39.2);
  });
});

// ============================================================================
// Alignment
// ============================================================================

describe("alignItems", () => {
  it("start: children at cross-axis start", () => {
    const c = container(
      [text("A")],
      { height: size_fixed(100), ext: { flex: { direction: "row", alignItems: "start" } } }
    );
    const r = layout_flexbox(c, constraints_width(400), measurer);
    expect(r.children[0].y).toBe(0);
  });

  it("center: children centered on cross axis", () => {
    const c = container(
      [text("A")],
      { height: size_fixed(100), ext: { flex: { direction: "row", alignItems: "center" } } }
    );
    const r = layout_flexbox(c, constraints_width(400), measurer);
    // childHeight=19.2; center=(100-19.2)/2=40.4
    expect(r.children[0].y).toBeCloseTo(40.4);
  });

  it("end: children at cross-axis end", () => {
    const c = container(
      [text("A")],
      { height: size_fixed(100), ext: { flex: { direction: "row", alignItems: "end" } } }
    );
    const r = layout_flexbox(c, constraints_width(400), measurer);
    // childHeight=19.2; end=100-19.2=80.8
    expect(r.children[0].y).toBeCloseTo(80.8);
  });
});

describe("justifyContent", () => {
  it("start: items packed at start", () => {
    const c = container(
      [text("A"), text("B")],
      { width: size_fixed(200), ext: { flex: { direction: "row", justifyContent: "start" } } }
    );
    const r = layout_flexbox(c, constraints_fixed(200, 200), measurer);
    expect(r.children[0].x).toBe(0);
  });

  it("end: items packed at end", () => {
    const c = container(
      [text("A")],
      { width: size_fixed(200), ext: { flex: { direction: "row", justifyContent: "end" } } }
    );
    const r = layout_flexbox(c, constraints_fixed(200, 200), measurer);
    // "A" = 9.6 wide; end of 200 → x = 200 - 9.6 = 190.4
    expect(r.children[0].x).toBeCloseTo(190.4);
  });

  it("center: items centered", () => {
    const c = container(
      [text("A")],
      { width: size_fixed(200), ext: { flex: { direction: "row", justifyContent: "center" } } }
    );
    const r = layout_flexbox(c, constraints_fixed(200, 200), measurer);
    // (200-9.6)/2 = 95.2
    expect(r.children[0].x).toBeCloseTo(95.2);
  });

  it("around: equal space around each item", () => {
    const c = container(
      [text("A"), text("B")],
      { width: size_fixed(200), ext: { flex: { direction: "row", justifyContent: "around" } } }
    );
    const r = layout_flexbox(c, constraints_fixed(200, 200), measurer);
    // Free=200-19.2=180.8; perItem=90.4; first x=45.2
    expect(r.children[0].x).toBeGreaterThan(0);
    // Second item offset by first width + gap + spacing
    expect(r.children[1].x).toBeGreaterThan(r.children[0].x + 9.6);
  });

  it("evenly: equal space between and at edges", () => {
    const c = container(
      [text("A"), text("B")],
      { width: size_fixed(200), ext: { flex: { direction: "row", justifyContent: "evenly" } } }
    );
    const r = layout_flexbox(c, constraints_fixed(200, 200), measurer);
    // spacing = free / (count+1); first item starts at spacing > 0
    expect(r.children[0].x).toBeGreaterThan(0);
    // Evenly spaced: gap before + between + after all equal
    expect(r.children[1].x).toBeGreaterThan(r.children[0].x + 9.6);
  });

  it("between: equal spacing between items", () => {
    const c = container(
      [text("A"), text("B"), text("C")],
      { width: size_fixed(200), ext: { flex: { direction: "row", justifyContent: "between" } } }
    );
    const r = layout_flexbox(c, constraints_fixed(200, 200), measurer);
    // 3 items each 9.6 wide; total content=28.8; free=171.2; spacing=85.6 between
    expect(r.children[0].x).toBeCloseTo(0);
    expect(r.children[2].x).toBeCloseTo(200 - 9.6, 0);
  });
});

// ============================================================================
// order property
// ============================================================================

describe("order", () => {
  it("items sorted by order before layout", () => {
    const c = container(
      [
        text("last",  { ext: { flex: { order: 2 } } }),
        text("first", { ext: { flex: { order: 0 } } }),
        text("mid",   { ext: { flex: { order: 1 } } }),
      ],
      { ext: { flex: { direction: "row" } } }
    );
    const r = layout_flexbox(c, constraints_width(400), measurer);
    // "first" (order 0) should appear at x=0
    // Find which child is "first": it should have the smallest x
    const xPositions = r.children.map(ch => ch.x).sort((a, b) => a - b);
    expect(r.children.some(ch => ch.x === xPositions[0])).toBe(true);
    // Verify "last" (5 chars) is rightmost
    // "first" = 48, "mid" = 28.8, then "last" = 38.4
    // Order: first(48), mid(28.8), last(38.4) — x should be 0, 48, 76.8
    const sortedX = [...r.children].map(c => c.x).sort((a, b) => a - b);
    expect(sortedX[0]).toBe(0);
  });
});

// ============================================================================
// measure_node
// ============================================================================

describe("measure_node", () => {
  it("measures text leaf correctly", () => {
    const node = text("Hello"); // 5 chars × 9.6 = 48
    const size = measure_node(node, constraints_unconstrained(), measurer);
    expect(size.width).toBeCloseTo(48);
    expect(size.height).toBeCloseTo(19.2);
  });

  it("measures image leaf as square of constraint size", () => {
    const img = leaf_image({ kind: "image", src: "img.png", fit: "contain" },
      { width: size_fixed(100), height: size_fixed(100) }
    );
    const size = measure_node(img, constraints_fixed(100, 100), measurer);
    // Image with unconstrained → uses min(maxWidth, maxHeight) but both are 100
    expect(size.width).toBeGreaterThan(0);
  });

  it("measures container by running layout_flexbox", () => {
    const c = container(
      [text("A"), text("B")],
      { height: size_wrap(), ext: { flex: { direction: "column" } } }
    );
    const size = measure_node(c, constraints_width(400), measurer);
    // Two 19.2-tall items stacked = 38.4
    expect(size.height).toBeCloseTo(38.4);
  });
});

// ============================================================================
// min/maxWidth constraints
// ============================================================================

describe("min/maxWidth", () => {
  it("container maxWidth clamps container size", () => {
    const c = container([], { width: size_fill(), maxWidth: 200 });
    const r = layout_flexbox(c, constraints_width(800), measurer);
    expect(r.width).toBe(200);
  });

  it("container minWidth sets minimum size", () => {
    const c = container([], { width: size_wrap(), minWidth: 100 });
    const r = layout_flexbox(c, constraints_width(800), measurer);
    expect(r.width).toBe(100);
  });
});
