/**
 * @coding-adventures/draw-instructions-text
 *
 * ASCII/Unicode text renderer for the draw-instructions scene model.
 *
 * This renderer proves the draw-instructions abstraction is truly backend-
 * neutral: the same DrawScene that produces SVG or paints a Canvas can also
 * render as box-drawing characters in a terminal.
 *
 * === How It Works ===
 *
 * The renderer maps pixel-coordinate scenes to a fixed-width character grid.
 * Each cell in the grid is one character. The mapping uses a configurable
 * scale factor (default: 8px per char width, 16px per char height).
 *
 * ```
 * Scene coordinates (pixels)     Character grid
 * ┌─────────────────────┐        ┌──────────┐
 * │ rect at (0,0,200,32)│   →    │██████████│
 * │                     │        │██████████│
 * └─────────────────────┘        └──────────┘
 * ```
 *
 * === Character Palette ===
 *
 * Box-drawing characters create clean table grids:
 *
 * ```
 * ┌──────┬─────┐     Corners: ┌ ┐ └ ┘
 * │ Name │ Age │     Edges:   ─ │
 * ├──────┼─────┤     Tees:    ┬ ┴ ├ ┤
 * │ Alice│  30 │     Cross:   ┼
 * └──────┴─────┘     Fill:    █
 * ```
 *
 * === Intersection Logic ===
 *
 * When two drawing operations overlap at the same cell, the renderer
 * merges them into the correct junction character. A horizontal line
 * crossing a vertical line becomes ┼. A line meeting a box corner
 * becomes the appropriate tee (┬ ┴ ├ ┤).
 *
 * This is tracked via a "tag" buffer parallel to the character buffer.
 * Each cell records which directions have lines passing through it
 * (up, down, left, right), and the tag is resolved to the correct
 * box-drawing character on each write.
 *
 * === Usage ===
 *
 * ```typescript
 * import { renderText, TEXT_RENDERER } from "@coding-adventures/draw-instructions-text";
 * import { createScene, drawRect, drawLine, drawText } from "@coding-adventures/draw-instructions";
 *
 * const scene = createScene(160, 48, [
 *   drawRect(0, 0, 160, 48, "transparent", { stroke: "#000", strokeWidth: 1 }),
 *   drawLine(0, 16, 160, 16, "#000", 1),
 *   drawText(8, 12, "Hello"),
 * ]);
 *
 * console.log(renderText(scene));
 * // ┌──────────────────┐
 * // │Hello             │
 * // ├──────────────────┤
 * // │                  │
 * // └──────────────────┘
 * ```
 */

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

export const VERSION = "0.1.0";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface TextRendererOptions {
  /** Pixels per character column. Default: 8. */
  scaleX?: number;
  /** Pixels per character row. Default: 16. */
  scaleY?: number;
}

/**
 * Direction flags for tracking what passes through a cell.
 *
 * Each cell in the tag buffer stores a bitmask of directions. When
 * multiple drawing operations overlap, we OR the flags together and
 * resolve the combined tag to the correct box-drawing character.
 *
 * ```
 *        UP (1)
 *         │
 * LEFT(8)─┼─RIGHT(2)
 *         │
 *       DOWN(4)
 * ```
 */
const UP = 1;
const RIGHT = 2;
const DOWN = 4;
const LEFT = 8;
const FILL = 16;
const TEXT = 32;

interface ClipBounds {
  minCol: number;
  minRow: number;
  maxCol: number;
  maxRow: number;
}

// ---------------------------------------------------------------------------
// Box-drawing character resolution
//
// Given a bitmask of directions (UP | DOWN | LEFT | RIGHT), return the
// correct Unicode box-drawing character. This table covers all 16
// combinations of the 4 direction bits.
// ---------------------------------------------------------------------------

