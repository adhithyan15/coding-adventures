/**
 * cowsay-visualizer
 *
 * A browser app that takes a user message, formats it as cowsay ASCII art,
 * and renders it through the PaintVM abstraction using either:
 *
 *   – Canvas (default, primary)  → PaintVM<CanvasRenderingContext2D>
 *   – SVG                        → renderToSvgString() → innerHTML
 *
 * The key insight: the scene builder (buildCowsayScene) is completely backend-
 * agnostic. It emits PaintText and PaintRect instructions using the
 * `canvas:` font_ref scheme. Both the canvas backend and the SVG backend can
 * consume this scheme — the SVG backend parses the same grammar to extract
 * font-family, font-weight, and font-style for SVG presentation attributes.
 *
 * Pipeline:
 *
 *   user message string
 *     ↓  cowsayLines(message, wrapWidth)
 *   string[]  (ASCII art lines with speech bubble and cow)
 *     ↓  buildCowsayScene(lines)
 *   PaintScene  (PaintRect background + PaintText per line)
 *     ↓  renderWithCanvas(scene, container)   OR
 *     ↓  renderWithSvg(scene, container)
 *   pixels on <canvas>  OR  SVG string injected into <div>
 */

import {
  paintScene,
  paintRect,
  paintText,
  type PaintScene,
  type PaintInstruction,
} from "@coding-adventures/paint-instructions";
import { createCanvasVM } from "@coding-adventures/paint-vm-canvas";
import { renderToSvgString } from "@coding-adventures/paint-vm-svg";

// ============================================================================
// Cowsay layout logic
// ============================================================================

/**
 * Word-wrap a message to at most `maxWidth` characters per line.
 *
 * Splitting on whitespace and greedy-fitting words preserves natural reading
 * flow even when the user pastes a paragraph into the textarea.
 */
function wrapMessage(message: string, maxWidth: number): string[] {
  const words = message.trim().replace(/\s+/g, " ").split(" ").filter(Boolean);
  if (!words.length) return ["..."];

  const lines: string[] = [];
  let line = "";

  for (const word of words) {
    if (!line) {
      line = word;
    } else if (line.length + 1 + word.length <= maxWidth) {
      line += " " + word;
    } else {
      lines.push(line);
      line = word;
    }
  }
  if (line) lines.push(line);
  return lines;
}

/**
 * Build the full cowsay ASCII art as an array of strings, one per display line.
 *
 * Speech bubble borders adapt to whether the bubble contains 1 line or multiple:
 *
 *   Single line:      < message >
 *   Multiple lines:   / first line \
 *                     | middle     |
 *                     \ last line  /
 *
 * The cow body uses standard cowsay characters:
 *
 *          \   ^__^
 *           \  (oo)\_______
 *              (__)\       )\/\
 *                  ||----w |
 *                  ||     ||
 */
export function cowsayLines(message: string, wrapWidth = 40): string[] {
  const wrapped = wrapMessage(message, wrapWidth);
  const maxLen = Math.max(...wrapped.map((l) => l.length));

  const lines: string[] = [];

  // Top border: space + N underscores spanning the text + quotes
  lines.push(" " + "_".repeat(maxLen + 2));

  // Bubble body
  if (wrapped.length === 1) {
    lines.push(`< ${wrapped[0].padEnd(maxLen)} >`);
  } else {
    for (let i = 0; i < wrapped.length; i++) {
      const padded = wrapped[i].padEnd(maxLen);
      if (i === 0) {
        lines.push(`/ ${padded} \\`);
      } else if (i === wrapped.length - 1) {
        lines.push(`\\ ${padded} /`);
      } else {
        lines.push(`| ${padded} |`);
      }
    }
  }

  // Bottom border
  lines.push(" " + "-".repeat(maxLen + 2));

  // Cow body — backslash tail hangs off the bottom-right of the bubble border
  lines.push("        \\   ^__^");
  lines.push("         \\  (oo)\\_______");
  lines.push("            (__)\\       )\\/\\");
  lines.push("                ||----w |");
  lines.push("                ||     ||");

  return lines;
}

// ============================================================================
// Scene builder — backend-agnostic PaintScene from cowsay lines
// ============================================================================

/**
 * Monospace font used for both canvas and SVG rendering.
 *
 * "Courier New" renders identically across browsers and is the canonical
 * choice for terminal art. The `canvas:` scheme prefix is understood by both
 * paint-vm-canvas (ctx.font shorthand) and the new paint-vm-svg text handler
 * (parsed into font-family / font-size / font-weight attributes).
 */
const FONT_FAMILY = "Courier New";
const FONT_SIZE = 15; // px

/**
 * Line height multiplier. 1.5× gives comfortable vertical spacing without
 * running lines together for monospace text.
 */
const LINE_HEIGHT = FONT_SIZE * 1.5;

/**
 * Courier New character width ratio ≈ 0.601 (width:height).
 *
 * This is a well-known property of fixed-pitch Courier New: each character
 * cell is ≈60% as wide as it is tall. At 15px that's ≈9px per column.
 * We use this to pre-compute scene width without needing ctx.measureText()
 * at build time. The canvas backend does its own shaping at fill time anyway.
 */
const CHAR_WIDTH = FONT_SIZE * 0.601;

/** Horizontal and vertical padding around the text block. */
const PAD_X = 20;
const PAD_Y = 20;

/** Foreground colour for the ASCII art text. */
const TEXT_COLOR = "#1e293b"; // slate-800

