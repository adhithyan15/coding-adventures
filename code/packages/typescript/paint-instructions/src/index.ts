/**
 * @coding-adventures/paint-instructions
 *
 * Universal 2D paint intermediate representation (IR).
 *
 * This package defines the complete type system for the PaintInstructions IR
 * (P2D00). It is the shared vocabulary between producers (chart builders,
 * diagram renderers, game engines) and backends (Canvas, SVG, Metal, terminal).
 *
 * ## Architecture
 *
 * The IR sits between producers and backends:
 *
 *   Producer (chart, barcode, mermaid diagram)
 *     → PaintScene / PaintInstruction[]        ← this package
 *     → PaintVM (P2D01, paint-vm package)
 *     → Backend (SVG, Canvas, Metal, terminal)
 *
 * This package is pure types. It has zero runtime dependencies. The only
 * runtime export is VERSION and a set of helper builder functions that make
 * constructing PaintInstruction objects ergonomic.
 *
 * ## Composable pipeline
 *
 * Every type in this package is designed to snap into a pipeline:
 *
 *   PaintScene  ──▶  PaintVM.execute()   ──▶  rendered output
 *   PaintScene  ──▶  PaintVM.export()    ──▶  PixelContainer
 *   PixelContainer  ──▶  ImageCodec.encode()  ──▶  .png / .webp / .jpg bytes
 *   bytes  ──▶  ImageCodec.decode()  ──▶  PixelContainer
 *   PixelContainer  ──▶  PaintImage.src  ──▶  embedded in another scene
 *
 * Nothing in this package depends on a specific backend. A codec never imports
 * from a VM. A VM never imports from a codec. This package is the shared contract.
 */
export const VERSION = "0.1.0";

import type { PixelContainer } from "@coding-adventures/pixel-container";

// ============================================================================
// PaintBase — shared fields on every instruction
// ============================================================================

/**
 * Every instruction extends PaintBase.
 *
 * The two fields here are optional on every instruction type.
 *
 * id — a stable, opaque identity string used by PaintVM.patch() to track
 * instructions across scene versions. Recommended format: UUID v4.
 * Short stable strings work too ("chart-title", "x-axis-label").
 * Instructions without an id fall back to positional diffing in patch().
 *
 * metadata — arbitrary key/value pairs for producers and debuggers.
 * The PaintVM ignores metadata — it is carried through unchanged.
 * Backends may expose it for dev-tools or accessibility annotations.
 * Example: { "source": "mermaid-node-42", "layer": "background" }
 */
export interface PaintBase {
  id?: string;
  metadata?: Record<string, string | number | boolean>;
}

// ============================================================================
// PathCommand — drawing commands inside a PaintPath
// ============================================================================

/**
 * A single drawing command inside a PaintPath.
 *
 * Think of it as one step for a pen plotter:
 *   move_to  → lift pen and move without drawing
 *   line_to  → draw a straight line
 *   quad_to  → draw a quadratic Bézier curve (one control point)
 *   cubic_to → draw a cubic Bézier curve (two control points)
 *   arc_to   → draw an elliptical arc (SVG arc command semantics)
 *   close    → straight line back to the last move_to, closes the subpath
 *
 * Example — house outline (bottom-left D → A → apex C → B → bottom-right E):
 *   [
 *     { kind: "move_to", x: 60,  y: 120 },
 *     { kind: "line_to", x: 60,  y: 60  },
 *     { kind: "line_to", x: 100, y: 20  },
 *     { kind: "line_to", x: 140, y: 60  },
 *     { kind: "line_to", x: 140, y: 120 },
 *     { kind: "close"                    },
 *   ]
 */
export type PathCommand =
  | { kind: "move_to"; x: number; y: number }
  | { kind: "line_to"; x: number; y: number }
  | { kind: "quad_to"; cx: number; cy: number; x: number; y: number }
  | {
      kind: "cubic_to";
      cx1: number;
      cy1: number;
      cx2: number;
      cy2: number;
      x: number;
      y: number;
    }
  | {
      kind: "arc_to";
      rx: number;
      ry: number;
      x_rotation: number; // degrees
      large_arc: boolean;
      sweep: boolean; // true = clockwise
      x: number;
      y: number;
    }
  | { kind: "close" };