const BOX_CHARS: Record<number, string> = {
  [LEFT | RIGHT]:                "─",  // horizontal
  [UP | DOWN]:                   "│",  // vertical
  [DOWN | RIGHT]:                "┌",  // top-left corner
  [DOWN | LEFT]:                 "┐",  // top-right corner
  [UP | RIGHT]:                  "└",  // bottom-left corner
  [UP | LEFT]:                   "┘",  // bottom-right corner
  [LEFT | RIGHT | DOWN]:         "┬",  // top tee
  [LEFT | RIGHT | UP]:           "┴",  // bottom tee
  [UP | DOWN | RIGHT]:           "├",  // left tee
  [UP | DOWN | LEFT]:            "┤",  // right tee
  [UP | DOWN | LEFT | RIGHT]:    "┼",  // cross
  [RIGHT]:                       "─",  // half-lines default to full
  [LEFT]:                        "─",
  [UP]:                          "│",
  [DOWN]:                        "│",
};

/**
 * Resolves a direction bitmask to a box-drawing character.
 * Falls back to "+" if the combination isn't in our table (shouldn't happen).
 */
function resolveBoxChar(tag: number): string {
  if (tag & FILL) return "█";
  if (tag & TEXT) return ""; // text chars are stored directly, not via tags
  return BOX_CHARS[tag & (UP | DOWN | LEFT | RIGHT)] ?? "+";
}

// ---------------------------------------------------------------------------
// Buffer
// ---------------------------------------------------------------------------

/**
 * A 2D character buffer with a parallel tag buffer for intersection logic.
 *
 * The char buffer stores the actual character at each cell. The tag buffer
 * stores a bitmask of directions passing through each cell. When writing
 * a box-drawing character, we update the tag buffer and resolve the correct
 * character from the combined tag.
 */
class CharBuffer {
  readonly rows: number;
  readonly cols: number;
  private chars: string[][];
  private tags: number[][];

  constructor(rows: number, cols: number) {
    this.rows = rows;
    this.cols = cols;
    this.chars = Array.from({ length: rows }, () =>
      Array.from({ length: cols }, () => " "),
    );
    this.tags = Array.from({ length: rows }, () =>
      Array.from({ length: cols }, () => 0),
    );
  }

  /**
   * Writes a box-drawing element at (row, col) by adding direction flags.
   * The actual character is resolved from the combined tag.
   */
  writeTag(row: number, col: number, dirFlags: number, clip: ClipBounds): void {
    if (row < clip.minRow || row >= clip.maxRow) return;
    if (col < clip.minCol || col >= clip.maxCol) return;
    if (row < 0 || row >= this.rows || col < 0 || col >= this.cols) return;

    const existing = this.tags[row]![col]!;

    // Don't overwrite text with box-drawing
    if (existing & TEXT) return;

    const merged = existing | dirFlags;
    this.tags[row]![col] = merged;
    this.chars[row]![col] = dirFlags & FILL ? "█" : resolveBoxChar(merged);
  }

  /**
   * Writes a text character directly at (row, col).
   * Text overwrites any existing content.
   */
  writeChar(row: number, col: number, ch: string, clip: ClipBounds): void {
    if (row < clip.minRow || row >= clip.maxRow) return;
    if (col < clip.minCol || col >= clip.maxCol) return;
    if (row < 0 || row >= this.rows || col < 0 || col >= this.cols) return;

    this.chars[row]![col] = ch;
    this.tags[row]![col] = TEXT;
  }

  /** Joins all rows, trims trailing whitespace, and returns the result. */
  toString(): string {
    return this.chars
      .map((row) => row.join("").trimEnd())
      .join("\n")
      .trimEnd();
  }
}

// ---------------------------------------------------------------------------
// Coordinate mapping
// ---------------------------------------------------------------------------

function toCol(x: number, scaleX: number): number {
  return Math.round(x / scaleX);
}

function toRow(y: number, scaleY: number): number {
  return Math.round(y / scaleY);
}

