/**
 * @coding-adventures/forme-style-to-terminal
 *
 * Third FM04 backend translator: Style IR → terminal ANSI.
 *
 * ```ts
 * import { translateToTerminal } from "@coding-adventures/forme-style-to-terminal";
 * import { emptyStyleDocument, styleRuleId, sel } from "@coding-adventures/forme-style-ir";
 *
 * const doc = emptyStyleDocument();
 * // … populate doc.tokens and doc.rules …
 *
 * const { output, emittedRules, warnings } = translateToTerminal(doc, {
 *   activeContexts: ["screen"],
 * });
 *
 * // output is a TS module source string exporting
 * //   const formeStyles: ReadonlyMap<string, AnsiStyle>
 * // where AnsiStyle is { prefix: string; suffix: string }.
 * // Consumers wrap document content with prefix + content + suffix.
 * ```
 *
 * Translation strategy (FM04 §9.4):
 *
 * - **Selectors** become Map keys (the rule id, optionally prefixed
 *   by `scope`).  Composition selectors don't drive output — they
 *   appear in the per-rule comment for traceability.
 * - **Properties** map to 24-bit truecolour SGR for color /
 *   background, plus bold / italic / underline / strikethrough /
 *   overline / conceal toggles.  Everything that needs page
 *   geometry (padding, max-width, page-break, etc.) warn-skips —
 *   terminals are a character grid.
 * - **Contexts** filter rules through `activeContexts` (kernel set
 *   only; `ext:*` warn-skips).  No per-context conditional emission
 *   machinery — the terminal IS what it is at render time.
 *
 * Same FM04 §9.6 robustness as the other translators: pure
 * (deterministic), never throws on shape, warn-skips on unknown
 * kinds / unresolved refs / non-expressible values.
 *
 * @module index
 */

export { translateToTerminal } from "./translate.js";
export type { TranslateOptions, TranslateResult } from "./translate.js";

// Mappers re-exported so plugins can compose them when building
// extension-property translators of their own.
export {
  colorToRgbTriple, colorToSgrFg, colorToSgrBg,
} from "./value-mappers.js";
export { selectorDescription } from "./selector-mapper.js";
export { contextRecognised } from "./context-mapper.js";
export { propertyToTerminal } from "./property-mappers.js";
export type { PropertyEmit } from "./property-mappers.js";
export {
  resolveRef,
  resolveColor, resolveLength, resolveShadow, resolveFontStack, resolveNumber,
} from "./token-resolver.js";
export {
  stripAnsiUnsafe, escapeTsString, sanitiseKey,
} from "./escape.js";
