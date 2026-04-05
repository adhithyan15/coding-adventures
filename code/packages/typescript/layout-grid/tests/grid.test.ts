/**
 * Tests for layout-grid (CSS Grid layout algorithm).
 */

import { describe, it, expect } from "vitest";
import { layout_grid, type GridContainerExt, type GridItemExt } from "../src/index.js";
import {
  constraints_width,
  font_spec,
  rgb,
} from "@coding-adventures/layout-ir";
import type { LayoutNode, PositionedNode } from "@coding-adventures/layout-ir";
import { createEstimatedMeasurer } from "@coding-adventures/layout-text-measure-estimated";

// ── Helpers ─────────────────────────────────────────────────────────────────

const font = font_spec("Arial", 16);
const black = rgb(0, 0, 0);
const measurer = createEstimatedMeasurer();

function gridItem(
  text: string,
  itemExt?: GridItemExt,
  extra?: Partial<LayoutNode>
): LayoutNode {
  return {
    width: { kind: "wrap" },
    height: { kind: "wrap" },
    content: { kind: "text", value: text, font, color: black, maxLines: null, textAlign: "start" },
    children: [],
    ext: { grid: itemExt ?? {} },
    minWidth: undefined,
    maxWidth: undefined,
    minHeight: undefined,
    maxHeight: undefined,
    margin: { top: 0, right: 0, bottom: 0, left: 0 },
    padding: { top: 0, right: 0, bottom: 0, left: 0 },
    ...extra,
  };
}

function gridContainer(
  children: LayoutNode[],
  containerExt: GridContainerExt,
  extra?: Partial<LayoutNode>
): LayoutNode {
  return {
    width: { kind: "fill" },
    height: { kind: "wrap" },
    content: null,
    children,
    ext: { grid: containerExt },
    minWidth: undefined,
    maxWidth: undefined,
    minHeight: undefined,
    maxHeight: undefined,
    margin: { top: 0, right: 0, bottom: 0, left: 0 },
    padding: { top: 0, right: 0, bottom: 0, left: 0 },
    ...extra,
  };
}

// ── Basic scene structure ────────────────────────────────────────────────────

describe("layout_grid — scene structure", () => {
  it("empty grid has zero height", () => {
    const root = gridContainer([], { templateColumns: "1fr 1fr" });
    const result = layout_grid(root, constraints_width(400), measurer);
    expect(result.height).toBe(0);
    expect(result.children).toHaveLength(0);
  });

  it("width fills available space", () => {
    const root = gridContainer([], { templateColumns: "1fr" });
    const result = layout_grid(root, constraints_width(600), measurer);
    expect(result.width).toBe(600);
  });

  it("fixed container width", () => {
    const root: LayoutNode = {
      ...gridContainer([], {}),
      width: { kind: "fixed", value: 300 },
    };
    const result = layout_grid(root, constraints_width(600), measurer);
    expect(result.width).toBe(300);
  });
});

// ── Track list parsing ───────────────────────────────────────────────────────

describe("track list parsing", () => {
  it("two equal fr columns split space evenly", () => {
    const root = gridContainer(
      [gridItem("A"), gridItem("B")],
      { templateColumns: "1fr 1fr" }
    );
    const result = layout_grid(root, constraints_width(400), measurer);
    // Both items get half the available width
    expect(result.children[0].width).toBeCloseTo(200);
    expect(result.children[1].width).toBeCloseTo(200);
  });

  it("repeat(3, 1fr) creates three equal columns", () => {
    const root = gridContainer(
      [gridItem("A"), gridItem("B"), gridItem("C")],
      { templateColumns: "repeat(3, 1fr)" }
    );
    const result = layout_grid(root, constraints_width(300), measurer);
    expect(result.children[0].width).toBeCloseTo(100);
    expect(result.children[1].width).toBeCloseTo(100);
    expect(result.children[2].width).toBeCloseTo(100);
  });

  it("fixed px column has exact size", () => {
    const root = gridContainer(
      [gridItem("A"), gridItem("B")],
      { templateColumns: "100px 1fr" }
    );
    const result = layout_grid(root, constraints_width(400), measurer);
    expect(result.children[0].width).toBe(100);
    expect(result.children[1].width).toBeCloseTo(300);
  });

  it("mixed px and fr columns", () => {
    const root = gridContainer(
      [gridItem("A"), gridItem("B"), gridItem("C")],
      { templateColumns: "100px 1fr 100px" }
    );
    const result = layout_grid(root, constraints_width(400), measurer);
    expect(result.children[0].width).toBe(100);
    expect(result.children[1].width).toBeCloseTo(200);
    expect(result.children[2].width).toBe(100);
  });

  it("repeat(2, 100px) creates two fixed columns", () => {
    const root = gridContainer(
      [gridItem("A"), gridItem("B")],
      { templateColumns: "repeat(2, 100px)" }
    );
    const result = layout_grid(root, constraints_width(400), measurer);
    expect(result.children[0].width).toBe(100);
    expect(result.children[1].width).toBe(100);
  });
});

