/**
 * Display -- VGA text-mode framebuffer simulator.
 *
 * Simulates an 80x25 character display with cursor tracking, scrolling,
 * and color attributes.
 */

// === Constants ===
export const BYTES_PER_CELL = 2;
export const DEFAULT_COLUMNS = 80;
export const DEFAULT_ROWS = 25;
export const DEFAULT_FRAMEBUFFER_BASE = 0xfffb0000;
export const DEFAULT_ATTRIBUTE = 0x07;

// === VGA colors ===
export const ColorBlack = 0;
export const ColorBlue = 1;
export const ColorGreen = 2;
export const ColorCyan = 3;
export const ColorRed = 4;
export const ColorMagenta = 5;
export const ColorBrown = 6;
export const ColorLightGray = 7;
export const ColorWhite = 15;

export function makeAttribute(fg: number, bg: number): number {
  return ((bg & 0x07) << 4) | (fg & 0x0f);
}

// === Config ===
export interface DisplayConfig {
  columns: number;
  rows: number;
  framebufferBase: number;
  defaultAttribute: number;
}

export function defaultDisplayConfig(): DisplayConfig {
  return { columns: DEFAULT_COLUMNS, rows: DEFAULT_ROWS, framebufferBase: DEFAULT_FRAMEBUFFER_BASE, defaultAttribute: DEFAULT_ATTRIBUTE };
}

// === Cell and Cursor ===
export interface Cell { character: number; attribute: number; }
export interface CursorPosition { row: number; col: number; }

// === Snapshot ===
export class DisplaySnapshot {
  constructor(
    public readonly lines: string[],
    public readonly cursor: CursorPosition,
    public readonly rows: number,
    public readonly columns: number,
  ) {}

  toString(): string {
    return this.lines.map(l => l + " ".repeat(this.columns - l.length)).join("\n");
  }

  contains(text: string): boolean {
    return this.lines.some(line => line.includes(text));
  }

  lineAt(row: number): string {
    return row >= 0 && row < this.lines.length ? this.lines[row] : "";
  }
}

// === Display Driver ===
export class DisplayDriver {
  readonly config: DisplayConfig;
  readonly memory: Uint8Array;
  cursor: CursorPosition;

  constructor(config: DisplayConfig, memory: Uint8Array) {
    this.config = config;
    this.memory = memory;
    this.cursor = { row: 0, col: 0 };
    this.clear();
  }

  putChar(ch: number): void {
    if (ch === 0x0a) { // newline
      this.cursor.col = 0;
      this.cursor.row++;
    } else if (ch === 0x0d) { // carriage return
      this.cursor.col = 0;
    } else if (ch === 0x09) { // tab
      this.cursor.col = (Math.floor(this.cursor.col / 8) + 1) * 8;
      if (this.cursor.col >= this.config.columns) {
        this.cursor.col = 0;
        this.cursor.row++;
      }
    } else if (ch === 0x08) { // backspace
      if (this.cursor.col > 0) this.cursor.col--;
    } else {
      const offset = (this.cursor.row * this.config.columns + this.cursor.col) * BYTES_PER_CELL;
      if (offset >= 0 && offset + 1 < this.memory.length) {
        this.memory[offset] = ch;
        this.memory[offset + 1] = this.config.defaultAttribute;
      }
      this.cursor.col++;
      if (this.cursor.col >= this.config.columns) {
        this.cursor.col = 0;
        this.cursor.row++;
      }
    }
    if (this.cursor.row >= this.config.rows) {
      this.scroll();
    }
  }

  putCharAt(row: number, col: number, ch: number, attr: number): void {
    if (row < 0 || row >= this.config.rows || col < 0 || col >= this.config.columns) return;
    const offset = (row * this.config.columns + col) * BYTES_PER_CELL;
    this.memory[offset] = ch;
    this.memory[offset + 1] = attr;
  }

  puts(s: string): void {
    for (let i = 0; i < s.length; i++) {
      this.putChar(s.charCodeAt(i));
    }
  }

  clear(): void {
    const total = this.config.columns * this.config.rows * BYTES_PER_CELL;
    for (let i = 0; i < total && i + 1 < this.memory.length; i += BYTES_PER_CELL) {
      this.memory[i] = 0x20; // space
      this.memory[i + 1] = this.config.defaultAttribute;
    }
    this.cursor = { row: 0, col: 0 };
  }

  scroll(): void {
    const bytesPerRow = this.config.columns * BYTES_PER_CELL;
    const totalBytes = this.config.rows * bytesPerRow;
    this.memory.copyWithin(0, bytesPerRow, totalBytes);
    const lastRowStart = (this.config.rows - 1) * bytesPerRow;
    for (let i = lastRowStart; i < totalBytes; i += BYTES_PER_CELL) {
      this.memory[i] = 0x20;
      this.memory[i + 1] = this.config.defaultAttribute;
    }
    this.cursor.row = this.config.rows - 1;
    this.cursor.col = 0;
  }

  setCursor(row: number, col: number): void {
    this.cursor.row = Math.max(0, Math.min(row, this.config.rows - 1));
    this.cursor.col = Math.max(0, Math.min(col, this.config.columns - 1));
  }

  getCell(row: number, col: number): Cell {
    if (row < 0 || row >= this.config.rows || col < 0 || col >= this.config.columns) {
      return { character: 0x20, attribute: this.config.defaultAttribute };
    }
    const offset = (row * this.config.columns + col) * BYTES_PER_CELL;
    return { character: this.memory[offset], attribute: this.memory[offset + 1] };
  }

  snapshot(): DisplaySnapshot {
    const lines: string[] = [];
    for (let row = 0; row < this.config.rows; row++) {
      let buf = "";
      for (let col = 0; col < this.config.columns; col++) {
        const offset = (row * this.config.columns + col) * BYTES_PER_CELL;
        buf += String.fromCharCode(this.memory[offset]);
      }
      lines.push(buf.trimEnd());
    }
    return new DisplaySnapshot(lines, { ...this.cursor }, this.config.rows, this.config.columns);
  }
}
