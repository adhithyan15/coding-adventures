/**
 * property-mappers.ts — one or more SGR parameter strings per Style
 * IR property (FM04 §9.4).
 *
 * Each mapper returns a list of numeric SGR parameter strings (e.g.
 * `["1"]` for bold, `["38;2;31;35;40"]` for fg colour); the
 * translator joins them all with `;` and wraps in `\x1b[...m` per
 * rule.  Some properties contribute multiple SGR fragments (rare —
 * `text-decoration` may pair underline + colour).
 *
 * Most properties **warn-skip** — terminals are a character grid
 * with no concept of pixel layout, padding, borders, or page breaks.
 * What DOES survive:
 *
 *   color           → SGR 38;2;R;G;B (fg truecolour)
 *   background      → SGR 48;2;R;G;B (bg truecolour)
 *   font-weight     → SGR 1 (bold) when >= 600
 *   font-style      → SGR 3 (italic) when italic/oblique
 *   text-decoration → SGR 4 (underline) / 9 (strikethrough)
 *   visible: false  → SGR 8 (concealed)
 *
 * Everything else warn-skips with a documented message.  The
 * "important" flag has no terminal equivalent (terminals don't
 * cascade) and is honoured as a no-op comment trailer at the
 * translator level.
 *
 * @module property-mappers
 */

import type {
  StyleProperty, TokenSet,
} from "@coding-adventures/forme-style-ir";
import { colorToSgrFg, colorToSgrBg } from "./value-mappers.js";
import {
  resolveColor, resolveNumber,
} from "./token-resolver.js";

/** One emit: zero-or-more SGR fragments, or a warning. */
export type PropertyEmit =
  | { ok: true; sgr: readonly string[] }
  | { ok: false; warning: string };

/**
 * Map one `StyleProperty` to a list of SGR parameter strings.
 * Empty list IS a valid success (means "the rule is allowed to
 * exist but contributes no SGR" — caller treats it as no-op).
 */
export function propertyToTerminal(
  prop: StyleProperty,
  tokens: TokenSet,
): PropertyEmit {
  switch (prop.kind) {
    // ─── Color / fill ────────────────────────────────────────────────────
    case "color": {
      const c = resolveColor(prop.value, tokens);
      if (!c) return warn(`color: unresolved`);
      const sgr = colorToSgrFg(c);
      if (!sgr) return warn(`color: model not expressible as terminal RGB`);
      return ok([sgr]);
    }
    case "background": {
      const c = resolveColor(prop.value, tokens);
      if (!c) return warn(`background: unresolved`);
      const sgr = colorToSgrBg(c);
      if (!sgr) return warn(`background: model not expressible as terminal RGB`);
      return ok([sgr]);
    }
    case "border-color":
    case "outline-color":
      return warn(`${prop.kind}: no terminal equivalent (terminals have no border / outline model)`);

    // ─── Typography ──────────────────────────────────────────────────────
    case "font-family":
      return warn(`font-family: no terminal equivalent (terminal renders in its configured font)`);
    case "font-size":
      return warn(`font-size: no terminal equivalent (terminal uses its configured cell size)`);
    case "font-weight": {
      const n = resolveNumber(prop.value, tokens);
      if (n === null) return warn(`font-weight: unresolved`);
      // CSS weights: 400 is normal, 700 is bold; the common semantic
      // threshold is ≥600 for "bold-ish".  Below that, no SGR (normal).
      return n >= 600 ? ok(["1"]) : ok([]);
    }
    case "font-style":
      switch (prop.value) {
        case "italic":
        case "oblique": return ok(["3"]);
        case "normal":  return ok([]);
      }
      return warn(`font-style: unknown value ${JSON.stringify((prop as { value: unknown }).value)}`);
    case "text-transform":
      // ANSI has no text-transform — terminals don't reflow content
      // through a transform pipeline; the document's text bytes
      // arrive as-is.  Caller would have to pre-transform.
      return warn(`text-transform: no terminal equivalent (apply at document-content time)`);
    case "leading":
    case "tracking":
      return warn(`${prop.kind}: no terminal equivalent (line-height / letter-spacing not expressible at SGR level)`);
    case "text-decoration":
      switch (prop.value.line) {
        case "underline":    return ok(["4"]);
        case "line-through": return ok(["9"]);
        case "none":         return ok([]);
        case "overline":     return ok(["53"]);   // SGR 53 (overline) is in ECMA-48 + supported by most modern terminals.
      }
      return warn(`text-decoration: unknown line ${JSON.stringify((prop.value as { line: unknown }).line)}`);

    // ─── Layout / spacing (none survive) ─────────────────────────────────
    case "space-before":
    case "space-after":
    case "indent":
    case "padding":
    case "max-width":
    case "min-height":
    case "align":
    case "vertical-align":
      return warn(`${prop.kind}: no terminal equivalent (terminals don't have a layout box model)`);

    // ─── Decoration (none survive) ───────────────────────────────────────
    case "border":
    case "border-radius":
    case "shadow":
    case "opacity":
      return warn(`${prop.kind}: no terminal equivalent (decorative properties require a graphics-capable backend)`);

    // ─── Page break ──────────────────────────────────────────────────────
    case "column-break":
    case "page-break":
    case "widow-orphan":
      return warn(`${prop.kind}: no terminal equivalent (terminals scroll; there's no page concept)`);

    // ─── Visibility ──────────────────────────────────────────────────────
    case "display":
      // CSS `display: none` could map to SGR 8 (conceal), but the
      // CSS semantics are richer (also "remove from layout") and
      // collapsing the two is misleading.  Warn-skip; the consumer
      // should make the decision at content-emit time.
      return warn(`display: no terminal equivalent (use \`visible: false\` if you mean SGR conceal)`);
    case "visible":
      // SGR 8 = conceal; the text is still emitted (terminal SGR
      // doesn't suppress bytes), but it's rendered as invisible.
      return prop.value ? ok([]) : ok(["8"]);

    // ─── Extension namespace ─────────────────────────────────────────────
    default: {
      const k = (prop as { kind: string }).kind;
      return warn(`unhandled property kind ${JSON.stringify(k)}`);
    }
  }
}

// ─── Per-shape helpers ───────────────────────────────────────────────────

function ok(sgr: readonly string[]): PropertyEmit {
  return { ok: true, sgr };
}

function warn(message: string): PropertyEmit {
  return { ok: false, warning: message };
}
