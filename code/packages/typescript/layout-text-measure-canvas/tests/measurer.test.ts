/**
 * Tests for the Canvas text measurer.
 *
 * Because this package targets browser environments where real Canvas font
 * measurement depends on installed fonts, the tests use a **mock** Canvas
 * context that returns predictable, deterministic measurements.
 *
 * The mock reports width as `text.length × font.size × 0.6` — the same model
 * as `layout-text-measure-estimated`. This lets us verify the wrapping logic
 * and line-count computation without needing real browser fonts.
 *
 * The goal is to test the measurer's *structure and logic* (font CSS string
 * construction, word wrapping, line counting, height accumulation) — not the
 * pixel accuracy of the underlying Canvas API, which is a browser concern.
 */

import { describe, it, expect, beforeEach } from "vitest";
import {
  createCanvasMeasurer,
  fontSpecToCss,
} from "../src/index.js";
import type { CanvasContext2D, TextMetricsLike } from "../src/index.js";
import type { FontSpec } from "@coding-adventures/layout-ir";

// ============================================================================
// Mock canvas context
// ============================================================================

/**
 * Creates a deterministic mock that measures text width as:
 *   width = text.length × fontSizeFromCssString × 0.6
 *
 * It parses the `ctx.font` string to extract the size.
 */
function makeMockCtx(): CanvasContext2D & { lastFontSet: string } {
  let currentFont = "";

  const ctx = {
    get font() { return currentFont; },
    set font(v: string) { currentFont = v; },
    get lastFontSet() { return currentFont; },
    measureText(text: string): TextMetricsLike {
      // Parse size from font string like "400 16px 'Arial'" or "italic 700 12px 'Mono'"
      const match = currentFont.match(/(\d+(?:\.\d+)?)px/);
      const size = match ? parseFloat(match[1]) : 16;
      const width = text.length * size * 0.6;
      const ascent = size * 0.8;
      const descent = size * 0.2;
      return { width, actualBoundingBoxAscent: ascent, actualBoundingBoxDescent: descent };
    },
  } as CanvasContext2D & { lastFontSet: string };
  return ctx;
}

const baseFont: FontSpec = {
  family: "Arial",
  size: 16,
  weight: 400,
  italic: false,
  lineHeight: 1.2,
};

// ============================================================================
// fontSpecToCss
// ============================================================================

describe("fontSpecToCss", () => {
  it("produces correct CSS string for regular font", () => {
    expect(fontSpecToCss(baseFont)).toBe("400 16px 'Arial'");
  });

  it("prepends 'italic' for italic fonts", () => {
    const italic: FontSpec = { ...baseFont, italic: true };
    expect(fontSpecToCss(italic)).toBe("italic 400 16px 'Arial'");
  });

  it("uses weight correctly", () => {
    const bold: FontSpec = { ...baseFont, weight: 700 };
    expect(fontSpecToCss(bold)).toBe("700 16px 'Arial'");
  });

  it("falls back to sans-serif when family is empty", () => {
    const f: FontSpec = { ...baseFont, family: "" };
    expect(fontSpecToCss(f)).toBe("400 16px 'sans-serif'");
  });

  it("handles bold italic", () => {
    const boldItalic: FontSpec = { ...baseFont, weight: 700, italic: true };
    expect(fontSpecToCss(boldItalic)).toBe("italic 700 16px 'Arial'");
  });

  it("formats fractional font sizes", () => {
    const f: FontSpec = { ...baseFont, size: 14.5 };
    expect(fontSpecToCss(f)).toBe("400 14.5px 'Arial'");
  });
});

// ============================================================================
// createCanvasMeasurer — single line
// ============================================================================

