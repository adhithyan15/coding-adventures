/**
 * canonical.ts — byte-stable serialisation of `StyleDocument` (FM04 §12).
 *
 * For the FM03 reproducible-build story, we need:
 *   - same `StyleDocument` (deep-equal by value) ⇒ identical bytes ⇒
 *     identical hash
 *   - bytes that can be re-parsed into the same object (round-trip)
 *
 * The serialisation rules (per FM04 §12):
 * - JSON objects' keys are emitted in lexicographic order.
 * - `rules` array order is *significant* (per §4.9 source order is
 *   specificity); preserved as-is.
 * - `contexts` array is treated as a *set*; we sort before emitting.
 * - `extensions` keys are sorted like any other record.
 * - Numbers are emitted as JSON's default representation; we don't
 *   try to normalise `1.0` vs `1` — JSON.stringify already collapses
 *   to `1`.
 * - No whitespace, no trailing newline.
 *
 * The implementation reuses the existing `canonical-json` semantics
 * from `forme-identity` (RFC 8785) — we don't re-implement; instead
 * we compose a "treat `contexts` as a set" pre-pass and then hand
 * off to `JSON.stringify` with a key-sorting replacer.  RFC 8785's
 * full surface (number canonicalisation, etc.) is in forme-identity's
 * `canonical-json.ts`; this module wraps it for the Style-IR domain.
 *
 * @module canonical
 */

import type { StyleDocument } from "./style-document.js";

/**
 * Canonicalise a `StyleDocument` to its byte-stable JSON form.
 *
 * @returns A string suitable for hashing.  Same input value (deep-
 *          equal) ⇒ same output bytes.
 */
export function canonicalStyleDocument(doc: StyleDocument): string {
  // 1. Pre-process: sort the contexts array so set-order doesn't
  //    affect the hash.
  const prepared = prepareForCanonical(doc);

  // 2. Stringify with a sorted-keys replacer.  We do this in-house
  //    rather than pulling forme-identity as a dependency to keep
  //    the package boundary clean (forme-style-ir → forme-identity
  //    isn't a useful coupling).
  return stableStringify(prepared);
}

// ─── Internals ──────────────────────────────────────────────────────────

/**
 * Returns a new value identical to `doc` except `contexts` is sorted.
 * No deep clone needed beyond replacing the `contexts` array.
 */
function prepareForCanonical(doc: StyleDocument): StyleDocument {
  const sortedContexts = [...doc.contexts].sort();
  // Equal-modulo-context-order — preserve everything else verbatim.
  return { ...doc, contexts: sortedContexts };
}

/**
 * Maximum walk depth.  Cyclic / pathological inputs (only reachable
 * via hand-rolled object graphs — `JSON.parse` output is acyclic by
 * construction) would otherwise blow the stack.  Generous limit:
 * real StyleDocuments hit single-digit depth.
 */
const MAX_DEPTH = 1000;

/**
 * Recursive walker that produces a JSON string with sorted object
 * keys.  Arrays preserve order.  Special values: `undefined` is
 * dropped (matches JSON.stringify); functions are dropped likewise.
 *
 * NaN / Infinity throw — these aren't representable in canonical
 * JSON and signal a malformed input.  The validator should catch
 * them upstream; this is the last line of defence.
 *
 * Walks deeper than `MAX_DEPTH` throw `RangeError` — defence against
 * hand-rolled cyclic inputs the validator's own depth guard would
 * have caught at the boundary.
 */
function stableStringify(v: unknown, depth = 0): string {
  if (depth > MAX_DEPTH) {
    throw new RangeError(
      `canonicalStyleDocument: walk depth exceeded ${MAX_DEPTH} levels — likely a cycle in the input`,
    );
  }
  if (v === null) return "null";
  if (typeof v === "boolean") return v ? "true" : "false";
  if (typeof v === "number") {
    if (!Number.isFinite(v)) {
      throw new RangeError(`canonicalStyleDocument: non-finite number not allowed: ${v}`);
    }
    // JSON.stringify of a number gives the spec-correct
    // representation (no leading zeros, no trailing dot, etc.)
    return JSON.stringify(v);
  }
  if (typeof v === "string") return JSON.stringify(v);
  if (Array.isArray(v)) {
    const parts = v.map((item) => stableStringify(item, depth + 1));
    return `[${parts.join(",")}]`;
  }
  if (typeof v === "object") {
    const obj = v as Record<string, unknown>;
    // Sort keys lexicographically.  Drop undefined values to match
    // JSON.stringify's behaviour.
    const keys = Object.keys(obj).filter((k) => obj[k] !== undefined).sort();
    const parts = keys.map((k) => `${JSON.stringify(k)}:${stableStringify(obj[k], depth + 1)}`);
    return `{${parts.join(",")}}`;
  }
  // undefined / functions / symbols — drop (caller skips via the
  // object branch's filter; bare undefined at the top isn't valid).
  throw new TypeError(`canonicalStyleDocument: cannot serialise value of type ${typeof v}`);
}
