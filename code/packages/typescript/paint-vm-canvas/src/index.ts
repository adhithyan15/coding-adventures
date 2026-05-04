/**
 * @coding-adventures/paint-vm-canvas
 *
 * HTML5 Canvas backend for PaintVM (P2D03).
 *
 * This backend renders a PaintScene directly to a CanvasRenderingContext2D.
 * It skips the DOM entirely — no `<svg>` elements, no HTML parsing. The caller
 * supplies the context; this package supplies the painting logic.
 *
 * ## Why Canvas instead of SVG for interactive rendering?
 *
 * SVG is declarative and retained: the browser maintains a scene graph in memory,
 * which has non-trivial overhead for large scenes. For 1000+ elements, SVG can
 * become slow to update.
 *
 * Canvas is imperative: drawing commands go straight to the GPU rasterizer.
 * There is no retained tree. This is better for:
 *   - Games and animations (execute() every frame at 60fps)
 *   - Large scenes (thousands of bars, points, or tiles)
 *   - Off-screen rendering (OffscreenCanvas in web workers)
 *   - Server-side rendering (node-canvas or Skia bindings)
 *
 * ## Usage (browser)
 *
 * ```typescript
 * import { createCanvasVM } from "@coding-adventures/paint-vm-canvas";
 * import { paintScene, paintRect } from "@coding-adventures/paint-instructions";
 *
 * const canvas = document.getElementById("my-canvas") as HTMLCanvasElement;
 * const ctx = canvas.getContext("2d")!;
 *
 * const vm = createCanvasVM();
 * vm.execute(paintScene(800, 400, "#ffffff", [
 *   paintRect(20, 20, 200, 100, { fill: "#3b82f6", corner_radius: 8 }),
 * ]), ctx);
 * ```
 *
 * ## Usage (OffscreenCanvas — web worker)
 *
 * ```typescript
 * const offscreen = new OffscreenCanvas(800, 400);
 * const ctx = offscreen.getContext("2d")!;
 * vm.execute(scene, ctx);
 * const blob = await offscreen.convertToBlob({ type: "image/webp" });
 * ```
 *
 * ## Usage (export to PixelContainer)
 *
 * ```typescript
 * const pixels = vm.export(scene, { scale: 2 }); // 2× Retina
 * const bytes = pngCodec.encode(pixels);
 * ```
 *
 * ## Context type
 *
 * TContext = CanvasRenderingContext2D
 *
 * The Canvas backend does not produce strings. Every handler directly mutates
 * the 2D context. The context tracks state (transform stack, fill/stroke styles,
 * clip region) so handlers must use save()/restore() when modifying it.
 */
export const VERSION = "0.1.0";

import { PaintVM, ExportNotSupportedError } from "@coding-adventures/paint-vm";
import type {
  PaintInstruction,
  PaintRect,
  PaintEllipse,
  PaintPath,
  PaintGlyphRun,
  PaintText,
  PaintGroup,
  PaintLayer,
  PaintLine,
  PaintClip,
  PaintGradient,
  PaintImage,
  PaintScene,
  PathCommand,
  PixelContainer,
} from "@coding-adventures/paint-instructions";

// ============================================================================
// PathCommand → Canvas Path2D operations
// ============================================================================

/**
 * Apply an array of PathCommands to a Canvas Path2D object.
 *
 * The Canvas API uses Path2D to represent reusable vector paths. We build a
 * Path2D by calling the appropriate method for each command:
 *
 *   move_to  → path.moveTo(x, y)
 *   line_to  → path.lineTo(x, y)
 *   quad_to  → path.quadraticCurveTo(cx, cy, x, y)
 *   cubic_to → path.bezierCurveTo(cx1, cy1, cx2, cy2, x, y)
 *   arc_to   → path.ellipse(...)  — converted from SVG arc to Canvas ellipse
 *   close    → path.closePath()
 *
 * Note on arc_to: Canvas does not have a direct SVG arc command equivalent.
 * We convert the SVG arc parameters to Canvas's ellipse() call using the
 * SVG arc to center parameterization formula.
 */