// ============================================================================
// Transform2D — affine transform matrix
// ============================================================================

/**
 * A 2D affine transform as a 6-element array [a, b, c, d, e, f].
 *
 * The transformation matrix is:
 *   | a  c  e |
 *   | b  d  f |
 *   | 0  0  1 |
 *
 * Maps a point (x, y) to:
 *   x' = a*x + c*y + e
 *   y' = b*x + d*y + f
 *
 * Common transforms:
 *   Identity:    [1, 0, 0, 1, 0, 0]
 *   Translate:   [1, 0, 0, 1, tx, ty]
 *   Scale:       [sx, 0, 0, sy, 0, 0]
 *   Rotate θ:   [cos θ, sin θ, -sin θ, cos θ, 0, 0]
 *
 * This matches the CanvasRenderingContext2D.transform(a,b,c,d,e,f) argument order.
 */
export type Transform2D = [
  number, // a
  number, // b
  number, // c
  number, // d
  number, // e (x translation)
  number, // f (y translation)
];

// ============================================================================
// FilterEffect — image filter effects for PaintLayer
// ============================================================================

/**
 * A filter effect to apply to a PaintLayer's offscreen buffer.
 *
 * Filters are applied in array order — each filter receives the output of the
 * previous one. All filters operate on the composited layer as a whole, after
 * all children have been rendered to the offscreen buffer.
 *
 * This is why filters belong on PaintLayer (not PaintGroup): a group renders
 * directly to the parent surface — there is no separate buffer to filter.
 * A layer allocates a separate offscreen buffer, renders its children into it,
 * applies filters to the whole buffer, then composites the result.
 */
export type FilterEffect =
  | { kind: "blur"; radius: number }
  // Gaussian blur. radius in user-space units.
  // Maps to: CSS filter: blur(Npx), SVG feGaussianBlur, Metal compute shader.

  | { kind: "drop_shadow"; dx: number; dy: number; blur: number; color: string }
  // Drop shadow offset by (dx, dy), blurred by `blur` radius, filled with `color`.
  // Maps to: CSS filter: drop-shadow(...), SVG feDropShadow.

  | { kind: "color_matrix"; matrix: number[] }
  // 4×5 color matrix (20 values, row-major).
  // Maps [R, G, B, A, 1] to [R', G', B', A'].
  // Same layout as SVG feColorMatrix type="matrix".

  | { kind: "brightness"; amount: number }
  // Multiply luminance. 1.0 = unchanged, 0.0 = black, 2.0 = double brightness.

  | { kind: "contrast"; amount: number }
  // Adjust contrast. 1.0 = unchanged, 0.0 = flat grey.

  | { kind: "saturate"; amount: number }
  // Adjust saturation. 0.0 = greyscale, 1.0 = unchanged, 2.0 = vivid.

  | { kind: "hue_rotate"; angle: number }
  // Rotate hue by `angle` degrees. 180 = complement.

  | { kind: "invert"; amount: number }
  // Invert colors. 0.0 = no change, 1.0 = fully inverted.

  | { kind: "opacity"; amount: number };
// Premultiplied opacity filter. 0.0 = transparent, 1.0 = opaque.
// Distinct from PaintLayer.opacity: this is a pipeline step (applied before
// the final opacity multiplier and before compositing).

// ============================================================================
// BlendMode — compositing mode for PaintLayer
// ============================================================================

/**
 * How a PaintLayer's offscreen buffer is composited back into the parent surface.
 *
 * Separable modes (operate per colour channel independently):
 *   normal      — Standard alpha compositing. The default.
 *   multiply    — Multiply source and destination. Darkens.
 *   screen      — Invert, multiply, invert. Lightens.
 *   overlay     — Multiply for darks, Screen for lights.
 *   darken      — min(src, dst) per channel.
 *   lighten     — max(src, dst) per channel.
 *   color_dodge — Divide dst by (1 − src). Brightens.
 *   color_burn  — Invert dst, divide by src, invert. Darkens.
 *   hard_light  — Overlay with src/dst swapped.
 *   soft_light  — Softer version of hard_light.
 *   difference  — |src − dst|. High contrast at edges.
 *   exclusion   — Like difference but lower contrast.
 *
 * Non-separable modes (operate on combined HSL representation):
 *   hue        — src hue + dst saturation + dst luminosity
 *   saturation — dst hue + src saturation + dst luminosity
 *   color      — src hue + src saturation + dst luminosity
 *   luminosity — dst hue + dst saturation + src luminosity
 */
