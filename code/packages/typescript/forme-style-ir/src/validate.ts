/**
 * validate.ts — one-pass-many-errors validator for `StyleDocument` (FM04 §14.1).
 *
 * The validator's job is to catch *shape* mistakes — typos in
 * property kinds, malformed colors, duplicate rule ids, etc. —
 * before the document reaches a translator.  Translator-side issues
 * (an unresolved `TokenRef`, a property kind the translator doesn't
 * understand) are runtime warnings, not validator errors (see
 * `style-error.ts` for the split).
 *
 * Style — same as `forme-pipeline-config`'s `validateConfig`:
 *
 * - Collect *every* violation in one walk; throw a single
 *   `StyleError` carrying all entries.  Don't bail on the first.
 * - Distinguish hard errors (push to `errors`, will throw) from soft
 *   warnings (push to `warnings`, returned alongside the validated
 *   document).
 * - Defensive checks at the outermost level (a non-object root, a
 *   missing `tokens` field) bail early because subsequent traversal
 *   would crash on `undefined.xyz` access.
 *
 * The implementation is mostly *structural* — we walk the value
 * tree, check each node's `kind` against the closed lists in
 * `properties.ts` / `selectors.ts` / `tokens.ts`, and verify each
 * variant's required fields are present and well-typed.  No clever
 * graph algorithms, just a careful tree walk.
 *
 * @module validate
 */

import type { JsonValue } from "@coding-adventures/forme-types";
import { isExtensionContext, isRecognisedContext } from "./contexts.js";
import {
  PROPERTY_KINDS, isExtensionKind,
  type PropertyKind, type StyleProperty,
} from "./properties.js";
import {
  SELECTOR_KINDS,
  type Selector, type SelectorKind,
} from "./selectors.js";
import {
  LENGTH_UNITS, isTokenRef,
  type Color, type Length, type Shadow, type TokenRef, type TokenSet,
} from "./tokens.js";
import {
  StyleError,
  type StyleErrorEntry, type StyleErrorCode, type StyleWarning,
} from "./style-error.js";
import type { StyleDocument, StyleRule } from "./style-document.js";

/**
 * Result of a successful validation.  The shape is intentionally
 * narrow: just the input document (typed) and the soft warnings.
 * Errors are thrown.
 */
export interface ValidatedStyleDocument {
  readonly document: StyleDocument;
  readonly warnings: readonly StyleWarning[];
}

/**
 * Validate a candidate `StyleDocument`.  Throws `StyleError` if any
 * hard violations are found; returns the typed document plus any
 * soft warnings (e.g. unrecognised context names).
 */
export function validateStyleDocument(value: unknown): ValidatedStyleDocument {
  const errors: StyleErrorEntry[] = [];
  const warnings: StyleWarning[] = [];

  // Top-level shape.  Cannot proceed without an object.
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new StyleError([{
      code: "MALFORMED",
      path: "",
      message: "root must be an object",
    }]);
  }
  const root = value as Record<string, unknown>;

  if (root.kind !== "StyleDocument") {
    errors.push(entry("MALFORMED", "kind", `expected "StyleDocument", got ${JSON.stringify(root.kind)}`));
  }

  // tokens (required, object)
  if (typeof root.tokens !== "object" || root.tokens === null || Array.isArray(root.tokens)) {
    errors.push(entry("MALFORMED", "tokens", "must be an object"));
    // Can't proceed into tokens further; throw early so the rest
    // of the walk doesn't crash on undefined.colors.
    throw new StyleError(errors);
  }
  validateTokenSet(root.tokens as Record<string, unknown>, "tokens", errors);

  // contexts (required, array of strings)
  if (!Array.isArray(root.contexts)) {
    errors.push(entry("MALFORMED", "contexts", "must be an array of strings"));
  } else {
    for (let i = 0; i < root.contexts.length; i++) {
      const c = root.contexts[i];
      if (typeof c !== "string") {
        errors.push(entry("MALFORMED", `contexts/${i}`, "must be a string"));
      }
    }
  }

  // theme (required, string | null)
  if (root.theme !== null && typeof root.theme !== "string") {
    errors.push(entry("MALFORMED", "theme", "must be a string or null"));
  }

  // rules (required, array of StyleRule)
  if (!Array.isArray(root.rules)) {
    errors.push(entry("MALFORMED", "rules", "must be an array"));
  } else {
    const seenIds = new Set<string>();
    const declaredContexts = Array.isArray(root.contexts)
      ? (root.contexts as unknown[]).filter((s): s is string => typeof s === "string")
      : [];
    for (let i = 0; i < root.rules.length; i++) {
      validateRule(root.rules[i], `rules/${i}`, seenIds, declaredContexts, errors, warnings);
    }
  }

  if (errors.length > 0) {
    throw new StyleError(errors);
  }
  return {
    document: value as StyleDocument,
    warnings: Object.freeze(warnings),
  };
}