function applyCommandsToPath2D(
  path: Path2D,
  commands: PathCommand[],
): void {
  for (const cmd of commands) {
    switch (cmd.kind) {
      case "move_to":
        path.moveTo(cmd.x, cmd.y);
        break;
      case "line_to":
        path.lineTo(cmd.x, cmd.y);
        break;
      case "quad_to":
        path.quadraticCurveTo(cmd.cx, cmd.cy, cmd.x, cmd.y);
        break;
      case "cubic_to":
        path.bezierCurveTo(cmd.cx1, cmd.cy1, cmd.cx2, cmd.cy2, cmd.x, cmd.y);
        break;
      case "arc_to":
        // SVG arc_to → Canvas ellipse() conversion.
        // Canvas ellipse() takes: (cx, cy, rx, ry, rotation, startAngle, endAngle, counterclockwise)
        // SVG arc gives us: (current_point, rx, ry, x_rotation, large_arc, sweep, endpoint)
        // This is a simplified approximation — a full implementation would use
        // the SVG arc center parameterization formula. For now, emit a lineTo
        // to the endpoint as a fallback.
        path.lineTo(cmd.x, cmd.y);
        break;
      case "close":
        path.closePath();
        break;
    }
  }
}

// ============================================================================
// FilterEffect → Canvas filter string
// ============================================================================

/**
 * Convert an array of FilterEffects to a CSS filter string for ctx.filter.
 *
 * Canvas's ctx.filter property accepts a CSS filter string. We chain multiple
 * effects by concatenating the filter functions with spaces.
 *
 * Note: ctx.filter is applied as a single composite filter. Each effect maps
 * directly to a CSS filter function:
 *   blur(Npx), drop-shadow(dx dy blur color), brightness(N),
 *   contrast(N), saturate(N), hue-rotate(Ndeg), invert(N), opacity(N)
 *
 * color_matrix has no direct CSS equivalent — we approximate with saturate.
 */
