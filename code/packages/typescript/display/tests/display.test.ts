import { describe, it, expect } from "vitest";
import { DisplayDriver, DisplaySnapshot, defaultDisplayConfig, BYTES_PER_CELL, makeAttribute, ColorWhite, ColorBlue } from "../src/index.js";

function makeDriver(cols = 40, rows = 10) {
  const config = { columns: cols, rows, framebufferBase: 0xfffb0000, defaultAttribute: 0x07 };
  const mem = new Uint8Array(cols * rows * BYTES_PER_CELL);
  return new DisplayDriver(config, mem);
}

describe("DisplayDriver", () => {
  it("starts with blank screen and cursor at 0,0", () => {
    const d = makeDriver();
    expect(d.cursor).toEqual({ row: 0, col: 0 });
    const snap = d.snapshot();
    expect(snap.lines.every(l => l === "")).toBe(true);
  });

  it("puts writes characters to framebuffer", () => {
    const d = makeDriver();
    d.puts("Hello");
    const snap = d.snapshot();
    expect(snap.lineAt(0)).toBe("Hello");
    expect(d.cursor.col).toBe(5);
  });

  it("handles newline", () => {
    const d = makeDriver();
    d.puts("AB\nCD");
    const snap = d.snapshot();
    expect(snap.lineAt(0)).toBe("AB");
    expect(snap.lineAt(1)).toBe("CD");
  });

  it("handles tab stops", () => {
    const d = makeDriver();
    d.puts("A\tB");
    expect(d.cursor.col).toBe(9); // tab to 8, then B at 8 -> cursor at 9
  });

  it("wraps at end of line", () => {
    const d = makeDriver(10, 5);
    d.puts("1234567890X");
    expect(d.cursor.row).toBe(1);
    expect(d.cursor.col).toBe(1);
    const snap = d.snapshot();
    expect(snap.lineAt(0)).toBe("1234567890");
    expect(snap.lineAt(1)).toBe("X");
  });

  it("scrolls when past last row", () => {
    const d = makeDriver(10, 3);
    d.puts("Line1\nLine2\nLine3\nLine4");
    const snap = d.snapshot();
    expect(snap.lineAt(0)).toBe("Line2");
    expect(snap.lineAt(1)).toBe("Line3");
    expect(snap.lineAt(2)).toBe("Line4");
  });

  it("clear resets screen", () => {
    const d = makeDriver();
    d.puts("Some text");
    d.clear();
    const snap = d.snapshot();
    expect(snap.lineAt(0)).toBe("");
    expect(d.cursor).toEqual({ row: 0, col: 0 });
  });

  it("setCursor clamps to bounds", () => {
    const d = makeDriver(10, 5);
    d.setCursor(-1, -1);
    expect(d.cursor).toEqual({ row: 0, col: 0 });
    d.setCursor(100, 100);
    expect(d.cursor).toEqual({ row: 4, col: 9 });
  });

  it("getCell reads character and attribute", () => {
    const d = makeDriver();
    d.puts("A");
    const cell = d.getCell(0, 0);
    expect(cell.character).toBe(0x41);
    expect(cell.attribute).toBe(0x07);
  });

  it("putCharAt writes at specific position", () => {
    const d = makeDriver();
    d.putCharAt(2, 5, 0x42, makeAttribute(ColorWhite, ColorBlue));
    const cell = d.getCell(2, 5);
    expect(cell.character).toBe(0x42);
    expect(cell.attribute).toBe(makeAttribute(ColorWhite, ColorBlue));
  });

  it("snapshot contains works", () => {
    const d = makeDriver();
    d.puts("Hello World");
    const snap = d.snapshot();
    expect(snap.contains("World")).toBe(true);
    expect(snap.contains("Nope")).toBe(false);
  });

  it("backspace moves cursor left", () => {
    const d = makeDriver();
    d.puts("AB");
    d.putChar(0x08); // backspace
    expect(d.cursor.col).toBe(1);
  });
});