// ─── helpers ─────────────────────────────────────────────────────────────

function entry(code: StyleErrorCode, path: string, message: string): StyleErrorEntry {
  return { code, path, message };
}

// ─── TokenSet ────────────────────────────────────────────────────────────

function validateTokenSet(
  ts: Record<string, unknown>,
  path: string,
  errors: StyleErrorEntry[],
): void {
  // Five required buckets: colors, typography, space, radii, shadows.
  validateRecord(ts.colors, `${path}/colors`, errors, "color", (v, p) => validateColorOrRef(v, p, errors));
  validateRecord(ts.space, `${path}/space`, errors, "length", (v, p) => validateLength(v, p, errors));
  validateRecord(ts.radii, `${path}/radii`, errors, "length", (v, p) => validateLength(v, p, errors));
  validateRecord(ts.shadows, `${path}/shadows`, errors, "shadow", (v, p) => validateShadow(v, p, errors));

  // typography sub-object
  if (typeof ts.typography !== "object" || ts.typography === null || Array.isArray(ts.typography)) {
    errors.push(entry("MALFORMED", `${path}/typography`, "must be an object"));
  } else {
    const ty = ts.typography as Record<string, unknown>;
    validateRecord(ty.families, `${path}/typography/families`, errors, "font-stack", (v, p) => {
      if (!Array.isArray(v) || v.some((s) => typeof s !== "string")) {
        errors.push(entry("MALFORMED", p, "must be an array of strings (FontStack)"));
      }
    });
    validateRecord(ty.scale, `${path}/typography/scale`, errors, "length", (v, p) => validateLength(v, p, errors));
    validateRecord(ty.weights, `${path}/typography/weights`, errors, "number", (v, p) => {
      if (typeof v !== "number" || !Number.isFinite(v)) {
        errors.push(entry("MALFORMED", p, "must be a finite number"));
      }
    });
    validateRecord(ty.leading, `${path}/typography/leading`, errors, "number", (v, p) => {
      if (typeof v !== "number" || !Number.isFinite(v)) {
        errors.push(entry("MALFORMED", p, "must be a finite number"));
      }
    });
    validateRecord(ty.tracking, `${path}/typography/tracking`, errors, "length", (v, p) => validateLength(v, p, errors));
  }

  // extensions (optional, object).  Keys must match ext:<package>:<group>.
  if (ts.extensions !== undefined) {
    if (typeof ts.extensions !== "object" || ts.extensions === null || Array.isArray(ts.extensions)) {
      errors.push(entry("MALFORMED", `${path}/extensions`, "must be an object"));
    } else {
      for (const k of Object.keys(ts.extensions)) {
        if (!isExtensionKeyValid(k)) {
          errors.push(entry("INVALID_EXTENSION_KEY", `${path}/extensions/${k}`,
            `extension key must match ext:<package>:<group>, got ${JSON.stringify(k)}`));
        }
      }
    }
  }
}

function validateRecord(
  v: unknown,
  path: string,
  errors: StyleErrorEntry[],
  _label: string,
  perValue: (v: unknown, path: string) => void,
): void {
  if (typeof v !== "object" || v === null || Array.isArray(v)) {
    errors.push(entry("MALFORMED", path, "must be a record"));
    return;
  }
  for (const k of Object.keys(v)) {
    perValue((v as Record<string, unknown>)[k], `${path}/${k}`);
  }
}

// ─── Colors ──────────────────────────────────────────────────────────────

function validateColorOrRef(v: unknown, path: string, errors: StyleErrorEntry[]): void {
  if (isTokenRef(v)) {
    validateTokenRefPath((v as TokenRef).path, `${path}/path`, errors);
    return;
  }
  validateColor(v, path, errors);
}

