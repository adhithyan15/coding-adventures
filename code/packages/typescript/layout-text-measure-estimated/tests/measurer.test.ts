import { describe, it, expect } from "vitest";
import { createEstimatedMeasurer } from "../src/index.js";
import type { FontSpec } from "@coding-adventures/layout-ir";

// A standard font for most tests: 16px, lineHeight 1.2 → line = 19.2px tall
const font: FontSpec = {
  family: "Arial",
  size: 16,
  weight: 400,
  italic: false,
  lineHeight: 1.2,
};

describe("createEstimatedMeasurer", () => {
  it("creates a measurer with default multiplier", () => {
    const m = createEstimatedMeasurer();
    expect(m).toBeDefined();
    expect(typeof m.measure).toBe("function");
  });

  it("throws for non-positive multiplier", () => {
    expect(() => createEstimatedMeasurer({ avgCharWidthMultiplier: 0 })).toThrow(RangeError);
    expect(() => createEstimatedMeasurer({ avgCharWidthMultiplier: -1 })).toThrow(RangeError);
  });

  it("accepts custom multiplier", () => {
    const m = createEstimatedMeasurer({ avgCharWidthMultiplier: 0.5 });
    const result = m.measure("A", font, null);
    // 1 char × 16 × 0.5 = 8
    expect(result.width).toBe(8);
  });
});

describe("measure — single line (maxWidth null)", () => {
  const m = createEstimatedMeasurer(); // multiplier = 0.6

  it("empty string → zero dimensions, zero line count", () => {
    const r = m.measure("", font, null);
    expect(r.width).toBe(0);
    expect(r.height).toBe(0);
    expect(r.lineCount).toBe(0);
  });

  it("single character width = font.size × 0.6", () => {
    // "A" → 1 char × 16 × 0.6 = 9.6
    const r = m.measure("A", font, null);
    expect(r.width).toBeCloseTo(9.6);
  });

  it("longer string width scales linearly with length", () => {
    // "Hello" → 5 chars × 16 × 0.6 = 48
    const r = m.measure("Hello", font, null);
    expect(r.width).toBeCloseTo(48);
  });

  it("lineCount is 1 for single-line measurement", () => {
    const r = m.measure("Hello world", font, null);
    expect(r.lineCount).toBe(1);
  });

  it("height = font.size × lineHeight = 16 × 1.2 = 19.2", () => {
    const r = m.measure("Any text", font, null);
    expect(r.height).toBeCloseTo(19.2);
  });

  it("works with Infinity maxWidth (treated as no constraint)", () => {
    const r = m.measure("Hello", font, Infinity);
    expect(r.lineCount).toBe(1);
    expect(r.width).toBeCloseTo(48);
  });

  it("scales with different font sizes", () => {
    const bigFont: FontSpec = { ...font, size: 32 };
    const r = m.measure("Hi", bigFont, null);
    // 2 × 32 × 0.6 = 38.4
    expect(r.width).toBeCloseTo(38.4);
  });

  it("respects font lineHeight multiplier", () => {
    const tallFont: FontSpec = { ...font, lineHeight: 2.0 };
    const r = m.measure("Hi", tallFont, null);
    // height = 16 × 2.0 = 32
    expect(r.height).toBeCloseTo(32);
  });
});

