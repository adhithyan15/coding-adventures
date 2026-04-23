/**
 * Tests for @coding-adventures/barcode-2d
 *
 * Coverage target: ≥90% lines
 *
 * Test strategy:
 *   1. makeModuleGrid — grid creation and initial state
 *   2. setModule — immutable updates and bounds checking
 *   3. layout (square) — background rect, dark rects, coordinate math
 *   4. layout (hex) — PaintPath generation, row offset, vertex geometry
 *   5. Validation — invalid config throws the right error type
 *   6. Constants — VERSION and DEFAULT_BARCODE_2D_LAYOUT_CONFIG
 */

import { describe, it, expect } from "vitest";
import {
  VERSION,
  DEFAULT_BARCODE_2D_LAYOUT_CONFIG,
  makeModuleGrid,
  setModule,
  layout,
  InvalidBarcode2DConfigError,
  Barcode2DError,
} from "../src/index.js";

// ============================================================================
// Helpers
// ============================================================================

/** Extract all PaintRect instructions from a PaintScene. */
function rects(scene: ReturnType<typeof layout>) {
  return scene.instructions.filter((i) => i.kind === "rect");
}

/** Extract all PaintPath instructions from a PaintScene. */
function paths(scene: ReturnType<typeof layout>) {
  return scene.instructions.filter((i) => i.kind === "path");
}

// ============================================================================
// 1. makeModuleGrid
// ============================================================================

describe("makeModuleGrid", () => {
  it("stores correct row and col dimensions", () => {
    const g = makeModuleGrid(7, 11);
    expect(g.rows).toBe(7);
    expect(g.cols).toBe(11);
  });

  it("initialises every module to false (light)", () => {
    const g = makeModuleGrid(5, 5);
    for (let r = 0; r < g.rows; r++) {
      for (let c = 0; c < g.cols; c++) {
        expect(g.modules[r][c]).toBe(false);
      }
    }
  });

  it("defaults to square shape", () => {
    const g = makeModuleGrid(3, 3);
    expect(g.moduleShape).toBe("square");
  });

  it("stores the provided moduleShape", () => {
    const g = makeModuleGrid(33, 30, "hex");
    expect(g.moduleShape).toBe("hex");
    expect(g.rows).toBe(33);
    expect(g.cols).toBe(30);
  });

  it("creates independent row arrays (different references)", () => {
    const g = makeModuleGrid(3, 3);
    expect(g.modules[0]).not.toBe(g.modules[1]);
  });
});

// ============================================================================
// 2. setModule
// ============================================================================

describe("setModule", () => {
  it("returns a new object (reference inequality)", () => {
    const g = makeModuleGrid(3, 3);
    const g2 = setModule(g, 0, 0, true);
    expect(g2).not.toBe(g);
  });

  it("sets the target module to true", () => {
    const g = makeModuleGrid(3, 3);
    const g2 = setModule(g, 1, 2, true);
    expect(g2.modules[1][2]).toBe(true);
  });

  it("leaves the original grid unchanged", () => {
    const g = makeModuleGrid(3, 3);
    setModule(g, 1, 2, true);
    expect(g.modules[1][2]).toBe(false);
  });

  it("sets a module to false (clearing a dark module)", () => {
    const g = makeModuleGrid(3, 3);
    const g1 = setModule(g, 0, 0, true);
    const g2 = setModule(g1, 0, 0, false);
    expect(g2.modules[0][0]).toBe(false);
  });

  it("does not affect other modules", () => {
    const g = makeModuleGrid(3, 3);
    const g2 = setModule(g, 1, 1, true);
    expect(g2.modules[0][0]).toBe(false);
    expect(g2.modules[2][2]).toBe(false);
    expect(g2.modules[1][0]).toBe(false);
    expect(g2.modules[1][2]).toBe(false);
  });

  it("preserves dimensions and moduleShape", () => {
    const g = makeModuleGrid(5, 7, "hex");
    const g2 = setModule(g, 2, 3, true);
    expect(g2.rows).toBe(5);
    expect(g2.cols).toBe(7);
    expect(g2.moduleShape).toBe("hex");
  });

  it("throws RangeError for negative row", () => {
    const g = makeModuleGrid(3, 3);
    expect(() => setModule(g, -1, 0, true)).toThrow(RangeError);
  });

  it("throws RangeError for row >= rows", () => {
    const g = makeModuleGrid(3, 3);
    expect(() => setModule(g, 3, 0, true)).toThrow(RangeError);
  });

  it("throws RangeError for negative col", () => {
    const g = makeModuleGrid(3, 3);
    expect(() => setModule(g, 0, -1, true)).toThrow(RangeError);
  });

  it("throws RangeError for col >= cols", () => {
    const g = makeModuleGrid(3, 3);
    expect(() => setModule(g, 0, 3, true)).toThrow(RangeError);
  });
});