function validateColor(v: unknown, path: string, errors: StyleErrorEntry[]): void {
  if (typeof v !== "object" || v === null) {
    errors.push(entry("INVALID_COLOR", path, "must be a Color object"));
    return;
  }
  const c = v as Record<string, unknown>;
  switch (c.kind) {
    case "rgb": {
      checkChannel(c.r, path, "r", 0, 255, errors);
      checkChannel(c.g, path, "g", 0, 255, errors);
      checkChannel(c.b, path, "b", 0, 255, errors);
      checkAlpha(c.a, path, errors);
      return;
    }
    case "hsl": {
      checkChannel(c.h, path, "h", 0, 360, errors);
      checkChannel(c.s, path, "s", 0, 100, errors);
      checkChannel(c.l, path, "l", 0, 100, errors);
      checkAlpha(c.a, path, errors);
      return;
    }
    case "oklch": {
      checkChannel(c.l, path, "l", 0, 1, errors);
      checkChannel(c.c, path, "c", 0, 1, errors);    // c is technically unbounded in OKLCh; clamp to 1 for our purposes
      checkChannel(c.h, path, "h", 0, 360, errors);
      checkAlpha(c.a, path, errors);
      return;
    }
    case "named": {
      if (typeof c.name !== "string" || c.name.length === 0) {
        errors.push(entry("INVALID_COLOR", `${path}/name`, "must be a non-empty string"));
      }
      return;
    }
    default:
      errors.push(entry("INVALID_COLOR", `${path}/kind`,
        `unknown color kind ${JSON.stringify(c.kind)}; expected rgb | hsl | oklch | named`));
  }
}

function checkChannel(v: unknown, path: string, name: string, lo: number, hi: number, errors: StyleErrorEntry[]): void {
  if (typeof v !== "number" || !Number.isFinite(v) || v < lo || v > hi) {
    errors.push(entry("INVALID_COLOR_CHANNEL", `${path}/${name}`,
      `must be a finite number in [${lo}, ${hi}], got ${JSON.stringify(v)}`));
  }
}

function checkAlpha(v: unknown, path: string, errors: StyleErrorEntry[]): void {
  if (v === undefined) return;
  if (typeof v !== "number" || !Number.isFinite(v) || v < 0 || v > 1) {
    errors.push(entry("INVALID_COLOR_CHANNEL", `${path}/a`,
      `must be a finite number in [0, 1], got ${JSON.stringify(v)}`));
  }
}

// ─── Length ──────────────────────────────────────────────────────────────

function validateLength(v: unknown, path: string, errors: StyleErrorEntry[]): void {
  if (typeof v !== "object" || v === null) {
    errors.push(entry("MALFORMED", path, "must be a Length object"));
    return;
  }
  const l = v as Record<string, unknown>;
  if (!(LENGTH_UNITS as readonly string[]).includes(l.unit as string)) {
    errors.push(entry("INVALID_LENGTH_UNIT", `${path}/unit`,
      `unknown length unit ${JSON.stringify(l.unit)}; expected one of ${LENGTH_UNITS.join(", ")}`));
  }
  if (typeof l.value !== "number" || !Number.isFinite(l.value)) {
    errors.push(entry("MALFORMED", `${path}/value`, "must be a finite number"));
  }
}

function validateLengthOrRef(v: unknown, path: string, errors: StyleErrorEntry[]): void {
  if (isTokenRef(v)) {
    validateTokenRefPath((v as TokenRef).path, `${path}/path`, errors);
    return;
  }
  validateLength(v, path, errors);
}

// ─── Shadow ──────────────────────────────────────────────────────────────

function validateShadow(v: unknown, path: string, errors: StyleErrorEntry[]): void {
  if (typeof v !== "object" || v === null) {
    errors.push(entry("MALFORMED", path, "must be a Shadow object"));
    return;
  }
  const s = v as Record<string, unknown>;
  validateLength(s.offsetX, `${path}/offsetX`, errors);
  validateLength(s.offsetY, `${path}/offsetY`, errors);
  validateLength(s.blur, `${path}/blur`, errors);
  validateLength(s.spread, `${path}/spread`, errors);
  validateColorOrRef(s.color, `${path}/color`, errors);
  if (s.inset !== undefined && typeof s.inset !== "boolean") {
    errors.push(entry("MALFORMED", `${path}/inset`, "must be a boolean if present"));
  }
}

// ─── TokenRef ────────────────────────────────────────────────────────────

const TOKEN_REF_PATH_RE = /^[a-zA-Z_][a-zA-Z0-9_]*(\.[a-zA-Z_][a-zA-Z0-9_-]*)*$/;