// ---------------------------------------------------------------------------
// Instruction renderers
// ---------------------------------------------------------------------------

function renderRect(
  inst: DrawRectInstruction,
  buf: CharBuffer,
  sx: number,
  sy: number,
  clip: ClipBounds,
): void {
  const c1 = toCol(inst.x, sx);
  const r1 = toRow(inst.y, sy);
  const c2 = toCol(inst.x + inst.width, sx);
  const r2 = toRow(inst.y + inst.height, sy);

  const hasStroke = inst.stroke !== undefined && inst.stroke !== "";
  const hasFill = inst.fill !== "" && inst.fill !== "transparent" && inst.fill !== "none";

  if (hasStroke) {
    // Draw the box outline

    // Top-left corner
    buf.writeTag(r1, c1, DOWN | RIGHT, clip);
    // Top-right corner
    buf.writeTag(r1, c2, DOWN | LEFT, clip);
    // Bottom-left corner
    buf.writeTag(r2, c1, UP | RIGHT, clip);
    // Bottom-right corner
    buf.writeTag(r2, c2, UP | LEFT, clip);

    // Top edge
    for (let c = c1 + 1; c < c2; c++) {
      buf.writeTag(r1, c, LEFT | RIGHT, clip);
    }
    // Bottom edge
    for (let c = c1 + 1; c < c2; c++) {
      buf.writeTag(r2, c, LEFT | RIGHT, clip);
    }
    // Left edge
    for (let r = r1 + 1; r < r2; r++) {
      buf.writeTag(r, c1, UP | DOWN, clip);
    }
    // Right edge
    for (let r = r1 + 1; r < r2; r++) {
      buf.writeTag(r, c2, UP | DOWN, clip);
    }
  } else if (hasFill) {
    // Fill the interior with block characters
    for (let r = r1; r <= r2; r++) {
      for (let c = c1; c <= c2; c++) {
        buf.writeTag(r, c, FILL, clip);
      }
    }
  }
}

function renderLine(
  inst: DrawLineInstruction,
  buf: CharBuffer,
  sx: number,
  sy: number,
  clip: ClipBounds,
): void {
  const c1 = toCol(inst.x1, sx);
  const r1 = toRow(inst.y1, sy);
  const c2 = toCol(inst.x2, sx);
  const r2 = toRow(inst.y2, sy);

  if (r1 === r2) {
    // Horizontal line
    // At endpoints, only set the direction pointing inward so that
    // junctions with perpendicular elements resolve correctly.
    // E.g., a left endpoint should be RIGHT (pointing inward), not
    // LEFT|RIGHT, so it merges with a vertical edge as ├ not ┼.
    const minC = Math.min(c1, c2);
    const maxC = Math.max(c1, c2);
    for (let c = minC; c <= maxC; c++) {
      let flags = 0;
      if (c > minC) flags |= LEFT;
      if (c < maxC) flags |= RIGHT;
      if (c === minC && c === maxC) flags = LEFT | RIGHT; // single-cell line
      buf.writeTag(r1, c, flags, clip);
    }
  } else if (c1 === c2) {
    // Vertical line — same endpoint logic
    const minR = Math.min(r1, r2);
    const maxR = Math.max(r1, r2);
    for (let r = minR; r <= maxR; r++) {
      let flags = 0;
      if (r > minR) flags |= UP;
      if (r < maxR) flags |= DOWN;
      if (r === minR && r === maxR) flags = UP | DOWN; // single-cell line
      buf.writeTag(r, c1, flags, clip);
    }
  } else {
    // Diagonal — approximate with Bresenham's algorithm using ─ and │
    const dr = Math.abs(r2 - r1);
    const dc = Math.abs(c2 - c1);
    const sr = r1 < r2 ? 1 : -1;
    const sc = c1 < c2 ? 1 : -1;
    let err = dc - dr;
    let r = r1;
    let c = c1;

    while (true) {
      // Use the dominant direction's character
      buf.writeTag(r, c, dc > dr ? LEFT | RIGHT : UP | DOWN, clip);
      if (r === r2 && c === c2) break;
      const e2 = 2 * err;
      if (e2 > -dr) { err -= dr; c += sc; }
      if (e2 < dc) { err += dc; r += sr; }
    }
  }
}