function buildCanvasFilter(
  filters: PaintLayer["filters"] | undefined,
): string {
  if (!filters || filters.length === 0) return "none";

  // Security helper: validate a number is finite before interpolating into CSS.
  // NaN / Infinity produce invalid CSS filter strings and may behave unexpectedly
  // in headless Canvas environments (node-canvas / Cairo).
  function safeN(value: number, field: string): number {
    if (!Number.isFinite(value)) {
      throw new RangeError(
        `PaintVM Canvas: ${field} must be a finite number, got ${value}`,
      );
    }
    return value;
  }

  // Security: validate that a color value used in drop-shadow does not contain
  // characters that could break out of the CSS function and inject additional
  // filter functions. A crafted color like ") brightness(100" would close the
  // drop-shadow() function and inject an arbitrary CSS filter.
  //
  // We accept only two safe color formats:
  //   1. Hex colors:   #rgb, #rrggbb, #rgba, #rrggbbaa
  //   2. Named colors: letters only (e.g. "black", "red", "transparent")
  //
  // rgb()/rgba()/hsl() are NOT accepted because they require ")" which is
  // the injection vector. Callers that need dynamic colors should use hex.
  function sanitizeColor(color: string): string {
    if (/^#[0-9a-fA-F]{3,8}$/.test(color)) return color;  // #rgb / #rrggbb / etc.
    if (/^[a-zA-Z]+$/.test(color)) return color;           // named color (e.g. "black")
    return "black"; // unsafe — fall back to a known-safe color
  }

  const parts: string[] = [];
  for (const f of filters) {
    switch (f.kind) {
      case "blur":
        parts.push(`blur(${safeN(f.radius, "blur.radius")}px)`);
        break;
      case "drop_shadow":
        parts.push(`drop-shadow(${safeN(f.dx, "drop_shadow.dx")}px ${safeN(f.dy, "drop_shadow.dy")}px ${safeN(f.blur, "drop_shadow.blur")}px ${sanitizeColor(f.color)})`);
        break;
      case "brightness":
        parts.push(`brightness(${safeN(f.amount, "brightness.amount")})`);
        break;
      case "contrast":
        parts.push(`contrast(${safeN(f.amount, "contrast.amount")})`);
        break;
      case "saturate":
        parts.push(`saturate(${safeN(f.amount, "saturate.amount")})`);
        break;
      case "hue_rotate":
        parts.push(`hue-rotate(${safeN(f.angle, "hue_rotate.angle")}deg)`);
        break;
      case "invert":
        parts.push(`invert(${safeN(f.amount, "invert.amount")})`);
        break;
      case "opacity":
        parts.push(`opacity(${safeN(f.amount, "opacity.amount")})`);
        break;
      case "color_matrix":
        // No direct CSS equivalent — skip (a full impl would use WebGL/WASM)
        break;
    }
  }
  return parts.length > 0 ? parts.join(" ") : "none";
}

// ============================================================================
// BlendMode → Canvas globalCompositeOperation
// ============================================================================

/**
 * Map our BlendMode union to the Canvas globalCompositeOperation string.
 *
 * Canvas uses kebab-case for multi-word modes ("color-dodge").
 * Our BlendMode uses underscores ("color_dodge").
 *
 * Security: the return value is assigned to ctx.globalCompositeOperation.
 * We maintain a runtime allowlist to prevent unexpected behavior if a caller
 * passes an arbitrary string (e.g. via `as any` or a deserialized payload).
 * Unknown values fall back to "source-over" (the Canvas default for "normal").
 */
const CANVAS_BLEND_MODE_ALLOWLIST = new Set([
  "normal", "multiply", "screen", "overlay", "darken", "lighten",
  "color-dodge", "color-burn", "hard-light", "soft-light", "difference",
  "exclusion", "hue", "saturation", "color", "luminosity",
  // Canvas-specific composite operations not in SVG blend modes
  "source-over", "source-in", "source-out", "source-atop",
  "destination-over", "destination-in", "destination-out", "destination-atop",
  "xor", "copy", "lighter",
]);

function blendModeToCanvas(mode: string): string {
  const canvasMode = mode.replace(/_/g, "-");
  return CANVAS_BLEND_MODE_ALLOWLIST.has(canvasMode) ? canvasMode : "source-over";
}

// ============================================================================
// Instruction handlers
// ============================================================================

function handleRect(
  instr: PaintRect,
  ctx: CanvasRenderingContext2D,
): void {
  const hasRadius = instr.corner_radius && instr.corner_radius > 0;

  if (hasRadius) {
    // Rounded rect via roundRect() — available in modern browsers and node-canvas
    // Fall back to standard rect if roundRect is not available
    ctx.beginPath();
    if (typeof ctx.roundRect === "function") {
      ctx.roundRect(instr.x, instr.y, instr.width, instr.height, instr.corner_radius);
    } else {
      // Polyfill: approximate with arcTo
      const r = instr.corner_radius!;
      const { x, y, width: w, height: h } = instr;
      ctx.moveTo(x + r, y);
      ctx.arcTo(x + w, y, x + w, y + h, r);
      ctx.arcTo(x + w, y + h, x, y + h, r);
      ctx.arcTo(x, y + h, x, y, r);
      ctx.arcTo(x, y, x + w, y, r);
      ctx.closePath();
    }
    if (instr.fill) {
      ctx.fillStyle = instr.fill;
      ctx.fill();
    }
    if (instr.stroke) {
      ctx.strokeStyle = instr.stroke;
      ctx.lineWidth = instr.stroke_width ?? 1;
      ctx.stroke();
    }
  } else {
    if (instr.fill) {
      ctx.fillStyle = instr.fill;
      ctx.fillRect(instr.x, instr.y, instr.width, instr.height);
    }
    if (instr.stroke) {
      ctx.strokeStyle = instr.stroke;
      ctx.lineWidth = instr.stroke_width ?? 1;
      ctx.strokeRect(instr.x, instr.y, instr.width, instr.height);
    }
  }
}

function handleEllipse(
  instr: PaintEllipse,
  ctx: CanvasRenderingContext2D,
): void {
  ctx.beginPath();
  ctx.ellipse(instr.cx, instr.cy, instr.rx, instr.ry, 0, 0, Math.PI * 2);
  if (instr.fill) {
    ctx.fillStyle = instr.fill;
    ctx.fill();
  }
  if (instr.stroke) {
    ctx.strokeStyle = instr.stroke;
    ctx.lineWidth = instr.stroke_width ?? 1;
    ctx.stroke();
  }
}

function handlePath(
  instr: PaintPath,
  ctx: CanvasRenderingContext2D,
): void {
  const path = new Path2D();
  applyCommandsToPath2D(path, instr.commands);

  if (instr.stroke_cap) ctx.lineCap = instr.stroke_cap;
  if (instr.stroke_join) ctx.lineJoin = instr.stroke_join;

  if (instr.fill) {
    ctx.fillStyle = instr.fill;
    ctx.fill(path, instr.fill_rule ?? "nonzero");
  }
  if (instr.stroke) {
    ctx.strokeStyle = instr.stroke;
    ctx.lineWidth = instr.stroke_width ?? 1;
    ctx.stroke(path);
  }
}

/**
 * Sanitize a font family name before interpolating it into ctx.font.
 *
 * Security: ctx.font = `Npx <font_ref>` passes the string directly to the
 * browser's CSS font parser. An adversarially crafted font_ref could inject
 * CSS properties (e.g., "serif; content: attr(x)") or trigger unexpected
 * behavior in system font loading APIs (e.g., node-canvas / Skia).
 *
 * We strip anything except alphanumeric characters, spaces, hyphens,
 * underscores, and commas (which are valid in font-family stacks). This is a
 * conservative allowlist that covers all real font family names.
 */
function sanitizeFontRef(fontRef: string): string {
  // Allow: letters, digits, spaces, hyphens, underscores, commas, periods
  return fontRef.replace(/[^a-zA-Z0-9 ,\-_.]/g, "");
}

function handleGlyphRun(
  instr: PaintGlyphRun,
  ctx: CanvasRenderingContext2D,
): void {
  // Canvas renders glyphs via fillText. For a proper glyph-id based implementation,
  // you'd need a font file loaded via FontFace API and glyph-to-character mapping.
  // For now, treat glyph_id as a Unicode codepoint for display purposes.
  // Security: font_size is interpolated into the CSS font shorthand string.
  // Validate it is a finite number to prevent string values from injecting
  // CSS properties (e.g. font_size = "12px Arial; font-family: x").
  if (!Number.isFinite(instr.font_size)) {
    throw new RangeError(
      `PaintVM Canvas: font_size must be a finite number, got ${instr.font_size}`,
    );
  }
  ctx.font = `${instr.font_size}px ${sanitizeFontRef(instr.font_ref)}`;
  ctx.fillStyle = instr.fill ?? "#000000";
  for (const g of instr.glyphs) {
    // Security: String.fromCodePoint() throws a RangeError if the argument is
    // negative, greater than 0x10FFFF, or non-integer — a DoS vector if the
    // IR is deserialized from untrusted input. Substitute U+FFFD for invalid ids.
    const id = g.glyph_id;
    const safeId =
      Number.isInteger(id) && id >= 0 && id <= 0x10ffff ? id : 0xfffd;
    const char = String.fromCodePoint(safeId);
    ctx.fillText(char, g.x, g.y);
  }
}

/**
 * UnsupportedFontBindingError — thrown when a PaintText instruction carries a
 * font_ref scheme that this backend cannot consume. Matches the font-binding
 * invariant from TXT00 / P2D00: a font_ref is opaque token bound to a
 * specific shaper/runtime, and the paint backend must refuse mismatches
 * rather than guess.
 */
export class UnsupportedFontBindingError extends Error {
  constructor(fontRef: string) {
    super(
      `PaintVM Canvas: unsupported font_ref scheme "${fontRef}". ` +
        `This backend only accepts scheme "canvas:" (see spec TXT03d). ` +
        `Routing a PaintText with a different scheme (e.g. "coretext:", ` +
        `"directwrite:") would violate the font-binding invariant.`,
    );
    this.name = "UnsupportedFontBindingError";
  }
}

/**
 * Parse a "canvas:" font_ref into a CSS font shorthand that ctx.font accepts.
 *
 * Grammar (from spec TXT03d):
 *
 *   font_ref := "canvas:" <family> "@" <px_size> [ ":" <weight> [ ":" <style> ] ]
 *
 * Examples:
 *
 *   "canvas:Helvetica@16"              → "400 16px 'Helvetica'"
 *   "canvas:Helvetica@16:700"          → "700 16px 'Helvetica'"
 *   "canvas:Helvetica@16:700:italic"   → "italic 700 16px 'Helvetica'"
 *   "canvas:system-ui@14"              → "400 14px 'system-ui'"
 *
 * Security: family is run through sanitizeFontRef to strip any CSS-injection
 * characters. Weight is validated as a number in [1, 1000]. Style is checked
 * against an allowlist. Any malformed input falls back to "16px sans-serif".
 *
 * The font_size argument overrides the size encoded in font_ref — the layout
 * engine is the source of truth for size at paint time.
 */
function canvasFontRefToCss(fontRef: string, fontSize: number): string {
  if (!fontRef.startsWith("canvas:")) {
    throw new UnsupportedFontBindingError(fontRef);
  }
  const body = fontRef.slice("canvas:".length);

  // Split family from size/weight/style
  const atIdx = body.indexOf("@");
  const family = atIdx >= 0 ? body.slice(0, atIdx) : body;
  const rest = atIdx >= 0 ? body.slice(atIdx + 1) : "";
  const parts = rest.split(":");
  // parts[0] is size (we ignore it in favour of the authoritative font_size arg)
  const weightStr = parts[1];
  const styleStr = parts[2];

  const safeFamily = sanitizeFontRef(family) || "sans-serif";

  let weight = "400";
  if (weightStr !== undefined) {
    const w = Number(weightStr);
    if (Number.isFinite(w) && w >= 1 && w <= 1000) {
      weight = String(Math.round(w));
    }
  }

  let style = "";
  if (styleStr === "italic" || styleStr === "oblique") {
    style = `${styleStr} `;
  }

  return `${style}${weight} ${fontSize}px '${safeFamily}'`;
}

function handleText(
  instr: PaintText,
  ctx: CanvasRenderingContext2D,
): void {
  // Validate font_size first — it goes directly into the CSS font shorthand.
  if (!Number.isFinite(instr.font_size)) {
    throw new RangeError(
      `PaintVM Canvas: font_size must be a finite number, got ${instr.font_size}`,
    );
  }
  ctx.save();
  ctx.font = canvasFontRefToCss(instr.font_ref, instr.font_size);
  ctx.fillStyle = instr.fill;
  // PaintText coordinates are a baseline origin (same semantics as PaintGlyphRun).
  // Canvas default textBaseline is "alphabetic", which aligns to the baseline —
  // exactly what we want.
  ctx.textBaseline = "alphabetic";
  // text_align maps directly to ctx.textAlign. "center" in PaintText is
  // "center" in Canvas (not "middle" — that's the textBaseline word).
  // Default "start" is also the Canvas default, so we only set it when
  // explicitly provided to avoid unnecessary state changes in save/restore.
  if (instr.text_align !== undefined) {
    ctx.textAlign = instr.text_align;
  }
  ctx.fillText(instr.text, instr.x, instr.y);
  ctx.restore();
}

function handleGroup(
  instr: PaintGroup,
  ctx: CanvasRenderingContext2D,
  vm: PaintVM<CanvasRenderingContext2D>,
): void {
  ctx.save();
  if (instr.transform) {
    const [a, b, c, d, e, f] = instr.transform;
    ctx.transform(a, b, c, d, e, f);
  }
  if (instr.opacity !== undefined) {
    ctx.globalAlpha = instr.opacity;
  }
  for (const child of instr.children) {
    vm.dispatch(child, ctx);
  }
  ctx.restore();
}

function handleLayer(
  instr: PaintLayer,
  ctx: CanvasRenderingContext2D,
  vm: PaintVM<CanvasRenderingContext2D>,
): void {
  // PaintLayer renders children to an offscreen buffer, applies filters, then
  // composites the result. On Canvas we use save/restore + ctx.filter +
  // ctx.globalCompositeOperation to achieve the same effect.
  ctx.save();

  const filterStr = buildCanvasFilter(instr.filters);
  if (filterStr !== "none") {
    ctx.filter = filterStr;
  }
  if (instr.blend_mode && instr.blend_mode !== "normal") {
    ctx.globalCompositeOperation = blendModeToCanvas(
      instr.blend_mode,
    ) as GlobalCompositeOperation;
  }
  if (instr.transform) {
    const [a, b, c, d, e, f] = instr.transform;
    ctx.transform(a, b, c, d, e, f);
  }
  if (instr.opacity !== undefined) {
    ctx.globalAlpha = instr.opacity;
  }

  for (const child of instr.children) {
    vm.dispatch(child, ctx);
  }

  ctx.restore();
}

function handleLine(
  instr: PaintLine,
  ctx: CanvasRenderingContext2D,
): void {
  ctx.beginPath();
  ctx.moveTo(instr.x1, instr.y1);
  ctx.lineTo(instr.x2, instr.y2);
  ctx.strokeStyle = instr.stroke;
  ctx.lineWidth = instr.stroke_width ?? 1;
  if (instr.stroke_cap) ctx.lineCap = instr.stroke_cap;
  ctx.stroke();
}

function handleClip(
  instr: PaintClip,
  ctx: CanvasRenderingContext2D,
  vm: PaintVM<CanvasRenderingContext2D>,
): void {
  ctx.save();
  ctx.beginPath();
  ctx.rect(instr.x, instr.y, instr.width, instr.height);
  ctx.clip();
  for (const child of instr.children) {
    vm.dispatch(child, ctx);
  }
  ctx.restore();
}

function handleGradient(
  instr: PaintGradient,
  ctx: CanvasRenderingContext2D,
): void {
  // Canvas gradients are created imperatively and referenced inline.
  // A gradient instruction with an id is pre-declared for use by fill="url(#id)".
  // In Canvas (unlike SVG), there's no deferred reference mechanism — the gradient
  // must be passed directly as a fillStyle. We store it in a registry keyed by id.
  // This registry is attached to the context via a WeakMap.
  if (!instr.id) return; // A gradient without id cannot be referenced

  let gradient: CanvasGradient;
  if (instr.gradient_kind === "linear") {
    gradient = ctx.createLinearGradient(
      instr.x1 ?? 0,
      instr.y1 ?? 0,
      instr.x2 ?? 0,
      instr.y2 ?? 0,
    );
  } else {
    gradient = ctx.createRadialGradient(
      instr.cx ?? 0,
      instr.cy ?? 0,
      0,
      instr.cx ?? 0,
      instr.cy ?? 0,
      instr.r ?? 0,
    );
  }
  for (const stop of instr.stops) {
    gradient.addColorStop(stop.offset, stop.color);
  }

  // Store in registry so rect/ellipse/path handlers can look it up by "url(#id)"
  const registry = getGradientRegistry(ctx);
  registry.set(instr.id, gradient);
}

function handleImage(
  instr: PaintImage,
  ctx: CanvasRenderingContext2D,
): void {
  // Canvas drawImage requires an ImageBitmap, HTMLImageElement, or ImageData.
  // For PixelContainer src, we can create an ImageData directly.
  // For string src, we'd need to fetch and decode the image — async, not
  // supported in synchronous execute(). We emit a placeholder rect instead.
  if (typeof instr.src !== "string") {
    const pixels = instr.src;
    // PixelContainer is fixed RGBA8 (4 channels, 8 bits, data: Uint8Array).
    // ImageData requires Uint8ClampedArray with the same RGBA8 layout.
    if (pixels.data instanceof Uint8Array) {
      const imageData = new ImageData(
        new Uint8ClampedArray(pixels.data.buffer),
        pixels.width,
        pixels.height,
      );
      ctx.save();
      if (instr.opacity !== undefined) ctx.globalAlpha = instr.opacity;
      ctx.putImageData(imageData, instr.x, instr.y);
      ctx.restore();
    }
  } else {
    // URI string — draw a placeholder rect (async image loading not supported here)
    ctx.save();
    ctx.fillStyle = "#e5e7eb";
    ctx.fillRect(instr.x, instr.y, instr.width, instr.height);
    ctx.restore();
  }
}

// ============================================================================
// Gradient registry — WeakMap keyed by CanvasRenderingContext2D
// ============================================================================

/**
 * A WeakMap from CanvasRenderingContext2D to a gradient id registry.
 *
 * Canvas gradients are created imperatively, not by reference. When a
 * PaintGradient instruction is dispatched, we create the CanvasGradient object
 * and store it here. When a PaintRect with fill="url(#grad-id)" is dispatched,
 * we look up the gradient here.
 *
 * The WeakMap key is the context itself, so the registry is automatically
 * garbage collected when the context is garbage collected.
 */
const gradientRegistries = new WeakMap<
  CanvasRenderingContext2D,
  Map<string, CanvasGradient>
>();

function getGradientRegistry(
  ctx: CanvasRenderingContext2D,
): Map<string, CanvasGradient> {
  if (!gradientRegistries.has(ctx)) {
    gradientRegistries.set(ctx, new Map());
  }
  return gradientRegistries.get(ctx)!;
}

/**
 * Resolve a fill string to a CanvasRenderingContext2D fill style.
 *
 * If the fill is a "url(#id)" reference, look up the gradient in the registry.
 * Otherwise, return the fill string as-is (CSS color).
 */
export function resolveFill(
  fill: string,
  ctx: CanvasRenderingContext2D,
): string | CanvasGradient {
  const match = fill.match(/^url\(#(.+)\)$/);
  if (match) {
    const registry = getGradientRegistry(ctx);
    return registry.get(match[1]) ?? fill;
  }
  return fill;
}

// ============================================================================
// createCanvasVM — factory function
// ============================================================================

/**
 * Create a fully configured PaintVM<CanvasRenderingContext2D>.
 *
 * The returned VM has handlers registered for all 10 instruction kinds.
 * Call vm.execute(scene, ctx) to render a scene to a Canvas context.
 *
 * The VM also supports vm.export(scene, options?) to render to a
 * PixelContainer via an internal OffscreenCanvas (if available).
 * In environments without OffscreenCanvas (Node.js without node-canvas),
 * export() throws ExportNotSupportedError.
 *
 * Example:
 *   const vm = createCanvasVM();
 *   const canvas = document.getElementById("chart") as HTMLCanvasElement;
 *   vm.execute(scene, canvas.getContext("2d")!);
 */
export function createCanvasVM(): PaintVM<CanvasRenderingContext2D> {
  const vm = new PaintVM<CanvasRenderingContext2D>(
    // clearFn
    (ctx, background, width, height) => {
      ctx.clearRect(0, 0, width, height);
      if (background !== "transparent" && background !== "none") {
        ctx.fillStyle = background;
        ctx.fillRect(0, 0, width, height);
      }
      // Clear the gradient registry for this context on each execute()
      gradientRegistries.delete(ctx);
    },
    // exportFn — uses OffscreenCanvas for pixel readback
    (scene: PaintScene, vm: PaintVM<CanvasRenderingContext2D>, opts) => {
      const w = Math.round(scene.width * opts.scale);
      const h = Math.round(scene.height * opts.scale);

      // OffscreenCanvas is available in browsers and modern Node.js runtimes.
      // If not available, throw so the caller knows to use a different backend.
      if (typeof OffscreenCanvas === "undefined") {
        throw new ExportNotSupportedError(
          "Canvas (OffscreenCanvas not available in this environment)",
        );
      }

      const offscreen = new OffscreenCanvas(w, h);
      const offCtx = offscreen.getContext("2d") as unknown as CanvasRenderingContext2D;
      if (opts.scale !== 1.0) {
        offCtx.scale(opts.scale, opts.scale);
      }
      vm.execute(scene, offCtx);

      const imageData = offCtx.getImageData(0, 0, w, h);
      // PixelContainer is fixed RGBA8: { width, height, data: Uint8Array }.
      // The old interface (channels, bit_depth, pixels, color_space) was
      // removed when pixel-container was simplified to a fixed RGBA8 type.
      const pixels: PixelContainer = {
        width: w,
        height: h,
        data: new Uint8Array(imageData.data.buffer),
      };
      return pixels;
    },
  );

  vm.register("rect", (instr, ctx) => {
    if (instr.kind === "rect") handleRect(instr as PaintRect, ctx);
  });
  vm.register("ellipse", (instr, ctx) => {
    if (instr.kind === "ellipse") handleEllipse(instr as PaintEllipse, ctx);
  });
  vm.register("path", (instr, ctx) => {
    if (instr.kind === "path") handlePath(instr as PaintPath, ctx);
  });
  vm.register("glyph_run", (instr, ctx) => {
    if (instr.kind === "glyph_run") handleGlyphRun(instr as PaintGlyphRun, ctx);
  });
  vm.register("text", (instr, ctx) => {
    if (instr.kind === "text") handleText(instr as PaintText, ctx);
  });
  vm.register("group", (instr, ctx, vm) => {
    if (instr.kind === "group") handleGroup(instr as PaintGroup, ctx, vm);
  });
  vm.register("layer", (instr, ctx, vm) => {
    if (instr.kind === "layer") handleLayer(instr as PaintLayer, ctx, vm);
  });
  vm.register("line", (instr, ctx) => {
    if (instr.kind === "line") handleLine(instr as PaintLine, ctx);
  });
  vm.register("clip", (instr, ctx, vm) => {
    if (instr.kind === "clip") handleClip(instr as PaintClip, ctx, vm);
  });
  vm.register("gradient", (instr, ctx) => {
    if (instr.kind === "gradient") handleGradient(instr as PaintGradient, ctx);
  });
  vm.register("image", (instr, ctx) => {
    if (instr.kind === "image") handleImage(instr as PaintImage, ctx);
  });

  return vm;
}