function validateTokenRefPath(path: unknown, where: string, errors: StyleErrorEntry[]): void {
  if (typeof path !== "string" || path.length === 0) {
    errors.push(entry("INVALID_TOKEN_REF_PATH", where, "must be a non-empty string"));
    return;
  }
  if (!TOKEN_REF_PATH_RE.test(path)) {
    errors.push(entry("INVALID_TOKEN_REF_PATH", where,
      `must be a dotted identifier path, got ${JSON.stringify(path)}`));
  }
}

// ─── StyleRule ───────────────────────────────────────────────────────────

function validateRule(
  v: unknown,
  path: string,
  seenIds: Set<string>,
  declaredContexts: readonly string[],
  errors: StyleErrorEntry[],
  warnings: StyleWarning[],
): void {
  if (typeof v !== "object" || v === null || Array.isArray(v)) {
    errors.push(entry("MALFORMED", path, "must be a StyleRule object"));
    return;
  }
  const r = v as Record<string, unknown>;

  // id (required, non-empty string, unique within document)
  if (typeof r.id !== "string") {
    errors.push(entry("MALFORMED", `${path}/id`, "must be a string"));
  } else if (r.id.length === 0) {
    errors.push(entry("EMPTY_RULE_ID", `${path}/id`, "must be non-empty"));
  } else if (seenIds.has(r.id)) {
    errors.push(entry("DUPLICATE_RULE_ID", `${path}/id`,
      `rule id ${JSON.stringify(r.id)} already used by an earlier rule`));
  } else {
    seenIds.add(r.id);
  }

  // selector (required)
  validateSelector(r.selector, `${path}/selector`, errors);

  // properties (required, array of StyleProperty)
  if (!Array.isArray(r.properties)) {
    errors.push(entry("MALFORMED", `${path}/properties`, "must be an array"));
  } else {
    for (let i = 0; i < r.properties.length; i++) {
      validateProperty(r.properties[i], `${path}/properties/${i}`, errors);
    }
  }

  // context (optional, string).  Soft-warn on unrecognised AND
  // undeclared-in-document contexts.
  if (r.context !== undefined) {
    if (typeof r.context !== "string") {
      errors.push(entry("MALFORMED", `${path}/context`, "must be a string if present"));
    } else {
      if (!isRecognisedContext(r.context)) {
        warnings.push({
          code: "UNKNOWN_CONTEXT",
          message: `rule context ${JSON.stringify(r.context)} is not a kernel-standard context name and lacks the ext: prefix — likely a typo`,
          ruleId: typeof r.id === "string" ? r.id : undefined,
        });
      } else if (!declaredContexts.includes(r.context) && !isExtensionContext(r.context)) {
        warnings.push({
          code: "CONTEXT_NOT_DECLARED",
          message: `rule context ${JSON.stringify(r.context)} is not listed in document.contexts — translators may still apply it but the document should declare its context vocabulary`,
          ruleId: typeof r.id === "string" ? r.id : undefined,
        });
      }
    }
  }
}

// ─── Selector ────────────────────────────────────────────────────────────

/**
 * Maximum selector composition depth.  Cyclic / pathological inputs
 * (only reachable via hand-rolled object graphs — `JSON.parse` output
 * is acyclic by construction) would otherwise blow the stack.  The
 * limit is generous: real composed selectors hit single-digit depth.
 */
const MAX_SELECTOR_DEPTH = 1000;

