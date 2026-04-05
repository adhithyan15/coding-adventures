/**
 * Estimated Text Measurer
 *
 * A fast, zero-dependency, font-independent text measurer that uses a fixed
 * character-width model. It does not load fonts, call any OS API, or perform
 * real glyph measurement. Instead it uses a single configurable multiplier to
 * estimate text dimensions from string length and font size.
 *
 * When to use this measurer
 * -------------------------
 *
 * - **CI and unit tests** — deterministic results regardless of installed fonts.
 *   Two runs on different machines always produce identical layout output.
 *
 * - **Server-side layout** — when rendering on a server (e.g. generating a PDF
 *   layout plan) and pixel-perfect accuracy is not required. Very fast.
 *
 * - **Headless environments** — environments with no browser, no Canvas API,
 *   and no font rendering stack.
 *
 * - **First-pass progressive rendering** — compute an approximate layout
 *   instantly; then re-run with an accurate measurer once fonts load.
 *
 * When NOT to use this measurer
 * ------------------------------
 *
 * - Browser Canvas rendering where glyph-accurate text wrap matters.
 * - PDF generation where line-break positions must match print output.
 * - Any application where text overflow or truncation must be pixel-exact.
 *
 * The model
 * ---------
 *
 * The estimated width of a string is:
 *
 *   estimated_width = text.length × font.size × avgCharWidthMultiplier
 *
 * The estimated height (single line) is:
 *
 *   estimated_height = font.size × font.lineHeight
 *
 * For multi-line text (when maxWidth is provided):
 *
 *   chars_per_line = floor(maxWidth / (font.size × avgCharWidthMultiplier))
 *   line_count     = ceil(text.length / chars_per_line)
 *   height         = line_count × font.size × font.lineHeight
 *
 * The default multiplier of **0.6** is a reasonable approximation for
 * proportional Latin fonts at typical weights. It undercounts for wide
 * characters (W, M) and overcounts for narrow characters (i, l, 1).
 * For a monospaced font, a multiplier of 0.6 is still a reasonable average.
 *
 * You can tune the multiplier at construction time:
 *
 *   const m = createEstimatedMeasurer({ avgCharWidthMultiplier: 0.5 });
 *
 * Trade-offs
 * ----------
 *
 * | Property       | Value                                  |
 * |----------------|----------------------------------------|
 * | Accuracy       | Low — approximation only               |
 * | Speed          | Very fast — O(n) string length         |
 * | Dependencies   | None                                   |
 * | Platform       | All (pure math)                        |
 * | Determinism    | Yes — same result everywhere           |
 */

import type { TextMeasurer, FontSpec, MeasureResult } from "@coding-adventures/layout-ir";

// ============================================================================
// Options
// ============================================================================

/**
 * Constructor options for the estimated measurer.
 */
export interface EstimatedMeasurerOptions {
  /**
   * Average character width as a multiplier of font size.
   *
   * Default: 0.6 — a reasonable approximation for proportional Latin fonts.
   *
   * Practical calibration:
   *   - 0.5 → narrow/condensed fonts or text dominated by narrow chars
   *   - 0.6 → typical body text (default)
   *   - 0.65 → wide/expanded fonts or text dominated by wide chars
   *   - 1.0 → monospaced font (one char = one em)
   */
  avgCharWidthMultiplier?: number;
}

// ============================================================================
// Implementation
// ============================================================================

/**
 * Create an estimated text measurer.
 *
 * Usage:
 *
 *   const measurer = createEstimatedMeasurer();
 *   const result = measurer.measure("Hello world", font, 200);
 *   // result.width ≈ length("Hello world") × font.size × 0.6
 *   // result.lineCount = 1 or more depending on maxWidth
 *
 * The returned `TextMeasurer` is immutable — the multiplier is fixed at
 * construction time. To use different multipliers, create multiple instances.
 */
export function createEstimatedMeasurer(
  opts: EstimatedMeasurerOptions = {}
): TextMeasurer {
  const multiplier = opts.avgCharWidthMultiplier ?? 0.6;

  if (multiplier <= 0) {
    throw new RangeError(
      `avgCharWidthMultiplier must be > 0, got ${multiplier}`
    );
  }

  return {
    measure(
      text: string,
      font: FontSpec,
      maxWidth: number | null
    ): MeasureResult {
      // Edge case: empty string always has zero dimensions.
      if (text.length === 0) {
        return { width: 0, height: 0, lineCount: 0 };
      }

      const charWidth = font.size * multiplier;
      const lineHeight = font.size * font.lineHeight;

      if (maxWidth === null || maxWidth === Infinity) {
        // Single-line: no wrapping.
        return {
          width: text.length * charWidth,
          height: lineHeight,
          lineCount: 1,
        };
      }

      // Multi-line: estimate how many characters fit per line.
      // If maxWidth is so small that no character fits, treat each character
      // as its own line (worst case).
      const charsPerLine = charWidth > 0
        ? Math.max(1, Math.floor(maxWidth / charWidth))
        : 1;

      const lineCount = Math.ceil(text.length / charsPerLine);

      // Width of the widest line. For the last (possibly shorter) line,
      // calculate its character count:
      const lastLineChars = text.length % charsPerLine || charsPerLine;
      const fullLineWidth = Math.min(charsPerLine * charWidth, maxWidth);
      const lastLineWidth = lastLineChars * charWidth;

      // Width of the whole block = widest line (full lines are all the same width).
      const width = lineCount === 1
        ? lastLineWidth
        : Math.max(fullLineWidth, lastLineWidth);

      return {
        width,
        height: lineCount * lineHeight,
        lineCount,
      };
    },
  };
}
