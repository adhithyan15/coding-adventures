/**
 * resolve.ts — `resolveTokenRefs(doc, refs)` per FM04 §13.3.
 *
 * Bulk `TokenRef` resolution.  Used by analyser pre-passes (the AOT
 * compiler's CSS-slice planner, the LaTeX preamble extractor, theme
 * coverage reporters, …) that want to know "if I resolve *these*
 * refs against *this* document's tokens, what do I get?" without
 * stepping through a full translator.
 *
 * Returns a `Map<string, ResolvedValue | null>` keyed by `ref.path`.
 * `null` means the ref couldn't be resolved — path not found, cycle,
 * type mismatch, or a non-leaf landing point.  The translator-side
 * mappers warn-and-skip on null per FM04 §9.6; analyser callers
 * decide their own policy.
 *
 * ## Resolution semantics
 *
 * - Walk the dotted path through `doc.tokens`.
 * - If the landing value is itself a `TokenRef`, recurse — token
 *   chains are valid (a "link" color pointing at a "text" color).
 * - Cap chains at `MAX_RESOLVE_DEPTH = 8` (covers any sensible
 *   design system; the cap converts a cycle into `null` rather
 *   than letting the stack overflow).
 * - Return one of the recognised leaf shapes (`Color`, `Length`,
 *   `Shadow`, `FontStack`, `number`) — anything else (object that
 *   doesn't match a known shape, undefined, etc.) yields `null`.
 *
 * ## Security note — prototype traversal
 *
 * Identical defence to `forme-style-to-css`'s token resolver: deny
 * `__proto__` / `constructor` / `prototype` AND require
 * `hasOwnProperty`.  Independently coded here so this package
 * doesn't transitively depend on a translator backend.
 *
 * @module resolve
 */

import {
  isTokenRef,
  type Color, type FontStack, type Length, type Shadow,
  type StyleDocument, type TokenRef, type TokenSet,
} from "@coding-adventures/forme-style-ir";

/** Cap on token-chain hops.  See module docstring. */
const MAX_RESOLVE_DEPTH = 8;

/**
 * The concrete leaf-value types a `TokenRef` may resolve to.  Note
 * that `number` covers font weights and leading multipliers — the
 * two scalar tokens in `TypographyTokens`.
 */
export type ResolvedValue = Color | Length | Shadow | FontStack | number;

/**
 * Bulk-resolve a list of `TokenRef`s against a `StyleDocument`'s
 * tokens.  Returns a map keyed by `ref.path`; duplicate paths in the
 * input collapse to one entry (last-write-wins, but since the
 * resolution is pure, the value is identical anyway).
 */
export function resolveTokenRefs(
  doc: StyleDocument,
  refs: readonly TokenRef[],
): Map<string, ResolvedValue | null> {
  const out = new Map<string, ResolvedValue | null>();
  for (const ref of refs) {
    out.set(ref.path, resolveOne(ref, doc.tokens));
  }
  return out;
}

// ─── Internals ───────────────────────────────────────────────────────────

function resolveOne(
  ref: TokenRef,
  tokens: TokenSet,
  depth = 0,
): ResolvedValue | null {
  if (depth > MAX_RESOLVE_DEPTH) return null;
  const v = walkPath(ref.path, tokens as unknown as Record<string, unknown>);
  if (v === undefined) return null;
  if (isTokenRef(v)) return resolveOne(v as TokenRef, tokens, depth + 1);
  return narrow(v);
}

/**
 * Walk a dotted path through an object tree.  Returns undefined if
 * any segment is missing, non-object, or hits a prototype-pollution
 * shield.  See module docstring for the security rationale.
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

/**
 * Narrow a resolved value to one of the recognised leaf shapes.
 * Returns null for anything we don't recognise (intermediate object
 * nodes, undefined, NaN, …).
 */
function narrow(v: unknown): ResolvedValue | null {
  if (typeof v === "number") return Number.isFinite(v) ? v : null;
  if (isFontStack(v)) return v;
  if (isLength(v)) return v;
  if (isColor(v)) return v;
  if (isShadow(v)) return v;
  return null;
}

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
    isLength(s.offsetX) && isLength(s.offsetY)
    && isLength(s.blur) && isLength(s.spread)
    && s.color !== undefined
  );
}

function isFontStack(v: unknown): v is FontStack {
  return Array.isArray(v) && v.every((x) => typeof x === "string");
}