function validateSelector(
  v: unknown,
  path: string,
  errors: StyleErrorEntry[],
  depth = 0,
): void {
  if (depth > MAX_SELECTOR_DEPTH) {
    errors.push(entry("MALFORMED", path,
      `selector nesting exceeds ${MAX_SELECTOR_DEPTH} levels — likely a cycle in the input`));
    return;
  }
  if (typeof v !== "object" || v === null || Array.isArray(v)) {
    errors.push(entry("MALFORMED", path, "must be a Selector object"));
    return;
  }
  const s = v as Record<string, unknown>;
  const kind = s.kind as SelectorKind | string;
  if (!(SELECTOR_KINDS as readonly string[]).includes(kind)) {
    errors.push(entry("UNKNOWN_SELECTOR_KIND", `${path}/kind`,
      `unknown selector kind ${JSON.stringify(kind)}; expected one of ${SELECTOR_KINDS.join(", ")}`));
    return;
  }
  switch (kind) {
    case "node-type":
      if (typeof s.type !== "string" || s.type.length === 0) {
        errors.push(entry("MALFORMED", `${path}/type`, "must be a non-empty string"));
      }
      return;
    case "node-type-level":
      if (s.type !== "heading") {
        errors.push(entry("MALFORMED", `${path}/type`, `must be "heading" for node-type-level`));
      }
      if (typeof s.level !== "number" || ![1, 2, 3, 4, 5, 6].includes(s.level as number)) {
        errors.push(entry("INVALID_HEADING_LEVEL", `${path}/level`,
          `must be one of 1, 2, 3, 4, 5, 6; got ${JSON.stringify(s.level)}`));
      }
      return;
    case "custom-kind":
      if (typeof s.customKind !== "string" || s.customKind.length === 0) {
        errors.push(entry("MALFORMED", `${path}/customKind`, "must be a non-empty string"));
      }
      return;
    case "tag":
      if (typeof s.tag !== "string" || s.tag.length === 0) {
        errors.push(entry("MALFORMED", `${path}/tag`, "must be a non-empty string"));
      }
      return;
    case "id":
      if (typeof s.id !== "string" || s.id.length === 0) {
        errors.push(entry("MALFORMED", `${path}/id`, "must be a non-empty string"));
      }
      return;
    case "role":
      if (typeof s.role !== "string" || s.role.length === 0) {
        errors.push(entry("MALFORMED", `${path}/role`, "must be a non-empty string"));
      }
      return;
    case "nth":
      validateSelector(s.of, `${path}/of`, errors, depth + 1);
      validateNthIndex(s.n, `${path}/n`, errors);
      return;
    case "child-of":
      validateSelector(s.parent, `${path}/parent`, errors, depth + 1);
      validateSelector(s.child, `${path}/child`, errors, depth + 1);
      return;
    case "descendant-of":
      validateSelector(s.ancestor, `${path}/ancestor`, errors, depth + 1);
      validateSelector(s.descendant, `${path}/descendant`, errors, depth + 1);
      return;
    case "adjacent":
      validateSelector(s.previous, `${path}/previous`, errors, depth + 1);
      validateSelector(s.following, `${path}/following`, errors, depth + 1);
      return;
    case "and":
      if (!Array.isArray(s.all)) {
        errors.push(entry("MALFORMED", `${path}/all`, "must be an array"));
      } else if (s.all.length === 0) {
        errors.push(entry("EMPTY_COMPOSITION", `${path}/all`, "and requires at least one inner selector"));
      } else {
        for (let i = 0; i < s.all.length; i++) {
          validateSelector(s.all[i], `${path}/all/${i}`, errors, depth + 1);
        }
      }
      return;
    case "or":
      if (!Array.isArray(s.any)) {
        errors.push(entry("MALFORMED", `${path}/any`, "must be an array"));
      } else if (s.any.length === 0) {
        errors.push(entry("EMPTY_COMPOSITION", `${path}/any`, "or requires at least one inner selector"));
      } else {
        for (let i = 0; i < s.any.length; i++) {
          validateSelector(s.any[i], `${path}/any/${i}`, errors, depth + 1);
        }
      }
      return;
    case "not":
      validateSelector(s.inner, `${path}/inner`, errors, depth + 1);
      return;
  }
}

function validateNthIndex(v: unknown, path: string, errors: StyleErrorEntry[]): void {
  if (typeof v === "number") {
    if (!Number.isInteger(v) || v < 0) {
      errors.push(entry("MALFORMED", path, "literal nth index must be a non-negative integer"));
    }
    return;
  }
  if (typeof v !== "object" || v === null) {
    errors.push(entry("MALFORMED", path, "must be a number or NthFormula object"));
    return;
  }
  const f = v as Record<string, unknown>;
  if (typeof f.a !== "number" || !Number.isFinite(f.a)) {
    errors.push(entry("MALFORMED", `${path}/a`, "NthFormula.a must be a finite number"));
  }
  if (typeof f.b !== "number" || !Number.isFinite(f.b)) {
    errors.push(entry("MALFORMED", `${path}/b`, "NthFormula.b must be a finite number"));
  }
  if (f.fromEnd !== undefined && typeof f.fromEnd !== "boolean") {
    errors.push(entry("MALFORMED", `${path}/fromEnd`, "must be a boolean if present"));
  }
}

