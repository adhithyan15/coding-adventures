/**
 * @coding-adventures/forme-style-to-latex
 *
 * Second FM04 backend translator: Style IR → LaTeX preamble.
 *
 * ```ts
 * import { translateToLatex } from "@coding-adventures/forme-style-to-latex";
 * import { emptyStyleDocument, styleRuleId, sel } from "@coding-adventures/forme-style-ir";
 *
 * const doc = emptyStyleDocument();
 * // … populate doc.tokens and doc.rules …
 *
 * const { output, emittedRules, warnings } = translateToLatex(doc, {
 *   activeContexts: ["print"],
 * });
 * console.log(output);
 * // → "% forme-style-to-latex generated preamble\n\\newif\\ifprint ..."
 * ```
 *
 * Translation strategy (see FM04 §9.3):
 *
 * - **Selectors** become named macros (`\formeNodeParagraph`, etc).
 *   Composition selectors (and/or/not/nth/child-of/...) have no
 *   preamble equivalent — they warn-and-skip per FM04 §9.6.
 * - **Properties** map to native LaTeX commands where possible
 *   (`\color`, `\fontsize`, `\linespread`, `\setlength`, page-break
 *   penalties).  Properties with no preamble equivalent (shadow,
 *   opacity, max-width, padding, border, …) warn-and-skip.
 * - **Contexts** become `\if<flag>` conditional blocks; the translator
 *   emits the `\newif\if<flag>` declarations at the top so document
 *   authors can toggle.
 *
 * Same FM04 §9.6 robustness as the CSS translator: pure
 * (deterministic), never throws on shape, warn-skips on unknown
 * kinds / unresolved refs / non-expressible values.
 *
 * @module index
 */

export { translateToLatex } from "./translate.js";
export type { TranslateOptions, TranslateResult } from "./translate.js";

// Mappers re-exported so plugins can compose them when building
// extension-property translators of their own.
export { colorToLatex, lengthToLatex, fontStackToLatex, fontStackFallbacksComment } from "./value-mappers.js";
export { selectorToLatex } from "./selector-mapper.js";
export type { SelectorEmit } from "./selector-mapper.js";
export { contextToLatex, CONTEXT_FLAG_DECLARATIONS } from "./context-mapper.js";
export type { LatexConditional } from "./context-mapper.js";
export { propertyToLatex } from "./property-mappers.js";
export type { PropertyEmit } from "./property-mappers.js";
export {
  resolveRef,
  resolveColor, resolveLength, resolveShadow, resolveFontStack, resolveNumber,
} from "./token-resolver.js";
export { escapeLatexText, latexIdent } from "./escape.js";
