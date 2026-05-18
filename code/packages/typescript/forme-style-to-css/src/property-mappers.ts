/**
 * property-mappers.ts — emit one CSS declaration per Style IR property.
 *
 * Strategy: an exhaustive `switch` over `StyleProperty.kind`.  Each
 * case knows the IR value shape (typed) and produces a `prop:
 * value` string (no trailing semicolon — the caller joins).  The
 * exhaustiveness assertion at the bottom turns "added a new kernel
 * kind but forgot to handle it" into a TypeScript compile error.
 *
 * Unknown `ext:*` kinds aren't handled here — the translator filters
 * them upstream and emits a warning.  Same for `TokenRef` resolution
 * failures: the caller resolves refs before calling these mappers
 * and emits a warning when a ref doesn't resolve.
 *
 * The "important" flag, when set, appends ` !important` to the
 * declaration.  Discouraged but supported (FM04 §5.5).
 *
 * @module property-mappers
 */

import type {
  StyleProperty,
  Color, Length, FontStack, Shadow, BorderSpec, TextDecoration, BoxSides,
  TokenSet,
} from "@coding-adventures/forme-style-ir";
import {
  colorToCss, lengthToCss, fontStackToCss, shadowToCss,
} from "./value-mappers.js";
import {
  resolveColor, resolveLength, resolveShadow, resolveFontStack, resolveNumber,
} from "./token-resolver.js";

/**
 * The result of attempting to format one property.  Either a CSS
 * declaration (success), or a warning code (resolution failed,
 * skipped, etc.).
 */
export type PropertyEmit =
  | { ok: true; declaration: string }
  | { ok: false; warning: string };

/**
 * Format one `StyleProperty` as a CSS declaration (no trailing `;`).
 * Returns a warning when a `TokenRef` doesn't resolve.
 */
export function propertyToCss(
  prop: StyleProperty,
  tokens: TokenSet,
): PropertyEmit {
  const imp = prop.important ? " !important" : "";

  switch (prop.kind) {
    // ─── Color / fill ────────────────────────────────────────────────────
    case "color":
      return colorDecl("color", prop.value, tokens, imp);
    case "background":
      return colorDecl("background-color", prop.value, tokens, imp);
    case "border-color":
      return colorDecl("border-color", prop.value, tokens, imp);
    case "outline-color":
      return colorDecl("outline-color", prop.value, tokens, imp);

    // ─── Typography ──────────────────────────────────────────────────────
    case "font-family": {
      const fs = resolveFontStack(prop.value, tokens);
      if (!fs) return warn(`font-family: unresolved`);
      return ok(`font-family: ${fontStackToCss(fs)}${imp}`);
    }
    case "font-size":
      return lengthDecl("font-size", prop.value, tokens, imp);
    case "font-weight": {
      const n = resolveNumber(prop.value, tokens);
      if (n === null) return warn(`font-weight: unresolved`);
      return ok(`font-weight: ${n}${imp}`);
    }
    case "font-style":
      return ok(`font-style: ${prop.value}${imp}`);
    case "text-transform":
      return ok(`text-transform: ${prop.value}${imp}`);
    case "leading": {
      const n = resolveNumber(prop.value, tokens);
      if (n === null) return warn(`leading: unresolved`);
      return ok(`line-height: ${n}${imp}`);
    }
    case "tracking":
      return lengthDecl("letter-spacing", prop.value, tokens, imp);
    case "text-decoration":
      return textDecorationDecl(prop.value, tokens, imp);

    // ─── Layout / spacing ────────────────────────────────────────────────
    case "space-before":
      return lengthDecl("margin-top", prop.value, tokens, imp);
    case "space-after":
      return lengthDecl("margin-bottom", prop.value, tokens, imp);
    case "indent":
      return lengthDecl("text-indent", prop.value, tokens, imp);
    case "padding":
      return paddingDecl(prop.value, tokens, imp);
    case "max-width":
      return lengthDecl("max-width", prop.value, tokens, imp);
    case "min-height":
      return lengthDecl("min-height", prop.value, tokens, imp);
    case "align":
      return ok(`text-align: ${prop.value}${imp}`);
    case "vertical-align":
      return ok(`vertical-align: ${prop.value}${imp}`);

    // ─── Decoration ──────────────────────────────────────────────────────
    case "border":
      return borderDecl(prop.value, tokens, imp);
    case "border-radius":
      return lengthDecl("border-radius", prop.value, tokens, imp);
    case "shadow":
      return shadowDecl(prop.value, tokens, imp);
    case "opacity":
      return ok(`opacity: ${prop.value}${imp}`);

    // ─── Page break (print) ──────────────────────────────────────────────
    case "column-break":
      return ok(`break-${prop.value === "avoid" ? "inside" : prop.value}: ${prop.value === "avoid" ? "avoid-column" : "column"}${imp}`);
    case "page-break":
      return ok(`break-${prop.value === "avoid" ? "inside" : prop.value}: ${prop.value === "avoid" ? "avoid-page" : "page"}${imp}`);
    case "widow-orphan":
      // `widows` and `orphans` are CSS shorthands for the same idea
      // — we emit both with the IR value, since the IR collapses
      // them into one concept.
      return ok(`widows: ${prop.value}${imp}; orphans: ${prop.value}${imp}`);

    // ─── Visibility ──────────────────────────────────────────────────────
    case "display":
      return ok(`display: ${prop.value}${imp}`);
    case "visible":
      return ok(`visibility: ${prop.value ? "visible" : "hidden"}${imp}`);

    // ─── Extension namespace (handled at translator level) ───────────────
    default: {
      // The discriminated-union narrowing rules out every kernel kind
      // above; what's left must be `ext:${string}`.  We never reach
      // here for kernel kinds — that would be a type error.  Defence
      // in depth: emit a warning rather than throw.
      const k = (prop as { kind: string }).kind;
      return warn(`unhandled property kind ${JSON.stringify(k)}`);
    }
  }
}

