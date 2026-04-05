/**
 * Layout IR — Universal Layout Intermediate Representation
 *
 * The Layout IR is the shared vocabulary between content producers (Mosaic IR,
 * DocumentAST, LaTeX IR) and layout algorithms (flexbox, block, grid). It sits
 * in the middle of the pipeline:
 *
 *   Producer → front-end converter → LayoutNode tree → layout algorithm
 *            → PositionedNode tree → layout-to-paint → PaintScene → renderer
 *
 * Design principles
 * -----------------
 *
 * 1. **Dumb data.** This package contains no logic — only types and builder
 *    helpers. The types describe structure; the logic lives in the algorithm
 *    packages downstream.
 *
 * 2. **No smartness.** The IR never validates that the right algorithm was
 *    chosen or that ext fields are correct. Wrong algorithm → wrong output.
 *    That is the caller's responsibility.
 *
 * 3. **Open extension bag.** Each algorithm adds its own schema to the `ext`
 *    map without modifying the core types. A node can carry data for multiple
 *    algorithms simultaneously.
 *
 * 4. **Logical units.** All measurements are in abstract logical units. The
 *    renderer converts to physical pixels by applying a device pixel ratio.
 *
 * Units and Coordinate Space
 * --------------------------
 *
 * "Logical units" are abstract real numbers. A `FontSpec.size` of 16 means
 * "16 logical units tall". A layout algorithm works entirely in logical units
 * and returns positions in the same coordinate space. The `layout-to-paint`
 * package applies `devicePixelRatio` exactly once, at the boundary with the
 * renderer, producing physical pixel coordinates.
 *
 * This means: never multiply by `devicePixelRatio` inside a layout algorithm.
 * The layout algorithms are pure math over logical units.
 */

// ============================================================================
// SizeValue
// ============================================================================

/**
 * A size hint for width or height. Three variants:
 *
 *   - `fixed(v)` — exactly v logical units, no flex
 *   - `fill`     — fill all available space (like CSS `flex: 1` or `width: 100%`)
 *   - `wrap`     — shrink to fit content (like CSS `fit-content` or `width: auto`)
 *
 * A `null` width or height on a `LayoutNode` means "no hint from this property".
 * The layout algorithm decides what to do — usually treated as `wrap`.
 *
 * Example — three items in a row, middle item fills remaining space:
 *
 *   left:   size_fixed(80)   → always 80 logical units wide
 *   center: size_fill()      → fills whatever the left and right don't use
 *   right:  size_fixed(120)  → always 120 logical units wide
 */
export type SizeValue =
  | { kind: "fixed"; value: number }
  | { kind: "fill" }
  | { kind: "wrap" };

// ============================================================================
// Edges
// ============================================================================

/**
 * Four-sided spacing value, used for padding and margin.
 *
 * All values are in logical units. The default for every side is 0.
 *
 * Visual diagram (CSS box model):
 *
 *   ┌─────────────────────────────┐
 *   │         margin.top          │
 *   │   ┌─────────────────────┐   │
 *   │ m │     padding.top     │ m │
 *   │ a │  ┌───────────────┐  │ a │
 *   │ r │  │               │  │ r │
 *   │ g │  │    content    │  │ g │
 *   │ i │  │               │  │ i │
 *   │ n │  └───────────────┘  │ n │
 *   │ . │   padding.bottom    │ . │
 *   │ l └─────────────────────┘ r │
 *   │        margin.bottom        │
 *   └─────────────────────────────┘
 */
export interface Edges {
  top: number;
  right: number;
  bottom: number;
  left: number;
}

// ============================================================================
// Color
// ============================================================================

/**
 * An RGBA color with components in range 0–255.
 *
 * `a = 255` means fully opaque; `a = 0` means fully transparent.
 *
 * Examples:
 *
 *   rgb(0, 0, 0)         → black, fully opaque
 *   rgba(255, 0, 0, 128) → red, 50% transparent
 *   color_transparent()  → invisible (a=0)
 */
export interface Color {
  r: number; // 0–255
  g: number; // 0–255
  b: number; // 0–255
  a: number; // 0–255, 255 = fully opaque
}

// ============================================================================
// FontSpec
// ============================================================================

/**
 * A fully-resolved font descriptor. No cascade, no inheritance, no CSS shorthand.
 * Every `TextContent` node carries a complete `FontSpec` with all fields explicit.
 *
 * This makes the IR self-contained: a consumer of a `LayoutNode` tree never
 * needs to walk ancestor nodes to resolve a font — every leaf carries its own.
 *
 * Field notes:
 *
 * - `family`: CSS-style font family name. Empty string means "system default UI
 *   font", which the renderer resolves to the platform's default sans-serif.
 *
 * - `size`: in logical units, NOT CSS pixels or typographic points. The renderer
 *   converts to physical units using the device pixel ratio.
 *
 * - `weight`: 100–900. Common values: 100=thin, 400=regular, 700=bold, 900=black.
 *
 * - `lineHeight`: multiplier applied to `size`. A value of 1.0 means line height
 *   equals font size exactly (tight). A value of 1.5 means 50% extra spacing.
 *   Must be > 0.
 *
 * Truth table for `font_bold` and `font_italic`:
 *
 *   | Source spec       | font_bold(spec)     | font_italic(spec)     |
 *   |-------------------|---------------------|------------------------|
 *   | weight=400, i=F   | weight=700, i=F     | weight=400, i=T        |
 *   | weight=700, i=F   | weight=700, i=F     | weight=700, i=T        |
 *   | weight=400, i=T   | weight=700, i=T     | weight=400, i=T        |
 */