export type BlendMode =
  | "normal"
  | "multiply"
  | "screen"
  | "overlay"
  | "darken"
  | "lighten"
  | "color_dodge"
  | "color_burn"
  | "hard_light"
  | "soft_light"
  | "difference"
  | "exclusion"
  | "hue"
  | "saturation"
  | "color"
  | "luminosity";

// ============================================================================
// PixelContainer and ImageCodec — re-exported from pixel-container (IC00)
// ============================================================================
//
// These types are defined in the standalone `@coding-adventures/pixel-container`
// package so that image codecs can depend only on that package without pulling
// in the full paint IR. Re-exporting them here preserves the existing import
// path so all downstream consumers compile unchanged.
//
// The IC00 model is fixed RGBA8: { width, height, data: Uint8Array }.
// The older fields (channels, bit_depth, color_space) have been removed —
// they added complexity with no benefit at the codec layer where callers
// always deal with RGBA8.
export type { PixelContainer, ImageCodec } from "@coding-adventures/pixel-container";

// ============================================================================
// Instruction types
// ============================================================================

/**
 * PaintRect — filled and/or stroked rectangle.
 *
 * x, y are the top-left corner in the current coordinate system.
 * fill and stroke use CSS color syntax (named, hex, rgba).
 * A rect with no fill and no stroke renders nothing visible.
 * corner_radius applies uniformly to all four corners.
 *
 * Example — a blue card with a white rounded border:
 *   { kind: "rect", id: "card-bg", x: 10, y: 10, width: 200, height: 120,
 *     fill: "#2563eb", stroke: "#ffffff", stroke_width: 2, corner_radius: 8 }
 */
export interface PaintRect extends PaintBase {
  kind: "rect";
  x: number;
  y: number;
  width: number; // must be >= 0
  height: number; // must be >= 0
  fill?: string; // CSS color; omit for no fill
  stroke?: string; // CSS color; omit for no stroke
  stroke_width?: number; // user-space units; default 1.0
  corner_radius?: number; // 0 or omit = sharp corners
}

/**
 * PaintEllipse — filled and/or stroked ellipse or circle.
 *
 * cx, cy is the geometric center (not the bounding-box origin).
 * rx, ry are the x-radius and y-radius. A circle has rx === ry.
 *
 * ASCII diagram:
 *          (cx, cy-ry)         ← top
 *               |
 *   (cx-rx, cy)─┼─(cx+rx, cy) ← left and right extremes
 *               |
 *          (cx, cy+ry)         ← bottom
 */
export interface PaintEllipse extends PaintBase {
  kind: "ellipse";
  cx: number; // center x
  cy: number; // center y
  rx: number; // x radius (half-width)
  ry: number; // y radius (half-height)
  fill?: string;
  stroke?: string;
  stroke_width?: number;
}

/**
 * PaintPath — arbitrary vector path built from PathCommands.
 *
 * This is the most expressive instruction. Any shape expressible as an SVG
 * <path d="..."> can be expressed here. The commands trace the path step by step,
 * like instructions to a pen plotter.
 *
 * fill_rule controls how overlapping subpaths are filled:
 *   "nonzero" (default) — inside if winding number is nonzero (most shapes)
 *   "evenodd"           — inside if crossing count is odd (donuts, stars, letters)
 *
 * stroke_cap controls how line endpoints look:
 *   "butt" (default) — flat cap exactly at endpoint
 *   "round"          — semicircular cap beyond endpoint
 *   "square"         — square cap beyond endpoint
 *
 * stroke_join controls how corners between segments look:
 *   "miter" (default) — sharp pointed join
 *   "round"           — rounded join
 *   "bevel"           — flat diagonal join
 */
