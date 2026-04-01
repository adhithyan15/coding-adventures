/**
 * @coding-adventures/draw-instructions-canvas
 *
 * Canvas renderer for backend-neutral draw instructions.
 *
 * This package paints a DrawScene directly to a CanvasRenderingContext2D.
 * It skips the DOM entirely — no `document.createElement`, no `<svg>`,
 * no HTML parsing. The caller supplies the context; this package supplies
 * the painting logic.
 *
 * ## Why Canvas instead of SVG?
 *
 * SVG is declarative: the scene becomes a tree of XML nodes that the browser
 * lays out, re-paints, and tracks in memory. That tree has non-trivial overhead
 * and the browser can't start painting until the entire document is parsed.
 *
 * Canvas is imperative: drawing commands go straight to the GPU rasterizer.
 * There is no retained-mode tree. This is better for:
 * - Large scenes with thousands of elements (e.g., a 200-barcode PDF page)
 * - Off-screen rendering (OffscreenCanvas in web workers)
 * - Server-side rendering via node-canvas or Skia bindings
 *
 * ## Usage (browser)
 *
 * ```typescript
 * const canvas = document.getElementById("my-canvas") as HTMLCanvasElement;
 * const ctx = canvas.getContext("2d")!;
 * renderCanvas(scene, ctx);
 * ```
 *
 * ## Usage (OffscreenCanvas, web worker)
 *
 * ```typescript
 * const offscreen = new OffscreenCanvas(800, 400);
 * const ctx = offscreen.getContext("2d")!;
 * renderCanvas(scene, ctx);
 * const blob = await offscreen.convertToBlob({ type: "image/png" });
 * ```
 *
 * ## Usage (server-side, node-canvas)
 *
 * ```typescript
 * import { createCanvas } from "canvas"; // node-canvas
 * const canvas = createCanvas(800, 400);
 * const ctx = canvas.getContext("2d");
 * renderCanvas(scene, ctx);
 * ```
 *
 * ## Architecture
 *
 * The entry points are:
 * - `createCanvasRenderer(ctx)` — returns a `DrawRenderer<void>` bound to a
 *   given context. Useful when you want to pass the renderer as a value.
 * - `renderCanvas(scene, ctx)` — convenience wrapper for the common case.
 *
 * Internally, `renderInstructions` walks the draw-instruction tree recursively.
 * Groups are transparent (no transform, no state save). Clips use Canvas's
 * built-in `save / clip / restore` idiom.
 */
export const VERSION = "0.1.0";

import type {
  DrawClipInstruction,
  DrawGroupInstruction,
  DrawInstruction,
  DrawLineInstruction,
  DrawRectInstruction,
  DrawRenderer,
  DrawScene,
  DrawTextInstruction,
} from "@coding-adventures/draw-instructions";

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Create a DrawRenderer<void> that paints to the given canvas context.
 *
 * The renderer closes over `ctx` and uses it for every subsequent `render()`
 * call. If you need to render to multiple canvases, call `createCanvasRenderer`
 * once per context.
 *
 * Implements the `DrawRenderer<void>` interface from draw-instructions, so it
 * can be passed to `renderWith()` alongside SVG or text renderers.
 *
 * ```typescript
 * const renderer = createCanvasRenderer(ctx);
 * renderer.render(scene); // paints to ctx
 * ```
 */
export function createCanvasRenderer(
  ctx: CanvasRenderingContext2D,
): DrawRenderer<void> {
  return {
    render(scene: DrawScene): void {
      renderCanvas(scene, ctx);
    },
  };
}

/**
 * Paint a DrawScene directly to a CanvasRenderingContext2D.
 *
 * This is the main entry point for most users. Supply the scene you want to
 * draw and the 2D canvas context to draw into.
 *
 * The function does not clear the canvas before painting — it simply draws
 * the scene background on top of whatever is there. If you need a clean slate,
 * call `ctx.clearRect(0, 0, canvas.width, canvas.height)` first.
 *
 * ```typescript
 * renderCanvas(scene, ctx);
 * ```
 */
export function renderCanvas(
  scene: DrawScene,
  ctx: CanvasRenderingContext2D,
): void {
  // Paint the background rectangle first so it sits below all instructions.
  ctx.fillStyle = scene.background;
  ctx.fillRect(0, 0, scene.width, scene.height);

  // Walk and paint every instruction in order.
  for (const instruction of scene.instructions) {
    renderInstruction(instruction, ctx);
  }
}

// ---------------------------------------------------------------------------
// Instruction dispatch
// ---------------------------------------------------------------------------

/**
 * Route one DrawInstruction to its specific paint function.
 *
 * We use a `switch` on the `kind` discriminant rather than a class hierarchy
 * or a map of functions. This keeps the dispatch explicit and lets TypeScript
 * enforce exhaustiveness — if we add a new instruction kind to draw-instructions
 * but forget to handle it here, the compiler will warn us.
 */
function renderInstruction(
  instruction: DrawInstruction,
  ctx: CanvasRenderingContext2D,
): void {
  switch (instruction.kind) {
    case "rect":
      renderRect(instruction, ctx);
      break;
    case "text":
      renderText(instruction, ctx);
      break;
    case "line":
      renderLine(instruction, ctx);
      break;
    case "clip":
      renderClip(instruction, ctx);
      break;
    case "group":
      renderGroup(instruction, ctx);
      break;
  }
}

// ---------------------------------------------------------------------------
// Per-type paint functions
// ---------------------------------------------------------------------------

