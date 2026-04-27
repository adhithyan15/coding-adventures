/**
 * @coding-adventures/paint-vm-ascii
 *
 * Terminal/ASCII backend for PaintVM.
 *
 * This backend executes a PaintScene into a character grid using Unicode
 * box-drawing characters, block fills, and direct glyph placement. It is the
 * text-mode counterpart to the SVG and Canvas backends.
 */

import { ExportNotSupportedError, PaintVM } from "@coding-adventures/paint-vm";
import type {
  PaintClip,
  PaintGlyphRun,
  PaintGroup,
  PaintInstruction,
  PaintLayer,
  PaintLine,
  PaintRect,
  PaintScene,
} from "@coding-adventures/paint-instructions";

export const VERSION = "0.1.0";

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

export interface AsciiOptions {
  scaleX?: number;
  scaleY?: number;
}

export interface AsciiContext {
  buffer: CharBuffer;
  clipStack: ClipBounds[];
}

// ---------------------------------------------------------------------------
// Direction flags
// ---------------------------------------------------------------------------

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

const BOX_CHARS: Record<number, string> = {
  [LEFT | RIGHT]: "─",
  [UP | DOWN]: "│",
  [DOWN | RIGHT]: "┌",
  [DOWN | LEFT]: "┐",
  [UP | RIGHT]: "└",
  [UP | LEFT]: "┘",
  [LEFT | RIGHT | DOWN]: "┬",
  [LEFT | RIGHT | UP]: "┴",
  [UP | DOWN | RIGHT]: "├",
  [UP | DOWN | LEFT]: "┤",
  [UP | DOWN | LEFT | RIGHT]: "┼",
  [RIGHT]: "─",
  [LEFT]: "─",
  [UP]: "│",
  [DOWN]: "│",
};

function resolveBoxChar(tag: number): string {
  if (tag & FILL) return "█";
  if (tag & TEXT) return "";
  return BOX_CHARS[tag & (UP | DOWN | LEFT | RIGHT)] ?? "+";
}

// ---------------------------------------------------------------------------
// Buffer
// ---------------------------------------------------------------------------

class CharBuffer {
  readonly rows: number;
  readonly cols: number;
  private readonly chars: string[][];
  private readonly tags: number[][];

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

  writeTag(row: number, col: number, dirFlags: number, clip: ClipBounds): void {
    if (row < clip.minRow || row >= clip.maxRow) return;
    if (col < clip.minCol || col >= clip.maxCol) return;
    if (row < 0 || row >= this.rows || col < 0 || col >= this.cols) return;

    const existing = this.tags[row]![col]!;
    if (existing & TEXT) return;

    const merged = existing | dirFlags;
    this.tags[row]![col] = merged;
    this.chars[row]![col] = dirFlags & FILL ? "█" : resolveBoxChar(merged);
  }

  writeChar(row: number, col: number, ch: string, clip: ClipBounds): void {
    if (row < clip.minRow || row >= clip.maxRow) return;
    if (col < clip.minCol || col >= clip.maxCol) return;
    if (row < 0 || row >= this.rows || col < 0 || col >= this.cols) return;

    this.chars[row]![col] = ch;
    this.tags[row]![col] = TEXT;
  }