describe("createCanvasMeasurer — single line", () => {
  let ctx: CanvasContext2D;
  let measurer: ReturnType<typeof createCanvasMeasurer>;

  beforeEach(() => {
    ctx = makeMockCtx();
    measurer = createCanvasMeasurer(ctx);
  });

  it("empty string returns zero dimensions", () => {
    const r = measurer.measure("", baseFont, null);
    expect(r.width).toBe(0);
    expect(r.height).toBe(0);
    expect(r.lineCount).toBe(0);
  });

  it("single line width = text.length × size × 0.6 (mock)", () => {
    // "Hello" = 5 × 16 × 0.6 = 48
    const r = measurer.measure("Hello", baseFont, null);
    expect(r.width).toBeCloseTo(48);
    expect(r.lineCount).toBe(1);
  });

  it("sets ctx.font before measuring", () => {
    measurer.measure("X", baseFont, null);
    expect((ctx as any).lastFontSet).toBe("400 16px 'Arial'");
  });

  it("height uses actualBoundingBoxAscent + Descent", () => {
    // mock: ascent = 16×0.8 = 12.8, descent = 16×0.2 = 3.2 → total = 16
    const r = measurer.measure("X", baseFont, null);
    expect(r.height).toBeCloseTo(16);
  });

  it("maxWidth=Infinity treated as single line", () => {
    const r = measurer.measure("Hello world", baseFont, Infinity);
    expect(r.lineCount).toBe(1);
  });
});

// ============================================================================
// createCanvasMeasurer — multi-line
// ============================================================================

describe("createCanvasMeasurer — multi-line", () => {
  let measurer: ReturnType<typeof createCanvasMeasurer>;

  beforeEach(() => {
    measurer = createCanvasMeasurer(makeMockCtx());
  });

  it("text fitting on one line stays at lineCount=1", () => {
    // "Hello" = 5 × 9.6 = 48 wide; maxWidth=200 → fits
    const r = measurer.measure("Hello", baseFont, 200);
    expect(r.lineCount).toBe(1);
  });

  it("text wrapping across two lines", () => {
    // "Hello world" — mock measures by length × 9.6
    // "Hello" = 48, "world" = 48, " " separator = space
    // "Hello world" = 11 × 9.6 = 105.6 → exceeds 60
    // Should wrap: "Hello" on line 1, "world" on line 2
    const r = measurer.measure("Hello world", baseFont, 60);
    expect(r.lineCount).toBe(2);
  });

  it("height = lines × font.size × lineHeight", () => {
    const r = measurer.measure("Hello world", baseFont, 60);
    // 2 lines × 16 × 1.2 = 38.4
    expect(r.height).toBeCloseTo(38.4);
  });

  it("width = widest line width", () => {
    // "Hello" (5 chars = 48) and "world" (5 chars = 48) → max = 48
    const r = measurer.measure("Hello world", baseFont, 60);
    expect(r.width).toBeCloseTo(48);
  });

  it("three-word wrap", () => {
    // "A B C" — each word is 1 char wide (9.6 each)
    // maxWidth = 10 → each word fits alone, space would push next word to new line
    const r = measurer.measure("A B C", baseFont, 10);
    expect(r.lineCount).toBe(3);
  });

  it("empty string with maxWidth → zero result", () => {
    const r = measurer.measure("", baseFont, 100);
    expect(r.width).toBe(0);
    expect(r.height).toBe(0);
    expect(r.lineCount).toBe(0);
  });

  it("single word that fits exactly on one line", () => {
    // "Hello" = 5 × 9.6 = 48 exactly matches maxWidth
    const r = measurer.measure("Hello", baseFont, 48);
    expect(r.lineCount).toBe(1);
  });

  it("italics and bold still produce correct font strings", () => {
    const boldItalic: FontSpec = { ...baseFont, weight: 700, italic: true };
    const r = measurer.measure("Hi", boldItalic, 100);
    expect(r.lineCount).toBe(1);
    // Ensure the right font was applied — width is based on boldItalic size
    expect(r.width).toBeCloseTo(2 * 16 * 0.6);
  });
});

// ============================================================================
// Fallback when bounding box metrics not available
// ============================================================================

describe("createCanvasMeasurer — fallback height", () => {
  it("falls back to font.size × lineHeight when bounding box not available", () => {
    // Simulate a context that returns NaN for bounding box metrics
    const limitedCtx: CanvasContext2D = {
      font: "",
      measureText(text: string): TextMetricsLike {
        return {
          width: text.length * 9.6,
          actualBoundingBoxAscent: NaN,
          actualBoundingBoxDescent: NaN,
        };
      },
    };

    const m = createCanvasMeasurer(limitedCtx);
    // With NaN bounding box: height = font.size × lineHeight = 16 × 1.2 = 19.2
    const r = m.measure("Hello", baseFont, null);
    expect(r.height).toBeCloseTo(19.2);
  });
});
