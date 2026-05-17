/**
 * selector-mapper.ts — `Selector` → CSS selector string (FM04 §9.2).
 *
 * Mapping table:
 *
 *   node-type          → element selector (`p`, `blockquote`, …)
 *   node-type-level    → `h1`..`h6`
 *   custom-kind        → `[data-kind="<name>"]`
 *   tag                → `[data-tag~="<name>"]`
 *   id                 → `#<id>`
 *   role               → `[role="<name>"]`
 *   nth (number)       → `:nth-child(<n+1>)`     (1-based in CSS)
 *   nth (number,end)   → `:nth-last-child(<n+1>)`
 *   nth (formula)      → `:nth-child(<an+b>)` or `:nth-last-child(...)`
 *   child-of           → `<parent> > <child>`
 *   descendant-of      → `<ancestor> <descendant>`
 *   adjacent           → `<previous> + <following>`
 *   and                → concatenate (no separator)
 *   or                 → comma-separate (each path emitted then joined)
 *   not                → `:not(<inner>)`
 *
 * The `nth` 0-based-to-1-based shift matters: the IR's `n: 0` means
 * "first element" (zero-indexed), which is CSS's `:nth-child(1)`.
 * This was a conscious IR-design choice — programmers index from 0;
 * CSS just happens to use a 1-based convention.  We translate at
 * the boundary.
 *
 * @module selector-mapper
 */

import type { Selector } from "@coding-adventures/forme-style-ir";

/**
 * Format a `Selector` as a CSS selector string.  Composition forms
 * (`and`, `or`, `not`, `nth`) recurse; depth is bounded by the
 * forme-style-ir validator (1000 levels) so no extra guard here.
 *
 * `or` is the only form that produces multiple comma-separated
 * paths.  Inner composition (`and(or(...), x)`) requires expanding
 * each `or`-branch into its own path before joining — we handle
 * that by computing per-path expansions for every level.
 */
export function selectorToCss(sel: Selector): string {
  // For top-level emission, just join `or` paths with commas.
  const paths = expandPaths(sel);
  return paths.join(", ");
}

/**
 * Expand a selector into the list of comma-separated CSS paths.
 * `or` is the only form that splits paths; everything else returns
 * a single-element array.
 */
function expandPaths(sel: Selector): string[] {
  switch (sel.kind) {
    case "node-type":
      return [escapeIdent(sel.type)];
    case "node-type-level":
      return [`h${sel.level}`];
    case "custom-kind":
      return [`[data-kind="${escapeAttrValue(sel.customKind)}"]`];
    case "tag":
      return [`[data-tag~="${escapeAttrValue(sel.tag)}"]`];
    case "id":
      return [`#${escapeIdent(sel.id)}`];
    case "role":
      return [`[role="${escapeAttrValue(sel.role)}"]`];
    case "nth": {
      const inner = expandPaths(sel.of);
      const indices = formatNth(sel.n);
      // Apply :nth-child suffix to every path the inner produces.
      return inner.map((p) => p + indices);
    }
    case "child-of": {
      const parents = expandPaths(sel.parent);
      const children = expandPaths(sel.child);
      const out: string[] = [];
      for (const p of parents) for (const c of children) out.push(`${p} > ${c}`);
      return out;
    }
    case "descendant-of": {
      const ancestors = expandPaths(sel.ancestor);
      const descendants = expandPaths(sel.descendant);
      const out: string[] = [];
      for (const a of ancestors) for (const d of descendants) out.push(`${a} ${d}`);
      return out;
    }
    case "adjacent": {
      const prevs = expandPaths(sel.previous);
      const nexts = expandPaths(sel.following);
      const out: string[] = [];
      for (const p of prevs) for (const n of nexts) out.push(`${p} + ${n}`);
      return out;
    }
    case "and": {
      // Cartesian product across the components.  Most uses are
      // small; nested or() inside and() multiplies, but that's exactly
      // what the user is asking for.
      let acc: string[] = [""];
      for (const inner of sel.all) {
        const exp = expandPaths(inner);
        const next: string[] = [];
        for (const a of acc) for (const e of exp) next.push(a + e);
        acc = next;
      }
      return acc;
    }
    case "or": {
      const out: string[] = [];
      for (const inner of sel.any) out.push(...expandPaths(inner));
      return out;
    }
    case "not": {
      const inner = expandPaths(sel.inner);
      // Per CSS Selectors L4, `:not()` accepts a comma-separated list.
      return [`:not(${inner.join(", ")})`];
    }
  }
}

/**
 * Format a CSS `:nth-child` index/formula suffix.  Style IR uses
 * 0-based indices; CSS uses 1-based.  We translate at this boundary.
 */
function formatNth(n: number | { a: number; b: number; fromEnd?: boolean }): string {
  if (typeof n === "number") {
    return `:nth-child(${n + 1})`;
  }
  const formula = `${n.a}n${n.b >= 0 ? `+${n.b}` : n.b}`;
  return n.fromEnd ? `:nth-last-child(${formula})` : `:nth-child(${formula})`;
}

// ─── Escaping ────────────────────────────────────────────────────────────

/**
 * CSS identifier escape.  Identifiers must match
 * `[-_a-zA-Z][a-zA-Z0-9_-]*` per CSS Syntax L3 (with leading-digit
 * rules we don't need to support — the validator already rejects
 * empty names).  For safety against attacker-controlled identifiers
 * we escape characters outside `[a-zA-Z0-9_-]` using CSS escape
 * sequences.  Real-world uses (HTML element names, data attribute
 * names) are clean ASCII so this is a defensive no-op in practice.
 */
function escapeIdent(s: string): string {
  return s.replace(/[^a-zA-Z0-9_-]/g, (ch) => {
    const code = ch.codePointAt(0)!;
    return `\\${code.toString(16)} `;
  });
}

/**
 * CSS attribute-value escape.  We wrap in double quotes; escape any
 * `"` and `\` inside.  Backslash escape is the CSS Strings L3 form.
 */
function escapeAttrValue(s: string): string {
  return s.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
}
