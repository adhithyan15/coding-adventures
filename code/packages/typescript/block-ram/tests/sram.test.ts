/**
 * Tests for SRAM cells and arrays.
 */

import { describe, it, expect } from "vitest";
import { SRAMCell, SRAMArray } from "../src/index.js";
import type { Bit } from "@coding-adventures/logic-gates";

// === SRAMCell ===

describe("SRAMCell", () => {
  it("initializes to 0", () => {
    const cell = new SRAMCell();
    expect(cell.value).toBe(0);
  });

  it("read returns value when wordLine=1", () => {
    const cell = new SRAMCell();
    expect(cell.read(1)).toBe(0);
  });

  it("read returns null when wordLine=0 (not selected)", () => {
    const cell = new SRAMCell();
    expect(cell.read(0)).toBeNull();
  });

  it("write stores value when wordLine=1", () => {
    const cell = new SRAMCell();
    cell.write(1, 1);
    expect(cell.value).toBe(1);
    expect(cell.read(1)).toBe(1);
  });

  it("write is ignored when wordLine=0", () => {
    const cell = new SRAMCell();
    cell.write(0, 1);
    expect(cell.value).toBe(0);
  });

  it("can overwrite stored value", () => {
    const cell = new SRAMCell();
    cell.write(1, 1);
    expect(cell.value).toBe(1);
    cell.write(1, 0);
    expect(cell.value).toBe(0);
  });

  it("read does not modify stored value", () => {
    const cell = new SRAMCell();
    cell.write(1, 1);
    cell.read(1);
    cell.read(1);
    expect(cell.value).toBe(1);
  });

  it("rejects invalid wordLine", () => {
    const cell = new SRAMCell();
    expect(() => cell.read(2 as Bit)).toThrow(RangeError);
    expect(() => cell.write(2 as Bit, 0)).toThrow(RangeError);
  });

  it("rejects invalid bitLine", () => {
    const cell = new SRAMCell();
    expect(() => cell.write(1, 2 as Bit)).toThrow(RangeError);
  });
});

// === SRAMArray ===

describe("SRAMArray", () => {
  it("initializes all cells to 0", () => {
    const arr = new SRAMArray(4, 8);
    expect(arr.read(0)).toEqual([0, 0, 0, 0, 0, 0, 0, 0]);
    expect(arr.read(3)).toEqual([0, 0, 0, 0, 0, 0, 0, 0]);
  });

  it("reports correct shape", () => {
    const arr = new SRAMArray(4, 8);
    expect(arr.shape).toEqual([4, 8]);
  });

  it("write and read back a row", () => {
    const arr = new SRAMArray(4, 4);
    const data: Bit[] = [1, 0, 1, 0];
    arr.write(0, data);
    expect(arr.read(0)).toEqual([1, 0, 1, 0]);
  });

  it("writes to different rows are independent", () => {
    const arr = new SRAMArray(4, 4);
    arr.write(0, [1, 1, 0, 0]);
    arr.write(1, [0, 0, 1, 1]);
    expect(arr.read(0)).toEqual([1, 1, 0, 0]);
    expect(arr.read(1)).toEqual([0, 0, 1, 1]);
    expect(arr.read(2)).toEqual([0, 0, 0, 0]);
  });

  it("overwrite row data", () => {
    const arr = new SRAMArray(2, 4);
    arr.write(0, [1, 1, 1, 1]);
    arr.write(0, [0, 0, 0, 0]);
    expect(arr.read(0)).toEqual([0, 0, 0, 0]);
  });

  it("rejects rows < 1", () => {
    expect(() => new SRAMArray(0, 4)).toThrow(RangeError);
  });

  it("rejects cols < 1", () => {
    expect(() => new SRAMArray(4, 0)).toThrow(RangeError);
  });

  it("rejects out-of-range row on read", () => {
    const arr = new SRAMArray(4, 4);
    expect(() => arr.read(-1)).toThrow(RangeError);
    expect(() => arr.read(4)).toThrow(RangeError);
  });

  it("rejects out-of-range row on write", () => {
    const arr = new SRAMArray(4, 4);
    expect(() => arr.write(4, [0, 0, 0, 0])).toThrow(RangeError);
  });

  it("rejects wrong data length on write", () => {
    const arr = new SRAMArray(4, 4);
    expect(() => arr.write(0, [0, 0, 0])).toThrow(RangeError);
    expect(() => arr.write(0, [0, 0, 0, 0, 0])).toThrow(RangeError);
  });

  it("rejects non-array data on write", () => {
    const arr = new SRAMArray(4, 4);
    expect(() => arr.write(0, "1010" as any)).toThrow(TypeError);
  });

  it("rejects invalid bit values in data", () => {
    const arr = new SRAMArray(4, 4);
    expect(() => arr.write(0, [0, 0, 2 as Bit, 0])).toThrow(RangeError);
  });

  it("rejects non-integer row", () => {
    const arr = new SRAMArray(4, 4);
    expect(() => arr.read(1.5)).toThrow(TypeError);
  });
});
