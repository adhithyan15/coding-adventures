/**
 * Tests for layout-to-paint.
 *
 * We build positioned node trees directly (bypassing the layout algorithm)
 * to test the paint emission logic in isolation.
 */

import { describe, it, expect } from "vitest";
import { layout_to_paint, colorToCss } from "../src/index.js";
import type { PositionedNode } from "@coding-adventures/layout-ir";
import { rgb, rgba, font_spec } from "@coding-adventures/layout-ir";
import type { PaintRect, PaintGlyphRun, PaintText, PaintImage, PaintLayer, PaintClip } from "@coding-adventures/paint-instructions";

// ============================================================================
// Helpers
// ============================================================================

const font = font_spec("Arial", 16);
const black = rgb(0, 0, 0);
const red = rgb(255, 0, 0);
const blue = rgb(0, 0, 255);

function textNode(
  x: number, y: number, w: number, h: number,
  text: string,
  extra?: Partial<PositionedNode>
): PositionedNode {
  return {
    x, y, width: w, height: h,
    content: { kind: "text", value: text, font, color: black, maxLines: null, textAlign: "start" },
    children: [],
    ext: {},
    ...extra,
  };
}

function imageNode(x: number, y: number, w: number, h: number, src: string): PositionedNode {
  return {
    x, y, width: w, height: h,
    content: { kind: "image", src, fit: "contain" },
    children: [],
    ext: {},
  };
}

function containerNode(
  x: number, y: number, w: number, h: number,
  children: PositionedNode[],
  ext?: Record<string, unknown>
): PositionedNode {
  return { x, y, width: w, height: h, content: null, children, ext: ext ?? {} };
}

// ============================================================================
// colorToCss
// ============================================================================