export interface PaintPath extends PaintBase {
  kind: "path";
  commands: PathCommand[];
  fill?: string;
  fill_rule?: "nonzero" | "evenodd";
  stroke?: string;
  stroke_width?: number;
  stroke_cap?: "butt" | "round" | "square";
  stroke_join?: "miter" | "round" | "bevel";
}

/**
 * PaintGlyphRun — pre-positioned glyphs from a font.
 *
 * A GlyphRun represents text that has already been shaped and positioned.
 * The producer has resolved the font, computed glyph IDs, applied kerning,
 * and calculated (x, y) for each glyph. The VM just paints them.
 *
 * This is different from "draw text at position (x,y) in font size 16" —
 * that higher-level operation belongs in the layout layer, which calls the
 * font-parser (FNT00) to resolve it into glyph IDs and positions, then
 * produces a PaintGlyphRun.
 *
 * Why not a "text" instruction?
 * Because text rendering requires font loading, shaping, line breaking, and
 * bidirectional text support — none of which the PaintVM should know about.
 * The GlyphRun is what you get AFTER all that work is done.
 *
 * font_ref is an opaque string the backend resolves (CSS font name, file path,
 * or pre-loaded font handle — depends on the backend).
 */
export interface PaintGlyphRun extends PaintBase {
  kind: "glyph_run";
  glyphs: Array<{
    glyph_id: number; // numeric glyph ID from the font's cmap
    x: number; // x origin for this glyph in scene coordinates
    y: number; // y origin (baseline position)
  }>;
  font_ref: string; // opaque font reference for the backend
  font_size: number; // in user-space units
  fill?: string; // glyph fill color; default black
}

/**
 * PaintText — a string of text with a font descriptor, positioned at a baseline.
 *
 * The sibling of PaintGlyphRun for runtimes that do their own shaping at paint
 * time and do NOT expose glyph IDs to the caller. The primary consumer is HTML
 * Canvas 2D (`ctx.fillText(text, x, y)`), which accepts only strings — see
 * spec TXT03d and the P2D00 "GlyphRun vs Text" section.
 *
 * Why two instructions, not one?
 * ------------------------------
 *
 * The font-binding invariant from TXT00 says: glyph IDs are opaque tokens bound
 * to the shaper that produced them. A Canvas backend cannot consume synthesized
 * glyph IDs — there is no `ctx.drawGlyphs(ids[])` API. Forcing a PaintGlyphRun
 * through canvas means mapping glyph_id back to a codepoint and guessing, which
 * breaks the invariant.
 *
 * PaintText is the honest representation for canvas: the layout engine has
 * measured the line (via `ctx.measureText`) to decide line wraps and positions,
 * but final shaping + fallback + rasterization happen inside `fillText` at
 * dispatch time. The browser's text stack owns that last mile.
 *
 * The font-binding invariant still holds — on a coarser grain. A PaintText
 * with `font_ref: "canvas:Helvetica@16"` is bindable only to a canvas-capable
 * backend. Routing it to paint-vm-metal or paint-vm-direct2d MUST throw
 * UnsupportedFontBindingError. The binding is the runtime, not the glyph index.
 *
 * Which instruction does layout-to-paint emit?
 * --------------------------------------------
 *
 *   Font-parser (TXT01) + naive/HarfBuzz shaper (TXT02/04) → PaintGlyphRun
 *   CoreText measurer (TXT03a)                             → PaintGlyphRun
 *   DirectWrite / Pango (TXT03b/c)                         → PaintGlyphRun
 *   Canvas measurer (TXT03d)                               → PaintText
 *
 * A pipeline picks one emitter at configuration time; instructions of both
 * kinds may coexist in one scene only if both backends are available
 * (uncommon).
 */
export interface PaintText extends PaintBase {
  kind: "text";
  x: number;         // alignment anchor x. By default this is the baseline origin (left edge
                     // of the first character). When `text_align` is set, it is the reference
                     // point relative to which alignment is computed: "start" aligns left,
                     // "center" aligns the midpoint, "end" aligns right.
  y: number;         // baseline origin y — NOT the top of the text, but the baseline
  text: string;      // the literal text to render (UTF-16 in TypeScript)
  font_ref: string;  // opaque font reference; e.g. "canvas:Helvetica@16:700"
                     // the scheme prefix (canvas:, css:, svg:) routes dispatch
  font_size: number; // in user-space units (same units as x, y)
  fill: string;      // color of the text — required (no default)

