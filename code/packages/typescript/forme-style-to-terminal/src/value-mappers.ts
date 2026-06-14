/**
 * value-mappers.ts — `Color` → ANSI SGR parameter triple.
 *
 * Terminals that support 24-bit truecolour ("xterm-direct" / "ansi
 * truecolor", widely available since ~2018) accept SGR sequences of
 * the form:
 *
 *   \x1b[38;2;R;G;Bm    — set foreground to RGB
 *   \x1b[48;2;R;G;Bm    — set background to RGB
 *
 * where R/G/B are 0–255 integers.  All four IR color models reduce
 * to an RGB triple at output time:
 *
 *   rgb   → pass through (clamped + rounded to 0–255 ints)
 *   hsl   → HSL→RGB conversion inline (lossless within sRGB)
 *   oklch → warn-skip for v0 (CIE round-trip out of scope)
 *   named → small built-in safe map of common CSS names → RGB
 *
 * Length and FontStack are not expressible in terminals — terminals
 * are a character grid (no pixels, no font choice from inside the
 * stream), so the property mappers handle them as warn-skips
 * without calling these functions.
 *
 * @module value-mappers
 */

import type { Color } from "@coding-adventures/forme-style-ir";

/**
 * Format a `Color` as an SGR parameter triple suitable for joining
 * into a `38;2;...` (foreground) or `48;2;...` (background) sequence.
 *
 * Returns a triple `[R, G, B]` of 0–255 integers — caller assembles
 * the full SGR string.  Returns null for OKLCH (out of scope v0)
 * or for named colors not in the safe map.
 */
export function colorToRgbTriple(c: Color): readonly [number, number, number] | null {
  switch (c.kind) {
    case "rgb":
      return [clampInt(c.r), clampInt(c.g), clampInt(c.b)];
    case "hsl":
      return hslToRgb(c.h, c.s, c.l);
    case "oklch":
      // CIE round-trip through sRGB is out of scope for v0.
      return null;
    case "named": {
      const safe = NAMED_COLORS.get(c.name.toLowerCase());
      return safe ?? null;
    }
  }
}

/**
 * Format the full SGR fragment for a foreground color, including
 * the `38;2;` prefix.  Returns null when `colorToRgbTriple` fails.
 *
 * Example: `colorToSgrFg({ kind: "rgb", r: 31, g: 35, b: 40 })`
 *          → `"38;2;31;35;40"`
 */
export function colorToSgrFg(c: Color): string | null {
  const triple = colorToRgbTriple(c);
  if (!triple) return null;
  return `38;2;${triple[0]};${triple[1]};${triple[2]}`;
}

/** Same as `colorToSgrFg` but for background (SGR `48;2;...`). */
export function colorToSgrBg(c: Color): string | null {
  const triple = colorToRgbTriple(c);
  if (!triple) return null;
  return `48;2;${triple[0]};${triple[1]};${triple[2]}`;
}

// ─── Helpers ─────────────────────────────────────────────────────────────

function clampInt(x: number): number {
  // `Math.round(NaN)` is NaN; `Math.max(0, NaN)` is NaN; cap with
  // `Number.isFinite` instead so NaN → 0 (defensive — validator
  // should reject, but we belt-and-brace).
  if (!Number.isFinite(x)) return 0;
  return Math.max(0, Math.min(255, Math.round(x)));
}

/**
 * HSL → RGB (8-bit per channel).  Algorithm from CSS Color L3 / W3C
 * "Algorithm for converting HSL to RGB".  Returns a `[r, g, b]`
 * triple of 0–255 integers.
 */
function hslToRgb(h: number, s: number, l: number): readonly [number, number, number] {
  const sFrac = s / 100;
  const lFrac = l / 100;
  const k = (n: number) => (n + h / 30) % 12;
  const a = sFrac * Math.min(lFrac, 1 - lFrac);
  const f = (n: number) => lFrac - a * Math.max(-1, Math.min(k(n) - 3, Math.min(9 - k(n), 1)));
  return [clampInt(f(0) * 255), clampInt(f(8) * 255), clampInt(f(4) * 255)];
}

/**
 * Small built-in safe map of CSS named colors → RGB triples.  Same
 * subset as `forme-style-to-latex`; expanding would require shipping
 * the full SVG / X11 name table (~140 entries) which is overkill for
 * the terminal use case.
 *
 * Lowercase keys for case-insensitive lookup.
 */
const NAMED_COLORS: ReadonlyMap<string, readonly [number, number, number]> = new Map([
  ["black",       [0, 0, 0]],
  ["white",       [255, 255, 255]],
  ["red",         [255, 0, 0]],
  ["green",       [0, 128, 0]],
  ["blue",        [0, 0, 255]],
  ["yellow",      [255, 255, 0]],
  ["cyan",        [0, 255, 255]],
  ["magenta",     [255, 0, 255]],
  ["gray",        [128, 128, 128]],
  ["grey",        [128, 128, 128]],
  ["orange",      [255, 165, 0]],
  ["purple",      [128, 0, 128]],
  ["pink",        [255, 192, 203]],
  ["brown",       [165, 42, 42]],
  ["tomato",      [255, 99, 71]],
  // Terminals can't render alpha; treat transparent as default
  // (we just don't emit a color — but the property mapper still
  // calls us, so map to white to keep behaviour predictable).
  ["transparent", [255, 255, 255]],
]);
