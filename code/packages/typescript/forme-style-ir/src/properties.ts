/**
 * properties.ts — closed-list style properties (FM04 §5).
 *
 * The Style IR's property vocabulary is intentionally *finite*.  CSS
 * has ~400 properties; documents need about thirty.  We define the
 * common thirty in a discriminated union so backends (CSS, LaTeX,
 * PDF, terminal) get exhaustive type-safety: a `switch (prop.kind)`
 * covers every case the type system knows about, and adding a new
 * property kind triggers a compile error in every backend that
 * doesn't yet handle it — exactly when we want to know.
 *
 * Anything outside the closed list lives under `ext:<plugin>:<name>`
 * (see `ExtensionProperty`).  Backends that understand a given
 * extension handle it; others ignore it per FM04 §7.4.
 *
 * The `important` flag promotes a property's priority among multiple
 * rules matching the same node.  Use sparingly — it's the same
 * footgun CSS's `!important` is.
 *
 * @module properties
 */

import type { JsonValue } from "@coding-adventures/forme-types";
import type { Color, FontStack, Length, Shadow, TokenRef } from "./tokens.js";

// ─── Supporting value types ──────────────────────────────────────────────

/** Generic per-side struct.  Used by padding; extensible to borders. */
export interface BoxSides<T> {
  readonly top: T;
  readonly right: T;
  readonly bottom: T;
  readonly left: T;
}

/** Text decoration (underline, strike-through, ...). */
export interface TextDecoration {
  readonly line: "none" | "underline" | "overline" | "line-through";
  readonly style?: "solid" | "dashed" | "dotted" | "wavy";
  readonly color?: Color | TokenRef;
  readonly thickness?: Length;
}

/** Border specification.  `sides` optional: omit for "all sides". */
export interface BorderSpec {
  readonly width: Length;
  readonly style: "none" | "solid" | "dashed" | "dotted" | "double";
  readonly color: Color | TokenRef;
  readonly sides?: ReadonlyArray<"top" | "right" | "bottom" | "left">;
}

// ─── The property union ───────────────────────────────────────────────────
//
// Each variant follows the same shape: a `kind` discriminant, a
// `value` typed appropriately, and an optional `important` boolean.
// Most properties accept either a literal value or a `TokenRef` so
// rules can defer to the theme.

// Color / fill
export interface ColorProperty           { readonly kind: "color"; readonly value: Color | TokenRef; readonly important?: boolean }
export interface BackgroundProperty      { readonly kind: "background"; readonly value: Color | TokenRef; readonly important?: boolean }
export interface BorderColorProperty     { readonly kind: "border-color"; readonly value: Color | TokenRef; readonly important?: boolean }
export interface OutlineColorProperty    { readonly kind: "outline-color"; readonly value: Color | TokenRef; readonly important?: boolean }

// Typography
export interface FontFamilyProperty      { readonly kind: "font-family"; readonly value: FontStack | TokenRef; readonly important?: boolean }
export interface FontSizeProperty        { readonly kind: "font-size"; readonly value: Length | TokenRef; readonly important?: boolean }
export interface FontWeightProperty      { readonly kind: "font-weight"; readonly value: number | TokenRef; readonly important?: boolean }
export interface FontStyleProperty       { readonly kind: "font-style"; readonly value: "normal" | "italic" | "oblique"; readonly important?: boolean }
export interface TextTransformProperty   { readonly kind: "text-transform"; readonly value: "none" | "uppercase" | "lowercase" | "capitalize"; readonly important?: boolean }
export interface LeadingProperty         { readonly kind: "leading"; readonly value: number | TokenRef; readonly important?: boolean }
export interface TrackingProperty        { readonly kind: "tracking"; readonly value: Length | TokenRef; readonly important?: boolean }
export interface TextDecorationProperty  { readonly kind: "text-decoration"; readonly value: TextDecoration; readonly important?: boolean }