  /**
   * Horizontal alignment of the text relative to `x`. Default: "start".
   *
   * - `"start"` — left edge of text at `x` (LTR). Matches Canvas `textAlign = "start"`.
   * - `"center"` — midpoint of text at `x`. Matches Canvas `textAlign = "center"`.
   * - `"end"` — right edge of text at `x` (LTR). Matches Canvas `textAlign = "end"`.
   *
   * Canvas backends translate this to `ctx.textAlign`. Backends that pre-measure
   * (SVG) or that always left-align internally will adjust `x` by
   * `+textWidth*0` / `-textWidth/2` / `-textWidth` at dispatch time.
   */
  text_align?: "start" | "center" | "end";

  // Optional: map from cluster (string index) to pen x-offset, computed by the
  // layout engine via its TextMeasurer. Enables hit-testing and selection
  // without re-measuring at paint time. Omit for simple rendering.
  cluster_positions?: Array<{ cluster: number; x: number }>;
}

/**
 * PaintGroup — logical container for transform and state inheritance.
 *
 * A group renders directly into the parent surface — no separate buffer is
 * allocated. It is useful for:
 *   - Applying a transform to a set of instructions as a unit
 *   - Applying an opacity to a set of instructions
 *   - Logical grouping for patch() id stability (give the group an id, not
 *     each child — when the group moves, only one diff entry is needed)
 *
 * For filters or blend modes, use PaintLayer instead. Layers allocate a
 * separate offscreen buffer so filters can operate on the composited result.
 *
 * transform is a 6-element affine matrix [a, b, c, d, e, f]:
 *   identity = [1, 0, 0, 1, 0, 0]
 *   translate by (tx, ty) = [1, 0, 0, 1, tx, ty]
 *   rotate by θ = [cos θ, sin θ, -sin θ, cos θ, 0, 0]
 */
export interface PaintGroup extends PaintBase {
  kind: "group";
  children: PaintInstruction[];
  transform?: Transform2D;
  opacity?: number; // 0.0–1.0; default 1.0
}

/**
 * PaintLayer — isolated offscreen compositing surface.
 *
 * A PaintLayer is fundamentally different from a PaintGroup.
 *
 * PaintGroup: renders children directly into the parent surface.
 *             Fast. No offscreen allocation. Cannot apply filters.
 *
 * PaintLayer: allocates a SEPARATE offscreen buffer. Renders children into
 *             that buffer. Applies filters to the composited result. Then
 *             composites the entire buffer back into the parent using blend_mode.
 *
 * This is the same model as Photoshop layers, CSS filter + mix-blend-mode,
 * and SVG <filter> elements. Use it when you need:
 *   - Blur that bleeds across multiple child elements (not just one rect)
 *   - Drop shadow applied to a group as a whole
 *   - Multiply blend mode for a set of overlapping shapes
 *   - Opacity applied uniformly before compositing (avoids overlap artifacts)
 *
 * Backend support:
 *   Canvas/SVG — native (ctx.filter, globalCompositeOperation, SVG <filter>)
 *   Metal/Vulkan — allocate texture, compute shaders for filters, blend in fragment shader
 *   Terminal — degrades: renders as PaintGroup, filters ignored, warn once
 *
 * Masking (path-based) is explicitly deferred. It will be designed as
 * PaintMask in a future spec (P2D08). It requires a two-pass render.
 */
export interface PaintLayer extends PaintBase {
  kind: "layer";
  children: PaintInstruction[];
  filters?: FilterEffect[];
  blend_mode?: BlendMode; // default "normal"
  opacity?: number; // 0.0–1.0; default 1.0; applied after filters
  transform?: Transform2D;
}

/**
 * PaintLine — a straight line segment between two points.
 *
 * This is a convenience instruction for the common case of a single straight
 * line. For multiple connected lines, use PaintPath with line_to commands.
 */
export interface PaintLine extends PaintBase {
  kind: "line";
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  stroke: string; // required — a line with no color is invisible
  stroke_width?: number; // default 1.0
  stroke_cap?: "butt" | "round" | "square";
}

