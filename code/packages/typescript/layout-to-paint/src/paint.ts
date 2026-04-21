/**
 * layout-to-paint: PositionedNode Tree → PaintScene
 *
 * Converts the output of a layout algorithm (a `PositionedNode` tree) into a
 * `PaintScene` that can be executed by any `paint-vm` backend (Canvas, SVG,
 * Metal, terminal).
 *
 * This is the bridge between the layout subsystem and the paint subsystem.
 * It has no knowledge of layout algorithms or the source that produced the
 * positioned tree — it only translates resolved geometry + content into paint
 * instructions.
 *
 * Pipeline position:
 *
 *   PositionedNode tree  ← output of layout_flexbox, layout_block, etc.
 *       ↓  layout_to_paint()
 *   PaintScene           → PaintVM.execute(scene, ctx) → pixels
 *
 * Coordinate transformation
 * -------------------------
 *
 * `PositionedNode` coordinates are in **logical units** — abstract, device-
 * independent measurements. `PaintInstruction` coordinates are in **physical
 * units** — actual pixels on the device.
 *
 * The transformation is simply:
 *
 *   physical = logical × devicePixelRatio
 *
 * This multiplication is applied exactly once in `layout_to_paint`. Downstream
 * paint-vm backends receive physical units and never need to know the DPR.
 *
 * Text rendering
 * --------------
 *
 * `TextContent` nodes are converted to `PaintGlyphRun` instructions. The glyph
 * run uses the Unicode code point of each character as its `glyph_id`. This
 * matches how the `paint-vm-canvas` backend renders glyphs — it calls
 * `String.fromCharCode(glyph_id)` to recover the character.
 *
 * Glyph x positions use the fixed 0.6 × font.size character-width estimate.
 * The y position is the baseline: absY + font.size × 0.8 (approximate ascender).
 *
 * For pixel-accurate glyph positions, the caller should pre-measure text with
 * a real TextMeasurer before calling layout_to_paint, and ensure the layout
 * dimensions match the actual rendered glyph widths.
 *
 * The `font_ref` field in `PaintGlyphRun` is set to the font family name (CSS
 * font string without size/weight). The paint-vm backend combines `font_ref`,
 * `font_size`, and the font weight/italic (carried in metadata) to reconstruct
 * the full font descriptor.
 *
 * PaintExt schema
 * ---------------
 *
 * `layout_to_paint` reads `ext["paint"]` (a `PaintExt` map) from each node.
 * Set this in the LayoutNode tree (e.g. in `mosaic-ir-to-layout`) to control
 * visual decoration:
 *
 *   ext["paint"] = {
 *     backgroundColor: Color          // fills the node's bounds with a rect
 *     borderWidth:     number         // stroke width in logical units
 *     borderColor:     Color          // stroke color
 *     cornerRadius:    number         // rounds the background rect corners
 *     opacity:         number         // 0.0–1.0; wraps node in PaintLayer
 *     shadowColor:     Color          // drop shadow
 *     shadowOffsetX:   number         // shadow x offset in logical units
 *     shadowOffsetY:   number         // shadow y offset in logical units
 *     shadowBlur:      number         // shadow blur radius in logical units
 *   }
 *
 * See: code/specs/UI04-layout-to-paint.md
 */

import type { PositionedNode, Color, TextContent, ImageContent } from "@coding-adventures/layout-ir";
import type {
  PaintScene,
  PaintInstruction,
  PaintGlyphRun,
  PaintText,
  PaintImage,
  PaintRect,
  PaintLayer,
  PaintClip,
} from "@coding-adventures/paint-instructions";

// ============================================================================
// Options
// ============================================================================

/**
 * Options for the `layout_to_paint` call.
 */
export interface LayoutToPaintOptions {
  /** Scene viewport width in logical units. */
  width: number;

  /** Scene viewport height in logical units. */
  height: number;

  /**
   * Scene background color. If null, the scene background is "transparent".
   *
   * This is the fill painted behind ALL instructions in the scene.
   * To paint individual node backgrounds, use `ext["paint"]["backgroundColor"]`.
   */
  background?: Color | null;

