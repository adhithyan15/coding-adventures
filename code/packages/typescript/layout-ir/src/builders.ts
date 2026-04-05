/**
 * Builder helpers for Layout IR types.
 *
 * These are thin constructors with sensible defaults. They reduce boilerplate
 * when building `LayoutNode` trees and make the call sites more readable by
 * naming the intent (e.g. `size_fill()` vs `{ kind: "fill" }`).
 *
 * All helpers are pure functions — no side effects, no global state.
 */

import type {
  SizeValue,
  Edges,
  Color,
  FontSpec,
  LayoutNode,
  NodeContent,
  TextContent,
  ImageContent,
  Constraints,
  PositionedNode,
} from "./types.js";

// ============================================================================
// SizeValue builders
// ============================================================================

/**
 * Create a fixed-size value: exactly `value` logical units.
 *
 * Use when the node must be a specific size regardless of available space.
 * Like CSS `width: 200px`.
 */
export function size_fixed(value: number): SizeValue {
  return { kind: "fixed", value };
}

/**
 * Create a fill size value: fill all available space.
 *
 * Use for items that should grow to fill their parent container.
 * Like CSS `flex: 1` or `width: 100%`.
 */
export function size_fill(): SizeValue {
  return { kind: "fill" };
}

/**
 * Create a wrap size value: shrink to content size.
 *
 * Use when the node should be exactly as large as its content requires,
 * up to the available space. Like CSS `width: fit-content`.
 */
export function size_wrap(): SizeValue {
  return { kind: "wrap" };
}

// ============================================================================
// Edges builders
// ============================================================================

/**
 * Create uniform padding/margin on all four sides.
 *
 *   edges_all(8) → { top: 8, right: 8, bottom: 8, left: 8 }
 */
export function edges_all(v: number): Edges {
  return { top: v, right: v, bottom: v, left: v };
}

/**
 * Create symmetric padding/margin: `x` on left/right, `y` on top/bottom.
 *
 *   edges_xy(16, 8) → { top: 8, right: 16, bottom: 8, left: 16 }
 *
 * Analogy: CSS `padding: 8px 16px` (top-bottom, left-right).
 */
export function edges_xy(x: number, y: number): Edges {
  return { top: y, right: x, bottom: y, left: x };
}

/**
 * Create zero spacing on all sides. The default state.
 */
export function edges_zero(): Edges {
  return { top: 0, right: 0, bottom: 0, left: 0 };
}

// ============================================================================
// Color builders
// ============================================================================

/**
 * Create an RGBA color with explicit alpha.
 *
 * All components are integers 0–255. `a = 255` = fully opaque.
 *
 *   rgba(255, 0, 0, 255)  → red, fully opaque
 *   rgba(0, 0, 255, 128)  → blue, 50% transparent
 */
export function rgba(r: number, g: number, b: number, a: number): Color {
  return { r, g, b, a };
}

/**
 * Create an RGB color with full opacity (alpha = 255).
 *
 *   rgb(0, 0, 0) → black
 *   rgb(255, 255, 255) → white
 */
export function rgb(r: number, g: number, b: number): Color {
  return { r, g, b, a: 255 };
}

/**
 * Create a fully transparent color (r=g=b=a=0).
 * Used as a sentinel "no color" value.
 */
export function color_transparent(): Color {
  return { r: 0, g: 0, b: 0, a: 0 };
}

// ============================================================================
// FontSpec builders
// ============================================================================

/**
 * Create a basic FontSpec with regular weight, no italic.
 *
 * Defaults: weight=400 (regular), italic=false, lineHeight=1.2.
 *
 * The lineHeight multiplier of 1.2 is a widely-used baseline for body text —
 * it adds 20% extra space between lines, enough for readability without
 * excessive whitespace.
 *
 *   font_spec("Arial", 16) → { family: "Arial", size: 16, weight: 400,
 *                               italic: false, lineHeight: 1.2 }
 */
export function font_spec(family: string, size: number): FontSpec {
  return { family, size, weight: 400, italic: false, lineHeight: 1.2 };
}

/**
 * Return a copy of `spec` with weight set to 700 (bold).
 *
 * Does not modify the original spec (immutable update).
 *
 *   font_bold(font_spec("Arial", 16)) → { ..., weight: 700, italic: false }
 */
export function font_bold(spec: FontSpec): FontSpec {
  return { ...spec, weight: 700 };
}

/**
 * Return a copy of `spec` with italic=true.
 *
 * Does not modify the original spec (immutable update).
 *
 *   font_italic(font_spec("Arial", 16)) → { ..., weight: 400, italic: true }
 */
export function font_italic(spec: FontSpec): FontSpec {
  return { ...spec, italic: true };
}

// ============================================================================
// Constraints builders
// ============================================================================

/**
 * Create fixed constraints: exactly `w` wide and `h` tall.
 *
 * Use when rendering into a known-size viewport (e.g. a 800×600 canvas).
 */
export function constraints_fixed(w: number, h: number): Constraints {
  return { minWidth: 0, maxWidth: w, minHeight: 0, maxHeight: h };
}