/**
 * PaintClip — clip mask for child instructions.
 *
 * Children are rendered clipped to the given rectangle. Pixels outside the
 * clip rect are not drawn. The clip is applied only to the children array —
 * instructions outside PaintClip are not affected.
 *
 * This is currently a rectangular clip. Arbitrary path clipping is deferred
 * to PaintMask (P2D08).
 *
 * Implementation note: backends use save/clip/restore semantics internally.
 * The clip does not permanently modify the context state.
 */
export interface PaintClip extends PaintBase {
  kind: "clip";
  x: number;
  y: number;
  width: number;
  height: number;
  children: PaintInstruction[];
}

/**
 * PaintGradient — linear or radial colour gradient.
 *
 * A PaintGradient is referenced by id from a PaintRect, PaintEllipse, or
 * PaintPath fill field: `fill: "url(#my-gradient)"`.
 *
 * The gradient is defined once and referenced multiple times — just like SVG
 * <linearGradient> elements defined in <defs>.
 *
 * Stops define colour transition points along the gradient axis:
 *   offset — 0.0 (start) to 1.0 (end)
 *   color  — CSS color at this point
 *
 * Linear gradient:
 *   The gradient runs from (x1, y1) to (x2, y2) in user space.
 *   Colour interpolates along the axis between those points.
 *
 *   gradient axis → → → → → → → → →
 *   (x1,y1)                           (x2,y2)
 *   stop0      stop1     stop2         stop3
 *
 * Radial gradient:
 *   The gradient radiates from center (cx, cy) outward to radius r.
 *   Innermost colour at (cx, cy), outermost at radius r.
 */
export interface PaintGradient extends PaintBase {
  kind: "gradient";
  gradient_kind: "linear" | "radial";
  stops: Array<{ offset: number; color: string }>;
  // Linear gradient fields:
  x1?: number;
  y1?: number;
  x2?: number;
  y2?: number;
  // Radial gradient fields:
  cx?: number;
  cy?: number;
  r?: number;
}

/**
 * PaintImage — raster or decoded pixel image.
 *
 * src accepts two forms:
 *
 *   string — A URI or data URL that the backend resolves at render time:
 *     "https://example.com/photo.jpg"     — remote URI
 *     "file:///assets/logo.png"           — local file URI
 *     "data:image/png;base64,iVBORw0K..." — inline data URL
 *   The backend is responsible for decoding the image. The IR does not fetch
 *   or validate URI strings.
 *
 *   PixelContainer — Already-decoded pixels. The VM paints them directly with
 *   no decoding step. This is the zero-copy path when you already have decoded
 *   pixels (e.g., the output of vm.export() fed into another scene for
 *   picture-in-picture, thumbnail strips, or sub-scene compositing).
 *
 * The VM does not care which form is used. Both produce the same visual result.
 *
 * Example — compositing a sub-scene into a larger scene:
 *   const sub_pixels = thumbnail_vm.export(sub_scene);
 *   const main_scene = { ..., instructions: [
 *     { kind: "image", x: 50, y: 50, width: 300, height: 200, src: sub_pixels },
 *   ]};
 */
export interface PaintImage extends PaintBase {
  kind: "image";
  x: number; // top-left x of the rendered rectangle
  y: number; // top-left y
  width: number; // rendered width (may differ from intrinsic image width)
  height: number; // rendered height
  src: string | PixelContainer;
  opacity?: number; // 0.0–1.0; default 1.0
}

// ============================================================================
// PaintInstruction — the union of all instruction types
// ============================================================================

/**
 * The complete union of all paint instruction types.
 *
 * Every instruction has a `kind` string discriminant so dispatch tables and
 * pattern-match arms can route without instanceof checks:
 *
 *   switch (instr.kind) {
 *     case "rect":      // PaintRect
 *     case "ellipse":   // PaintEllipse
 *     case "path":      // PaintPath
 *     case "glyph_run": // PaintGlyphRun
 *     case "text":      // PaintText (Canvas-runtime string-level text; see TXT03d)
 *     case "group":     // PaintGroup
 *     case "layer":     // PaintLayer
 *     case "line":      // PaintLine
 *     case "clip":      // PaintClip
 *     case "gradient":  // PaintGradient
 *     case "image":     // PaintImage
 *   }
 */