  /**
   * Physical pixels per logical unit. Default 1.0.
   *
   * Set to `window.devicePixelRatio` (e.g. 2.0 on Retina displays) to produce
   * a sharp scene. All logical coordinates are multiplied by this value before
   * emitting PaintInstructions.
   *
   * Example: a node at x=10, y=20 with dpr=2.0 → PaintInstruction x=20, y=40.
   */
  devicePixelRatio?: number;

  /**
   * How TextContent nodes are emitted. Default: `"glyph_run"`.
   *
   * - `"glyph_run"` — emit PaintGlyphRun, one glyph per character, using a
   *   `font.size × 0.6` advance approximation. Works with every paint
   *   backend, including those that consume glyph IDs (Metal, Direct2D).
   *   This is the historical behaviour.
   *
   * - `"text"` — emit a single PaintText instruction per TextContent,
   *   carrying the literal string and a `canvas:<family>@<size>...`
   *   font_ref. The paint backend (paint-vm-canvas) sets `ctx.font` and
   *   calls `ctx.fillText(str, x, y_baseline)` at dispatch time, letting
   *   the browser handle shaping + fallback. See spec TXT03d and the
   *   P2D00 "GlyphRun vs Text" section.
   *
   * Pipelines driven by a canvas-backed TextMeasurer (TXT03d) should use
   * `"text"`; pipelines driven by a font-parser or OS-native measurer
   * (TXT01/02/04, TXT03a/b/c) should use `"glyph_run"`.
   */
  textEmitMode?: "glyph_run" | "text";
}

// ============================================================================
// PaintExt — optional per-node visual decoration
// ============================================================================

/**
 * Visual decoration properties read from `ext["paint"]` on each PositionedNode.
 *
 * These properties do not affect layout — they are purely cosmetic. Front-end
 * converters (mosaic-ir-to-layout, document-ast-to-layout) populate these when
 * a source element needs a background, border, or shadow.
 */
export interface PaintExt {
  backgroundColor?: Color | null;
  borderWidth?: number | null;
  borderColor?: Color | null;
  cornerRadius?: number | null;
  opacity?: number | null;
  shadowColor?: Color | null;
  shadowOffsetX?: number | null;
  shadowOffsetY?: number | null;
  shadowBlur?: number | null;
}

// ============================================================================
// Color helpers
// ============================================================================

/**
 * Convert a `Color` value to a CSS `rgba()` string.
 *
 * Used when writing fill and stroke fields in PaintInstruction values, which
 * expect CSS color strings.
 *
 * Examples:
 *   colorToCss({ r:255, g:0, b:0, a:255 }) → "rgba(255,0,0,1)"
 *   colorToCss({ r:0, g:0, b:0, a:128 })   → "rgba(0,0,0,0.502)"
 */
export function colorToCss(c: Color): string {
  const alpha = (c.a / 255).toFixed(3).replace(/\.?0+$/, "");
  return `rgba(${c.r},${c.g},${c.b},${alpha})`;
}

// ============================================================================
// layout_to_paint
// ============================================================================

/**
 * Convert a `PositionedNode` tree to a `PaintScene`.
 *
 * The returned scene can be passed directly to any `paint-vm` backend.
 *
 * Usage:
 *
 *   const scene = layout_to_paint(
 *     [positionedRoot],
 *     { width: 800, height: 600, devicePixelRatio: 2.0 }
 *   );
 *   paintVMCanvas.execute(scene, ctx);
 */
export function layout_to_paint(
  nodes: PositionedNode[],
  options: LayoutToPaintOptions
): PaintScene {
  const dpr = options.devicePixelRatio ?? 1.0;
  const bg = options.background ? colorToCss(options.background) : "transparent";
  const textMode = options.textEmitMode ?? "glyph_run";

  const instructions: PaintInstruction[] = [];

  for (const node of nodes) {
    emitNode(node, 0, 0, dpr, textMode, instructions);
  }

  return {
    width: options.width * dpr,
    height: options.height * dpr,
    background: bg,
    instructions,
  };
}

// ============================================================================
// Node emission
// ============================================================================