// ── Auto placement ───────────────────────────────────────────────────────────

describe("auto placement", () => {
  it("items flow left to right across rows", () => {
    const root = gridContainer(
      [gridItem("A"), gridItem("B"), gridItem("C"), gridItem("D")],
      { templateColumns: "1fr 1fr" }
    );
    const result = layout_grid(root, constraints_width(400), measurer);
    // 2 columns: A(row1,col1), B(row1,col2), C(row2,col1), D(row2,col2)
    const [a, b, c, d] = result.children;
    expect(a.x).toBeCloseTo(0);
    expect(b.x).toBeCloseTo(200);
    expect(c.x).toBeCloseTo(0);
    expect(d.x).toBeCloseTo(200);
    // C and D are on a second row
    expect(c.y).toBeGreaterThan(a.y);
    expect(d.y).toBeGreaterThan(b.y);
  });

  it("items in a 3-column grid wrap at column 3", () => {
    const root = gridContainer(
      [gridItem("A"), gridItem("B"), gridItem("C"), gridItem("D")],
      { templateColumns: "1fr 1fr 1fr" }
    );
    const result = layout_grid(root, constraints_width(300), measurer);
    const [a, b, c, d] = result.children;
    // First row: A, B, C; second row: D
    expect(a.x).toBeCloseTo(0);
    expect(b.x).toBeCloseTo(100);
    expect(c.x).toBeCloseTo(200);
    expect(d.x).toBeCloseTo(0);  // wraps to col 1
    expect(d.y).toBeGreaterThan(a.y);
  });
});

// ── Explicit placement ───────────────────────────────────────────────────────

describe("explicit placement", () => {
  it("item at explicit row and column", () => {
    const root = gridContainer(
      [
        gridItem("A", { rowStart: 1, columnStart: 1 }),
        gridItem("B", { rowStart: 1, columnStart: 2 }),
      ],
      { templateColumns: "1fr 1fr" }
    );
    const result = layout_grid(root, constraints_width(400), measurer);
    // A at col 1 (x=0), B at col 2 (x=200)
    expect(result.children[0].x).toBeCloseTo(0);
    expect(result.children[1].x).toBeCloseTo(200);
  });

  it("item spans two columns", () => {
    const root = gridContainer(
      [
        gridItem("Wide", { rowStart: 1, columnStart: 1, columnSpan: 2 }),
      ],
      { templateColumns: "1fr 1fr" }
    );
    const result = layout_grid(root, constraints_width(400), measurer);
    // Stretches across both columns
    expect(result.children[0].width).toBeCloseTo(400);
  });

  it("item spans two rows", () => {
    const root = gridContainer(
      [
        gridItem("Tall", { rowStart: 1, columnStart: 1, rowSpan: 2 }),
        gridItem("Short1", { rowStart: 1, columnStart: 2 }),
        gridItem("Short2", { rowStart: 2, columnStart: 2 }),
      ],
      { templateColumns: "1fr 1fr", templateRows: "auto auto" }
    );
    const result = layout_grid(root, constraints_width(400), measurer);
    // Tall item spans rows 1-2, so its height = row1 + rowGap + row2
    expect(result.children[0].height).toBeGreaterThan(0);
  });

  it("columnEnd sets item column end", () => {
    const root = gridContainer(
      [gridItem("A", { rowStart: 1, columnStart: 1, columnEnd: 3 })],
      { templateColumns: "1fr 1fr 1fr" }
    );
    const result = layout_grid(root, constraints_width(300), measurer);
    // Spans cols 1-2 = 200px
    expect(result.children[0].width).toBeCloseTo(200);
  });
});

// ── Gaps ─────────────────────────────────────────────────────────────────────

