/**
 * token-resolver.ts — `TokenRef` → concrete value resolution.
 *
 * `TokenRef` carries a dotted path like `"colors.text"`,
 * `"typography.scale.lg"`, or `"space.md"`.  We resolve it by
 * walking the `TokenSet` tree along that path.  Resolution can
 * **chain** — a token can itself be a `TokenRef` (e.g. a "link"
 * color that points to "text" color) — so we follow refs until we
 * hit a concrete value or detect a cycle.
 *
 * Cycle detection bounds resolution at `MAX_RESOLVE_DEPTH` chain
 * hops (8 is comfortably more than any sensible design system uses).
 * Hitting the limit returns `null` and emits a warning rather than
 * throwing — translator-level robustness per FM04 §9.6.
 *
 * @module token-resolver
 */

import {
  isTokenRef,
  type Color, type FontStack, type Length, type Shadow,
  type TokenRef, type TokenSet,
} from "@coding-adventures/forme-style-ir";

/** Max chain hops before we assume a cycle. */
const MAX_RESOLVE_DEPTH = 8;

/**
 * Resolve a `TokenRef | T` to a concrete `T` (where T is one of the
 * leaf value types: Color, Length, Shadow, FontStack, number).
 * Returns null if the ref is unresolvable (path not found, cycle,
 * type mismatch — caller decides what to do).  This function is
 * intentionally lax about the leaf type: it just walks the tree and
 * returns whatever it lands on, leaving discriminant validation to
 * the caller (it already knows what kind of value it expects given
 * the property kind).
 */
export function resolveRef(
  ref: TokenRef,
  tokens: TokenSet,
  depth = 0,
): unknown | null {
  if (depth > MAX_RESOLVE_DEPTH) return null;
  const value = walkPath(ref.path, tokens as unknown as Record<string, unknown>);
  if (value === undefined) return null;
  if (isTokenRef(value)) return resolveRef(value as TokenRef, tokens, depth + 1);
  return value;
}

/** Walk a dotted path through an object tree.  Returns undefined if
 *  any segment is missing or non-object.
 *
 *  Defence in depth against prototype traversal: only **own** enumerable
 *  properties are followed, and the well-known prototype-pollution
 *  vectors (`__proto__`, `constructor`, `prototype`) are refused
 *  unconditionally.  In practice the forme-style-ir validator
 *  restricts `TokenRef.path` to a dotted-identifier grammar that
 *  doesn't admit `__proto__`, but defending here too means a stage
 *  that hands us a hand-rolled `TokenRef` (bypassing the validator)
 *  can't surface inert `Function.prototype` members in CSS output.
 */
function walkPath(path: string, root: Record<string, unknown>): unknown {
  const parts = path.split(".");
  let cursor: unknown = root;
  for (const part of parts) {
    if (typeof cursor !== "object" || cursor === null) return undefined;
    if (part === "__proto__" || part === "constructor" || part === "prototype") return undefined;
    const obj = cursor as Record<string, unknown>;
    if (!Object.prototype.hasOwnProperty.call(obj, part)) return undefined;
    cursor = obj[part];
    if (cursor === undefined) return undefined;
  }
  return cursor;
}

// ─── Typed convenience wrappers ──────────────────────────────────────────
//
// Per-leaf-type guards: the caller asks for the type it expects, and
// we filter out values that don't match (rather than coerce).  When a
// mismatch happens, the caller emits a warning per FM04 §9.6.

export function resolveColor(v: Color | TokenRef, tokens: TokenSet): Color | null {
  if (!isTokenRef(v)) return v;
  const r = resolveRef(v, tokens);
  return isColor(r) ? r : null;
}

export function resolveLength(v: Length | TokenRef, tokens: TokenSet): Length | null {
  if (!isTokenRef(v)) return v;
  const r = resolveRef(v, tokens);
  return isLength(r) ? r : null;
}

export function resolveShadow(v: Shadow | TokenRef, tokens: TokenSet): Shadow | null {
  if (!isTokenRef(v)) return v;
  const r = resolveRef(v, tokens);
  return isShadow(r) ? r : null;
}

export function resolveFontStack(v: FontStack | TokenRef, tokens: TokenSet): FontStack | null {
  if (!isTokenRef(v)) return v;
  const r = resolveRef(v, tokens);
  return isFontStack(r) ? r : null;
}

export function resolveNumber(v: number | TokenRef, tokens: TokenSet): number | null {
  if (!isTokenRef(v)) return v;
  const r = resolveRef(v, tokens);
  return typeof r === "number" && Number.isFinite(r) ? r : null;
}

// ─── Type guards ────────────────────────────────────────────────────────

function isColor(v: unknown): v is Color {
  if (typeof v !== "object" || v === null) return false;
  const k = (v as { kind?: unknown }).kind;
  return k === "rgb" || k === "hsl" || k === "oklch" || k === "named";
}

function isLength(v: unknown): v is Length {
  if (typeof v !== "object" || v === null) return false;
  const u = (v as { unit?: unknown }).unit;
  return typeof u === "string" && typeof (v as { value?: unknown }).value === "number";
}

function isShadow(v: unknown): v is Shadow {
  if (typeof v !== "object" || v === null) return false;
  const s = v as Record<string, unknown>;
  return (
    typeof s.offsetX === "object" && s.offsetX !== null
    && typeof s.offsetY === "object" && s.offsetY !== null
    && typeof s.blur === "object" && s.blur !== null
    && typeof s.spread === "object" && s.spread !== null
    && s.color !== undefined
  );
}

function isFontStack(v: unknown): v is FontStack {
  return Array.isArray(v) && v.every((x) => typeof x === "string");
}