// ─── StyleProperty ──────────────────────────────────────────────────────

function validateProperty(v: unknown, path: string, errors: StyleErrorEntry[]): void {
  if (typeof v !== "object" || v === null || Array.isArray(v)) {
    errors.push(entry("MALFORMED", path, "must be a StyleProperty object"));
    return;
  }
  const p = v as Record<string, unknown>;
  const kind = p.kind;
  if (typeof kind !== "string") {
    errors.push(entry("MALFORMED", `${path}/kind`, "must be a string"));
    return;
  }

  // Extension namespace — value is opaque JsonValue; we only check
  // that the kind matches `ext:<something>`.
  if (isExtensionKind(kind)) {
    if (p.value === undefined) {
      errors.push(entry("INVALID_PROPERTY_VALUE", `${path}/value`,
        `extension property ${JSON.stringify(kind)} requires a value`));
    }
    return;
  }

  // Closed-list property.  Verify kind is known and value is shape-correct.
  if (!(PROPERTY_KINDS as readonly string[]).includes(kind as PropertyKind)) {
    errors.push(entry("UNKNOWN_PROPERTY_KIND", `${path}/kind`,
      `unknown property kind ${JSON.stringify(kind)}; expected one of ${PROPERTY_KINDS.join(", ")} or ext:<name>`));
    return;
  }

  if (p.important !== undefined && typeof p.important !== "boolean") {
    errors.push(entry("MALFORMED", `${path}/important`, "must be a boolean if present"));
  }

  validatePropertyValue(kind as PropertyKind, p.value, `${path}/value`, errors);
}

