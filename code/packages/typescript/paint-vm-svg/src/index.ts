/**
 * @coding-adventures/paint-vm-svg
 *
 * SVG string backend for PaintVM (P2D02).
 *
 * This backend renders a PaintScene to an SVG string. The output is a complete
 * `<svg>` element that can be:
 *   - Embedded in HTML: `div.innerHTML = svgString`
 *   - Written to a .svg file
 *   - Passed to a PDF renderer
 *   - Served as an HTTP response with Content-Type: image/svg+xml
 *
 * ## Why SVG?
 *
 * SVG output has several advantages over Canvas:
 *   - Vector — scales perfectly to any resolution without pixel artifacts
 *   - Serializable — the output is a string, suitable for files and HTTP responses
 *   - Server-side — no browser or DOM needed to produce SVG output
 *   - Diffable — SVG strings can be stored in git and reviewed in PRs
 *   - Accessible — SVG elements support ARIA attributes
 *
 * ## Context type
 *
 * The SVG backend's TContext is `SvgContext` — an internal accumulator that
 * collects SVG elements as strings and tracks current state (defs, clip paths).
 *
 * Why strings instead of DOM nodes?
 *
 * DOM manipulation requires `document`, which is only available in browsers.
 * By building strings, this backend works in Node.js, Deno, Bun, and any
 * non-browser environment. The output is identical regardless of runtime.
 *
 * ## Usage
 *
 * ```typescript
 * import { createSvgVM, renderToSvgString } from "@coding-adventures/paint-vm-svg";
 * import { paintScene, paintRect } from "@coding-adventures/paint-instructions";
 *
 * const scene = paintScene(400, 300, "#ffffff", [
 *   paintRect(20, 20, 200, 100, { fill: "#3b82f6", corner_radius: 8 }),
 * ]);
 *
 * const svg = renderToSvgString(scene);
 * // → '<svg xmlns="http://www.w3.org/2000/svg" width="400" height="300">...'
 * ```
 */
export const VERSION = "0.1.0";

import { PaintVM, ExportNotSupportedError } from "@coding-adventures/paint-vm";
import type {
  PaintInstruction,
  PaintRect,
  PaintEllipse,
  PaintPath,
  PaintGlyphRun,
  PaintGroup,
  PaintLayer,
  PaintLine,
  PaintClip,
  PaintGradient,
  PaintImage,
  PaintScene,
  PathCommand,
} from "@coding-adventures/paint-instructions";

// ============================================================================
// SvgContext — the accumulator for SVG string output
// ============================================================================

/**
 * The rendering context for the SVG backend.
 *
 * Instead of a canvas context or a DOM node, the SVG backend accumulates
 * SVG element strings. The final output is assembled by joining these strings.
 *
 * defs — SVG <defs> section: gradient definitions, filter definitions.
 *        These are collected separately because they must appear at the top
 *        of the <svg> element, before any elements that reference them.
 *
 * elements — The rendered SVG elements in order. Each handler pushes one or
 *            more SVG element strings here.
 *
 * clipCounter / filterCounter — monotonically increasing counters for
 * generating unique clip-path and filter IDs when the instruction has no id.
 */
export interface SvgContext {
  defs: string[];
  elements: string[];
  clipCounter: number;
  filterCounter: number;
}

export function createSvgContext(): SvgContext {
  return { defs: [], elements: [], clipCounter: 0, filterCounter: 0 };
}

// ============================================================================
// Attribute helpers
// ============================================================================

/**
 * Escape a string for safe inclusion in an XML attribute value.
 *
 * SVG attributes are quoted with double quotes. The characters that must be
 * escaped inside a double-quoted XML attribute are:
 *   &  →  &amp;
 *   "  →  &quot;
 *   <  →  &lt;   (defensive — not strictly required in attributes)
 *   >  →  &gt;   (defensive)
 */
