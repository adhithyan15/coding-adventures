/**
 * Canvas Text Measurer
 *
 * Uses `CanvasRenderingContext2D.measureText()` for accurate browser font
 * measurements. Requires a live 2D canvas context — this measurer is
 * TypeScript/browser-only.
 *
 * How it works
 * ------------
 *
 * The measurer holds a reference to a `CanvasRenderingContext2D`. For each
 * `measure()` call:
 *
 *   1. Set `ctx.font` from the `FontSpec`:
 *        `ctx.font = "400 16px 'Arial'"`
 *   2. Call `ctx.measureText(text)` → `TextMetrics`
 *   3. Width = `metrics.width`
 *   4. Height = `metrics.actualBoundingBoxAscent + metrics.actualBoundingBoxDescent`
 *   5. For multi-line (maxWidth not null): binary-search word-wrap boundaries
 *      to find line count and total height.
 *
 * Font loading
 * ------------
 *
 * `measureText` returns inaccurate results if fonts are not loaded yet.
 * Create this measurer **after** `await document.fonts.ready`. That is the
 * caller's responsibility, not the measurer's.
 *
 * Example:
 *
 *   await document.fonts.ready;
 *   const canvas = document.createElement("canvas");
 *   const ctx = canvas.getContext("2d")!;
 *   const measurer = createCanvasMeasurer(ctx);
 *
 * Node.js
 * -------
 *
 * Works in Node.js with the `canvas` npm package:
 *
 *   import { createCanvas } from "canvas";
 *   const { ctx } = createCanvas(1, 1).getContext("2d");
 *
 * Trade-offs
 * ----------
 *
 * | Property       | Value                                          |
 * |----------------|------------------------------------------------|
 * | Accuracy       | High — uses actual browser font metrics        |
 * | Speed          | Moderate — measureText is fast but not free    |
 * | Dependencies   | Browser Canvas API                             |
 * | Platform       | Browser / Node.js with canvas package          |
 * | Determinism    | No — depends on installed fonts                |
 */

import type { TextMeasurer, FontSpec, MeasureResult } from "@coding-adventures/layout-ir";

// ============================================================================
// Canvas context interface
// ============================================================================

/**
 * Minimal interface for the Canvas 2D context used by this measurer.
 *
 * Using a structural interface (rather than importing `CanvasRenderingContext2D`
 * directly) keeps the package testable in environments where the full Canvas
 * type isn't available (e.g. Node.js without `@types/node`).
 */
export interface CanvasContext2D {
  font: string;
  measureText(text: string): TextMetricsLike;
}

/**
 * Minimal subset of the `TextMetrics` DOM interface.
 *
 * `width` — the advance width of the text in CSS pixels.
 * `actualBoundingBoxAscent` — distance above the baseline.
 * `actualBoundingBoxDescent` — distance below the baseline.
 */
export interface TextMetricsLike {
  readonly width: number;
  readonly actualBoundingBoxAscent: number;
  readonly actualBoundingBoxDescent: number;
  // Optional: populated by modern Chrome/Safari/Firefox. These are the font-
  // wide ascent/descent (constant per font+size) rather than the per-string
  // glyph envelope. Preferred for line-height and baseline math because
  // measuring "Hi" and "hi" against a font-wide ascent gives the same line
  // height — matching what browsers do internally.
  readonly fontBoundingBoxAscent?: number;
  readonly fontBoundingBoxDescent?: number;
}

// ============================================================================
// Implementation
// ============================================================================

/**
 * Build a CSS font string from a `FontSpec`.
 *
 * Format: `"<italic?> <weight> <size>px '<family>'"`.
 *
 * Examples:
 *   font_spec("Arial", 16)         → "400 16px 'Arial'"
 *   font_bold(font_spec("Arial"))  → "700 16px 'Arial'"
 *   font_italic(...)               → "italic 700 16px 'Arial'"
 *   font_spec("", 14)              → "400 14px sans-serif"   (empty → system default)
 */
export function fontSpecToCss(spec: FontSpec): string {
  const family = spec.family || "sans-serif";
  const italic = spec.italic ? "italic " : "";
  return `${italic}${spec.weight} ${spec.size}px '${family}'`;
}

