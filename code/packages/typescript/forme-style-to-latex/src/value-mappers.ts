/**
 * value-mappers.ts — `Color`, `Length`, `FontStack` → LaTeX literals.
 *
 * LaTeX-specific quirks the IR has to defer to:
 *
 *   - **Color models.**  `xcolor` understands `rgb`, `RGB` (0–255),
 *     `cmyk`, `gray`, `HTML`, and a small set of named colors.  Our
 *     IR uses 0–255 `rgb`, so we pick `xcolor`'s `RGB` model (the
 *     uppercase form — lowercase `rgb` expects 0–1 floats).
 *   - **HSL / OKLCH** aren't first-class in `xcolor`.  We approximate
 *     by converting HSL→RGB inline (lossless for our v0 use) and
 *     warning on OKLCH (round-trip through sRGB is lossy and out of
 *     scope here).  Named CSS colors fall back to `xcolor`'s built-in
 *     name table when it has a match, else warn-and-skip.
 *   - **Length units.**  LaTeX natively supports `pt`, `mm`, `cm`,
 *     `in`, `ex`, `em`.  We pass these through.  `px` converts to
 *     `pt` at the CSS standard (1px = 0.75pt; 96px = 72pt = 1in).
 *     `rem` becomes `em` (CSS rem = root em; LaTeX em = current
 *     font em — close enough for v0 — caller can override the
 *     document's root font size to compensate).  `vh`/`vw`/`%`/`ch`
 *     warn-and-skip — they need page-geometry context LaTeX doesn't
 *     expose at the preamble level.
 *   - **Font stacks.**  We emit the first family as `\setmainfont{...}`
 *     (XeLaTeX / LuaLaTeX with `fontspec`).  Fallbacks are dropped
 *     with a comment — LaTeX has no native fallback chain.  Caller
 *     is expected to use a font that's installed (validator can't
 *     check that).
 *   - **Shadow.**  LaTeX has no native drop-shadow.  We warn-and-skip
 *     at the property level rather than fake it with TikZ (which
 *     pulls a heavy dependency).
 *
 * All string interpolation paths route through `escape.ts` first.
 *
 * @module value-mappers
 */

import type { Color, FontStack, Length } from "@coding-adventures/forme-style-ir";
import { escapeLatexText } from "./escape.js";

/** Conversion to fundamentally-LaTeX units.  null = no equivalent. */
const LATEX_UNIT_PASSTHROUGH = new Set(["pt", "mm", "in", "ex", "em"]);

/**
 * Format a `Color` as an `xcolor` model+spec pair, e.g.
 * `{RGB}{31,35,40}`.  Returns null when the color can't be expressed
 * (oklch with no inline conversion; named color that xcolor doesn't
 * recognise).
 *
 * Returned form is suitable for direct injection into
 * `\definecolor{<name>}<here>` or `\color<here>{<name>}`.
 */
export function colorToLatex(c: Color): string | null {
  switch (c.kind) {
    case "rgb": {
      // xcolor's `RGB` model takes 0–255 ints; we round to the
      // nearest integer to be safe.
      const r = clampInt(c.r), g = clampInt(c.g), b = clampInt(c.b);
      return `{RGB}{${r},${g},${b}}`;
    }
    case "hsl": {
      // Inline HSL→RGB.  Lossless within sRGB.
      const { r, g, b } = hslToRgb(c.h, c.s, c.l);
      return `{RGB}{${r},${g},${b}}`;
    }
    case "oklch":
      // Round-trip through CIE conversion is outside our scope.
      // Caller (property mapper) warn-skips at the call site.
      return null;
    case "named": {
      const safe = NAMED_COLORS.get(c.name.toLowerCase());
      return safe ?? null;
    }
  }
}

/**
 * Format a `Length` as a LaTeX dimension, e.g. `12pt`, `1em`.
 * Returns null for units LaTeX can't natively express (`vh`, `vw`,
 * `%`, `ch`); caller warn-skips.
 */