/**
 * Convert an array of cowsay lines into a PaintScene.
 *
 * The scene contains:
 *   1. A full-bleed background rect (light slate).
 *   2. One PaintText instruction per cowsay line, positioned on the baseline.
 *
 * No layout engine is needed — cowsay is monospace and pre-formatted, so we
 * only need to advance the y baseline by LINE_HEIGHT per line.
 */
export function buildCowsayScene(lines: string[]): PaintScene {
  const maxLen = Math.max(...lines.map((l) => l.length));
  const sceneWidth = Math.ceil(maxLen * CHAR_WIDTH) + PAD_X * 2;
  const sceneHeight = Math.ceil(lines.length * LINE_HEIGHT) + PAD_Y * 2;

  // font_ref: "canvas:<family>@<size>[:<weight>[:<style>]]"
  // Consumed by both paint-vm-canvas (CSS shorthand) and paint-vm-svg
  // (individual SVG presentation attributes via parseSvgFontRef).
  const fontRef = `canvas:${FONT_FAMILY}@${FONT_SIZE}`;

  const instructions: PaintInstruction[] = [
    // Background
    paintRect(0, 0, sceneWidth, sceneHeight, { fill: "#f8fafc" }),
  ];

  for (let i = 0; i < lines.length; i++) {
    const x = PAD_X;
    // y is the alphabetic baseline. We push down by one full LINE_HEIGHT from
    // the top pad for the first line, then advance LINE_HEIGHT per line.
    // Subtracting half the leading keeps the ascender above the top pad.
    const y = PAD_Y + (i + 1) * LINE_HEIGHT - (LINE_HEIGHT - FONT_SIZE) * 0.5;
    instructions.push(
      paintText(x, y, lines[i], fontRef, FONT_SIZE, TEXT_COLOR),
    );
  }

  return paintScene(sceneWidth, sceneHeight, "#ffffff", instructions);
}

// ============================================================================
// Backend renderers
// ============================================================================

/**
 * Render a PaintScene to a newly created <canvas> element inside `container`.
 *
 * This is the primary path. HiDPI handling: scale the canvas backing buffer
 * by devicePixelRatio so text appears crisp on Retina displays. The logical
 * CSS size (style.width/height) remains at scene dimensions so layout doesn't
 * shift.
 *
 * The canvas VM calls ctx.fillText() for each PaintText instruction, which
 * delegates to the browser's text-shaping stack — ideal for monospace cowsay
 * art where pixel-alignment between characters matters.
 */
export function renderWithCanvas(
  scene: PaintScene,
  container: HTMLElement,
): void {
  container.innerHTML = "";

  const dpr = window.devicePixelRatio || 1;
  const canvas = document.createElement("canvas");

  // Physical pixels (backing store)
  canvas.width = Math.round(scene.width * dpr);
  canvas.height = Math.round(scene.height * dpr);

  // CSS logical pixels
  canvas.style.width = `${scene.width}px`;
  canvas.style.height = `${scene.height}px`;

  const ctx = canvas.getContext("2d")!;

  // Scale the 2D context so all coordinates are in logical pixels (scene units)
  // while the backing store is at physical resolution.
  ctx.scale(dpr, dpr);

  const vm = createCanvasVM();
  vm.execute(scene, ctx);

  container.appendChild(canvas);
}

/**
 * Render a PaintScene to an inline SVG string and inject it into `container`.
 *
 * The SVG backend produces a pure string — no canvas context required. The
 * PaintText handler emits `<text font-family="..." font-size="..." fill="...">`
 * elements with x/y baseline coordinates matching the scene's layout.
 * text-anchor maps from PaintText.text_align via the SVG backend's
 * textAlignToSvgAnchor() helper.
 */
export function renderWithSvg(scene: PaintScene, container: HTMLElement): void {
  container.innerHTML = renderToSvgString(scene);
}

// ============================================================================
// DOM wiring
// ============================================================================

type Backend = "canvas" | "svg";

/** Read the currently selected backend from the radio group. */
function currentBackend(): Backend {
  const checked = document.querySelector<HTMLInputElement>(
    'input[name="backend"]:checked',
  );
  return (checked?.value ?? "canvas") as Backend;
}

/** Read the wrap-width field, clamped to a sensible range. */
function currentWrapWidth(): number {
  const input = document.getElementById("wrap-width") as HTMLInputElement;
  const v = parseInt(input.value, 10);
  return Number.isFinite(v) && v >= 10 ? Math.min(v, 120) : 40;
}

/** Re-build and re-render the scene from current UI state. */
function render(): void {
  const messageEl = document.getElementById("message") as HTMLTextAreaElement;
  const outputEl = document.getElementById("output")!;

  const message = messageEl.value.trim() || "Moo!";
  const backend = currentBackend();
  const wrapWidth = currentWrapWidth();

  const lines = cowsayLines(message, wrapWidth);
  const scene = buildCowsayScene(lines);

  if (backend === "canvas") {
    renderWithCanvas(scene, outputEl);
  } else {
    renderWithSvg(scene, outputEl);
  }
}

// Wire up live re-render on every input change.
document.getElementById("message")!.addEventListener("input", render);
document.getElementById("wrap-width")!.addEventListener("input", render);
document
  .querySelectorAll<HTMLInputElement>('input[name="backend"]')
  .forEach((radio) => radio.addEventListener("change", render));

// Initial render on page load (canvas backend is pre-selected in HTML).
render();