/**
 * Measure a single line of text using the canvas context.
 *
 * Returns width and height in the same logical units as the font size
 * (assumes 1 CSS pixel = 1 logical unit at `devicePixelRatio = 1`).
 */
function measureSingleLine(
  ctx: CanvasContext2D,
  text: string,
  font: FontSpec
): { width: number; height: number } {
  ctx.font = fontSpecToCss(font);
  const metrics = ctx.measureText(text);
  // Prefer fontBoundingBox* — constant per font, so two words in the same font
  // report the same height regardless of which letters they contain. Using
  // actualBoundingBox* would make "Heading" (has 'd' ascender) taller than
  // "on" (x-height only), which misaligns their baselines on a shared line.
  const fontAscent = metrics.fontBoundingBoxAscent;
  const fontDescent = metrics.fontBoundingBoxDescent;
  const actualAscent = metrics.actualBoundingBoxAscent;
  const actualDescent = metrics.actualBoundingBoxDescent;

  const height =
    isFiniteNum(fontAscent) && isFiniteNum(fontDescent)
      ? fontAscent + fontDescent
      : isFiniteNum(actualAscent) && isFiniteNum(actualDescent)
        ? actualAscent + actualDescent
        : font.size * font.lineHeight; // last-resort fallback

  return { width: metrics.width, height };
}

function isFiniteNum(v: number | undefined): v is number {
  return typeof v === "number" && !isNaN(v) && isFinite(v);
}

/**
 * Word-wrap `text` to fit within `maxWidth` using binary search.
 *
 * Returns the array of wrapped lines. Each line is a non-empty string.
 *
 * Algorithm: greedy left-to-right word wrapping.
 *   1. Split text into "words" (space-separated tokens, keeping whitespace).
 *   2. Accumulate words onto the current line until adding the next word
 *      would exceed `maxWidth`.
 *   3. When overflow detected: push current line, start new line.
 *
 * This matches the behaviour of most text renderers for simple paragraph
 * text. It does not implement Unicode line-break opportunities or hyphenation.
 */
function wrapWords(
  ctx: CanvasContext2D,
  text: string,
  font: FontSpec,
  maxWidth: number
): string[] {
  ctx.font = fontSpecToCss(font);

  const words = text.split(" ");
  const lines: string[] = [];
  let current = "";

  for (const word of words) {
    const candidate = current ? `${current} ${word}` : word;
    const { width } = ctx.measureText(candidate);
    if (width > maxWidth && current) {
      lines.push(current);
      current = word;
    } else {
      current = candidate;
    }
  }
  if (current) lines.push(current);
  return lines;
}

/**
 * Create a Canvas-backed text measurer.
 *
 * The provided `ctx` is used only for measurement — it is never drawn to.
 *
 * Usage:
 *
 *   const canvas = document.createElement("canvas");
 *   const ctx = canvas.getContext("2d")!;
 *   const measurer = createCanvasMeasurer(ctx);
 *
 *   const font = font_spec("Helvetica", 16);
 *   const r = measurer.measure("Hello world", font, 200);
 *   // r.width — actual browser-measured width (font-accurate)
 *   // r.height — actual bounding box height
 *   // r.lineCount — number of wrapped lines
 */
export function createCanvasMeasurer(ctx: CanvasContext2D): TextMeasurer {
  return {
    measure(
      text: string,
      font: FontSpec,
      maxWidth: number | null
    ): MeasureResult {
      if (text.length === 0) {
        return { width: 0, height: 0, lineCount: 0 };
      }

      if (maxWidth === null || maxWidth === Infinity) {
        // Single-line: measure the full string.
        const { width, height } = measureSingleLine(ctx, text, font);
        return { width, height, lineCount: 1 };
      }

      // Multi-line: word-wrap and measure each line.
      const lines = wrapWords(ctx, text, font, maxWidth);
      if (lines.length === 0) {
        return { width: 0, height: 0, lineCount: 0 };
      }

      // Width = width of widest line.
      let maxLineWidth = 0;
      let totalHeight = 0;
      const lineHeightPx = font.size * font.lineHeight;

      for (const line of lines) {
        const { width } = measureSingleLine(ctx, line, font);
        if (width > maxLineWidth) maxLineWidth = width;
        totalHeight += lineHeightPx;
      }

      return {
        width: maxLineWidth,
        height: totalHeight,
        lineCount: lines.length,
      };
    },
  };
}