export interface FontSpec {
  family: string;
  size: number;
  weight: number; // 100–900
  italic: boolean;
  lineHeight: number; // multiplier, e.g. 1.2 = 120% of font size
}

// ============================================================================
// TextAlign
// ============================================================================

/**
 * Horizontal alignment of text within its containing box.
 *
 * `start` and `end` are logical (writing-direction-aware). For LTR text:
 *   start = left, end = right.
 * RTL support is a future extension — for now assume LTR.
 */
export type TextAlign = "start" | "center" | "end";

// ============================================================================
// ImageFit
// ============================================================================

/**
 * How an image fills its containing box. Mirrors CSS `object-fit`.
 *
 * Visual examples for a wide image in a square box:
 *
 *   contain → [  ════════════  ]   letterbox bars on top/bottom
 *   cover   → [ ══════════════ ]   image cropped left/right
 *   fill    → [ ══════════════ ]   image stretched to fit
 *   none    → [ ══════════════ ]   image at natural size, clipped
 */
export type ImageFit = "contain" | "cover" | "fill" | "none";

// ============================================================================
// Content types
// ============================================================================

/**
 * Text content carried by a leaf node.
 *
 * `maxLines = null` means unlimited — the text wraps at the containing width
 * and may use as many lines as needed.
 *
 * `maxLines = 1` means single-line: text is truncated (renderer decides the
 * overflow character, typically "…").
 *
 * The `font` and `color` fields are fully resolved — no inheritance needed.
 */
export interface TextContent {
  kind: "text";
  value: string;
  font: FontSpec;
  color: Color;
  maxLines: number | null;
  textAlign: TextAlign;
}

/**
 * Image content carried by a leaf node.
 *
 * `src` can be:
 *   - A URL: "https://example.com/img.png"
 *   - A data URI: "data:image/png;base64,..."
 *   - An opaque handle understood by a specific renderer
 *
 * The `kind: "image"` discriminant lets algorithms distinguish image leaves
 * from text leaves without checking field names.
 */
export interface ImageContent {
  kind: "image";
  src: string;
  fit: ImageFit;
}

/** Union of all content types. A leaf node carries exactly one. */
export type NodeContent = TextContent | ImageContent;

// ============================================================================
// LayoutNode
// ============================================================================

/**
 * The central type of the layout system.
 *
 * A `LayoutNode` represents either a leaf (text or image) or a container
 * (has children). The two are distinguished by whether `content` is non-null:
 *
 *   Leaf node:      content ≠ null, children = []
 *   Container node: content = null, children = [...]
 *
 * The `ext` bag is the key to extensibility. Each algorithm adds its own
 * schema to a named key:
 *
 *   ext["flex"]  → { direction: "row", gap: 8, grow: 1 }
 *   ext["block"] → { display: "inline" }
 *   ext["grid"]  → { templateColumns: "1fr 1fr", columnStart: 2 }
 *
 * A node can carry data for multiple algorithms at once, e.g. a node that is
 * a flex item in a parent container AND lays out its own children as a grid.
 *
 * Contract: the layout algorithm reads its own ext key and ignores everything
 * else. Unknown keys are silently ignored.
 */
export interface LayoutNode {
  /** Stable identifier for debugging and incremental diffing. Optional. */
  id?: string;

  /**
   * Leaf content. Non-null means this is a leaf node (text or image).
   * Null means this is a container node — look at `children` instead.
   */
  content: NodeContent | null;

  /** Child nodes. Empty for leaf nodes. */
  children: LayoutNode[];

  /** Width hint. Null = algorithm decides (typically treated as `wrap`). */
  width: SizeValue | null;

  /** Height hint. Null = algorithm decides (typically treated as `wrap`). */
  height: SizeValue | null;

  /** Minimum width constraint in logical units. Null = no minimum. */
  minWidth?: number | null;

  /** Maximum width constraint in logical units. Null = no maximum. */
  maxWidth?: number | null;

  /** Minimum height constraint in logical units. Null = no minimum. */
  minHeight?: number | null;

  /** Maximum height constraint in logical units. Null = no maximum. */
  maxHeight?: number | null;

  /** Space inside the node's border, between border and content/children. */
  padding?: Edges | null;

  /** Space outside the node's border, between border and adjacent siblings. */
  margin?: Edges | null;