// ============================================================================
// 3. layout — square modules
// ============================================================================

describe("layout (square modules)", () => {
  it("all-light 1×1 grid: only 1 background rect, no dark rects", () => {
    const g = makeModuleGrid(1, 1);
    const scene = layout(g);
    // Only the background rect — no dark modules.
    expect(scene.instructions.length).toBe(1);
    expect(scene.instructions[0].kind).toBe("rect");
  });

  it("all-dark 1×1 grid: background rect + 1 dark rect", () => {
    let g = makeModuleGrid(1, 1);
    g = setModule(g, 0, 0, true);
    const scene = layout(g, { moduleSizePx: 10, quietZoneModules: 0 });
    expect(rects(scene).length).toBe(2);
  });

  it("dark module at [0][0] is placed at (quietZonePx, quietZonePx)", () => {
    let g = makeModuleGrid(5, 5);
    g = setModule(g, 0, 0, true);
    const cfg = { moduleSizePx: 10, quietZoneModules: 4 };
    const scene = layout(g, cfg);
    const darkRects = rects(scene).slice(1); // skip background
    expect(darkRects.length).toBe(1);
    const r = darkRects[0] as { kind: "rect"; x: number; y: number };
    expect(r.x).toBe(40); // 4 * 10
    expect(r.y).toBe(40); // 4 * 10
  });

  it("dark module at [row][col] has correct pixel coordinates", () => {
    let g = makeModuleGrid(10, 10);
    g = setModule(g, 2, 3, true);
    const cfg = { moduleSizePx: 8, quietZoneModules: 2 };
    const scene = layout(g, cfg);
    const darkRects = rects(scene).slice(1);
    expect(darkRects.length).toBe(1);
    const r = darkRects[0] as { kind: "rect"; x: number; y: number; width: number; height: number };
    // quietZonePx = 2 * 8 = 16
    // x = 16 + 3 * 8 = 40
    // y = 16 + 2 * 8 = 32
    expect(r.x).toBe(40);
    expect(r.y).toBe(32);
    expect(r.width).toBe(8);
    expect(r.height).toBe(8);
  });

  it("total canvas size for a 21×21 QR grid (v1 size) is correct", () => {
    const g = makeModuleGrid(21, 21);
    const cfg = { moduleSizePx: 10, quietZoneModules: 4 };
    const scene = layout(g, cfg);
    // totalWidth = (21 + 2*4) * 10 = 29 * 10 = 290
    expect(scene.width).toBe(290);
    expect(scene.height).toBe(290);
  });

  it("background color is painted as the first instruction", () => {
    const g = makeModuleGrid(3, 3);
    const scene = layout(g, { background: "#aabbcc" });
    expect(scene.background).toBe("#aabbcc");
    const firstRect = scene.instructions[0] as { kind: "rect"; fill: string };
    expect(firstRect.kind).toBe("rect");
    expect(firstRect.fill).toBe("#aabbcc");
  });

  it("foreground color is applied to all dark module rects", () => {
    let g = makeModuleGrid(3, 3);
    g = setModule(g, 0, 0, true);
    g = setModule(g, 2, 2, true);
    const scene = layout(g, { foreground: "#ff0000", quietZoneModules: 0 });
    const darkRects = rects(scene).slice(1); // skip background
    expect(darkRects.length).toBe(2);
    for (const r of darkRects as { fill: string }[]) {
      expect(r.fill).toBe("#ff0000");
    }
  });

  it("zero quiet zone: dark module [0][0] starts at pixel (0, 0)", () => {
    let g = makeModuleGrid(3, 3);
    g = setModule(g, 0, 0, true);
    const scene = layout(g, { moduleSizePx: 5, quietZoneModules: 0 });
    const darkRects = rects(scene).slice(1);
    const r = darkRects[0] as { x: number; y: number };
    expect(r.x).toBe(0);
    expect(r.y).toBe(0);
  });

  it("partial config merges with defaults", () => {
    const g = makeModuleGrid(5, 5);
    const scene = layout(g, { moduleSizePx: 20 });
    // quietZoneModules defaults to 4, so totalWidth = (5 + 8) * 20 = 260
    expect(scene.width).toBe(260);
    expect(scene.background).toBe("#ffffff");
  });

  it("produces no path instructions for square grids", () => {
    let g = makeModuleGrid(5, 5);
    g = setModule(g, 2, 2, true);
    const scene = layout(g);
    expect(paths(scene).length).toBe(0);
  });
});