// Layout / spacing
export interface SpaceBeforeProperty     { readonly kind: "space-before"; readonly value: Length | TokenRef; readonly important?: boolean }
export interface SpaceAfterProperty      { readonly kind: "space-after"; readonly value: Length | TokenRef; readonly important?: boolean }
export interface IndentProperty          { readonly kind: "indent"; readonly value: Length | TokenRef; readonly important?: boolean }
export interface PaddingProperty         { readonly kind: "padding"; readonly value: BoxSides<Length | TokenRef>; readonly important?: boolean }
export interface MaxWidthProperty        { readonly kind: "max-width"; readonly value: Length | TokenRef; readonly important?: boolean }
export interface MinHeightProperty       { readonly kind: "min-height"; readonly value: Length | TokenRef; readonly important?: boolean }
export interface AlignProperty           { readonly kind: "align"; readonly value: "start" | "end" | "center" | "justify"; readonly important?: boolean }
export interface VerticalAlignProperty   { readonly kind: "vertical-align"; readonly value: "baseline" | "top" | "middle" | "bottom"; readonly important?: boolean }

// Decoration
export interface BorderProperty          { readonly kind: "border"; readonly value: BorderSpec; readonly important?: boolean }
export interface BorderRadiusProperty    { readonly kind: "border-radius"; readonly value: Length | TokenRef; readonly important?: boolean }
export interface ShadowProperty          { readonly kind: "shadow"; readonly value: Shadow | TokenRef; readonly important?: boolean }
export interface OpacityProperty         { readonly kind: "opacity"; readonly value: number; readonly important?: boolean }

// Page break (print)
export interface ColumnBreakProperty     { readonly kind: "column-break"; readonly value: "before" | "after" | "avoid"; readonly important?: boolean }
export interface PageBreakProperty       { readonly kind: "page-break"; readonly value: "before" | "after" | "avoid"; readonly important?: boolean }
export interface WidowOrphanProperty     { readonly kind: "widow-orphan"; readonly value: number; readonly important?: boolean }

// Visibility
export interface DisplayProperty         { readonly kind: "display"; readonly value: "block" | "inline" | "inline-block" | "none"; readonly important?: boolean }
export interface VisibleProperty         { readonly kind: "visible"; readonly value: boolean; readonly important?: boolean }

// Extension namespace
//
// Plugin-contributed property kinds.  The kind discriminant is a
// template literal type — any string starting with `ext:` is an
// extension property.  The value is opaque `JsonValue`; only the
// contributing plugin's translator understands its shape.
export interface ExtensionProperty       { readonly kind: `ext:${string}`; readonly value: JsonValue; readonly important?: boolean }

/**
 * The full `StyleProperty` discriminated union.  Backends should
 * exhaustively `switch` on `prop.kind`.  Adding a new variant is a
 * backward-compatible minor-version bump.
 */
export type StyleProperty =
  | ColorProperty
  | BackgroundProperty
  | BorderColorProperty
  | OutlineColorProperty
  | FontFamilyProperty
  | FontSizeProperty
  | FontWeightProperty
  | FontStyleProperty
  | TextTransformProperty
  | LeadingProperty
  | TrackingProperty
  | TextDecorationProperty
  | SpaceBeforeProperty
  | SpaceAfterProperty
  | IndentProperty
  | PaddingProperty
  | MaxWidthProperty
  | MinHeightProperty
  | AlignProperty
  | VerticalAlignProperty
  | BorderProperty
  | BorderRadiusProperty
  | ShadowProperty
  | OpacityProperty
  | ColumnBreakProperty
  | PageBreakProperty
  | WidowOrphanProperty
  | DisplayProperty
  | VisibleProperty
  | ExtensionProperty;

/**
 * Frozen list of *kernel-known* property kinds (excludes `ext:*`,
 * which is open-ended).  The validator uses this set to detect typos
 * — a property whose `kind` is not in this list and not an `ext:*`
 * prefix is rejected.
 */
export const PROPERTY_KINDS = Object.freeze([
  "color",
  "background",
  "border-color",
  "outline-color",
  "font-family",
  "font-size",
  "font-weight",
  "font-style",
  "text-transform",
  "leading",
  "tracking",
  "text-decoration",
  "space-before",
  "space-after",
  "indent",
  "padding",
  "max-width",
  "min-height",
  "align",
  "vertical-align",
  "border",
  "border-radius",
  "shadow",
  "opacity",
  "column-break",
  "page-break",
  "widow-orphan",
  "display",
  "visible",
] as const);

export type PropertyKind = (typeof PROPERTY_KINDS)[number];

/** Detect an `ext:` namespaced property kind without instantiating regex. */
export function isExtensionKind(kind: string): kind is `ext:${string}` {
  return kind.startsWith("ext:") && kind.length > 4;
}