/**
 * Paint a filled (and optionally stroked) rectangle.
 *
 * Canvas's `fillRect()` fills without affecting any current path. Likewise,
 * `strokeRect()` strokes without affecting the path. We use those dedicated
 * helpers rather than `beginPath() + rect() + fill() + stroke()` to keep each
 * operation atomic.
 *
 * Stroke is optional. When `instruction.stroke` is undefined, no outline is
 * drawn — just the fill. When both are set, the fill is painted first so the
 * stroke sits on top (matching the SVG painter's-model convention).
 */
function renderRect(
  instruction: DrawRectInstruction,
  ctx: CanvasRenderingContext2D,
): void {
  ctx.fillStyle = instruction.fill;
  ctx.fillRect(instruction.x, instruction.y, instruction.width, instruction.height);

  if (instruction.stroke !== undefined) {
    ctx.strokeStyle = instruction.stroke;
    ctx.lineWidth = instruction.strokeWidth ?? 1;
    ctx.strokeRect(
      instruction.x,
      instruction.y,
      instruction.width,
      instruction.height,
    );
  }
}

/**
 * Paint a text label at (x, y).
 *
 * ### Coordinate convention
 *
 * In Canvas 2D, text coordinates refer to the *baseline*, not the top-left
 * corner. The DrawTextInstruction `y` follows the same convention: producers
 * place the y coordinate at the text baseline.
 *
 * ### Alignment mapping
 *
 * DrawTextInstruction uses `"start" | "middle" | "end"`. Canvas uses
 * `"start" | "center" | "end"` (note "center" not "middle"). We map
 * "middle" → "center" to bridge the difference.
 *
 * ### Font string
 *
 * Canvas's `font` property accepts a CSS font-shorthand string like
 * `"bold 16px monospace"`. We synthesize it from the instruction's fields:
 * - fontWeight defaults to "normal" when undefined
 * - fontSize is in pixels (same units as Canvas)
 * - fontFamily is passed through as-is
 */
function renderText(
  instruction: DrawTextInstruction,
  ctx: CanvasRenderingContext2D,
): void {
  const weight = instruction.fontWeight ?? "normal";
  // Canvas font property: "<weight> <size>px <family>"
  ctx.font = `${weight} ${instruction.fontSize}px ${instruction.fontFamily}`;
  ctx.fillStyle = instruction.fill;

  // DrawTextInstruction uses "middle"; Canvas uses "center".
  // "start" and "end" are identical in both APIs.
  const align: CanvasTextAlign =
    instruction.align === "middle" ? "center" : instruction.align;
  ctx.textAlign = align;

  ctx.fillText(instruction.value, instruction.x, instruction.y);
}

/**
 * Paint a straight line segment.
 *
 * Canvas lines are always part of a path. The sequence is:
 * 1. `beginPath()` — discard any existing path so we start fresh
 * 2. `moveTo(x1, y1)` — lift the pen and place it at the start point
 * 3. `lineTo(x2, y2)` — draw the path segment (not yet visible)
 * 4. `stroke()` — render the path with the current stroke style
 *
 * We set `lineWidth` before `stroke()`. Setting it after has no effect.
 *
 * Lines are always stroked (never filled) — a line has no interior area.
 */
function renderLine(
  instruction: DrawLineInstruction,
  ctx: CanvasRenderingContext2D,
): void {
  ctx.strokeStyle = instruction.stroke;
  ctx.lineWidth = instruction.strokeWidth;
  ctx.beginPath();
  ctx.moveTo(instruction.x1, instruction.y1);
  ctx.lineTo(instruction.x2, instruction.y2);
  ctx.stroke();
}

/**
 * Paint a group of instructions.
 *
 * Groups are transparent containers. They carry no Canvas state change —
 * no `save()`, no transform, no opacity. Their only purpose is to let
 * producers attach semantic metadata (like `{ layer: "bars" }`) without
 * introducing visual side-effects.
 *
 * If you need transforms or opacity, add a `DrawTransformInstruction` later.
 * The architecture supports it cleanly.
 */
function renderGroup(
  instruction: DrawGroupInstruction,
  ctx: CanvasRenderingContext2D,
): void {
  for (const child of instruction.children) {
    renderInstruction(child, ctx);
  }
}

/**
 * Paint a clipping region.
 *
 * Canvas clipping uses the state stack:
 * 1. `save()` — push all canvas state onto the stack
 * 2. `beginPath()` — start a fresh path
 * 3. `rect(x, y, w, h)` — add the clip rectangle to the path
 * 4. `clip()` — make the current path the active clip region
 * 5. ... paint children (only content inside the rect is visible) ...
 * 6. `restore()` — pop the stack, removing the clip region
 *
 * The `save/restore` pair is critical. Without it, the clip region
 * would accumulate across all subsequent instructions, not just the children
 * of this clip. Each nested clip instruction intersects with any outer clip
 * that is still active at restore time — exactly the behavior the spec
 * describes.
 *
 * Why `beginPath()` before `rect()`?
 * Without `beginPath()`, any previous open path segments would be included
 * in the clip shape, producing an incorrect (and surprising) clip region.
 */
function renderClip(
  instruction: DrawClipInstruction,
  ctx: CanvasRenderingContext2D,
): void {
  ctx.save();
  ctx.beginPath();
  ctx.rect(instruction.x, instruction.y, instruction.width, instruction.height);
  ctx.clip();

  for (const child of instruction.children) {
    renderInstruction(child, ctx);
  }

  ctx.restore();
}
