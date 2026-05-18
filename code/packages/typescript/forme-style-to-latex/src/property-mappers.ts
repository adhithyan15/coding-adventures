/**
 * property-mappers.ts — one LaTeX style-command per Style IR property.
 *
 * The translator's `emitRule` calls `propertyToLatex` for each
 * property in a rule; the returned `commands` get concatenated
 * inside the rule's `\newcommand{\formeRule...}{...}` body.
 *
 * Mapping philosophy:
 *
 *   - **Map what LaTeX expresses naturally.**  Color, font size,
 *     leading, weight, page-break — these have clean LaTeX forms.
 *   - **Warn-and-skip what it doesn't.**  Shadow, opacity, layout
 *     primitives (max-width, min-height, padding, border-radius),
 *     display semantics — LaTeX either has no equivalent at all
 *     (drop-shadow, opacity) or requires runtime context the
 *     preamble can't provide (max-width depends on column geometry).
 *   - **Document any conversion fudges** (px→pt, rem→em) inline.
 *
 * Every emit goes through the escape helpers — no caller-controlled
 * data lands in output unescaped.
 *
 * The `important` flag on `StyleProperty` has no LaTeX equivalent
 * (LaTeX has no specificity to override).  We honour it as a comment
 * trailer for traceability.
 *
 * @module property-mappers
 */

import type {
  StyleProperty,
  Color, Length, FontStack, TokenSet,
} from "@coding-adventures/forme-style-ir";
import { colorToLatex, lengthToLatex, fontStackToLatex } from "./value-mappers.js";
import {
  resolveColor, resolveLength, resolveFontStack, resolveNumber,
} from "./token-resolver.js";

/**
 * One emit per property: either a LaTeX-command fragment or a
 * skip-with-warning.
 */
export type PropertyEmit =
  | { ok: true; commands: string }
  | { ok: false; warning: string };

/**
 * Map a `StyleProperty` to LaTeX style commands.  Each successful
 * emit returns a string of `\command{arg}` calls (no leading/trailing
 * whitespace — caller joins).  Failures return a warning string.
 */