// ============================================================================
// 4. layout — hex modules (MaxiCode)
// ============================================================================

describe("layout (hex modules)", () => {
  it("produces PaintPath (not PaintRect) for dark hex modules", () => {
    let g = makeModuleGrid(5, 5, "hex");
    g = setModule(g, 0, 0, true);
    const scene = layout(g, { moduleShape: "hex", quietZoneModules: 0 });
    expect(paths(scene).length).toBe(1);
    // The background is still a rect, but dark modules are paths.
    const r = scene.instructions[0];
    expect(r.kind).toBe("rect");
    const p = scene.instructions[1];
    expect(p.kind).toBe("path");
  });

  it("each hex path has 7 commands: move_to + 5 line_to + close", () => {
    let g = makeModuleGrid(3, 3, "hex");
    g = setModule(g, 0, 0, true);
    const scene = layout(g, { moduleShape: "hex", quietZoneModules: 0 });
    const hexPath = paths(scene)[0] as { commands: { kind: string }[] };
    expect(hexPath.commands.length).toBe(7);
    expect(hexPath.commands[0].kind).toBe("move_to");
    expect(hexPath.commands[1].kind).toBe("line_to");
    expect(hexPath.commands[6].kind).toBe("close");
  });

  it("even row (row 0) has no x-offset", () => {
    let g = makeModuleGrid(3, 3, "hex");
    g = setModule(g, 0, 0, true); // even row
    const cfg = { moduleShape: "hex" as const, moduleSizePx: 10, quietZoneModules: 0 };
    const scene = layout(g, cfg);
    const p = paths(scene)[0] as {
      commands: { kind: string; x?: number; y?: number }[];
    };
    // For row 0, col 0: cx = 0, cy = 0
    // Vertex at 0°: x = circumR = 10/√3 ≈ 5.774
    const circumR = 10 / Math.sqrt(3);
    const vertex0x = p.commands[0].x!;
    expect(vertex0x).toBeCloseTo(circumR, 5);
  });

  it("odd row (row 1) is offset by hexWidth/2", () => {
    let g = makeModuleGrid(3, 3, "hex");
    g = setModule(g, 1, 0, true); // odd row
    const cfg = { moduleShape: "hex" as const, moduleSizePx: 10, quietZoneModules: 0 };
    const scene = layout(g, cfg);
    const p = paths(scene)[0] as {
      commands: { kind: string; x?: number; y?: number }[];
    };
    // For row 1, col 0: cx = 0 + 10/2 = 5 (hexWidth/2 offset)
    // hexHeight = 10 * √3/2 ≈ 8.66
    // cy = 1 * hexHeight
    const hexWidth = 10;
    const hexHeight = 10 * (Math.sqrt(3) / 2);
    const circumR = hexWidth / Math.sqrt(3);
    const expectedCx = hexWidth / 2;
    const expectedCy = hexHeight;
    // Vertex at 0° from (expectedCx, expectedCy)
    const vertex0x = p.commands[0].x!;
    const vertex0y = p.commands[0].y!;
    expect(vertex0x).toBeCloseTo(expectedCx + circumR, 5);
    expect(vertex0y).toBeCloseTo(expectedCy, 5);
  });

  it("circumradius: all hex vertices are exactly circumR from center", () => {
    let g = makeModuleGrid(3, 3, "hex");
    g = setModule(g, 0, 1, true); // col 1, row 0
    const moduleSizePx = 12;
    const cfg = { moduleShape: "hex" as const, moduleSizePx, quietZoneModules: 0 };
    const scene = layout(g, cfg);
    const p = paths(scene)[0] as {
      commands: { kind: string; x?: number; y?: number }[];
    };

    const hexWidth = moduleSizePx;
    const circumR = hexWidth / Math.sqrt(3);
    // Center of module at row=0, col=1: cx = 1 * hexWidth, cy = 0
    const cx = 1 * hexWidth;
    const cy = 0;

    // Check all 6 vertex commands (commands[0] to commands[5]).
    const vertexCommands = p.commands.slice(0, 6) as { x: number; y: number }[];
    for (const v of vertexCommands) {
      const dist = Math.sqrt((v.x - cx) ** 2 + (v.y - cy) ** 2);
      expect(dist).toBeCloseTo(circumR, 5);
    }
  });

  it("total width adds hexWidth/2 to accommodate odd-row offset", () => {
    const g = makeModuleGrid(4, 6, "hex");
    const cfg = { moduleShape: "hex" as const, moduleSizePx: 10, quietZoneModules: 2 };
    const scene = layout(g, cfg);
    // totalWidth = (6 + 2*2) * 10 + 10/2 = 100 + 5 = 105
    expect(scene.width).toBeCloseTo(105, 5);
  });
});