export type PaintInstruction =
  | PaintRect
  | PaintEllipse
  | PaintPath
  | PaintGlyphRun
  | PaintText
  | PaintGroup
  | PaintLayer
  | PaintLine
  | PaintClip
  | PaintGradient
  | PaintImage;

// ============================================================================
// PaintScene — the top-level container
// ============================================================================

/**
 * A PaintScene is the top-level value passed to PaintVM.execute() or patch().
 *
 * It defines the viewport dimensions, the background fill, and the ordered
 * list of instructions. Instructions are rendered back-to-front (painter's
 * algorithm): the first instruction in the array is painted first (furthest back).
 *
 * background is a CSS color painted before all instructions:
 *   "#ffffff"     — white background
 *   "#000000"     — black background
 *   "transparent" — no background fill (useful for compositing)
 *
 * The optional id field identifies the scene for PaintVM.patch(). If two scenes
 * have the same id, patch() can assert they are versions of the same scene.
 */
export interface PaintScene {
  width: number; // viewport width in user-space units
  height: number; // viewport height in user-space units
  background: string; // CSS color; painted before all instructions
  instructions: PaintInstruction[];
  id?: string;
  metadata?: Record<string, string | number | boolean>;
}

// ============================================================================
// Builder helpers — ergonomic construction of PaintInstruction objects
// ============================================================================

/**
 * Create a PaintScene.
 *
 * Example:
 *   const scene = paintScene(800, 600, "#f8fafc", [
 *     paintRect(0, 0, 100, 50, { fill: "#2563eb" }),
 *   ]);
 */
export function paintScene(
  width: number,
  height: number,
  background: string,
  instructions: PaintInstruction[],
  options?: { id?: string; metadata?: Record<string, string | number | boolean> },
): PaintScene {
  return { width, height, background, instructions, ...options };
}

/**
 * Create a PaintRect instruction.
 *
 * Example:
 *   paintRect(10, 10, 200, 120, { fill: "#2563eb", stroke: "#fff", stroke_width: 2 })
 */
export function paintRect(
  x: number,
  y: number,
  width: number,
  height: number,
  options?: Omit<PaintRect, "kind" | "x" | "y" | "width" | "height">,
): PaintRect {
  return { kind: "rect", x, y, width, height, ...options };
}

/**
 * Create a PaintEllipse instruction.
 *
 * cx, cy is the center. rx, ry are the radii.
 * For a circle: paintEllipse(cx, cy, r, r, options).
 */
export function paintEllipse(
  cx: number,
  cy: number,
  rx: number,
  ry: number,
  options?: Omit<PaintEllipse, "kind" | "cx" | "cy" | "rx" | "ry">,
): PaintEllipse {
  return { kind: "ellipse", cx, cy, rx, ry, ...options };
}

/**
 * Create a PaintPath instruction.
 *
 * Example:
 *   paintPath([
 *     { kind: "move_to", x: 0, y: 0 },
 *     { kind: "line_to", x: 100, y: 0 },
 *     { kind: "line_to", x: 50, y: 86 },
 *     { kind: "close" },
 *   ], { fill: "#ef4444" })
 */
export function paintPath(
  commands: PathCommand[],
  options?: Omit<PaintPath, "kind" | "commands">,
): PaintPath {
  return { kind: "path", commands, ...options };
}

/**
 * Create a PaintLine instruction.
 *
 * Example:
 *   paintLine(0, 50, 200, 50, "#9ca3af", { stroke_width: 1 })
 */
export function paintLine(
  x1: number,
  y1: number,
  x2: number,
  y2: number,
  stroke: string,
  options?: Omit<PaintLine, "kind" | "x1" | "y1" | "x2" | "y2" | "stroke">,
): PaintLine {
  return { kind: "line", x1, y1, x2, y2, stroke, ...options };
}

/**
 * Create a PaintGroup instruction.
 *
 * Example:
 *   paintGroup([paintRect(0, 0, 100, 50)], { transform: [1, 0, 0, 1, 50, 50] })
 */