export function propertyToLatex(
  prop: StyleProperty,
  tokens: TokenSet,
): PropertyEmit {
  switch (prop.kind) {
    // ─── Color / fill ────────────────────────────────────────────────────
    case "color": {
      const c = resolveColor(prop.value, tokens);
      if (!c) return warn(`color: unresolved`);
      const spec = colorToLatex(c);
      if (!spec) return warn(`color: model not expressible in xcolor`);
      // \color[<model>]{<spec>} is the inline form (no \definecolor needed).
      return ok(`\\color${spec}`);
    }
    case "background":
    case "border-color":
    case "outline-color":
      // No native LaTeX preamble equivalent.  Backgrounds/borders
      // require boxing machinery (\colorbox / \fbox) that's per-use,
      // not per-style.  Caller (document body) layers these as needed.
      return warn(`${prop.kind}: no LaTeX preamble equivalent (use \\colorbox / \\fbox at the call site)`);

    // ─── Typography ──────────────────────────────────────────────────────
    case "font-family": {
      const fs = resolveFontStack(prop.value, tokens);
      if (!fs) return warn(`font-family: unresolved`);
      const name = fontStackToLatex(fs);
      if (!name) return warn(`font-family: empty stack`);
      // `fontspec` is the modern XeLaTeX/LuaLaTeX way.  Caller is
      // responsible for `\usepackage{fontspec}` in their preamble.
      return ok(`\\setmainfont{${name}}`);
    }
    case "font-size": {
      const l = resolveLength(prop.value, tokens);
      if (!l) return warn(`font-size: unresolved`);
      const dim = lengthToLatex(l);
      if (!dim) return warn(`font-size: unit ${JSON.stringify(l.unit)} has no LaTeX dimension`);
      // \fontsize{<size>}{<leading>}\selectfont — we don't know
      // leading here, so use 1.2× size as a sensible default.  If
      // the rule also sets `leading`, it'll override via \linespread.
      const lead = leadingDefault(l);
      return ok(`\\fontsize{${dim}}{${lead}}\\selectfont`);
    }
    case "font-weight": {
      const n = resolveNumber(prop.value, tokens);
      if (n === null) return warn(`font-weight: unresolved`);
      // CSS weights → LaTeX \fontseries codes (NFSS).
      // 100–500 → m (medium), 600+ → b (bold), 800+ → bx (bold extended).
      const series = n >= 800 ? "bx" : n >= 600 ? "b" : "m";
      return ok(`\\fontseries{${series}}\\selectfont`);
    }
    case "font-style":
      // italic/oblique/normal → \fontshape (NFSS).
      // CSS values: "normal" | "italic" | "oblique".
      switch (prop.value) {
        case "italic":  return ok(`\\fontshape{it}\\selectfont`);
        case "oblique": return ok(`\\fontshape{sl}\\selectfont`);
        case "normal":  return ok(`\\fontshape{n}\\selectfont`);
      }
      return warn(`font-style: unknown value ${JSON.stringify((prop as { value: unknown }).value)}`);
    case "text-transform":
      // LaTeX has \MakeUppercase / \MakeLowercase but only as inline
      // wrappers, not declarative.  Emit a per-style toggle macro.
      switch (prop.value) {
        case "uppercase":  return ok(`\\let\\formeXform=\\MakeUppercase`);
        case "lowercase":  return ok(`\\let\\formeXform=\\MakeLowercase`);
        case "capitalize": return warn(`text-transform: "capitalize" has no LaTeX equivalent (would require per-word logic)`);
        case "none":       return ok(`\\let\\formeXform=\\relax`);
      }
      return warn(`text-transform: unknown value ${JSON.stringify((prop as { value: unknown }).value)}`);
    case "leading": {
      const n = resolveNumber(prop.value, tokens);
      if (n === null) return warn(`leading: unresolved`);
      // \linespread takes a multiplier (1.0 = single, 1.5 = 1.5× …).
      return ok(`\\linespread{${num(n)}}\\selectfont`);
    }
    case "tracking":
      // letter-spacing has no native LaTeX equivalent — `microtype`
      // provides `\letterspacing` but it's a heavy dependency we
      // don't want to require.  Warn.
      return warn(`tracking: requires the \`microtype\` package; install it and emit \\letterspacing manually`);
    case "text-decoration":
      switch (prop.value.line) {
        case "underline":    return ok(`\\let\\formeDecor=\\underline`);
        case "none":         return ok(`\\let\\formeDecor=\\relax`);
        case "line-through": return warn(`text-decoration: "line-through" requires the \`ulem\` package; emit \\sout{} manually`);
        case "overline":     return warn(`text-decoration: "overline" has no built-in LaTeX equivalent`);
      }
      return warn(`text-decoration: unknown line ${JSON.stringify((prop.value as { line: unknown }).line)}`);

    // ─── Layout / spacing ────────────────────────────────────────────────
    case "space-before":
      return lengthCommand("\\parskip", prop.value, tokens, "space-before");
    case "space-after":
      return lengthCommand("\\parskip", prop.value, tokens, "space-after");
    case "indent":
      return lengthCommand("\\parindent", prop.value, tokens, "indent");
    case "padding":
      // No analogue at the preamble level; \fboxsep is the closest
      // but it's tied to box machinery.  Skip.
      return warn(`padding: no preamble-level LaTeX equivalent (set \\fboxsep at the call site)`);
    case "max-width":
      return lengthCommand("\\linewidth", prop.value, tokens, "max-width");
    case "min-height":
      return warn(`min-height: no LaTeX equivalent (height is content-driven)`);
    case "align":
      // `start` / `end` are bidi-aware in the IR — we treat them as
      // left / right for LTR.  A future i18n layer can re-emit
      // contextually; v0 is LTR-only per FM04 §15.4.
      switch (prop.value) {
        case "start":   return ok(`\\raggedright`);
        case "end":     return ok(`\\raggedleft`);
        case "center":  return ok(`\\centering`);
        case "justify": return ok(`\\leftskip=0pt\\rightskip=0pt plus 1fil minus 1fil`);
      }
      return warn(`align: unknown value ${JSON.stringify((prop as { value: unknown }).value)}`);
    case "vertical-align":
      return warn(`vertical-align: no preamble-level LaTeX equivalent`);

    // ─── Decoration ──────────────────────────────────────────────────────
    case "border":
    case "border-radius":
    case "shadow":
    case "opacity":
      return warn(`${prop.kind}: no LaTeX preamble equivalent (decorative properties require TikZ or tcolorbox at the call site)`);

    // ─── Page break (print) ──────────────────────────────────────────────
    case "column-break":
      switch (prop.value) {
        case "before": return ok(`\\columnbreak`);
        case "after":  return ok(`\\columnbreak`);   // post-content; user places \columnbreak manually
        case "avoid":  return ok(`\\nobreak`);
      }
      return warn(`column-break: unknown value ${JSON.stringify((prop as { value: unknown }).value)}`);
    case "page-break":
      switch (prop.value) {
        case "before": return ok(`\\pagebreak`);
        case "after":  return ok(`\\pagebreak`);
        case "avoid":  return ok(`\\nopagebreak`);
      }
      return warn(`page-break: unknown value ${JSON.stringify((prop as { value: unknown }).value)}`);
    case "widow-orphan": {
      // LaTeX `\widowpenalty` / `\clubpenalty` take 0–10000.  We
      // scale the IR value (commonly 1–4) by 1000 for a sensible
      // mapping; the value 0 disables, 4 is maximum discouragement.
      const v = prop.value;
      if (typeof v !== "number" || !Number.isFinite(v)) return warn(`widow-orphan: non-numeric`);
      const penalty = Math.max(0, Math.min(10000, Math.round(v * 2500)));
      return ok(`\\widowpenalty=${penalty}\\clubpenalty=${penalty}`);
    }

    // ─── Visibility ──────────────────────────────────────────────────────
    case "display":
      // CSS display values don't map onto LaTeX's "every paragraph
      // is its own box" model.  Warn.
      return warn(`display: no LaTeX equivalent (LaTeX has no inline/block dichotomy at the preamble level)`);
    case "visible":
      return prop.value
        ? ok(`\\let\\formeVisible=\\relax`)
        : ok(`\\let\\formeVisible=\\hphantom`);

    // ─── Extension namespace ─────────────────────────────────────────────
    default: {
      const k = (prop as { kind: string }).kind;
      return warn(`unhandled property kind ${JSON.stringify(k)}`);
    }
  }
}