describe("measure — multi-line (maxWidth provided)", () => {
  const m = createEstimatedMeasurer(); // multiplier = 0.6

  it("text that fits in one line stays at lineCount=1", () => {
    // "Hi" = 2 chars × 9.6 = 19.2 wide; maxWidth=200 → fits on one line
    const r = m.measure("Hi", font, 200);
    expect(r.lineCount).toBe(1);
  });

  it("text that wraps produces correct line count", () => {
    // charWidth = 16 × 0.6 = 9.6; charsPerLine = floor(96 / 9.6) = 10
    // "HelloWorld!" = 11 chars → ceil(11/10) = 2 lines
    const r = m.measure("HelloWorld!", font, 96);
    expect(r.lineCount).toBe(2);
  });

  it("height scales with line count", () => {
    // 2 lines × 16 × 1.2 = 38.4
    const r = m.measure("HelloWorld!", font, 96);
    expect(r.height).toBeCloseTo(38.4);
  });

  it("exactly fitting text → 1 line", () => {
    // 10 chars, maxWidth = 10 × 9.6 = 96 → exactly 1 line
    const r = m.measure("1234567890", font, 96);
    expect(r.lineCount).toBe(1);
  });

  it("very narrow maxWidth forces many lines", () => {
    // charWidth=9.6, maxWidth=9.6 → 1 char per line
    // "Hello" = 5 chars → 5 lines
    const r = m.measure("Hello", font, 9.6);
    expect(r.lineCount).toBe(5);
  });

  it("very very narrow (< charWidth) → 1 char per line (clamped)", () => {
    // maxWidth=1 < charWidth=9.6 → charsPerLine clamped to 1
    const r = m.measure("Hi", font, 1);
    expect(r.lineCount).toBe(2);
  });

  it("three equal lines", () => {
    // charWidth=9.6, maxWidth=96=10 chars/line
    // 30 chars → 3 lines
    const text = "a".repeat(30);
    const r = m.measure(text, font, 96);
    expect(r.lineCount).toBe(3);
    expect(r.height).toBeCloseTo(3 * 16 * 1.2);
  });

  it("empty string with maxWidth → zero dimensions", () => {
    const r = m.measure("", font, 100);
    expect(r.width).toBe(0);
    expect(r.height).toBe(0);
    expect(r.lineCount).toBe(0);
  });

  it("single character with tight maxWidth → 1 line", () => {
    const r = m.measure("X", font, 100);
    expect(r.lineCount).toBe(1);
    // width = 1 × 9.6 = 9.6
    expect(r.width).toBeCloseTo(9.6);
  });
});

describe("measure — custom multiplier", () => {
  it("multiplier 1.0 (monospaced): 1 char = 1 em wide", () => {
    const m = createEstimatedMeasurer({ avgCharWidthMultiplier: 1.0 });
    // "Code" = 4 chars × 16 × 1.0 = 64
    const r = m.measure("Code", font, null);
    expect(r.width).toBeCloseTo(64);
  });

  it("multiplier 0.5: narrower estimate", () => {
    const m = createEstimatedMeasurer({ avgCharWidthMultiplier: 0.5 });
    // "Code" = 4 × 16 × 0.5 = 32
    const r = m.measure("Code", font, null);
    expect(r.width).toBeCloseTo(32);
  });

  it("affects wrapping calculations proportionally", () => {
    const m = createEstimatedMeasurer({ avgCharWidthMultiplier: 1.0 });
    // charWidth = 16 × 1.0 = 16; maxWidth=32 → 2 chars/line
    // "ABCD" = 4 chars → 2 lines
    const r = m.measure("ABCD", font, 32);
    expect(r.lineCount).toBe(2);
  });
});

describe("measure — various fonts", () => {
  const m = createEstimatedMeasurer();

  it("small font: smaller width", () => {
    const smallFont: FontSpec = { ...font, size: 8 };
    const r = m.measure("Hello", smallFont, null);
    // 5 × 8 × 0.6 = 24
    expect(r.width).toBeCloseTo(24);
  });

  it("tall lineHeight: taller output", () => {
    const tallFont: FontSpec = { ...font, lineHeight: 3.0 };
    const r = m.measure("X", tallFont, null);
    // 16 × 3.0 = 48
    expect(r.height).toBeCloseTo(48);
  });

  it("zero-size font produces zero dimensions for non-empty string (no maxWidth)", () => {
    const zeroFont: FontSpec = { ...font, size: 0 };
    const r = m.measure("Hello", zeroFont, null);
    expect(r.width).toBe(0);
    expect(r.height).toBe(0);
  });

  it("zero-size font with maxWidth → falls back to 1 char per line", () => {
    const zeroFont: FontSpec = { ...font, size: 0 };
    // charWidth = 0 × 0.6 = 0, so the charWidth>0 branch is false → charsPerLine=1
    const r = m.measure("Hello", zeroFont, 100);
    expect(r.lineCount).toBe(5); // 5 chars, 1 per line
    expect(r.width).toBe(0);
  });
});