describe("colorToCss", () => {
  it("converts fully opaque color", () => {
    expect(colorToCss(rgb(255, 0, 0))).toBe("rgba(255,0,0,1)");
  });

  it("converts fully transparent", () => {
    expect(colorToCss(rgba(0, 0, 0, 0))).toBe("rgba(0,0,0,0)");
  });

  it("converts mid-opacity", () => {
    // 128/255 ≈ 0.502
    const css = colorToCss(rgba(0, 128, 255, 128));
    expect(css).toMatch(/^rgba\(0,128,255,/);
  });

  it("black", () => {
    expect(colorToCss(rgb(0, 0, 0))).toBe("rgba(0,0,0,1)");
  });

  it("white", () => {
    expect(colorToCss(rgb(255, 255, 255))).toBe("rgba(255,255,255,1)");
  });
});

// ============================================================================
// Basic scene structure
// ============================================================================

describe("layout_to_paint — scene structure", () => {
  it("creates scene with correct dimensions", () => {
    const scene = layout_to_paint([], { width: 800, height: 600 });
    expect(scene.width).toBe(800);
    expect(scene.height).toBe(600);
  });

  it("applies devicePixelRatio to dimensions", () => {
    const scene = layout_to_paint([], { width: 400, height: 300, devicePixelRatio: 2.0 });
    expect(scene.width).toBe(800);
    expect(scene.height).toBe(600);
  });

  it("defaults to dpr=1", () => {
    const scene = layout_to_paint([], { width: 100, height: 100 });
    expect(scene.width).toBe(100);
  });

  it("transparent background by default", () => {
    const scene = layout_to_paint([], { width: 100, height: 100 });
    expect(scene.background).toBe("transparent");
  });

  it("uses provided background color", () => {
    const scene = layout_to_paint([], { width: 100, height: 100, background: rgb(255, 255, 255) });
    expect(scene.background).toBe("rgba(255,255,255,1)");
  });

  it("empty nodes → empty instructions", () => {
    const scene = layout_to_paint([], { width: 100, height: 100 });
    expect(scene.instructions).toHaveLength(0);
  });
});

// ============================================================================
// Text content → PaintGlyphRun
// ============================================================================

describe("text nodes", () => {
  it("emits a PaintGlyphRun for text content", () => {
    const scene = layout_to_paint([textNode(0, 0, 100, 20, "Hi")], { width: 100, height: 20 });
    expect(scene.instructions).toHaveLength(1);
    const run = scene.instructions[0] as PaintGlyphRun;
    expect(run.kind).toBe("glyph_run");
  });

  it("empty text does not emit instructions", () => {
    const scene = layout_to_paint([textNode(0, 0, 100, 20, "")], { width: 100, height: 20 });
    expect(scene.instructions).toHaveLength(0);
  });

  it("glyph count matches text length", () => {
    const scene = layout_to_paint([textNode(0, 0, 100, 20, "Hello")], { width: 100, height: 20 });
    const run = scene.instructions[0] as PaintGlyphRun;
    expect(run.glyphs).toHaveLength(5);
  });

  it("glyph_id is Unicode code point of character", () => {
    const scene = layout_to_paint([textNode(0, 0, 100, 20, "A")], { width: 100, height: 20 });
    const run = scene.instructions[0] as PaintGlyphRun;
    expect(run.glyphs[0].glyph_id).toBe("A".codePointAt(0)); // 65
  });

  it("glyph x positions increase monotonically", () => {
    const scene = layout_to_paint([textNode(0, 0, 200, 20, "ABC")], { width: 200, height: 20 });
    const run = scene.instructions[0] as PaintGlyphRun;
    expect(run.glyphs[1].x).toBeGreaterThan(run.glyphs[0].x);
    expect(run.glyphs[2].x).toBeGreaterThan(run.glyphs[1].x);
  });

  it("applies dpr to glyph positions", () => {
    const scene1 = layout_to_paint([textNode(10, 5, 100, 20, "A")], { width: 100, height: 20, devicePixelRatio: 1.0 });
    const scene2 = layout_to_paint([textNode(10, 5, 100, 20, "A")], { width: 100, height: 20, devicePixelRatio: 2.0 });
    const run1 = scene1.instructions[0] as PaintGlyphRun;
    const run2 = scene2.instructions[0] as PaintGlyphRun;
    expect(run2.glyphs[0].x).toBeCloseTo(run1.glyphs[0].x * 2);
  });

  it("fill color comes from text content color", () => {
    const node: PositionedNode = {
      ...textNode(0, 0, 100, 20, "X"),
      content: { kind: "text", value: "X", font, color: red, maxLines: null, textAlign: "start" },
    };
    const scene = layout_to_paint([node], { width: 100, height: 20 });
    const run = scene.instructions[0] as PaintGlyphRun;
    expect(run.fill).toBe("rgba(255,0,0,1)");
  });

  it("font_ref includes family and weight", () => {
    const scene = layout_to_paint([textNode(0, 0, 100, 20, "X")], { width: 100, height: 20 });
    const run = scene.instructions[0] as PaintGlyphRun;
    expect(run.font_ref).toContain("Arial");
    expect(run.font_ref).toContain("400");
  });

  it("italic fonts prepend 'italic' to font_ref", () => {
    const italicFont = { ...font, italic: true };
    const node: PositionedNode = {
      ...textNode(0, 0, 100, 20, "X"),
      content: { kind: "text", value: "X", font: italicFont, color: black, maxLines: null, textAlign: "start" },
    };
    const scene = layout_to_paint([node], { width: 100, height: 20 });
    const run = scene.instructions[0] as PaintGlyphRun;
    expect(run.font_ref).toMatch(/^italic /);
  });

  it("font_size is font.size × dpr", () => {
    const scene = layout_to_paint([textNode(0, 0, 100, 20, "X")], {
      width: 100, height: 20, devicePixelRatio: 2.0
    });
    const run = scene.instructions[0] as PaintGlyphRun;
    expect(run.font_size).toBe(32); // 16 × 2
  });
});

// ============================================================================
// TXT03d — textEmitMode: "text" emits PaintText instead of PaintGlyphRun
// ============================================================================

describe("textEmitMode \"text\" (canvas-native path)", () => {
  it("emits a PaintText, not a PaintGlyphRun, for text content", () => {
    const scene = layout_to_paint(
      [textNode(10, 5, 100, 20, "Hello")],
      { width: 100, height: 20, textEmitMode: "text" },
    );
    expect(scene.instructions).toHaveLength(1);
    const t = scene.instructions[0] as PaintText;
    expect(t.kind).toBe("text");
    expect(t.text).toBe("Hello");
  });

  it("emits exactly one PaintText per TextContent node (no per-glyph splitting)", () => {
    const scene = layout_to_paint(
      [textNode(0, 0, 200, 20, "Hello world")],
      { width: 200, height: 20, textEmitMode: "text" },
    );
    const texts = scene.instructions.filter((i) => i.kind === "text");
    expect(texts).toHaveLength(1);
  });

  it("font_ref uses canvas: scheme with family@size:weight", () => {
    const scene = layout_to_paint(
      [textNode(0, 0, 100, 20, "X")],
      { width: 100, height: 20, textEmitMode: "text" },
    );
    const t = scene.instructions[0] as PaintText;
    expect(t.font_ref).toMatch(/^canvas:[A-Za-z][A-Za-z0-9 ]*@\d+:\d+$/);
    expect(t.font_ref).toContain("Arial");
    expect(t.font_ref).toContain("@16");
    expect(t.font_ref).toContain(":400");
  });

  it("italic font appends :italic to font_ref", () => {
    const italicFont = { ...font, italic: true };
    const node: PositionedNode = {
      ...textNode(0, 0, 100, 20, "X"),
      content: { kind: "text", value: "X", font: italicFont, color: black, maxLines: null, textAlign: "start" },
    };
    const scene = layout_to_paint([node], { width: 100, height: 20, textEmitMode: "text" });
    const t = scene.instructions[0] as PaintText;
    expect(t.font_ref).toMatch(/:italic$/);
  });

  it("fill color comes from text content color", () => {
    const node: PositionedNode = {
      ...textNode(0, 0, 100, 20, "X"),
      content: { kind: "text", value: "X", font, color: red, maxLines: null, textAlign: "start" },
    };
    const scene = layout_to_paint([node], { width: 100, height: 20, textEmitMode: "text" });
    const t = scene.instructions[0] as PaintText;
    expect(t.fill).toBe("rgba(255,0,0,1)");
  });

  it("position and font_size scale with dpr", () => {
    const scene = layout_to_paint(
      [textNode(10, 5, 100, 20, "X")],
      { width: 100, height: 20, devicePixelRatio: 2.0, textEmitMode: "text" },
    );
    const t = scene.instructions[0] as PaintText;
    expect(t.x).toBe(20); // 10 × 2
    expect(t.font_size).toBe(32); // 16 × 2
  });

  it("empty text does not emit a PaintText", () => {
    const scene = layout_to_paint(
      [textNode(0, 0, 100, 20, "")],
      { width: 100, height: 20, textEmitMode: "text" },
    );
    expect(scene.instructions).toHaveLength(0);
  });

  it("default textEmitMode is glyph_run (backward compat)", () => {
    const scene = layout_to_paint(
      [textNode(0, 0, 100, 20, "X")],
      { width: 100, height: 20 },
    );
    expect(scene.instructions[0].kind).toBe("glyph_run");
  });
});

// ============================================================================
// Image content → PaintImage
// ============================================================================

describe("image nodes", () => {
  it("emits a PaintImage for image content", () => {
    const scene = layout_to_paint([imageNode(0, 0, 100, 80, "img.png")], { width: 100, height: 80 });
    expect(scene.instructions).toHaveLength(1);
    const img = scene.instructions[0] as PaintImage;
    expect(img.kind).toBe("image");
  });

  it("image src passes through unchanged", () => {
    const scene = layout_to_paint([imageNode(0, 0, 100, 80, "data:image/png;base64,abc")], { width: 100, height: 80 });
    const img = scene.instructions[0] as PaintImage;
    expect(img.src).toBe("data:image/png;base64,abc");
  });

  it("image dimensions apply dpr", () => {
    const scene = layout_to_paint([imageNode(10, 20, 100, 80, "img.png")], {
      width: 300, height: 200, devicePixelRatio: 2.0
    });
    const img = scene.instructions[0] as PaintImage;
    expect(img.x).toBe(20);
    expect(img.y).toBe(40);
    expect(img.width).toBe(200);
    expect(img.height).toBe(160);
  });
});

// ============================================================================
// ext["paint"] — background, border, opacity
// ============================================================================

describe("ext paint decoration", () => {
  it("backgroundColor emits a PaintRect before content", () => {
    const node = containerNode(0, 0, 200, 100, [], {
      paint: { backgroundColor: blue }
    });
    const scene = layout_to_paint([node], { width: 200, height: 100 });
    expect(scene.instructions).toHaveLength(1);
    const rect = scene.instructions[0] as PaintRect;
    expect(rect.kind).toBe("rect");
    expect(rect.fill).toBe("rgba(0,0,255,1)");
  });

  it("background rect covers full node bounds", () => {
    const node = containerNode(10, 20, 150, 80, [], {
      paint: { backgroundColor: blue }
    });
    const scene = layout_to_paint([node], { width: 200, height: 200 });
    const rect = scene.instructions[0] as PaintRect;
    expect(rect.x).toBe(10);
    expect(rect.y).toBe(20);
    expect(rect.width).toBe(150);
    expect(rect.height).toBe(80);
  });

  it("borderWidth + borderColor emits a stroked PaintRect", () => {
    const node = containerNode(0, 0, 100, 50, [], {
      paint: { borderWidth: 2, borderColor: red }
    });
    const scene = layout_to_paint([node], { width: 200, height: 100 });
    const border = scene.instructions[0] as PaintRect;
    expect(border.kind).toBe("rect");
    expect(border.stroke).toBe("rgba(255,0,0,1)");
    expect(border.stroke_width).toBe(2);
    expect(border.fill).toBeUndefined();
  });

  it("cornerRadius on border rect", () => {
    const node = containerNode(0, 0, 100, 50, [], {
      paint: { borderWidth: 1, borderColor: red, cornerRadius: 6 }
    });
    const scene = layout_to_paint([node], { width: 100, height: 100 });
    const border = scene.instructions[0] as PaintRect;
    expect(border.corner_radius).toBe(6);
  });

  it("cornerRadius on background rect", () => {
    const node = containerNode(0, 0, 100, 50, [], {
      paint: { backgroundColor: blue, cornerRadius: 8 }
    });
    const scene = layout_to_paint([node], { width: 100, height: 100 });
    const rect = scene.instructions[0] as PaintRect;
    expect(rect.corner_radius).toBe(8);
  });

  it("opacity wraps node in a PaintLayer", () => {
    const node = containerNode(0, 0, 100, 50, [], {
      paint: { opacity: 0.5 }
    });
    const scene = layout_to_paint([node], { width: 100, height: 100 });
    expect(scene.instructions).toHaveLength(1);
    const layer = scene.instructions[0] as PaintLayer;
    expect(layer.kind).toBe("layer");
    expect(layer.opacity).toBe(0.5);
  });

  it("full opacity (1.0) does NOT create a layer", () => {
    const node = containerNode(0, 0, 100, 50, [], {
      paint: { opacity: 1.0 }
    });
    const scene = layout_to_paint([node], { width: 100, height: 100 });
    // No instructions emitted since no background/content/children
    expect(scene.instructions).toHaveLength(0);
  });
});

// ============================================================================
// Children and coordinate accumulation
// ============================================================================

describe("children positioning", () => {
  it("child absolute position = parent pos + child pos", () => {
    const child = textNode(5, 10, 50, 20, "A");
    const parent = containerNode(20, 30, 100, 100, [child]);
    const scene = layout_to_paint([parent], { width: 200, height: 200 });
    const run = scene.instructions[0] as PaintGlyphRun;
    expect(run.kind).toBe("glyph_run");
    // absX = 20+5=25, absY baseline = (30+10+16*0.8) = 52.8
    expect(run.glyphs[0].x).toBeCloseTo(25);
    expect(run.glyphs[0].y).toBeCloseTo(52.8);
  });

  it("multiple children produce multiple instructions in order", () => {
    const c1 = textNode(0, 0, 50, 20, "A");
    const c2 = textNode(50, 0, 50, 20, "B");
    const parent = containerNode(0, 0, 100, 20, [c1, c2]);
    const scene = layout_to_paint([parent], { width: 100, height: 20 });
    expect(scene.instructions).toHaveLength(2);
    const run1 = scene.instructions[0] as PaintGlyphRun;
    const run2 = scene.instructions[1] as PaintGlyphRun;
    // First child at x=0, second at x=50
    expect(run1.glyphs[0].x).toBeCloseTo(0);
    expect(run2.glyphs[0].x).toBeCloseTo(50);
  });

  it("nested children accumulate correctly", () => {
    const grandchild = textNode(5, 5, 30, 20, "X");
    const child = containerNode(10, 10, 80, 80, [grandchild]);
    const parent = containerNode(20, 20, 120, 120, [child]);
    const scene = layout_to_paint([parent], { width: 200, height: 200 });
    const run = scene.instructions[0] as PaintGlyphRun;
    // absX = 20+10+5 = 35
    expect(run.glyphs[0].x).toBeCloseTo(35);
  });
});

// ============================================================================
// cornerRadius clips children
// ============================================================================

describe("cornerRadius clip", () => {
  it("cornerRadius wraps children in a PaintClip", () => {
    const child = textNode(5, 5, 40, 20, "X");
    const parent = containerNode(0, 0, 100, 100, [child], {
      paint: { cornerRadius: 8 }
    });
    const scene = layout_to_paint([parent], { width: 100, height: 100 });
    const last = scene.instructions[scene.instructions.length - 1] as PaintClip;
    expect(last.kind).toBe("clip");
    expect(last.children).toHaveLength(1);
  });
});
