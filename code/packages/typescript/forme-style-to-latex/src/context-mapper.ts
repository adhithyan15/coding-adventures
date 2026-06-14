/**
 * context-mapper.ts — context name → LaTeX conditional macro (FM04 §9.3).
 *
 * LaTeX has no built-in equivalent of CSS `@media` queries — context
 * switches must be driven by user-controlled flags (the document's
 * preamble sets `\printtrue` / `\printfalse`, the translator emits
 * `\ifprint ... \fi`).
 *
 * We map each kernel-blessed context to a conventional flag name:
 *
 *   print          → \ifprint        ... \fi
 *   screen         → \ifscreen       ... \fi
 *   dark           → \ifdark         ... \fi
 *   narrow         → \ifnarrow       ... \fi
 *   wide           → \ifwide         ... \fi
 *   reduced-motion → \ifreducedmotion ... \fi
 *   high-contrast  → \ifhighcontrast ... \fi
 *
 * The caller (translator) emits a preamble header that defines these
 * flags as `\newif` once at the top of the output, so the rule
 * bodies can use them without further setup.
 *
 * `ext:*` contexts have no built-in mapping and return null — the
 * translator emits a warning and skips the rule.
 *
 * @module context-mapper
 */

/** Result: the conditional command prefix, e.g. `\ifprint`. */
export type LatexConditional = string;

/**
 * Map a context name to a LaTeX `\if<name>` conditional.  Returns
 * null for unknown / `ext:*` contexts — the translator emits a
 * warning at the call site.
 */
export function contextToLatex(name: string): LatexConditional | null {
  switch (name) {
    case "print":          return "\\ifprint";
    case "screen":         return "\\ifscreen";
    case "dark":           return "\\ifdark";
    case "narrow":         return "\\ifnarrow";
    case "wide":           return "\\ifwide";
    case "reduced-motion": return "\\ifreducedmotion";
    case "high-contrast":  return "\\ifhighcontrast";
    default:               return null;
  }
}

/**
 * The list of `\newif` declarations the translator emits at the top
 * of the preamble so every conditional name above is defined,
 * regardless of whether the document actually uses every context.
 *
 * Each `\newif\if<name>` simultaneously defines `\<name>true` and
 * `\<name>false` so the document author can switch contexts on/off.
 */
export const CONTEXT_FLAG_DECLARATIONS: readonly string[] = Object.freeze([
  "\\newif\\ifprint",
  "\\newif\\ifscreen",
  "\\newif\\ifdark",
  "\\newif\\ifnarrow",
  "\\newif\\ifwide",
  "\\newif\\ifreducedmotion",
  "\\newif\\ifhighcontrast",
]);
