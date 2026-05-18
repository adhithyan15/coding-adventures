/**
 * @coding-adventures/forme-style-to-css
 *
 * Reference Style IR translator (FM04 §9.2).  Takes a
 * `StyleDocument` and emits a CSS string plus the per-rule metadata
 * the AOT compiler (FM06) uses for per-page slicing.
 *
 * ```ts
 * import { translateToCss } from "@coding-adventures/forme-style-to-css";
 * import { emptyStyleDocument, styleRuleId, sel } from "@coding-adventures/forme-style-ir";
 *
 * const doc = emptyStyleDocument();
 * // … populate doc.tokens and doc.rules …
 *
 * const { output, emittedRules, warnings } = translateToCss(doc, {
 *   activeContexts: ["screen"],
 * });
 *
 * console.log(output);
 * // → "p {\n  color: rgb(31 35 40);\n}\n..."
 * ```
 *
 * The translator is **pure**: same input → byte-identical output
 * (no time, no random, no ambient I/O), which drives FM03
 * reproducible builds.  It also **never throws** on Style IR shape
 * issues — that's the validator's job.  Unknown property kinds,
 * `ext:*` contexts without translators, and unresolved `TokenRef`s
 * all emit `StyleWarning`s and are skipped (FM04 §9.6).
 *
 * Three options on `translateToCss`:
 *
 * - `activeContexts: readonly string[]` — which named contexts the
 *   consumer wants active.  Rules with a `context` field apply only
 *   when their context is in this list.
 * - `usedRuleIds?: readonly StyleRuleId[]` — per-page CSS slicing.
 *   When set, ONLY rules with these ids are emitted.  Drives FM06.
 * - `scope?: string` — optional CSS prefix applied to every
 *   selector.  Used for per-page CSS scoping.
 *
 * @module index
 */

export { translateToCss } from "./translate.js";
export type { TranslateOptions, TranslateResult } from "./translate.js";

// Mappers are exported so plugins can compose them when building
// extension property translators of their own.
export { colorToCss, lengthToCss, fontStackToCss, shadowToCss } from "./value-mappers.js";
export { selectorToCss } from "./selector-mapper.js";
export { contextToMedia } from "./context-mapper.js";
export type { MediaQuery } from "./context-mapper.js";
export { propertyToCss } from "./property-mappers.js";
export type { PropertyEmit } from "./property-mappers.js";
export {
  resolveRef,
  resolveColor, resolveLength, resolveShadow, resolveFontStack, resolveNumber,
} from "./token-resolver.js";