/**
 * Create width-constrained, unconstrained-height constraints.
 *
 * Use for scrollable content: the width is fixed (e.g. the page column width)
 * but the height can grow as much as needed.
 *
 *   constraints_width(800) → maxWidth=800, maxHeight=Infinity
 */
export function constraints_width(w: number): Constraints {
  return { minWidth: 0, maxWidth: w, minHeight: 0, maxHeight: Infinity };
}

/**
 * Create fully unconstrained constraints.
 *
 * Use to measure the natural (intrinsic) size of a node — how large it wants
 * to be with no restrictions.
 */
export function constraints_unconstrained(): Constraints {
  return {
    minWidth: 0,
    maxWidth: Infinity,
    minHeight: 0,
    maxHeight: Infinity,
  };
}

/**
 * Shrink existing constraints by `dw` horizontally and `dh` vertically.
 *
 * Used to subtract padding from a container's constraints before passing
 * them to children:
 *
 *   const inner = constraints_shrink(outer, padding.left + padding.right,
 *                                            padding.top + padding.bottom);
 *
 * Clamps to zero: if `dw > maxWidth`, the result has `maxWidth = 0`.
 */
export function constraints_shrink(
  c: Constraints,
  dw: number,
  dh: number
): Constraints {
  return {
    minWidth: Math.max(0, c.minWidth - dw),
    maxWidth: Math.max(0, c.maxWidth - dw),
    minHeight: Math.max(0, c.minHeight - dh),
    maxHeight: Math.max(0, c.maxHeight - dh),
  };
}

// ============================================================================
// LayoutNode builders
// ============================================================================

/** Options accepted by all node builders. */
export interface NodeOpts {
  id?: string;
  width?: SizeValue | null;
  height?: SizeValue | null;
  minWidth?: number | null;
  maxWidth?: number | null;
  minHeight?: number | null;
  maxHeight?: number | null;
  padding?: Edges | null;
  margin?: Edges | null;
  ext?: Record<string, unknown>;
}

/**
 * Create a bare `LayoutNode` with explicit children and optional opts.
 *
 * The most general constructor — use the more specific helpers below for
 * common cases.
 */
export function node(
  opts: NodeOpts & {
    content?: NodeContent | null;
    children?: LayoutNode[];
  }
): LayoutNode {
  return {
    id: opts.id,
    content: opts.content ?? null,
    children: opts.children ?? [],
    width: opts.width ?? null,
    height: opts.height ?? null,
    minWidth: opts.minWidth ?? null,
    maxWidth: opts.maxWidth ?? null,
    minHeight: opts.minHeight ?? null,
    maxHeight: opts.maxHeight ?? null,
    padding: opts.padding ?? null,
    margin: opts.margin ?? null,
    ext: opts.ext ?? {},
  };
}

/**
 * Create a text leaf node.
 *
 * The resulting node has `content` set to the provided `TextContent` and
 * an empty `children` array. Width and height default to `wrap` (sized to
 * the measured text).
 *
 *   leaf_text({ kind: "text", value: "Hello", font: f, color: c,
 *               maxLines: null, textAlign: "start" })
 */
export function leaf_text(content: TextContent, opts?: NodeOpts): LayoutNode {
  return node({
    ...opts,
    content,
    children: [],
    width: opts?.width ?? size_wrap(),
    height: opts?.height ?? size_wrap(),
  });
}

/**
 * Create an image leaf node.
 *
 * Width and height default to `wrap` (sized by constraints or natural image
 * dimensions, whichever is smaller). Override with `size_fixed(v)` for
 * fixed-size images.
 *
 *   leaf_image({ kind: "image", src: "hero.png", fit: "cover" },
 *              { width: size_fixed(200), height: size_fixed(200) })
 */
export function leaf_image(
  content: ImageContent,
  opts?: NodeOpts
): LayoutNode {
  return node({
    ...opts,
    content,
    children: [],
    width: opts?.width ?? size_wrap(),
    height: opts?.height ?? size_wrap(),
  });
}

/**
 * Create a container node with children.
 *
 * A container node has `content = null` and a non-empty `children` list.
 * The layout algorithm for the container is determined by the `ext` field.
 *
 *   container(
 *     [leafText1, leafText2],
 *     { ext: { flex: { direction: "row", gap: 8 } } }
 *   )
 */
export function container(children: LayoutNode[], opts?: NodeOpts): LayoutNode {
  return node({
    ...opts,
    content: null,
    children,
    width: opts?.width ?? null,
    height: opts?.height ?? null,
  });
}

// ============================================================================
// PositionedNode builder
// ============================================================================

/**
 * Create a `PositionedNode` with resolved geometry.
 *
 * Primarily used by layout algorithm implementations to construct their output.
 */
export function positioned(
  x: number,
  y: number,
  width: number,
  height: number,
  opts: {
    id?: string;
    content?: NodeContent | null;
    children?: PositionedNode[];
    ext?: Record<string, unknown>;
  }
): PositionedNode {
  return {
    x,
    y,
    width,
    height,
    id: opts.id,
    content: opts.content ?? null,
    children: opts.children ?? [],
    ext: opts.ext ?? {},
  };
}