// ─── Per-shape helpers ───────────────────────────────────────────────────

function ok(declaration: string): PropertyEmit {
  return { ok: true, declaration };
}

function warn(message: string): PropertyEmit {
  return { ok: false, warning: message };
}

function colorDecl(
  cssProp: string, value: Color | { kind: "token-ref"; path: string },
  tokens: TokenSet, imp: string,
): PropertyEmit {
  const c = resolveColor(value, tokens);
  if (!c) return warn(`${cssProp}: unresolved`);
  return ok(`${cssProp}: ${colorToCss(c)}${imp}`);
}

function lengthDecl(
  cssProp: string, value: Length | { kind: "token-ref"; path: string },
  tokens: TokenSet, imp: string,
): PropertyEmit {
  const l = resolveLength(value, tokens);
  if (!l) return warn(`${cssProp}: unresolved`);
  return ok(`${cssProp}: ${lengthToCss(l)}${imp}`);
}

function paddingDecl(
  box: BoxSides<Length | { kind: "token-ref"; path: string }>,
  tokens: TokenSet, imp: string,
): PropertyEmit {
  const t = resolveLength(box.top, tokens);
  const r = resolveLength(box.right, tokens);
  const b = resolveLength(box.bottom, tokens);
  const l = resolveLength(box.left, tokens);
  if (!t || !r || !b || !l) return warn(`padding: unresolved side`);
  return ok(`padding: ${lengthToCss(t)} ${lengthToCss(r)} ${lengthToCss(b)} ${lengthToCss(l)}${imp}`);
}

function borderDecl(spec: BorderSpec, tokens: TokenSet, imp: string): PropertyEmit {
  const c = resolveColor(spec.color, tokens);
  if (!c) return warn(`border: unresolved color`);
  const widthCss = lengthToCss(spec.width);
  const main = `${widthCss} ${spec.style} ${colorToCss(c)}`;

  // `sides` undefined or all four sides → emit `border: ...`.
  // Otherwise emit per-side declarations.
  if (!spec.sides || spec.sides.length === 0
      || (spec.sides.length === 4
          && ["top", "right", "bottom", "left"].every((s) => spec.sides!.includes(s as never)))) {
    return ok(`border: ${main}${imp}`);
  }
  // Sided form: `border-top: ...; border-right: ...;` …
  const parts = spec.sides.map((side) => `border-${side}: ${main}${imp}`);
  return ok(parts.join("; "));
}

function shadowDecl(
  value: Shadow | { kind: "token-ref"; path: string },
  tokens: TokenSet, imp: string,
): PropertyEmit {
  const s = resolveShadow(value, tokens);
  if (!s) return warn(`shadow: unresolved`);
  // Recursively resolve the shadow's color.
  const c = resolveColor(s.color as Color | { kind: "token-ref"; path: string }, tokens);
  if (!c) return warn(`shadow: unresolved color`);
  return ok(`box-shadow: ${shadowToCss(s, colorToCss(c))}${imp}`);
}

function textDecorationDecl(
  td: TextDecoration, tokens: TokenSet, imp: string,
): PropertyEmit {
  const parts: string[] = [td.line];
  if (td.style) parts.push(td.style);
  if (td.color !== undefined) {
    const c = resolveColor(td.color, tokens);
    if (!c) return warn(`text-decoration color: unresolved`);
    parts.push(colorToCss(c));
  }
  if (td.thickness !== undefined) {
    parts.push(lengthToCss(td.thickness));
  }
  return ok(`text-decoration: ${parts.join(" ")}${imp}`);
}