export function paintGroup(
  children: PaintInstruction[],
  options?: Omit<PaintGroup, "kind" | "children">,
): PaintGroup {
  return { kind: "group", children, ...options };
}

/**
 * Create a PaintLayer instruction.
 *
 * Use PaintLayer (not PaintGroup) when you need filters or blend modes.
 * The layer allocates an offscreen buffer, renders children into it, applies
 * filters, and composites the result using blend_mode.
 *
 * Example — blurred glow layer:
 *   paintLayer([paintEllipse(100, 100, 50, 50, { fill: "#3b82f6" })], {
 *     filters: [{ kind: "blur", radius: 15 }],
 *   })
 */
export function paintLayer(
  children: PaintInstruction[],
  options?: Omit<PaintLayer, "kind" | "children">,
): PaintLayer {
  return { kind: "layer", children, ...options };
}

/**
 * Create a PaintClip instruction.
 *
 * Children are clipped to the rectangle (x, y, width, height).
 *
 * Example:
 *   paintClip(0, 0, 400, 300, [paintRect(-50, -50, 600, 500, { fill: "#e0f2fe" })])
 */
export function paintClip(
  x: number,
  y: number,
  width: number,
  height: number,
  children: PaintInstruction[],
  options?: Omit<PaintClip, "kind" | "x" | "y" | "width" | "height" | "children">,
): PaintClip {
  return { kind: "clip", x, y, width, height, children, ...options };
}

/**
 * Create a PaintGradient instruction.
 *
 * Linear gradient example:
 *   paintGradient("linear", [
 *     { offset: 0, color: "#3b82f6" },
 *     { offset: 1, color: "#8b5cf6" },
 *   ], { id: "blue-purple", x1: 0, y1: 0, x2: 400, y2: 0 })
 *
 * Radial gradient example:
 *   paintGradient("radial", [
 *     { offset: 0, color: "#ffffff" },
 *     { offset: 1, color: "#3b82f6" },
 *   ], { cx: 200, cy: 150, r: 100 })
 */
export function paintGradient(
  gradient_kind: "linear" | "radial",
  stops: Array<{ offset: number; color: string }>,
  options?: Omit<PaintGradient, "kind" | "gradient_kind" | "stops">,
): PaintGradient {
  return { kind: "gradient", gradient_kind, stops, ...options };
}

/**
 * Create a PaintImage instruction.
 *
 * src can be a URI string or a PixelContainer (already-decoded pixels).
 *
 * Example (URI):
 *   paintImage(50, 50, 300, 200, "file:///assets/logo.png")
 *
 * Example (PixelContainer — zero-copy from another VM's export):
 *   const pixels = thumbnailVm.export(subScene);
 *   paintImage(50, 50, 300, 200, pixels)
 */
export function paintImage(
  x: number,
  y: number,
  width: number,
  height: number,
  src: string | PixelContainer,
  options?: Omit<PaintImage, "kind" | "x" | "y" | "width" | "height" | "src">,
): PaintImage {
  return { kind: "image", x, y, width, height, src, ...options };
}

/**
 * Create a PaintText instruction.
 *
 * Use this when the paint backend is a runtime that does its own shaping at
 * paint time (HTML Canvas 2D via `ctx.fillText`). For backends that consume
 * pre-shaped glyph indices (Metal, Direct2D with DirectWrite), use
 * PaintGlyphRun instead.
 *
 * font_ref is an opaque string with a scheme prefix that routes dispatch:
 *   "canvas:<family>@<size>[:<weight>[:<style>]]"  — HTML Canvas
 *   "css:<css-font-shorthand>"                      — deferred
 *   "svg:<family>@<size>"                           — deferred
 *
 * Example:
 *   paintText(20, 40, "Hello world", "canvas:Helvetica@16", 16, "#111")
 */
export function paintText(
  x: number,
  y: number,
  text: string,
  font_ref: string,
  font_size: number,
  fill: string,
  options?: Omit<PaintText, "kind" | "x" | "y" | "text" | "font_ref" | "font_size" | "fill">,
): PaintText {
  return { kind: "text", x, y, text, font_ref, font_size, fill, ...options };
}