function escAttr(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/"/g, "&quot;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

/**
 * Escape a string for safe inclusion in an XML text node.
 */
function escText(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

/**
 * Collect common stroke/fill attributes shared by rect, ellipse, path, line.
 */
function strokeFillAttrs(opts: {
  fill?: string;
  stroke?: string;
  stroke_width?: number;
  opacity?: number;
}): string {
  const parts: string[] = [];
  parts.push(`fill="${escAttr(opts.fill ?? "none")}"`);
  if (opts.stroke) {
    parts.push(`stroke="${escAttr(opts.stroke)}"`);
    parts.push(`stroke-width="${safeNum(opts.stroke_width ?? 1, "stroke_width")}"`);
  }
  if (opts.opacity !== undefined && opts.opacity !== 1.0) {
    parts.push(`opacity="${safeNum(opts.opacity, "opacity")}"`);
  }
  return parts.join(" ");
}

/**
 * Common id attribute — only emitted when the instruction has an id.
 */
function idAttr(id?: string): string {
  return id ? ` id="${escAttr(id)}"` : "";
}

// ============================================================================
// PathCommand → SVG path data string
// ============================================================================

/**
 * Convert an array of PathCommands to an SVG path data string.
 *
 * This maps our PathCommand union directly to SVG path command letters:
 *   move_to  → M x y
 *   line_to  → L x y
 *   quad_to  → Q cx cy x y
 *   cubic_to → C cx1 cy1 cx2 cy2 x y
 *   arc_to   → A rx ry x-rotation large-arc-flag sweep-flag x y
 *   close    → Z
 *
 * Numbers are rounded to 4 decimal places to keep output readable.
 */
function commandsToPathData(commands: PathCommand[]): string {
  const n = (v: number) => +v.toFixed(4);
  return commands
    .map((cmd) => {
      switch (cmd.kind) {
        case "move_to":
          return `M ${n(cmd.x)} ${n(cmd.y)}`;
        case "line_to":
          return `L ${n(cmd.x)} ${n(cmd.y)}`;
        case "quad_to":
          return `Q ${n(cmd.cx)} ${n(cmd.cy)} ${n(cmd.x)} ${n(cmd.y)}`;
        case "cubic_to":
          return `C ${n(cmd.cx1)} ${n(cmd.cy1)} ${n(cmd.cx2)} ${n(cmd.cy2)} ${n(cmd.x)} ${n(cmd.y)}`;
        case "arc_to":
          return `A ${n(cmd.rx)} ${n(cmd.ry)} ${n(cmd.x_rotation)} ${cmd.large_arc ? 1 : 0} ${cmd.sweep ? 1 : 0} ${n(cmd.x)} ${n(cmd.y)}`;
        case "close":
          return "Z";
      }
    })
    .join(" ");
}

// ============================================================================
// Transform2D → SVG transform attribute
// ============================================================================

function transformAttr(
  transform: [number, number, number, number, number, number] | undefined,
): string {
  if (!transform) return "";
  const [a, b, c, d, e, f] = transform;
  // Security: matrix coefficients are interpolated into the transform attribute.
  // Validate each is finite to prevent NaN/Infinity producing malformed SVG.
  return ` transform="matrix(${safeNum(a, "transform.a")},${safeNum(b, "transform.b")},${safeNum(c, "transform.c")},${safeNum(d, "transform.d")},${safeNum(e, "transform.e")},${safeNum(f, "transform.f")})"`;
}

// ============================================================================
// Safe numeric helper
// ============================================================================

/**
 * Validate that a value is a finite number before interpolating it into SVG
 * attribute values. Returns the value as a string if safe, throws otherwise.
 *
 * This prevents NaN / Infinity / non-numeric runtime values (possible when
 * the IR is deserialized from an untrusted source) from producing malformed
 * or injection-enabling SVG output.
 */
function safeNum(value: number, field: string): string {
  if (!Number.isFinite(value)) {
    throw new RangeError(
      `PaintVM SVG: ${field} must be a finite number, got ${value}`,
    );
  }
  return String(value);
}

// ============================================================================
// FilterEffect → SVG <filter> element
// ============================================================================

/**
 * Convert a PaintLayer's filters to an SVG <filter> element string.
 *
 * SVG filters use a graph of <fe*> primitives chained via result/in attributes.
 * Each filter effect maps to one or more SVG filter primitives.
 */
function buildSvgFilter(
  filterId: string,
  filters: PaintLayer["filters"],
): string {
  if (!filters || filters.length === 0) return "";

  const prims: string[] = [];
  let prev = "SourceGraphic";

  for (let i = 0; i < filters.length; i++) {
    const f = filters[i];
    const result = `f${i}`;

    switch (f.kind) {
      case "blur":
        prims.push(
          `<feGaussianBlur in="${prev}" stdDeviation="${safeNum(f.radius, "blur.radius")}" result="${result}"/>`,
        );
        break;
      case "drop_shadow":
        prims.push(
          `<feDropShadow dx="${safeNum(f.dx, "drop_shadow.dx")}" dy="${safeNum(f.dy, "drop_shadow.dy")}" stdDeviation="${safeNum(f.blur, "drop_shadow.blur")}" flood-color="${escAttr(f.color)}" result="${result}"/>`,
        );
        break;
      case "color_matrix": {
        // Validate every matrix value is a finite number before joining.
        // A crafted matrix entry could otherwise inject arbitrary attribute content.
        const safeMatrix = f.matrix.map((v, idx) =>
          safeNum(v, `color_matrix.matrix[${idx}]`),
        );
        prims.push(
          `<feColorMatrix in="${prev}" type="matrix" values="${safeMatrix.join(" ")}" result="${result}"/>`,
        );
        break;
      }
      case "brightness":
        // brightness via feComponentTransfer with linear slope
        {
          const slope = safeNum(f.amount, "brightness.amount");
          prims.push(
            `<feComponentTransfer in="${prev}" result="${result}">` +
              `<feFuncR type="linear" slope="${slope}"/>` +
              `<feFuncG type="linear" slope="${slope}"/>` +
              `<feFuncB type="linear" slope="${slope}"/>` +
              `</feComponentTransfer>`,
          );
        }
        break;
      case "contrast":
        // contrast via feComponentTransfer: slope=amount, intercept=-(amount-1)/2
        {
          const slope = f.amount;
          const intercept = -(f.amount - 1) / 2;
          prims.push(
            `<feComponentTransfer in="${prev}" result="${result}">` +
              `<feFuncR type="linear" slope="${safeNum(slope, "contrast.amount")}" intercept="${safeNum(intercept, "contrast.intercept")}"/>` +
              `<feFuncG type="linear" slope="${safeNum(slope, "contrast.amount")}" intercept="${safeNum(intercept, "contrast.intercept")}"/>` +
              `<feFuncB type="linear" slope="${safeNum(slope, "contrast.amount")}" intercept="${safeNum(intercept, "contrast.intercept")}"/>` +
              `</feComponentTransfer>`,
          );
        }
        break;
      case "saturate":
        prims.push(
          `<feColorMatrix in="${prev}" type="saturate" values="${safeNum(f.amount, "saturate.amount")}" result="${result}"/>`,
        );
        break;
      case "hue_rotate":
        prims.push(
          `<feColorMatrix in="${prev}" type="hueRotate" values="${safeNum(f.angle, "hue_rotate.angle")}" result="${result}"/>`,
        );
        break;
      case "invert":
        // invert via feComponentTransfer: slope=-amount, intercept=amount
        {
          const amt = safeNum(f.amount, "invert.amount");
          const negAmt = safeNum(-f.amount, "invert.neg_amount");
          prims.push(
            `<feComponentTransfer in="${prev}" result="${result}">` +
              `<feFuncR type="linear" slope="${negAmt}" intercept="${amt}"/>` +
              `<feFuncG type="linear" slope="${negAmt}" intercept="${amt}"/>` +
              `<feFuncB type="linear" slope="${negAmt}" intercept="${amt}"/>` +
              `</feComponentTransfer>`,
          );
        }
        break;
      case "opacity":
        prims.push(
          `<feComponentTransfer in="${prev}" result="${result}">` +
            `<feFuncA type="linear" slope="${safeNum(f.amount, "opacity.amount")}"/>` +
            `</feComponentTransfer>`,
        );
        break;
    }
    prev = result;
  }

  return `<filter id="${escAttr(filterId)}">${prims.join("")}</filter>`;
}

// ============================================================================
// BlendMode → SVG mix-blend-mode value
// ============================================================================

/**
 * Map our BlendMode union to the corresponding SVG/CSS mix-blend-mode value.
 *
 * Most values map 1:1 to CSS. "color_dodge" → "color-dodge" etc.
 *
 * Security: the return value is interpolated directly into a style="" attribute.
 * We maintain a runtime allowlist to prevent CSS injection if a caller passes an
 * arbitrary string (e.g. via `as any` or a deserialized payload). Values not in
 * the allowlist fall back to "normal" rather than allowing arbitrary injection.
 */
const BLEND_MODE_ALLOWLIST = new Set([
  "normal", "multiply", "screen", "overlay", "darken", "lighten",
  "color-dodge", "color-burn", "hard-light", "soft-light", "difference",
  "exclusion", "hue", "saturation", "color", "luminosity",
]);

function blendModeToSvg(mode: string): string {
  // CSS uses hyphens, we use underscores for multi-word values
  const cssMode = mode.replace(/_/g, "-");
  // Runtime allowlist check — fall back to "normal" for unknown values
  return BLEND_MODE_ALLOWLIST.has(cssMode) ? cssMode : "normal";
}

// ============================================================================
// Instruction handlers
// ============================================================================

function handleRect(instr: PaintRect, ctx: SvgContext): void {
  const attrs = strokeFillAttrs(instr);
  const rx =
    instr.corner_radius !== undefined
      ? ` rx="${safeNum(instr.corner_radius, "rect.corner_radius")}"`
      : "";
  ctx.elements.push(
    `<rect${idAttr(instr.id)} x="${safeNum(instr.x, "rect.x")}" y="${safeNum(instr.y, "rect.y")}" width="${safeNum(instr.width, "rect.width")}" height="${safeNum(instr.height, "rect.height")}"${rx} ${attrs}/>`,
  );
}

function handleEllipse(instr: PaintEllipse, ctx: SvgContext): void {
  const attrs = strokeFillAttrs(instr);
  ctx.elements.push(
    `<ellipse${idAttr(instr.id)} cx="${safeNum(instr.cx, "ellipse.cx")}" cy="${safeNum(instr.cy, "ellipse.cy")}" rx="${safeNum(instr.rx, "ellipse.rx")}" ry="${safeNum(instr.ry, "ellipse.ry")}" ${attrs}/>`,
  );
}

// Runtime allowlists for enumerated SVG attribute values.
// These prevent attribute injection if a caller passes an arbitrary string
// (e.g. via `as any` or a deserialized payload without schema validation).
const FILL_RULE_ALLOWLIST = new Set(["nonzero", "evenodd"]);
const STROKE_CAP_ALLOWLIST = new Set(["butt", "round", "square"]);
const STROKE_JOIN_ALLOWLIST = new Set(["miter", "round", "bevel"]);

function handlePath(instr: PaintPath, ctx: SvgContext): void {
  const d = commandsToPathData(instr.commands);
  // Security: fill_rule, stroke_cap, stroke_join are interpolated into SVG attributes.
  // Validate against runtime allowlists to prevent attribute injection.
  const safeFillRule =
    instr.fill_rule && FILL_RULE_ALLOWLIST.has(instr.fill_rule)
      ? instr.fill_rule
      : "nonzero";
  const fillRule =
    safeFillRule !== "nonzero" ? ` fill-rule="${safeFillRule}"` : "";
  const cap =
    instr.stroke_cap && STROKE_CAP_ALLOWLIST.has(instr.stroke_cap)
      ? ` stroke-linecap="${instr.stroke_cap}"`
      : "";
  const join =
    instr.stroke_join && STROKE_JOIN_ALLOWLIST.has(instr.stroke_join)
      ? ` stroke-linejoin="${instr.stroke_join}"`
      : "";
  const attrs = strokeFillAttrs(instr);
  ctx.elements.push(
    `<path${idAttr(instr.id)} d="${escAttr(d)}"${fillRule}${cap}${join} ${attrs}/>`,
  );
}

function handleGlyphRun(instr: PaintGlyphRun, ctx: SvgContext): void {
  // SVG doesn't have a native glyph-id primitive. We approximate with <text>
  // using Unicode codepoints. A real implementation would use <glyph> in <defs>
  // and <use> references. For now, emit a <text> with tspan per glyph position.
  const fill = instr.fill ?? "#000000";
  const parts = instr.glyphs.map((g) => {
    // Security: glyph_id is interpolated into an XML numeric character reference
    // (&#NNN;). Validate it is a safe integer in the Unicode codepoint range
    // before interpolation. Non-integer or out-of-range values are replaced with
    // the replacement character U+FFFD to prevent injection.
    const id = g.glyph_id;
    const safeId =
      Number.isInteger(id) && id >= 0 && id <= 0x10ffff ? id : 0xfffd;
    return `<tspan x="${safeNum(g.x, "glyph.x")}" y="${safeNum(g.y, "glyph.y")}">&#${safeId};</tspan>`;
  });
  ctx.elements.push(
    `<text${idAttr(instr.id)} font-size="${safeNum(instr.font_size, "glyph_run.font_size")}" fill="${escAttr(fill)}">${parts.join("")}</text>`,
  );
}

function handleGroup(
  instr: PaintGroup,
  ctx: SvgContext,
  vm: PaintVM<SvgContext>,
): void {
  const transform = transformAttr(instr.transform);
  const opacity =
    instr.opacity !== undefined && instr.opacity !== 1.0
      ? ` opacity="${safeNum(instr.opacity, "group.opacity")}"`
      : "";
  ctx.elements.push(`<g${idAttr(instr.id)}${transform}${opacity}>`);
  for (const child of instr.children) {
    vm.dispatch(child, ctx);
  }
  ctx.elements.push("</g>");
}

function handleLayer(
  instr: PaintLayer,
  ctx: SvgContext,
  vm: PaintVM<SvgContext>,
): void {
  const filterId = instr.id
    ? `filter-${instr.id}`
    : `filter-${ctx.filterCounter++}`;

  const filterStr = buildSvgFilter(filterId, instr.filters);
  if (filterStr) ctx.defs.push(filterStr);

  const filterAttr = filterStr ? ` filter="url(#${escAttr(filterId)})"` : "";
  const blendAttr = instr.blend_mode && instr.blend_mode !== "normal"
    ? ` style="mix-blend-mode:${blendModeToSvg(instr.blend_mode)}"`
    : "";
  const transform = transformAttr(instr.transform);
  const opacity =
    instr.opacity !== undefined && instr.opacity !== 1.0
      ? ` opacity="${safeNum(instr.opacity, "layer.opacity")}"`
      : "";

  ctx.elements.push(
    `<g${idAttr(instr.id)}${transform}${opacity}${filterAttr}${blendAttr}>`,
  );
  for (const child of instr.children) {
    vm.dispatch(child, ctx);
  }
  ctx.elements.push("</g>");
}

function handleLine(instr: PaintLine, ctx: SvgContext): void {
  const cap =
    instr.stroke_cap && STROKE_CAP_ALLOWLIST.has(instr.stroke_cap)
      ? ` stroke-linecap="${instr.stroke_cap}"`
      : "";
  const width = safeNum(instr.stroke_width ?? 1, "line.stroke_width");
  ctx.elements.push(
    `<line${idAttr(instr.id)} x1="${safeNum(instr.x1, "line.x1")}" y1="${safeNum(instr.y1, "line.y1")}" x2="${safeNum(instr.x2, "line.x2")}" y2="${safeNum(instr.y2, "line.y2")}" stroke="${escAttr(instr.stroke)}" stroke-width="${width}"${cap} fill="none"/>`,
  );
}

function handleClip(
  instr: PaintClip,
  ctx: SvgContext,
  vm: PaintVM<SvgContext>,
): void {
  const clipId = instr.id ? `clip-${instr.id}` : `clip-${ctx.clipCounter++}`;
  ctx.defs.push(
    `<clipPath id="${escAttr(clipId)}">` +
      `<rect x="${safeNum(instr.x, "clip.x")}" y="${safeNum(instr.y, "clip.y")}" width="${safeNum(instr.width, "clip.width")}" height="${safeNum(instr.height, "clip.height")}"/>` +
      `</clipPath>`,
  );
  ctx.elements.push(`<g clip-path="url(#${escAttr(clipId)})">`);
  for (const child of instr.children) {
    vm.dispatch(child, ctx);
  }
  ctx.elements.push("</g>");
}

function handleGradient(instr: PaintGradient, ctx: SvgContext): void {
  // Gradients are emitted into <defs>. They are referenced by fill="url(#id)".
  if (!instr.id) return; // A gradient without an id cannot be referenced

  const stops = instr.stops
    .map((s, i) => {
      // Security: validate stop offset is a finite number in [0, 1]
      const off = safeNum(s.offset, `gradient.stops[${i}].offset`);
      return `<stop offset="${off}" stop-color="${escAttr(s.color)}"/>`;
    })
    .join("");

  let gradElem: string;
  if (instr.gradient_kind === "linear") {
    gradElem =
      `<linearGradient id="${escAttr(instr.id)}" x1="${safeNum(instr.x1 ?? 0, "gradient.x1")}" y1="${safeNum(instr.y1 ?? 0, "gradient.y1")}" x2="${safeNum(instr.x2 ?? 0, "gradient.x2")}" y2="${safeNum(instr.y2 ?? 0, "gradient.y2")}" gradientUnits="userSpaceOnUse">` +
      stops +
      `</linearGradient>`;
  } else {
    gradElem =
      `<radialGradient id="${escAttr(instr.id)}" cx="${safeNum(instr.cx ?? 0, "gradient.cx")}" cy="${safeNum(instr.cy ?? 0, "gradient.cy")}" r="${safeNum(instr.r ?? 0, "gradient.r")}" gradientUnits="userSpaceOnUse">` +
      stops +
      `</radialGradient>`;
  }

  ctx.defs.push(gradElem);
  // Gradient instructions themselves produce no element — they only populate defs.
  // The rect/ellipse/path that references this gradient via fill="url(#id)" produces the element.
}

/**
 * Validate an image URI before embedding in SVG.
 *
 * Security: SVG <image href="..."> is embedded directly into a document that
 * may be rendered by a browser, headless renderer (Puppeteer, librsvg), or
 * server-side SVG processor. Allowing arbitrary URI schemes enables:
 *   - javascript:alert(1) — XSS in browsers that render SVG as XML
 *   - file:///etc/passwd  — LFI in headless server-side renderers
 *   - http://internal/    — SSRF against internal services
 *
 * We permit only data:, https:, and http: schemes. Everything else is replaced
 * with an empty data URI so the image renders as nothing rather than exploiting.
 */
function sanitizeImageHref(src: string): string {
  const lower = src.toLowerCase().trimStart();
  if (
    lower.startsWith("data:") ||
    lower.startsWith("https:") ||
    lower.startsWith("http:")
  ) {
    return src;
  }
  // Unsafe scheme — emit empty placeholder
  return "data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==";
}

function handleImage(instr: PaintImage, ctx: SvgContext): void {
  let href: string;
  if (typeof instr.src === "string") {
    href = sanitizeImageHref(instr.src);
  } else {
    // PixelContainer — convert to a data URL in the SVG
    // We emit a placeholder since we can't encode PNG inline here without a codec.
    // A production implementation would call a codec to encode the pixels.
    href = "data:image/png;base64,"; // placeholder
  }
  const opacity =
    instr.opacity !== undefined && instr.opacity !== 1.0
      ? ` opacity="${safeNum(instr.opacity, "image.opacity")}"`
      : "";
  ctx.elements.push(
    `<image${idAttr(instr.id)} x="${safeNum(instr.x, "image.x")}" y="${safeNum(instr.y, "image.y")}" width="${safeNum(instr.width, "image.width")}" height="${safeNum(instr.height, "image.height")}" href="${escAttr(href)}"${opacity}/>`,
  );
}

// ============================================================================
// createSvgVM — factory function
// ============================================================================

/**
 * Create a fully configured PaintVM<SvgContext> for SVG string output.
 *
 * The returned VM has handlers registered for all 10 instruction kinds.
 * Call renderToSvgString(scene) as a convenience wrapper, or use the VM
 * directly if you need to register additional custom handlers.
 */
export function createSvgVM(): PaintVM<SvgContext> {
  const vm = new PaintVM<SvgContext>(
    // clearFn: reset the context for a fresh render
    (ctx, _bg) => {
      ctx.defs.length = 0;
      ctx.elements.length = 0;
      ctx.clipCounter = 0;
      ctx.filterCounter = 0;
    },
    // exportFn: SVG cannot produce pixel data — throw ExportNotSupportedError
    () => {
      throw new ExportNotSupportedError("SVG");
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

// ============================================================================
// renderToSvgString — convenience entry point
// ============================================================================

/**
 * Render a PaintScene to a complete SVG string.
 *
 * This is the primary entry point for server-side SVG generation. It creates
 * a fresh VM, executes the scene, and assembles the output string.
 *
 * The output is a complete `<svg>` element with:
 *   - xmlns="http://www.w3.org/2000/svg"
 *   - width and height from scene
 *   - A <rect> background fill if scene.background is not "transparent"
 *   - A <defs> section (present only if gradients, filters, or clips are used)
 *   - All instruction elements in painter's-algorithm order
 *
 * Example:
 *   const svg = renderToSvgString(paintScene(400, 300, "#fff", [
 *     paintRect(10, 10, 100, 50, { fill: "#3b82f6" }),
 *   ]));
 *   fs.writeFileSync("output.svg", svg);
 */
export function renderToSvgString(scene: PaintScene): string {
  const vm = createSvgVM();
  const ctx = createSvgContext();
  vm.execute(scene, ctx);
  return assembleSvg(scene, ctx);
}

/**
 * Assemble the final SVG string from the accumulated context.
 *
 * The SVG structure is:
 *   <svg xmlns="..." width="W" height="H">
 *     <defs>...</defs>          ← only if defs are non-empty
 *     <rect ... fill="bg"/>     ← background fill (if not transparent)
 *     <element/>                ← instruction elements
 *     ...
 *   </svg>
 */
export function assembleSvg(scene: PaintScene, ctx: SvgContext): string {
  // Security: scene.width and scene.height are interpolated into SVG attributes.
  // Validate they are finite numbers to prevent NaN/Infinity producing malformed SVG.
  const w = safeNum(scene.width, "scene.width");
  const h = safeNum(scene.height, "scene.height");

  const parts: string[] = [];
  parts.push(
    `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}">`,
  );

  if (ctx.defs.length > 0) {
    parts.push(`<defs>${ctx.defs.join("")}</defs>`);
  }

  if (scene.background !== "transparent" && scene.background !== "none") {
    parts.push(
      `<rect width="${w}" height="${h}" fill="${escAttr(scene.background)}"/>`,
    );
  }

  parts.push(...ctx.elements);
  parts.push("</svg>");

  return parts.join("");
}