describe("gaps", () => {
  it("columnGap creates space between columns", () => {
    const root = gridContainer(
      [gridItem("A"), gridItem("B")],
      { templateColumns: "1fr 1fr", columnGap: 20 }
    );
    const result = layout_grid(root, constraints_width(420), measurer);
    // Total space: 420px, gap: 20px → each column = (420-20)/2 = 200px
    expect(result.children[0].width).toBeCloseTo(200);
    expect(result.children[1].x).toBeCloseTo(220); // 200 + 20 gap
  });

  it("rowGap creates space between rows", () => {
    const root = gridContainer(
      [gridItem("A"), gridItem("B")],
      { templateColumns: "1fr", rowGap: 10 }
    );
    const result = layout_grid(root, constraints_width(400), measurer);
    // B is below A + row gap
    const a = result.children[0];
    const b = result.children[1];
    // b.y = a.y + a.height + rowGap
    expect(b.y).toBeGreaterThanOrEqual(a.y + a.height + 10 - 1);
  });
});

// ── Alignment ────────────────────────────────────────────────────────────────

describe("alignment", () => {
  it("justifyItems: stretch fills cell width", () => {
    const root = gridContainer(
      [gridItem("A")],
      { templateColumns: "200px", justifyItems: "stretch" }
    );
    const result = layout_grid(root, constraints_width(400), measurer);
    expect(result.children[0].width).toBe(200);
  });

  it("justifyItems: start aligns to start", () => {
    const root = gridContainer(
      [gridItem("A")],
      { templateColumns: "200px", justifyItems: "start" }
    );
    const result = layout_grid(root, constraints_width(400), measurer);
    expect(result.children[0].x).toBe(0);
    expect(result.children[0].width).toBeLessThanOrEqual(200);
  });

  it("justifySelf overrides justifyItems", () => {
    const root = gridContainer(
      [gridItem("A", { justifySelf: "start" })],
      { templateColumns: "200px", justifyItems: "stretch" }
    );
    const result = layout_grid(root, constraints_width(400), measurer);
    // Item uses start, not stretch — width = natural content width
    expect(result.children[0].width).toBeLessThanOrEqual(200);
  });

  it("alignItems: stretch fills cell height", () => {
    const root = gridContainer(
      [gridItem("A"), gridItem("B")],
      { templateColumns: "1fr 1fr", templateRows: "50px", alignItems: "stretch" }
    );
    const result = layout_grid(root, constraints_width(400), measurer);
    // Both items in fixed 50px rows, stretched height = 50
    expect(result.children[0].height).toBe(50);
  });

  it("alignSelf: auto inherits alignItems", () => {
    const root = gridContainer(
      [gridItem("A", { alignSelf: "auto" })],
      { templateColumns: "1fr", templateRows: "50px", alignItems: "stretch" }
    );
    const result = layout_grid(root, constraints_width(400), measurer);
    expect(result.children[0].height).toBe(50);
  });
});

// ── Implicit tracks ──────────────────────────────────────────────────────────

describe("implicit tracks", () => {
  it("auto-flow creates implicit rows for overflow items", () => {
    const root = gridContainer(
      [gridItem("A"), gridItem("B"), gridItem("C")],
      { templateColumns: "1fr 1fr", templateRows: "30px" } // only 1 explicit row
    );
    const result = layout_grid(root, constraints_width(400), measurer);
    // C goes to row 2 (implicit row)
    expect(result.children[2].y).toBeGreaterThan(result.children[0].y);
  });

  it("autoRows controls implicit row size", () => {
    const root = gridContainer(
      [gridItem("A"), gridItem("B"), gridItem("C")],
      {
        templateColumns: "1fr 1fr",
        templateRows: "30px",
        autoRows: "60px",
      }
    );
    const result = layout_grid(root, constraints_width(400), measurer);
    // C is in implicit row of 60px
    const c = result.children[2];
    // c.y should be at 30 (explicit row height) + possible row gap
    expect(c.y).toBeGreaterThanOrEqual(30);
  });
});

// ── Padding ──────────────────────────────────────────────────────────────────

describe("padding", () => {
  it("padding offsets all items", () => {
    const root: LayoutNode = {
      ...gridContainer([gridItem("A")], { templateColumns: "1fr" }),
      padding: { top: 10, right: 10, bottom: 10, left: 20 },
    };
    const result = layout_grid(root, constraints_width(420), measurer);
    expect(result.children[0].x).toBeGreaterThanOrEqual(20);
    expect(result.children[0].y).toBeGreaterThanOrEqual(10);
  });

  it("padding is included in container height", () => {
    const root: LayoutNode = {
      ...gridContainer([gridItem("A")], { templateColumns: "1fr", templateRows: "50px" }),
      padding: { top: 10, right: 0, bottom: 10, left: 0 },
    };
    const result = layout_grid(root, constraints_width(400), measurer);
    expect(result.height).toBeGreaterThanOrEqual(70); // 50 + 10 + 10
  });
});