function emitNode(
  node: PositionedNode,
  parentAbsX: number,
  parentAbsY: number,
  dpr: number,
  textMode: "glyph_run" | "text",
  out: PaintInstruction[]
): void {
  const absX = parentAbsX + node.x;
  const absY = parentAbsY + node.y;

  const paintExt = (node.ext["paint"] ?? {}) as PaintExt;
  const opacity = paintExt.opacity ?? 1.0;
  const hasOpacity = opacity < 1.0;

  // If opacity < 1, wrap everything in a PaintLayer for correct compositing.
  if (hasOpacity) {
    const layerChildren: PaintInstruction[] = [];
    emitNodeInstructions(node, absX, absY, dpr, textMode, layerChildren);
    const layer: PaintLayer = {
      kind: "layer",
      opacity,
      children: layerChildren,
    };
    out.push(layer);
    return;
  }

  emitNodeInstructions(node, absX, absY, dpr, textMode, out);
}

function emitNodeInstructions(
  node: PositionedNode,
  absX: number,
  absY: number,
  dpr: number,
  textMode: "glyph_run" | "text",
  out: PaintInstruction[]
): void {
  const paintExt = (node.ext["paint"] ?? {}) as PaintExt;

  const physX = absX * dpr;
  const physY = absY * dpr;
  const physW = node.width * dpr;
  const physH = node.height * dpr;

  // ── Step 1: Background fill ───────────────────────────────────────────────

  if (paintExt.backgroundColor) {
    const rect: PaintRect = {
      kind: "rect",
      x: physX,
      y: physY,
      width: physW,
      height: physH,
      fill: colorToCss(paintExt.backgroundColor),
    };
    if (paintExt.cornerRadius) {
      rect.corner_radius = paintExt.cornerRadius * dpr;
    }
    out.push(rect);
  }

  // ── Step 2: Border ────────────────────────────────────────────────────────

  if (paintExt.borderWidth && paintExt.borderWidth > 0 && paintExt.borderColor) {
    const border: PaintRect = {
      kind: "rect",
      x: physX,
      y: physY,
      width: physW,
      height: physH,
      stroke: colorToCss(paintExt.borderColor),
      stroke_width: paintExt.borderWidth * dpr,
    };
    if (paintExt.cornerRadius) {
      border.corner_radius = paintExt.cornerRadius * dpr;
    }
    out.push(border);
  }

  // ── Step 3 + 4 + 5: Content and children (optionally clipped) ─────────────

  if (paintExt.cornerRadius && (node.content !== null || node.children.length > 0)) {
    // Clip content and children to rounded rect bounds.
    const clipped: PaintInstruction[] = [];
    emitContent(node, absX, absY, dpr, textMode, clipped);
    emitChildren(node, absX, absY, dpr, textMode, clipped);

    const clip: PaintClip = {
      kind: "clip",
      x: physX,
      y: physY,
      width: physW,
      height: physH,
      children: clipped,
    };
    out.push(clip);
  } else {
    emitContent(node, absX, absY, dpr, textMode, out);
    emitChildren(node, absX, absY, dpr, textMode, out);
  }
}

// ============================================================================
// Content emission
// ============================================================================

function emitContent(
  node: PositionedNode,
  absX: number,
  absY: number,
  dpr: number,
  textMode: "glyph_run" | "text",
  out: PaintInstruction[]
): void {
  if (node.content === null) return;

  if (node.content.kind === "text") {
    if (textMode === "text") {
      emitTextAsPaintText(node.content, absX, absY, node.width, dpr, out);
    } else {
      emitText(node.content, absX, absY, node.width, dpr, out);
    }
  } else if (node.content.kind === "image") {
    emitImage(node.content, absX, absY, node.width, node.height, dpr, out);
  }
}

/**
 * Convert `TextContent` to a `PaintGlyphRun`.
 *
 * Glyph IDs: each character's Unicode code point (U+0000–U+FFFF).
 * This matches the `paint-vm-canvas` convention: it renders each glyph via
 * `ctx.fillText(String.fromCharCode(glyph_id), glyph.x, glyph.y)`.
 *
 * X positions: estimated using `font.size × 0.6` per character — the same
 * approximation as `layout-text-measure-estimated`. This produces correct
 * spacing for the majority of typical Latin text at regular weight.
 *
 * Y (baseline): `absY + font.size × 0.8` in logical units, then × dpr.
 * This is an approximation of the baseline from the top of the font.
 *
 * Note: for pixel-accurate glyph positioning, use a font-metric measurer
 * (layout-text-measure-canvas or layout-text-measure-rs) during layout, and
 * pass the resulting laid-out positions into the PositionedNode tree.
 */