// Per-property-kind value checker.  Mirrors the StyleProperty union
// in properties.ts; if a new kind is added there, the type system
// will NOT remind us to add a case here — keep these in sync by
// staying disciplined.  (A switch-with-`never` exhaustiveness check
// against the union's `kind` field would be nicer, but the value
// types vary per kind so the standard exhaustive-switch idiom
// doesn't quite fit; see properties.test.ts which checks that every
// kind in PROPERTY_KINDS is handled.)
function validatePropertyValue(
  kind: PropertyKind,
  value: unknown,
  path: string,
  errors: StyleErrorEntry[],
): void {
  switch (kind) {
    // Color | TokenRef
    case "color":
    case "background":
    case "border-color":
    case "outline-color":
      validateColorOrRef(value, path, errors);
      return;

    // Length | TokenRef
    case "font-size":
    case "tracking":
    case "space-before":
    case "space-after":
    case "indent":
    case "max-width":
    case "min-height":
    case "border-radius":
      validateLengthOrRef(value, path, errors);
      return;

    // number | TokenRef (font-weight, leading)
    case "font-weight":
    case "leading":
      if (!isTokenRef(value)) {
        if (typeof value !== "number" || !Number.isFinite(value)) {
          errors.push(entry("INVALID_PROPERTY_VALUE", path,
            `${kind} value must be a finite number or TokenRef`));
        }
      } else {
        validateTokenRefPath((value as TokenRef).path, `${path}/path`, errors);
      }
      return;

    // bare number
    case "opacity":
      if (typeof value !== "number" || !Number.isFinite(value)) {
        errors.push(entry("INVALID_PROPERTY_VALUE", path, "opacity must be a finite number"));
      } else if (value < 0 || value > 1) {
        errors.push(entry("INVALID_PROPERTY_VALUE", path,
          `opacity must be in [0, 1], got ${value}`));
      }
      return;
    case "widow-orphan":
      if (typeof value !== "number" || !Number.isInteger(value) || value < 0) {
        errors.push(entry("INVALID_PROPERTY_VALUE", path, "widow-orphan must be a non-negative integer"));
      }
      return;

    // bare bool
    case "visible":
      if (typeof value !== "boolean") {
        errors.push(entry("INVALID_PROPERTY_VALUE", path, "visible must be a boolean"));
      }
      return;

    // enums
    case "font-style":
      checkEnum(value, ["normal", "italic", "oblique"], kind, path, errors);
      return;
    case "text-transform":
      checkEnum(value, ["none", "uppercase", "lowercase", "capitalize"], kind, path, errors);
      return;
    case "align":
      checkEnum(value, ["start", "end", "center", "justify"], kind, path, errors);
      return;
    case "vertical-align":
      checkEnum(value, ["baseline", "top", "middle", "bottom"], kind, path, errors);
      return;
    case "column-break":
    case "page-break":
      checkEnum(value, ["before", "after", "avoid"], kind, path, errors);
      return;
    case "display":
      checkEnum(value, ["block", "inline", "inline-block", "none"], kind, path, errors);
      return;

    // FontStack | TokenRef
    case "font-family":
      if (isTokenRef(value)) {
        validateTokenRefPath((value as TokenRef).path, `${path}/path`, errors);
      } else if (!Array.isArray(value) || (value as unknown[]).some((s) => typeof s !== "string")) {
        errors.push(entry("INVALID_PROPERTY_VALUE", path,
          "font-family value must be a FontStack (array of strings) or TokenRef"));
      }
      return;

    // BoxSides<Length | TokenRef>
    case "padding": {
      if (typeof value !== "object" || value === null) {
        errors.push(entry("INVALID_PROPERTY_VALUE", path, "padding value must be a BoxSides object"));
        return;
      }
      const b = value as Record<string, unknown>;
      for (const side of ["top", "right", "bottom", "left"] as const) {
        if (b[side] === undefined) {
          errors.push(entry("INVALID_PROPERTY_VALUE", `${path}/${side}`, "padding side is required"));
        } else {
          validateLengthOrRef(b[side], `${path}/${side}`, errors);
        }
      }
      return;
    }

    // TextDecoration
    case "text-decoration": {
      if (typeof value !== "object" || value === null) {
        errors.push(entry("INVALID_PROPERTY_VALUE", path, "text-decoration value must be a TextDecoration object"));
        return;
      }
      const td = value as Record<string, unknown>;
      checkEnum(td.line, ["none", "underline", "overline", "line-through"], "text-decoration.line", `${path}/line`, errors);
      if (td.style !== undefined) {
        checkEnum(td.style, ["solid", "dashed", "dotted", "wavy"], "text-decoration.style", `${path}/style`, errors);
      }
      if (td.color !== undefined) validateColorOrRef(td.color, `${path}/color`, errors);
      if (td.thickness !== undefined) validateLength(td.thickness, `${path}/thickness`, errors);
      return;
    }

    // BorderSpec
    case "border": {
      if (typeof value !== "object" || value === null) {
        errors.push(entry("INVALID_PROPERTY_VALUE", path, "border value must be a BorderSpec object"));
        return;
      }
      const bs = value as Record<string, unknown>;
      validateLength(bs.width, `${path}/width`, errors);
      checkEnum(bs.style, ["none", "solid", "dashed", "dotted", "double"], "border.style", `${path}/style`, errors);
      validateColorOrRef(bs.color, `${path}/color`, errors);
      if (bs.sides !== undefined) {
        if (!Array.isArray(bs.sides) || (bs.sides as unknown[]).some((s) => !["top", "right", "bottom", "left"].includes(s as string))) {
          errors.push(entry("INVALID_PROPERTY_VALUE", `${path}/sides`,
            "border.sides must be an array drawn from ['top', 'right', 'bottom', 'left']"));
        }
      }
      return;
    }

    // Shadow | TokenRef
    case "shadow":
      if (isTokenRef(value)) {
        validateTokenRefPath((value as TokenRef).path, `${path}/path`, errors);
      } else {
        validateShadow(value, path, errors);
      }
      return;
  }
}

function checkEnum<T extends string>(
  value: unknown,
  allowed: readonly T[],
  label: string,
  path: string,
  errors: StyleErrorEntry[],
): void {
  if (typeof value !== "string" || !(allowed as readonly string[]).includes(value)) {
    errors.push(entry("INVALID_PROPERTY_VALUE", path,
      `${label} must be one of ${allowed.join(", ")}; got ${JSON.stringify(value)}`));
  }
}

// ─── Misc helpers ───────────────────────────────────────────────────────

const EXTENSION_KEY_RE = /^ext:[a-zA-Z0-9_-]+(?::[a-zA-Z0-9_-]+)?$/;

function isExtensionKeyValid(k: string): boolean {
  return EXTENSION_KEY_RE.test(k);
}

// Suppress unused-import warning for JsonValue (used in declaration sites
// but TypeScript doesn't pick that up because it's a type-only annotation).
export type { JsonValue } from "@coding-adventures/forme-types";
// Suppress unused-import warning for StyleRule (referenced via index export).
export type { StyleRule } from "./style-document.js";