// ── minmax tracks ────────────────────────────────────────────────────────────

describe("minmax tracks", () => {
  it("minmax(100px, 1fr) grows to fill available space", () => {
    const root = gridContainer(
      [gridItem("A"), gridItem("B")],
      { templateColumns: "minmax(100px, 1fr) minmax(100px, 1fr)" }
    );
    const result = layout_grid(root, constraints_width(400), measurer);
    // Both tracks get half of 400 = 200 each (≥ 100px min)
    expect(result.children[0].width).toBeGreaterThanOrEqual(100);
    expect(result.children[1].width).toBeGreaterThanOrEqual(100);
  });
});

// ── alignItems: end / center ─────────────────────────────────────────────────

describe("alignItems end and center", () => {
  it("alignItems: end aligns item to cell bottom", () => {
    const root = gridContainer(
      [gridItem("A")],
      { templateColumns: "1fr", templateRows: "80px", alignItems: "end" }
    );
    const result = layout_grid(root, constraints_width(400), measurer);
    const item = result.children[0];
    expect(item.y + item.height).toBeCloseTo(80);
  });

  it("alignItems: center centers item vertically", () => {
    const root = gridContainer(
      [gridItem("A")],
      { templateColumns: "1fr", templateRows: "80px", alignItems: "center" }
    );
    const result = layout_grid(root, constraints_width(400), measurer);
    const item = result.children[0];
    expect(item.y + item.height / 2).toBeCloseTo(40);
  });

  it("justifyItems: end aligns item to cell right", () => {
    const root = gridContainer(
      [gridItem("A")],
      { templateColumns: "200px", justifyItems: "end" }
    );
    const result = layout_grid(root, constraints_width(400), measurer);
    const item = result.children[0];
    expect(item.x + item.width).toBeCloseTo(200);
  });

  it("justifyItems: center centers item horizontally", () => {
    const root = gridContainer(
      [gridItem("A")],
      { templateColumns: "200px", justifyItems: "center" }
    );
    const result = layout_grid(root, constraints_width(400), measurer);
    const item = result.children[0];
    expect(item.x + item.width / 2).toBeCloseTo(100);
  });
});

// ── Fixed container height ───────────────────────────────────────────────────

describe("fixed container height", () => {
  it("fixed height container uses that height", () => {
    const root: LayoutNode = {
      ...gridContainer([gridItem("A")], { templateColumns: "1fr" }),
      height: { kind: "fixed", value: 300 },
    };
    const result = layout_grid(root, constraints_width(400), measurer);
    expect(result.height).toBe(300);
  });
});

// ── Image items ──────────────────────────────────────────────────────────────

describe("image items", () => {
  it("image item is placed in grid cell", () => {
    const imgItem: LayoutNode = {
      width: { kind: "fixed", value: 80 },
      height: { kind: "fixed", value: 60 },
      content: { kind: "image", src: "img.png", fit: "contain" },
      children: [],
      ext: { grid: {} },
      minWidth: undefined,
      maxWidth: undefined,
      minHeight: undefined,
      maxHeight: undefined,
      margin: { top: 0, right: 0, bottom: 0, left: 0 },
      padding: { top: 0, right: 0, bottom: 0, left: 0 },
    };
    const root = gridContainer([imgItem], { templateColumns: "1fr" });
    const result = layout_grid(root, constraints_width(400), measurer);
    expect(result.children).toHaveLength(1);
    expect(result.children[0].width).toBeGreaterThan(0);
  });
});

// ── autoFlow: column ─────────────────────────────────────────────────────────

describe("autoFlow: column", () => {
  it("items flow top-to-bottom then wrap to next column", () => {
    const root = gridContainer(
      [gridItem("A"), gridItem("B"), gridItem("C"), gridItem("D")],
      {
        templateColumns: "1fr 1fr",
        templateRows: "repeat(2, auto)",
        autoFlow: "column",
      }
    );
    const result = layout_grid(root, constraints_width(400), measurer);
    // Column flow: A(r1,c1), B(r2,c1), C(r1,c2), D(r2,c2)
    const [a, b, c, d] = result.children;
    expect(a.x).toBeCloseTo(b.x); // A and B in same column
    expect(c.x).toBeCloseTo(d.x); // C and D in same column
    expect(c.x).toBeGreaterThan(a.x); // C is in a later column
  });
});