// ============================================================================
// 5. Validation — invalid config
// ============================================================================

describe("layout validation", () => {
  it("throws InvalidBarcode2DConfigError for moduleSizePx = 0", () => {
    const g = makeModuleGrid(3, 3);
    expect(() => layout(g, { moduleSizePx: 0 })).toThrow(
      InvalidBarcode2DConfigError,
    );
  });

  it("throws InvalidBarcode2DConfigError for moduleSizePx < 0", () => {
    const g = makeModuleGrid(3, 3);
    expect(() => layout(g, { moduleSizePx: -5 })).toThrow(
      InvalidBarcode2DConfigError,
    );
  });

  it("throws InvalidBarcode2DConfigError for quietZoneModules < 0", () => {
    const g = makeModuleGrid(3, 3);
    expect(() => layout(g, { quietZoneModules: -1 })).toThrow(
      InvalidBarcode2DConfigError,
    );
  });

  it("throws InvalidBarcode2DConfigError when config.moduleShape !== grid.moduleShape", () => {
    // Grid is square, config says hex
    const g = makeModuleGrid(3, 3, "square");
    expect(() => layout(g, { moduleShape: "hex" })).toThrow(
      InvalidBarcode2DConfigError,
    );
  });

  it("throws InvalidBarcode2DConfigError when grid is hex but config is square", () => {
    const g = makeModuleGrid(3, 3, "hex");
    expect(() => layout(g, { moduleShape: "square" })).toThrow(
      InvalidBarcode2DConfigError,
    );
  });

  it("InvalidBarcode2DConfigError is an instance of Barcode2DError", () => {
    const g = makeModuleGrid(3, 3);
    try {
      layout(g, { moduleSizePx: 0 });
    } catch (e) {
      expect(e).toBeInstanceOf(Barcode2DError);
      expect(e).toBeInstanceOf(InvalidBarcode2DConfigError);
    }
  });

  it("InvalidBarcode2DConfigError has the correct name", () => {
    const g = makeModuleGrid(3, 3);
    try {
      layout(g, { moduleSizePx: -1 });
    } catch (e) {
      if (e instanceof InvalidBarcode2DConfigError) {
        expect(e.name).toBe("InvalidBarcode2DConfigError");
      }
    }
  });
});

// ============================================================================
// 6. Constants
// ============================================================================

describe("constants", () => {
  it("VERSION is '0.1.0'", () => {
    expect(VERSION).toBe("0.1.0");
  });

  it("DEFAULT_BARCODE_2D_LAYOUT_CONFIG has correct moduleSizePx", () => {
    expect(DEFAULT_BARCODE_2D_LAYOUT_CONFIG.moduleSizePx).toBe(10);
  });

  it("DEFAULT_BARCODE_2D_LAYOUT_CONFIG has correct quietZoneModules", () => {
    expect(DEFAULT_BARCODE_2D_LAYOUT_CONFIG.quietZoneModules).toBe(4);
  });

  it("DEFAULT_BARCODE_2D_LAYOUT_CONFIG has correct foreground", () => {
    expect(DEFAULT_BARCODE_2D_LAYOUT_CONFIG.foreground).toBe("#000000");
  });

  it("DEFAULT_BARCODE_2D_LAYOUT_CONFIG has correct background", () => {
    expect(DEFAULT_BARCODE_2D_LAYOUT_CONFIG.background).toBe("#ffffff");
  });

  it("DEFAULT_BARCODE_2D_LAYOUT_CONFIG has showAnnotations = false", () => {
    expect(DEFAULT_BARCODE_2D_LAYOUT_CONFIG.showAnnotations).toBe(false);
  });

  it("DEFAULT_BARCODE_2D_LAYOUT_CONFIG has moduleShape = 'square'", () => {
    expect(DEFAULT_BARCODE_2D_LAYOUT_CONFIG.moduleShape).toBe("square");
  });
});
