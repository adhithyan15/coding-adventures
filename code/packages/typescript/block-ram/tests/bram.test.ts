/**
 * Tests for ConfigurableBRAM.
 */

import { describe, it, expect } from "vitest";
import { ConfigurableBRAM } from "../src/index.js";
import type { Bit } from "@coding-adventures/logic-gates";

// Helper: perform a write cycle on port A
function writeA(
  bram: ConfigurableBRAM,
  address: number,
  data: Bit[],
): Bit[] {
  bram.tickA(0, address, data, 1);
  return bram.tickA(1, address, data, 1);
}

// Helper: perform a read cycle on port A
function readA(bram: ConfigurableBRAM, address: number): Bit[] {
  const zeros = Array(bram.width).fill(0) as Bit[];
  bram.tickA(0, address, zeros, 0);
  return bram.tickA(1, address, zeros, 0);
}

describe("ConfigurableBRAM", () => {
  it("computes depth from totalBits / width", () => {
    const bram = new ConfigurableBRAM(1024, 8);
    expect(bram.depth).toBe(128);
    expect(bram.width).toBe(8);
    expect(bram.totalBits).toBe(1024);
  });

  it("write and read via port A", () => {
    const bram = new ConfigurableBRAM(256, 4);
    writeA(bram, 0, [1, 0, 1, 0]);
    expect(readA(bram, 0)).toEqual([1, 0, 1, 0]);
  });

  it("write via port A, read via port B", () => {
    const bram = new ConfigurableBRAM(256, 4);
    writeA(bram, 0, [1, 1, 0, 0]);
    // Read via port B
    const zeros: Bit[] = [0, 0, 0, 0];
    bram.tickB(0, 0, zeros, 0);
    const out = bram.tickB(1, 0, zeros, 0);
    expect(out).toEqual([1, 1, 0, 0]);
  });

  it("reconfigure changes depth and width", () => {
    const bram = new ConfigurableBRAM(256, 4);
    expect(bram.depth).toBe(64);
    expect(bram.width).toBe(4);

    bram.reconfigure(8);
    expect(bram.depth).toBe(32);
    expect(bram.width).toBe(8);
  });

  it("reconfigure clears stored data", () => {
    const bram = new ConfigurableBRAM(256, 4);
    writeA(bram, 0, [1, 1, 1, 1]);
    expect(readA(bram, 0)).toEqual([1, 1, 1, 1]);

    bram.reconfigure(4);
    // Data should be cleared after reconfiguration
    expect(readA(bram, 0)).toEqual([0, 0, 0, 0]);
  });

  it("reconfigure to width=1 gives maximum depth", () => {
    const bram = new ConfigurableBRAM(64, 8);
    expect(bram.depth).toBe(8);

    bram.reconfigure(1);
    expect(bram.depth).toBe(64);
    expect(bram.width).toBe(1);
  });

  it("default totalBits is 18432", () => {
    const bram = new ConfigurableBRAM();
    expect(bram.totalBits).toBe(18432);
    expect(bram.width).toBe(8);
    expect(bram.depth).toBe(2304);
  });

  it("rejects totalBits < 1", () => {
    expect(() => new ConfigurableBRAM(0, 8)).toThrow(RangeError);
  });

  it("rejects width < 1", () => {
    expect(() => new ConfigurableBRAM(1024, 0)).toThrow(RangeError);
  });

  it("rejects width that doesn't divide totalBits", () => {
    expect(() => new ConfigurableBRAM(100, 7)).toThrow(RangeError);
  });

  it("reconfigure rejects width < 1", () => {
    const bram = new ConfigurableBRAM(256, 4);
    expect(() => bram.reconfigure(0)).toThrow(RangeError);
  });

  it("reconfigure rejects non-divisible width", () => {
    const bram = new ConfigurableBRAM(256, 4);
    expect(() => bram.reconfigure(3)).toThrow(RangeError);
  });
});