  toString(): string {
    return this.chars
      .map((row) => row.join("").trimEnd())
      .join("\n")
      .trimEnd();
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function toCol(x: number, scaleX: number): number {
  return Math.round(x / scaleX);
}

function toRow(y: number, scaleY: number): number {
  return Math.round(y / scaleY);
}

function topClip(ctx: AsciiContext): ClipBounds {
  return ctx.clipStack[ctx.clipStack.length - 1]!;
}

function fullClip(cols: number, rows: number): ClipBounds {
  return { minCol: 0, minRow: 0, maxCol: cols, maxRow: rows };
}

function isSafeTerminalCodePoint(codePoint: number): boolean {
  if (codePoint < 0x20) return false;
  if (codePoint >= 0x7f && codePoint <= 0x9f) return false;
  if (codePoint === 0x200e || codePoint === 0x200f || codePoint === 0x061c) {
    return false;
  }
  if (codePoint >= 0x202a && codePoint <= 0x202e) return false;
  if (codePoint >= 0x2066 && codePoint <= 0x2069) return false;
  return true;
}

function isIdentityTransform(
  transform: [number, number, number, number, number, number] | undefined,
): boolean {
  return (
    transform === undefined ||
    (transform[0] === 1 &&
      transform[1] === 0 &&
      transform[2] === 0 &&
      transform[3] === 1 &&
      transform[4] === 0 &&
      transform[5] === 0)
  );
}

function assertPlainGroup(group: PaintGroup): void {
  if (!isIdentityTransform(group.transform)) {
    throw new Error("paint-vm-ascii does not support transformed groups");
  }
  if (group.opacity !== undefined && group.opacity !== 1) {
    throw new Error("paint-vm-ascii does not support group opacity");
  }
}

function assertPlainLayer(layer: PaintLayer): void {
  if (!isIdentityTransform(layer.transform)) {
    throw new Error("paint-vm-ascii does not support transformed layers");
  }
  if (layer.opacity !== undefined && layer.opacity !== 1) {
    throw new Error("paint-vm-ascii does not support layer opacity");
  }
  if (layer.filters && layer.filters.length > 0) {
    throw new Error("paint-vm-ascii does not support layer filters");
  }
  if (layer.blend_mode && layer.blend_mode !== "normal") {
    throw new Error("paint-vm-ascii does not support layer blend modes");
  }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

function handleRect(
  inst: PaintRect,
  ctx: AsciiContext,
  scaleX: number,
  scaleY: number,
): void {
  const clip = topClip(ctx);
  const c1 = toCol(inst.x, scaleX);
  const r1 = toRow(inst.y, scaleY);
  const c2 = toCol(inst.x + inst.width, scaleX);
  const r2 = toRow(inst.y + inst.height, scaleY);

  const hasStroke = inst.stroke !== undefined && inst.stroke !== "";
  const hasFill =
    inst.fill !== undefined &&
    inst.fill !== "" &&
    inst.fill !== "transparent" &&
    inst.fill !== "none";

  if (hasFill) {
    for (let r = r1; r <= r2; r++) {
      for (let c = c1; c <= c2; c++) {
        ctx.buffer.writeTag(r, c, FILL, clip);
      }
    }
  }

  if (!hasStroke) return;

  ctx.buffer.writeTag(r1, c1, DOWN | RIGHT, clip);
  ctx.buffer.writeTag(r1, c2, DOWN | LEFT, clip);
  ctx.buffer.writeTag(r2, c1, UP | RIGHT, clip);
  ctx.buffer.writeTag(r2, c2, UP | LEFT, clip);

  for (let c = c1 + 1; c < c2; c++) {
    ctx.buffer.writeTag(r1, c, LEFT | RIGHT, clip);
    ctx.buffer.writeTag(r2, c, LEFT | RIGHT, clip);
  }

  for (let r = r1 + 1; r < r2; r++) {
    ctx.buffer.writeTag(r, c1, UP | DOWN, clip);
    ctx.buffer.writeTag(r, c2, UP | DOWN, clip);
  }
}

function handleLine(
  inst: PaintLine,
  ctx: AsciiContext,
  scaleX: number,
  scaleY: number,
): void {
  const clip = topClip(ctx);
  const c1 = toCol(inst.x1, scaleX);
  const r1 = toRow(inst.y1, scaleY);
  const c2 = toCol(inst.x2, scaleX);
  const r2 = toRow(inst.y2, scaleY);

  if (r1 === r2) {
    const minC = Math.min(c1, c2);
    const maxC = Math.max(c1, c2);
    for (let c = minC; c <= maxC; c++) {
      let flags = 0;
      if (c > minC) flags |= LEFT;
      if (c < maxC) flags |= RIGHT;
      if (c === minC && c === maxC) flags = LEFT | RIGHT;
      ctx.buffer.writeTag(r1, c, flags, clip);
    }
    return;
  }

  if (c1 === c2) {
    const minR = Math.min(r1, r2);
    const maxR = Math.max(r1, r2);
    for (let r = minR; r <= maxR; r++) {
      let flags = 0;
      if (r > minR) flags |= UP;
      if (r < maxR) flags |= DOWN;
      if (r === minR && r === maxR) flags = UP | DOWN;
      ctx.buffer.writeTag(r, c1, flags, clip);
    }
    return;
  }

  const dr = Math.abs(r2 - r1);
  const dc = Math.abs(c2 - c1);
  const sr = r1 < r2 ? 1 : -1;
  const sc = c1 < c2 ? 1 : -1;
  let err = dc - dr;
  let r = r1;
  let c = c1;

  while (true) {
    ctx.buffer.writeTag(r, c, dc > dr ? LEFT | RIGHT : UP | DOWN, clip);
    if (r === r2 && c === c2) break;
    const e2 = 2 * err;
    if (e2 > -dr) {
      err -= dr;
      c += sc;
    }
    if (e2 < dc) {
      err += dc;
      r += sr;
    }
  }
}

function handleGlyphRun(
  inst: PaintGlyphRun,
  ctx: AsciiContext,
  scaleX: number,
  scaleY: number,
): void {
  const clip = topClip(ctx);
  for (const glyph of inst.glyphs) {
    let ch = "?";
    try {
      ch = isSafeTerminalCodePoint(glyph.glyph_id)
        ? String.fromCodePoint(glyph.glyph_id)
        : "?";
    } catch {
      ch = "?";
    }
    ctx.buffer.writeChar(toRow(glyph.y, scaleY), toCol(glyph.x, scaleX), ch, clip);
  }
}

function handleClip(
  inst: PaintClip,
  ctx: AsciiContext,
  vm: PaintVM<AsciiContext>,
  scaleX: number,
  scaleY: number,
): void {
  const parent = topClip(ctx);
  const next: ClipBounds = {
    minCol: Math.max(parent.minCol, toCol(inst.x, scaleX)),
    minRow: Math.max(parent.minRow, toRow(inst.y, scaleY)),
    maxCol: Math.min(parent.maxCol, toCol(inst.x + inst.width, scaleX)),
    maxRow: Math.min(parent.maxRow, toRow(inst.y + inst.height, scaleY)),
  };
  ctx.clipStack.push(next);
  try {
    for (const child of inst.children) vm.dispatch(child, ctx);
  } finally {
    ctx.clipStack.pop();
  }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

export function createAsciiContext(): AsciiContext {
  return {
    buffer: new CharBuffer(0, 0),
    clipStack: [fullClip(0, 0)],
  };
}

export function createAsciiVM(options: AsciiOptions = {}): PaintVM<AsciiContext> {
  const scaleX = options.scaleX ?? 8;
  const scaleY = options.scaleY ?? 16;

  const vm = new PaintVM<AsciiContext>(
    (ctx, _background, width, height) => {
      const cols = Math.ceil(width / scaleX);
      const rows = Math.ceil(height / scaleY);
      ctx.buffer = new CharBuffer(rows, cols);
      ctx.clipStack = [fullClip(cols, rows)];
    },
    () => {
      throw new ExportNotSupportedError("paint-vm-ascii");
    },
  );

  vm.register("rect", (instruction, ctx) => {
    if (instruction.kind !== "rect") return;
    handleRect(instruction, ctx, scaleX, scaleY);
  });

  vm.register("line", (instruction, ctx) => {
    if (instruction.kind !== "line") return;
    handleLine(instruction, ctx, scaleX, scaleY);
  });

  vm.register("glyph_run", (instruction, ctx) => {
    if (instruction.kind !== "glyph_run") return;
    handleGlyphRun(instruction, ctx, scaleX, scaleY);
  });

  vm.register("group", (instruction, ctx, innerVm) => {
    if (instruction.kind !== "group") return;
    assertPlainGroup(instruction);
    for (const child of instruction.children) innerVm.dispatch(child, ctx);
  });

  vm.register("clip", (instruction, ctx, innerVm) => {
    if (instruction.kind !== "clip") return;
    handleClip(instruction, ctx, innerVm, scaleX, scaleY);
  });

  vm.register("layer", (instruction, ctx, innerVm) => {
    if (instruction.kind !== "layer") return;
    assertPlainLayer(instruction);
    for (const child of instruction.children) innerVm.dispatch(child, ctx);
  });

  return vm;
}

export function renderToAscii(
  scene: PaintScene,
  options: AsciiOptions = {},
): string {
  const vm = createAsciiVM(options);
  const ctx = createAsciiContext();
  vm.execute(scene, ctx);
  return ctx.buffer.toString();
}
