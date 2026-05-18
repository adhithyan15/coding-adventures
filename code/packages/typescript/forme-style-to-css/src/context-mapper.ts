/**
 * context-mapper.ts — context name → `@media` query (FM04 §9.2).
 *
 * Standard mappings (FM04 §9.2):
 *
 *   print          → @media print
 *   screen         → @media screen
 *   dark           → @media (prefers-color-scheme: dark)
 *   narrow         → @media (max-width: 40rem)
 *   wide           → @media (min-width: 80rem)
 *   reduced-motion → @media (prefers-reduced-motion: reduce)
 *   high-contrast  → @media (prefers-contrast: more)
 *
 * Plugin-contributed `ext:*` contexts have no built-in mapping —
 * the caller (translator) emits a warning and skips the rule.
 *
 * @module context-mapper
 */

/** A `@media` query body, e.g. `screen`, `(max-width: 40rem)`. */
export type MediaQuery = string;

/**
 * Map a kernel-blessed context name to a CSS `@media` query body.
 * Returns null for unknown / `ext:*` contexts — the translator emits
 * a warning at the call site.
 */
export function contextToMedia(name: string): MediaQuery | null {
  switch (name) {
    case "print":          return "print";
    case "screen":         return "screen";
    case "dark":           return "(prefers-color-scheme: dark)";
    case "narrow":         return "(max-width: 40rem)";
    case "wide":           return "(min-width: 80rem)";
    case "reduced-motion": return "(prefers-reduced-motion: reduce)";
    case "high-contrast":  return "(prefers-contrast: more)";
    default:               return null;
  }
}
