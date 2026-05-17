/**
 * selector-mapper.ts — `Selector` → LaTeX macro name (FM04 §9.3).
 *
 * LaTeX has no equivalent of CSS selectors — there's no document
 * tree to walk against in the preamble.  Instead each Style IR rule
 * becomes a named macro the document body calls:
 *
 *   \newcommand{\formeNodeParagraph}{...style commands...}
 *   ...
 *   ...in the body...
 *   {\formeNodeParagraph Some paragraph text.\par}
 *
 * Each *simple* selector kind produces a stable macro name:
 *
 *   node-type        → \formeNode<Type>           (e.g. \formeNodeParagraph)
 *   node-type-level  → \formeHeading<level>       (e.g. \formeHeading1)
 *   custom-kind      → \formeKind<sluggedName>
 *   tag              → \formeTag<sluggedName>
 *   id               → \formeId<sluggedName>
 *   role             → \formeRole<sluggedName>
 *
 * Composition selectors (`and`, `or`, `not`, `child-of`, `descendant-of`,
 * `adjacent`, `nth`) have no preamble-level equivalent — translating
 * them would require runtime document-walking machinery LaTeX doesn't
 * have at the macro level.  These return a `warning` result; the
 * translator skips the rule per FM04 §9.6.
 *
 * Macro names are always TitleCase ASCII identifiers — `latexIdent`
 * encodes anything outside `[A-Za-z]` as `Z<hex>Z` so the result is
 * always a valid `\newcommand` argument.
 *
 * @module selector-mapper
 */

import type { Selector } from "@coding-adventures/forme-style-ir";
import { latexIdent } from "./escape.js";

/** Either a resolved macro name, or a warning explaining why we
 *  can't make one. */
export type SelectorEmit =
  | { ok: true; macroName: string; description: string }
  | { ok: false; warning: string };

/**
 * Format a `Selector` as a LaTeX macro name + a one-line description
 * (used as a comment above the `\newcommand`).  Returns a warning
 * for composition selectors that have no preamble equivalent.
 */
export function selectorToLatex(sel: Selector): SelectorEmit {
  switch (sel.kind) {
    case "node-type":
      return {
        ok: true,
        macroName: `\\formeNode${capitalise(latexIdent(sel.type))}`,
        description: `node type: ${sel.type}`,
      };
    case "node-type-level":
      return {
        ok: true,
        macroName: `\\formeHeading${arabicToLetter(sel.level)}`,
        description: `heading level: ${sel.level}`,
      };
    case "custom-kind":
      return {
        ok: true,
        macroName: `\\formeKind${capitalise(latexIdent(sel.customKind))}`,
        description: `custom kind: ${sel.customKind}`,
      };
    case "tag":
      return {
        ok: true,
        macroName: `\\formeTag${capitalise(latexIdent(sel.tag))}`,
        description: `tag: ${sel.tag}`,
      };
    case "id":
      return {
        ok: true,
        macroName: `\\formeId${capitalise(latexIdent(sel.id))}`,
        description: `id: ${sel.id}`,
      };
    case "role":
      return {
        ok: true,
        macroName: `\\formeRole${capitalise(latexIdent(sel.role))}`,
        description: `role: ${sel.role}`,
      };
    case "nth":
    case "child-of":
    case "descendant-of":
    case "adjacent":
    case "and":
    case "or":
    case "not":
      return {
        ok: false,
        warning: `selector kind ${JSON.stringify(sel.kind)} has no LaTeX preamble equivalent (composition / structural selectors require runtime document-tree walking)`,
      };
  }
}

// ─── Helpers ─────────────────────────────────────────────────────────────

function capitalise(s: string): string {
  if (s.length === 0) return s;
  return s[0]!.toUpperCase() + s.slice(1);
}

/**
 * Map a small Arabic numeral to its LaTeX-safe English letter form.
 * LaTeX commands can't contain digits — `\formeHeading1` is invalid.
 * Heading levels are 1–6 by convention; the named-letter form keeps
 * the common case readable.
 *
 * Out-of-range numerics (negative, fractional, NaN, Infinity) route
 * through `latexIdent` for defence in depth.  Decimal points and
 * minus signs (`-1`, `1.5`) aren't LaTeX *specials* — they wouldn't
 * cause injection — but they ARE invalid in a `\command` name and
 * would produce broken output.  `latexIdent` encodes them as
 * `Z<hex>Z` so the resulting macro name is always a letter run.
 */
function arabicToLetter(n: number): string {
  const names = ["Zero", "One", "Two", "Three", "Four", "Five", "Six"];
  if (Number.isInteger(n) && n >= 0 && n < names.length) return names[n]!;
  return latexIdent(String(n));
}
