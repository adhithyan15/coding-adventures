/**
 * Tests for LUT (Look-Up Table).
 */

import { describe, it, expect } from "vitest";
import { LUT } from "../src/index.js";
import type { Bit } from "@coding-adventures/logic-gates";

describe("LUT", () => {
  it("default k=4, all zeros", () => {
    const lut = new LUT();
    expect(lut.k).toBe(4);
    expect(lut.truthTable).toEqual(Array(16).fill(0));
  });

  it("evaluate with all-zero truth table returns 0", () => {
    const lut = new LUT(4);
    expect(lut.evaluate([0, 0, 0, 0])).toBe(0);
    expect(lut.evaluate([1, 1, 1, 1])).toBe(0);
  });

  it("configure and evaluate as 2-input AND", () => {
    const tt = Array(16).fill(0) as Bit[];
    tt[3] = 1; // I0=1, I1=1 -> index 3
    const lut = new LUT(4, tt);
    expect(lut.evaluate([0, 0, 0, 0])).toBe(0);
    expect(lut.evaluate([1, 0, 0, 0])).toBe(0);
    expect(lut.evaluate([0, 1, 0, 0])).toBe(0);
    expect(lut.evaluate([1, 1, 0, 0])).toBe(1);
  });

  it("configure and evaluate as 2-input XOR", () => {
    const tt = Array(16).fill(0) as Bit[];
    tt[1] = 1; // I0=1, I1=0
    tt[2] = 1; // I0=0, I1=1
    const lut = new LUT(4, tt);
    expect(lut.evaluate([0, 0, 0, 0])).toBe(0);
    expect(lut.evaluate([1, 0, 0, 0])).toBe(1);
    expect(lut.evaluate([0, 1, 0, 0])).toBe(1);
    expect(lut.evaluate([1, 1, 0, 0])).toBe(0);
  });

  it("reconfigure changes behavior", () => {
    const lut = new LUT(4);
    expect(lut.evaluate([1, 1, 0, 0])).toBe(0);

    // Configure as AND
    const tt = Array(16).fill(0) as Bit[];
    tt[3] = 1;
    lut.configure(tt);
    expect(lut.evaluate([1, 1, 0, 0])).toBe(1);
  });

  it("k=2 LUT works", () => {
    const tt: Bit[] = [0, 0, 0, 1]; // AND
    const lut = new LUT(2, tt);
    expect(lut.evaluate([0, 0])).toBe(0);
    expect(lut.evaluate([1, 1])).toBe(1);
  });

  it("k=3 LUT works", () => {
    const tt = Array(8).fill(0) as Bit[];
    tt[7] = 1; // All inputs = 1
    const lut = new LUT(3, tt);
    expect(lut.evaluate([1, 1, 1])).toBe(1);
    expect(lut.evaluate([1, 1, 0])).toBe(0);
  });

  it("truthTable property returns copy of current table", () => {
    const tt = Array(16).fill(0) as Bit[];
    tt[5] = 1;
    const lut = new LUT(4, tt);
    const result = lut.truthTable;
    expect(result[5]).toBe(1);
    expect(result.length).toBe(16);
  });

  it("rejects k < 2", () => {
    expect(() => new LUT(1)).toThrow(RangeError);
  });

  it("rejects k > 6", () => {
    expect(() => new LUT(7)).toThrow(RangeError);
  });

  it("rejects non-integer k", () => {
    expect(() => new LUT("4" as any)).toThrow(TypeError);
  });

  it("rejects wrong truth table length", () => {
    const lut = new LUT(4);
    expect(() => lut.configure([0, 0, 0, 0])).toThrow(RangeError);
  });

  it("rejects non-array truth table", () => {
    const lut = new LUT(4);
    expect(() => lut.configure("0000" as any)).toThrow(TypeError);
  });

  it("rejects wrong inputs length", () => {
    const lut = new LUT(4);
    expect(() => lut.evaluate([0, 0, 0])).toThrow(RangeError);
  });

  it("rejects non-array inputs", () => {
    const lut = new LUT(4);
    expect(() => lut.evaluate("0000" as any)).toThrow(TypeError);
  });
});