// ─── Per-shape helpers ───────────────────────────────────────────────────

function ok(commands: string): PropertyEmit {
  return { ok: true, commands };
}

function warn(message: string): PropertyEmit {
  return { ok: false, warning: message };
}

function lengthCommand(
  cmd: string,
  value: Length | { kind: "token-ref"; path: string },
  tokens: TokenSet,
  propName: string,
): PropertyEmit {
  const l = resolveLength(value, tokens);
  if (!l) return warn(`${propName}: unresolved`);
  const dim = lengthToLatex(l);
  if (!dim) return warn(`${propName}: unit ${JSON.stringify(l.unit)} has no LaTeX dimension`);
  return ok(`\\setlength{${cmd}}{${dim}}`);
}

/** Pick a sensible default LaTeX leading (1.2×) given a font size. */
function leadingDefault(size: Length): string {
  // Same unit as the input dimension so LaTeX doesn't promote to pt.
  if (size.unit === "px") {
    return `${num(size.value * 0.75 * 1.2)}pt`;
  }
  if (size.unit === "rem") {
    return `${num(size.value * 1.2)}em`;
  }
  return `${num(size.value * 1.2)}${size.unit}`;
}

function num(n: number): string {
  if (Number.isInteger(n)) return String(n);
  return Number(n.toFixed(4)).toString();
}

// Re-export for tests that want direct access without going through translate().
export type { Color, FontStack };