export function lengthToLatex(l: Length): string | null {
  if (LATEX_UNIT_PASSTHROUGH.has(l.unit)) {
    return `${num(l.value)}${l.unit}`;
  }
  if (l.unit === "px") {
    // 96px = 72pt → 1px = 0.75pt.
    const pt = l.value * 0.75;
    return `${num(pt)}pt`;
  }
  if (l.unit === "rem") {
    // CSS rem ≈ LaTeX em (root vs current font — see module docstring).
    return `${num(l.value)}em`;
  }
  // %, vh, vw, ch — no LaTeX equivalent without page-geometry context.
  return null;
}

/**
 * Format a `FontStack` as the *first* family's name (escaped for
 * LaTeX text-mode).  Fallbacks are dropped — LaTeX has no native
 * fallback chain.  The returned string is suitable for use inside
 * `\setmainfont{...}` (fontspec).
 *
 * Returns null on an empty stack (defensive — the validator should
 * have rejected it).
 */
export function fontStackToLatex(stack: FontStack): string | null {
  if (stack.length === 0) return null;
  return escapeLatexText(stack[0]!);
}

/**
 * Format the *complete* font stack as a comment listing each
 * fallback.  Useful for traceability: `% font-fallbacks: Inter,
 * system-ui, sans-serif`.
 */
export function fontStackFallbacksComment(stack: FontStack): string {
  if (stack.length <= 1) return "";
  return `% font-fallbacks: ${stack.slice(1).map(escapeLatexText).join(", ")}`;
}

// ─── Helpers ─────────────────────────────────────────────────────────────

/** Format a number with no trailing `.0` and bounded precision. */
function num(n: number): string {
  if (Number.isInteger(n)) return String(n);
  // 4 decimals is more than enough for typography; avoids
  // `0.10000000000000001`-style float noise in output.
  return Number(n.toFixed(4)).toString();
}

function clampInt(x: number): number {
  return Math.max(0, Math.min(255, Math.round(x)));
}

/**
 * HSL → RGB (8-bit per channel).  Algorithm from CSS Color L3 / W3C
 * "Algorithm for converting HSL to RGB".
 */
function hslToRgb(h: number, s: number, l: number): { r: number; g: number; b: number } {
  const sFrac = s / 100;
  const lFrac = l / 100;
  const k = (n: number) => (n + h / 30) % 12;
  const a = sFrac * Math.min(lFrac, 1 - lFrac);
  const f = (n: number) => lFrac - a * Math.max(-1, Math.min(k(n) - 3, Math.min(9 - k(n), 1)));
  return {
    r: clampInt(f(0) * 255),
    g: clampInt(f(8) * 255),
    b: clampInt(f(4) * 255),
  };
}

/**
 * A small subset of CSS named colors that `xcolor` recognises
 * natively (the `dvipsnames` / `svgnames` option packages cover
 * more, but require user-side `\usepackage` config we don't want
 * to assume).  The values we emit are pre-resolved RGB triples
 * so the output works with a bare `\usepackage{xcolor}`.
 *
 * Lowercase keys for case-insensitive lookup.
 */
const NAMED_COLORS: ReadonlyMap<string, string> = new Map([
  ["black",       "{RGB}{0,0,0}"],
  ["white",       "{RGB}{255,255,255}"],
  ["red",         "{RGB}{255,0,0}"],
  ["green",       "{RGB}{0,128,0}"],
  ["blue",        "{RGB}{0,0,255}"],
  ["yellow",      "{RGB}{255,255,0}"],
  ["cyan",        "{RGB}{0,255,255}"],
  ["magenta",     "{RGB}{255,0,255}"],
  ["gray",        "{RGB}{128,128,128}"],
  ["grey",        "{RGB}{128,128,128}"],
  ["orange",      "{RGB}{255,165,0}"],
  ["purple",      "{RGB}{128,0,128}"],
  ["pink",        "{RGB}{255,192,203}"],
  ["brown",       "{RGB}{165,42,42}"],
  ["tomato",      "{RGB}{255,99,71}"],
  ["transparent", "{RGB}{255,255,255}"],   // approximation; LaTeX has no alpha
]);