function renderTextInst(
  inst: DrawTextInstruction,
  buf: CharBuffer,
  sx: number,
  sy: number,
  clip: ClipBounds,
): void {
  const row = toRow(inst.y, sy);
  const text = inst.value;

  let startCol: number;
  switch (inst.align) {
    case "middle":
      startCol = toCol(inst.x, sx) - Math.floor(text.length / 2);
      break;
    case "end":
      startCol = toCol(inst.x, sx) - text.length;
      break;
    default: // "start"
      startCol = toCol(inst.x, sx);
  }

  for (let i = 0; i < text.length; i++) {
    buf.writeChar(row, startCol + i, text[i]!, clip);
  }
}

function renderGroup(
  inst: DrawGroupInstruction,
  buf: CharBuffer,
  sx: number,
  sy: number,
  clip: ClipBounds,
): void {
  for (const child of inst.children) {
    renderInstruction(child, buf, sx, sy, clip);
  }
}

function renderClip(
  inst: DrawClipInstruction,
  buf: CharBuffer,
  sx: number,
  sy: number,
  parentClip: ClipBounds,
): void {
  // Intersect the new clip with the parent clip
  const newClip: ClipBounds = {
    minCol: Math.max(parentClip.minCol, toCol(inst.x, sx)),
    minRow: Math.max(parentClip.minRow, toRow(inst.y, sy)),
    maxCol: Math.min(parentClip.maxCol, toCol(inst.x + inst.width, sx)),
    maxRow: Math.min(parentClip.maxRow, toRow(inst.y + inst.height, sy)),
  };

  for (const child of inst.children) {
    renderInstruction(child, buf, sx, sy, newClip);
  }
}

function renderInstruction(
  inst: DrawInstruction,
  buf: CharBuffer,
  sx: number,
  sy: number,
  clip: ClipBounds,
): void {
  switch (inst.kind) {
    case "rect":
      renderRect(inst, buf, sx, sy, clip);
      break;
    case "line":
      renderLine(inst, buf, sx, sy, clip);
      break;
    case "text":
      renderTextInst(inst, buf, sx, sy, clip);
      break;
    case "group":
      renderGroup(inst, buf, sx, sy, clip);
      break;
    case "clip":
      renderClip(inst, buf, sx, sy, clip);
      break;
  }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Creates a text renderer with the given scale options.
 *
 * The returned object implements DrawRenderer<string> and can be used
 * with renderWith() from the draw-instructions package.
 */
export function createTextRenderer(
  options: TextRendererOptions = {},
): DrawRenderer<string> {
  const sx = options.scaleX ?? 8;
  const sy = options.scaleY ?? 16;

  return {
    render(scene: DrawScene): string {
      const cols = Math.ceil(scene.width / sx);
      const rows = Math.ceil(scene.height / sy);
      const buf = new CharBuffer(rows, cols);

      const fullClip: ClipBounds = {
        minCol: 0,
        minRow: 0,
        maxCol: cols,
        maxRow: rows,
      };

      for (const inst of scene.instructions) {
        renderInstruction(inst, buf, sx, sy, fullClip);
      }

      return buf.toString();
    },
  };
}

/** Default text renderer with standard scale (8px/char, 16px/row). */
export const TEXT_RENDERER: DrawRenderer<string> = createTextRenderer();

/** Convenience wrapper: scene in, text string out. */
export function renderText(
  scene: DrawScene,
  options: TextRendererOptions = {},
): string {
  return createTextRenderer(options).render(scene);
}