  /**
   * Extension bag. Each layout algorithm reads its own key namespace.
   *
   * Type is `Record<string, unknown>` rather than `any` to preserve
   * the TypeScript strict-mode guarantees on the core node fields.
   */
  ext: Record<string, unknown>;
}

// ============================================================================
// Constraints
// ============================================================================

/**
 * The available space passed into a layout call.
 *
 * Think of `Constraints` as the "box" the layout must fit inside. The algorithm
 * tries to fill at most `maxWidth × maxHeight` and at least `minWidth × minHeight`.
 *
 * `maxWidth = Infinity` means unconstrained width — the layout can be as wide
 * as it needs. Same for `maxHeight = Infinity`.
 *
 * Analogy: Constraints is the negotiation between a parent (I give you this
 * much space) and a child (I need at least this much).
 *
 *   constraints_fixed(800, 600)  → exactly 800×600 px canvas
 *   constraints_width(800)       → 800 wide, unlimited height (e.g. scrollable page)
 *   constraints_unconstrained()  → measure natural size (no constraints)
 */
export interface Constraints {
  minWidth: number;
  maxWidth: number; // Infinity = unconstrained
  minHeight: number;
  maxHeight: number; // Infinity = unconstrained
}

// ============================================================================
// PositionedNode
// ============================================================================

/**
 * The output of a layout pass. Every node has a concrete position and size.
 *
 * Coordinate system: `x` and `y` are **relative to the parent's content area
 * origin** — the top-left corner of the parent after padding is applied. The
 * `layout-to-paint` package performs the recursive accumulation to absolute
 * coordinates when building the `PaintScene`.
 *
 *   ┌─ parent content area ────────────────────────────┐
 *   │ (0, 0)                                           │
 *   │   ┌─ child A (x=0, y=0, w=100, h=40) ────────┐  │
 *   │   └────────────────────────────────────────────┘  │
 *   │   ┌─ child B (x=0, y=48, w=100, h=40) ───────┐   │
 *   │   └────────────────────────────────────────────┘  │
 *   └──────────────────────────────────────────────────-┘
 *
 * The `ext` map from the source `LayoutNode` is carried through unchanged so
 * that downstream packages (e.g. `layout-to-paint`) can read paint decoration
 * from `ext["paint"]`.
 */
export interface PositionedNode {
  /** Left edge, relative to parent's content area origin. */
  x: number;

  /** Top edge, relative to parent's content area origin. */
  y: number;

  /** Resolved width in logical units. */
  width: number;

  /** Resolved height in logical units. */
  height: number;

  /** Stable identifier, carried from `LayoutNode.id`. */
  id?: string;

  /** Leaf content, carried from `LayoutNode.content`. */
  content: NodeContent | null;

  /** Positioned children. */
  children: PositionedNode[];

  /** Extension bag carried through unchanged from the source `LayoutNode`. */
  ext: Record<string, unknown>;
}

// ============================================================================
// TextMeasurer interface
// ============================================================================

/**
 * The result of measuring a text string.
 *
 * All measurements are in the same logical units as the rest of the IR.
 *
 *   width:     how wide the text block is (longest line for multi-line)
 *   height:    total height of all lines (lineCount × lineHeight × font.size)
 *   lineCount: number of lines after wrapping (1 for single-line text)
 */
export interface MeasureResult {
  width: number;
  height: number;
  lineCount: number;
}

/**
 * The text measurement interface.
 *
 * Layout algorithms call `measure()` to find out how large a piece of text will
 * be when rendered. The measurer is injected as a parameter — the algorithm
 * never imports a concrete measurer.
 *
 * Three standard implementations:
 *
 * - `layout-text-measure-estimated` — fast fixed-character-width approximation.
 *   All 9 languages. No dependencies. Used in tests, CI, and server-side layout
 *   where pixel-perfect accuracy is not required.
 *
 * - `layout-text-measure-canvas` — TypeScript only. Wraps
 *   `CanvasRenderingContext2D.measureText()` for accurate browser font metrics.
 *
 * - `layout-text-measure-rs` — Rust + fontdue. Accurate font-metric-based
 *   measurement with a C ABI FFI surface callable from all other languages.
 *
 * Why is the TextMeasurer an interface rather than a concrete type?
 *
 * Because the layout algorithm must work in every environment — Node.js, the
 * browser, Lua, Perl, a Rust PDF generator — and each environment has different
 * font access. The algorithm remains pure (no I/O, no native dependencies) by
 * delegating all font knowledge to the injected measurer.
 *
 * This follows the Dependency Inversion Principle: high-level layout policy
 * does not depend on low-level font rendering details.
 *
 * Usage example:
 *
 *   import { createEstimatedMeasurer } from "@coding-adventures/layout-text-measure-estimated";
 *   import { layoutFlexbox } from "@coding-adventures/layout-flexbox";
 *
 *   const measurer = createEstimatedMeasurer();
 *   const result = layoutFlexbox(tree, constraints, measurer);
 */
export interface TextMeasurer {
  measure(
    text: string,
    font: FontSpec,
    maxWidth: number | null
  ): MeasureResult;
}
