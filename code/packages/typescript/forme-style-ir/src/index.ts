/**
 * @coding-adventures/forme-style-ir
 *
 * The Style IR — design tokens, selectors, rules, contexts, and the
 * top-level `StyleDocument` value (FM04).
 *
 * This package is **pure types + a validator + a canonical-JSON
 * serializer**.  Translators (CSS, LaTeX, terminal, PDF) live in
 * separate packages; this is just the substrate they agree on.
 *
 * === Quick taste ===
 *
 * ```ts
 * import {
 *   styleRuleId, sel,
 *   type StyleDocument, type StyleProperty,
 * } from "@coding-adventures/forme-style-ir";
 *
 * const properties: readonly StyleProperty[] = [
 *   { kind: "color",       value: { kind: "token-ref", path: "colors.text" } },
 *   { kind: "font-family", value: { kind: "token-ref", path: "typography.families.body" } },
 *   { kind: "font-size",   value: { kind: "token-ref", path: "typography.scale.md" } },
 * ];
 *
 * const doc: StyleDocument = {
 *   kind: "StyleDocument",
 *   tokens: {
 *     colors: {
 *       text: { kind: "rgb", r: 31, g: 35, b: 40 },
 *     },
 *     typography: {
 *       families: { body: ["Inter", "system-ui", "sans-serif"] },
 *       scale:    { md: { unit: "rem", value: 1 } },
 *       weights:  { regular: 400 },
 *       leading:  { normal: 1.5 },
 *       tracking: { normal: { unit: "em", value: 0 } },
 *     },
 *     space:   {},
 *     radii:   {},
 *     shadows: {},
 *   },
 *   rules: [
 *     {
 *       id: styleRuleId("body-text"),
 *       selector: sel.type("paragraph"),
 *       properties,
 *     },
 *   ],
 *   contexts: [],
 *   theme: null,
 * };
 * ```
 *
 * === Modules ===
 *
 *   tokens.ts          — `TokenSet`, `Color`, `Length`, `Shadow`, `TokenRef`
 *   selectors.ts       — `Selector` union + `sel.*` constructors
 *   properties.ts      — `StyleProperty` union + value types
 *   contexts.ts        — context constants (`CONTEXT_PRINT`, …)
 *   style-document.ts  — `StyleDocument`, `Theme`, `StyleRule`, `StyleRuleId`
 *   style-error.ts     — `StyleError` (throw), `StyleWarning` (return)
 *   validate.ts        — `validateStyleDocument(value)`
 *   canonical.ts       — `canonicalStyleDocument(doc)` for hashing
 *
 * @module index
 */

// Tokens
export type {
  Color, Length, Shadow, TokenRef, TokenSet,
  TypographyTokens, FontStack, LengthUnit,
} from "./tokens.js";
export { LENGTH_UNITS, isTokenRef, emptyTokenSet } from "./tokens.js";

// Selectors
export type {
  Selector, SelectorKind,
  NodeTypeSelector, NodeTypeLevelSelector, CustomKindSelector,
  TagSelector, IdSelector, RoleSelector,
  NthSelector, NthFormula,
  ChildOfSelector, DescendantOfSelector, AdjacentSelector,
  AndSelector, OrSelector, NotSelector,
} from "./selectors.js";
export { SELECTOR_KINDS, sel } from "./selectors.js";

// Properties
export type {
  StyleProperty, PropertyKind,
  ColorProperty, BackgroundProperty, BorderColorProperty, OutlineColorProperty,
  FontFamilyProperty, FontSizeProperty, FontWeightProperty, FontStyleProperty,
  TextTransformProperty, LeadingProperty, TrackingProperty, TextDecorationProperty,
  SpaceBeforeProperty, SpaceAfterProperty, IndentProperty, PaddingProperty,
  MaxWidthProperty, MinHeightProperty, AlignProperty, VerticalAlignProperty,
  BorderProperty, BorderRadiusProperty, ShadowProperty, OpacityProperty,
  ColumnBreakProperty, PageBreakProperty, WidowOrphanProperty,
  DisplayProperty, VisibleProperty, ExtensionProperty,
  BoxSides, TextDecoration, BorderSpec,
} from "./properties.js";
export { PROPERTY_KINDS, isExtensionKind } from "./properties.js";

// Contexts
export {
  CONTEXT_PRINT, CONTEXT_SCREEN, CONTEXT_DARK,
  CONTEXT_NARROW, CONTEXT_WIDE,
  CONTEXT_REDUCED_MOTION, CONTEXT_HIGH_CONTRAST,
  STANDARD_CONTEXTS,
  isExtensionContext, isRecognisedContext,
} from "./contexts.js";
export type { StandardContext } from "./contexts.js";

// Style document + theme
export type {
  StyleDocument, StyleRule, StyleRuleId, Theme,
} from "./style-document.js";
export { styleRuleId, emptyStyleDocument } from "./style-document.js";

// Errors + warnings
export type {
  StyleErrorCode, StyleErrorEntry, StyleWarning,
} from "./style-error.js";
export { StyleError, STYLE_ERROR_CODES } from "./style-error.js";

// Validator
export { validateStyleDocument } from "./validate.js";
export type { ValidatedStyleDocument } from "./validate.js";

// Canonical
export { canonicalStyleDocument } from "./canonical.js";
