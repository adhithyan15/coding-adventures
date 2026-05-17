/**
 * selector-mapper.ts — Selector → description string (FM04 §9.4).
 *
 * Terminals don't have a document tree to walk against.  The Map
 * key in our output is the **rule id** (already opaque + unique),
 * NOT a selector slug — consumers look up by id when they reach a
 * node in their own document model whose `usedStyle` set contains
 * that id.
 *
 * This module therefore produces a human-readable *description*
 * string only, suitable as a comment in the generated output for
 * traceability ("rule X applies to: heading level 1").  Composition
 * selectors (and/or/not/...) get a structural rendering ("and(p,
 * .intro)") rather than warn-skip — they don't need to drive output
 * machinery; they just narrate.
 *
 * @module selector-mapper
 */

import type { Selector } from "@coding-adventures/forme-style-ir";

/**
 * Defensive depth cap matching `token-resolver.ts`'s
 * `MAX_RESOLVE_DEPTH`.  Real selectors hit single-digit depth;
 * an adversarial hand-rolled IR could nest composition operators
 * arbitrarily, blowing the JS call stack.  At the limit we return
 * a truncation marker rather than throw — the translator never
 * crashes on shape (FM04 §9.6).
 */
const MAX_DESC_DEPTH = 64;

/**
 * Format a `Selector` as a one-line human-readable description.
 * Never returns an empty string and never throws.
 *
 * @param sel    selector to describe
 * @param depth  recursion depth — internal; default 0
 */
export function selectorDescription(sel: Selector, depth = 0): string {
  if (depth > MAX_DESC_DEPTH) return "…(truncated)";
  switch (sel.kind) {
    case "node-type":        return `node-type:${safe(sel.type)}`;
    case "node-type-level":  return `heading-level:${sel.level}`;
    case "custom-kind":      return `custom-kind:${safe(sel.customKind)}`;
    case "tag":              return `tag:${safe(sel.tag)}`;
    case "id":               return `id:${safe(sel.id)}`;
    case "role":             return `role:${safe(sel.role)}`;
    case "nth": {
      const inner = selectorDescription(sel.of, depth + 1);
      const idx = typeof sel.n === "number"
        ? `${sel.n}`
        : `${sel.n.a}n${sel.n.b >= 0 ? "+" : ""}${sel.n.b}${sel.n.fromEnd ? " from-end" : ""}`;
      return `nth(${idx}, ${inner})`;
    }
    case "child-of":         return `child-of(${selectorDescription(sel.parent, depth + 1)}, ${selectorDescription(sel.child, depth + 1)})`;
    case "descendant-of":    return `descendant-of(${selectorDescription(sel.ancestor, depth + 1)}, ${selectorDescription(sel.descendant, depth + 1)})`;
    case "adjacent":         return `adjacent(${selectorDescription(sel.previous, depth + 1)}, ${selectorDescription(sel.following, depth + 1)})`;
    case "and":              return `and(${sel.all.map((s) => selectorDescription(s, depth + 1)).join(", ")})`;
    case "or":               return `or(${sel.any.map((s) => selectorDescription(s, depth + 1)).join(", ")})`;
    case "not":              return `not(${selectorDescription(sel.inner, depth + 1)})`;
  }
}

/**
 * Sanitise a free-form name for inclusion in a description string.
 * Strip ANSI-unsafe control bytes so a hand-rolled IR that bypasses
 * the validator can't inject a cursor-move into the *comment* of
 * the generated source.
 *
 * (The TS-string-escape pass on the output side ALSO neutralises
 * the bytes — defense in depth.)
 */
function safe(s: string): string {
  // eslint-disable-next-line no-control-regex
  return s.replace(/[\x00-\x1F\x7F-\x9F]/g, "");
}