function emitText(
  content: TextContent,
  absX: number,
  absY: number,
  nodeWidth: number,
  dpr: number,
  out: PaintInstruction[]
): void {
  if (content.value.length === 0) return;

  const font = content.font;
  const charWidth = font.size * 0.6;
  const baselineY = absY + font.size * 0.8; // approximate baseline from top

  const glyphs = Array.from(content.value).map((char, i) => ({
    glyph_id: char.codePointAt(0) ?? 0,
    x: (absX + i * charWidth) * dpr,
    y: baselineY * dpr,
  }));

  const italicPrefix = font.italic ? "italic " : "";
  const fontRef = `${italicPrefix}${font.weight} ${font.family || "sans-serif"}`;

  const glyphRun: PaintGlyphRun = {
    kind: "glyph_run",
    glyphs,
    font_ref: fontRef,
    font_size: font.size * dpr,
    fill: colorToCss(content.color),
    metadata: {
      "layout:text": content.value,
      "layout:maxWidth": nodeWidth * dpr,
      "layout:textAlign": content.textAlign,
    },
  };

  out.push(glyphRun);
}

/**
 * Convert `TextContent` to a single `PaintText` instruction (canvas-native path).
 *
 * This emits one PaintText per TextContent node, carrying the literal string
 * and a `canvas:` font_ref that encodes family, size, weight, and style per
 * spec TXT03d. Line wrapping is expected to have happened in the layout phase
 * (via CanvasTextMeasurer in UI09). At paint time, the browser re-shapes and
 * rasterizes via `ctx.fillText`.
 *
 * This does NOT split per-glyph — Canvas has no addressable glyph IDs. Font
 * fallback (e.g. Apple Color Emoji for a single emoji inside a Latin run) is
 * delegated to the browser and is invisible at the paint IR level.
 *
 * Trade-off: we lose per-character hit-testing information. A future
 * enhancement can populate `cluster_positions` by walking each character and
 * calling `ctx.measureText` to build a cluster→x offset map.
 */
function emitTextAsPaintText(
  content: TextContent,
  absX: number,
  absY: number,
  nodeWidth: number,
  dpr: number,
  out: PaintInstruction[]
): void {
  if (content.value.length === 0) return;

  const font = content.font;
  // Baseline offset from the top of the node box. The measurer reports height
  // using fontBoundingBoxAscent + fontBoundingBoxDescent, which for typical
  // Latin fonts matches font.size × 1.15 — with ascent ≈ font.size × 0.93
  // and descent ≈ font.size × 0.22. Using 0.93 here places the baseline
  // inside the reported box consistently across same-font tokens on a line.
  const baselineY = absY + font.size * 0.93;

  const family = font.family || "sans-serif";
  // Build "canvas:<family>@<size>:<weight>[:italic]" per TXT03d grammar.
  // The family string is encoded as-is; paint-vm-canvas sanitizes it before
  // interpolating into ctx.font.
  let fontRef = `canvas:${family}@${font.size}:${font.weight}`;
  if (font.italic) fontRef += ":italic";

  const paintText: PaintText = {
    kind: "text",
    x: absX * dpr,
    y: baselineY * dpr,
    text: content.value,
    font_ref: fontRef,
    font_size: font.size * dpr,
    fill: colorToCss(content.color),
    metadata: {
      "layout:maxWidth": nodeWidth * dpr,
      "layout:textAlign": content.textAlign,
    },
  };

  out.push(paintText);
}

/**
 * Convert `ImageContent` to a `PaintImage`.
 */
function emitImage(
  content: ImageContent,
  absX: number,
  absY: number,
  width: number,
  height: number,
  dpr: number,
  out: PaintInstruction[]
): void {
  const image: PaintImage = {
    kind: "image",
    x: absX * dpr,
    y: absY * dpr,
    width: width * dpr,
    height: height * dpr,
    src: content.src,
    metadata: {
      "layout:fit": content.fit,
    },
  };
  out.push(image);
}

function emitChildren(
  node: PositionedNode,
  absX: number,
  absY: number,
  dpr: number,
  textMode: "glyph_run" | "text",
  out: PaintInstruction[]
): void {
  for (const child of node.children) {
    emitNode(child, absX, absY, dpr, textMode, out);
  }
}
