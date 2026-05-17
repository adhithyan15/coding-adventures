/**
 * token-resolver.ts — `TokenRef` → concrete value resolution.
 *
 * Mirrors the resolver in `forme-style-to-css`'s token-resolver.ts —
 * intentionally duplicated rather than depended-upon so each
 * translator package is independent.  Same security posture
 * (prototype-pollution deny-list + `hasOwnProperty` own-only).
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
 * Resolve a `TokenRef | T` to whatever it lands on (caller narrows).
 * Returns null on missing path, cycle, or other failure.
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

/**
 * Walk a dotted path through an object tree.  Returns undefined on
 * any missing or non-object segment, or on a prototype-pollution
 * shield hit.
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
