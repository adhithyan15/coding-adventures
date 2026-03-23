/**
 * Tests for SinglePortRAM and DualPortRAM.
 */

import { describe, it, expect } from "vitest";
import {
  SinglePortRAM,
  DualPortRAM,
  ReadMode,
  WriteCollisionError,
} from "../src/index.js";
import type { Bit } from "@coding-adventures/logic-gates";

// Helper: perform a complete clock cycle (low then high)
function writeCycle(
  ram: SinglePortRAM,
  address: number,
  data: Bit[],
): Bit[] {
  ram.tick(0, address, data, 1);
  return ram.tick(1, address, data, 1);
}

function readCycle(
  ram: SinglePortRAM,
  address: number,
): Bit[] {
  const zeros = Array(ram.width).fill(0) as Bit[];
  ram.tick(0, address, zeros, 0);
  return ram.tick(1, address, zeros, 0);
}

// === SinglePortRAM ===

describe("SinglePortRAM", () => {
  it("initializes to all zeros", () => {
    const ram = new SinglePortRAM(4, 8);
    expect(ram.depth).toBe(4);
    expect(ram.width).toBe(8);
    expect(readCycle(ram, 0)).toEqual([0, 0, 0, 0, 0, 0, 0, 0]);
  });

  it("write then read back", () => {
    const ram = new SinglePortRAM(4, 4);
    writeCycle(ram, 0, [1, 0, 1, 0]);
    expect(readCycle(ram, 0)).toEqual([1, 0, 1, 0]);
  });

  it("writes to different addresses", () => {
    const ram = new SinglePortRAM(4, 4);
    writeCycle(ram, 0, [1, 1, 0, 0]);
    writeCycle(ram, 1, [0, 0, 1, 1]);
    expect(readCycle(ram, 0)).toEqual([1, 1, 0, 0]);
    expect(readCycle(ram, 1)).toEqual([0, 0, 1, 1]);
  });

  it("READ_FIRST mode returns old value on write", () => {
    const ram = new SinglePortRAM(4, 4, ReadMode.READ_FIRST);
    writeCycle(ram, 0, [1, 1, 1, 1]);
    // Now overwrite -- should return old value [1,1,1,1]
    const out = writeCycle(ram, 0, [0, 0, 0, 0]);
    expect(out).toEqual([1, 1, 1, 1]);
    // Verify new value was written
    expect(readCycle(ram, 0)).toEqual([0, 0, 0, 0]);
  });

  it("WRITE_FIRST mode returns new value on write", () => {
    const ram = new SinglePortRAM(4, 4, ReadMode.WRITE_FIRST);
    writeCycle(ram, 0, [1, 1, 1, 1]);
    const out = writeCycle(ram, 0, [0, 1, 0, 1]);
    expect(out).toEqual([0, 1, 0, 1]);
  });

  it("NO_CHANGE mode retains previous read on write", () => {
    const ram = new SinglePortRAM(4, 4, ReadMode.NO_CHANGE);
    // Read to establish last_read
    const initial = readCycle(ram, 0); // [0,0,0,0]
    expect(initial).toEqual([0, 0, 0, 0]);
    // Write -- output should stay at previous read value
    const out = writeCycle(ram, 0, [1, 1, 1, 1]);
    expect(out).toEqual([0, 0, 0, 0]);
    // Verify data was written
    expect(readCycle(ram, 0)).toEqual([1, 1, 1, 1]);
  });

  it("no operation on falling edge", () => {
    const ram = new SinglePortRAM(4, 4);
    // Rising edge: write
    ram.tick(0, 0, [1, 1, 1, 1], 1);
    ram.tick(1, 0, [1, 1, 1, 1], 1);
    // Falling edge: should not perform new operation
    const out = ram.tick(0, 0, [0, 0, 0, 0], 1);
    // Should return last read value, not perform a new write
    expect(readCycle(ram, 0)).toEqual([1, 1, 1, 1]);
  });

  it("dump returns all rows", () => {
    const ram = new SinglePortRAM(2, 4);
    writeCycle(ram, 0, [1, 0, 1, 0]);
    writeCycle(ram, 1, [0, 1, 0, 1]);
    expect(ram.dump()).toEqual([
      [1, 0, 1, 0],
      [0, 1, 0, 1],
    ]);
  });

  it("rejects depth < 1", () => {
    expect(() => new SinglePortRAM(0, 4)).toThrow(RangeError);
  });

  it("rejects width < 1", () => {
    expect(() => new SinglePortRAM(4, 0)).toThrow(RangeError);
  });

  it("rejects out-of-range address", () => {
    const ram = new SinglePortRAM(4, 4);
    expect(() => ram.tick(1, 4, [0, 0, 0, 0], 0)).toThrow(RangeError);
    expect(() => ram.tick(1, -1, [0, 0, 0, 0], 0)).toThrow(RangeError);
  });

  it("rejects wrong data length", () => {
    const ram = new SinglePortRAM(4, 4);
    expect(() => ram.tick(1, 0, [0, 0, 0], 0)).toThrow(RangeError);
  });

  it("rejects invalid clock", () => {
    const ram = new SinglePortRAM(4, 4);
    expect(() => ram.tick(2 as Bit, 0, [0, 0, 0, 0], 0)).toThrow(RangeError);
  });
});

// === DualPortRAM ===

describe("DualPortRAM", () => {
  const zeros4: Bit[] = [0, 0, 0, 0];

  it("initializes to all zeros", () => {
    const ram = new DualPortRAM(4, 4);
    expect(ram.depth).toBe(4);
    expect(ram.width).toBe(4);
  });

  it("write on port A, read on port B", () => {
    const ram = new DualPortRAM(4, 4);
    // Write via port A
    ram.tick(0, 0, [1, 0, 1, 0], 1, 0, zeros4, 0);
    ram.tick(1, 0, [1, 0, 1, 0], 1, 0, zeros4, 0);
    // Read via port B
    ram.tick(0, 0, zeros4, 0, 0, zeros4, 0);
    const [, outB] = ram.tick(1, 0, zeros4, 0, 0, zeros4, 0);
    expect(outB).toEqual([1, 0, 1, 0]);
  });

  it("simultaneous read on different addresses", () => {
    const ram = new DualPortRAM(4, 4);
    // Write to addr 0 and 1
    ram.tick(0, 0, [1, 1, 0, 0], 1, 1, [0, 0, 1, 1], 1);
    ram.tick(1, 0, [1, 1, 0, 0], 1, 1, [0, 0, 1, 1], 1);
    // Read both simultaneously
    ram.tick(0, 0, zeros4, 0, 1, zeros4, 0);
    const [outA, outB] = ram.tick(1, 0, zeros4, 0, 1, zeros4, 0);
    expect(outA).toEqual([1, 1, 0, 0]);
    expect(outB).toEqual([0, 0, 1, 1]);
  });

  it("write collision throws WriteCollisionError", () => {
    const ram = new DualPortRAM(4, 4);
    // Both ports write to address 0 simultaneously
    ram.tick(0, 0, [1, 1, 1, 1], 1, 0, [0, 0, 0, 0], 1);
    expect(() =>
      ram.tick(1, 0, [1, 1, 1, 1], 1, 0, [0, 0, 0, 0], 1),
    ).toThrow(WriteCollisionError);
  });

  it("WriteCollisionError has address property", () => {
    const err = new WriteCollisionError(42);
    expect(err.address).toBe(42);
    expect(err.message).toContain("42");
  });

  it("no collision when writing to different addresses", () => {
    const ram = new DualPortRAM(4, 4);
    ram.tick(0, 0, [1, 1, 1, 1], 1, 1, [0, 1, 0, 1], 1);
    expect(() =>
      ram.tick(1, 0, [1, 1, 1, 1], 1, 1, [0, 1, 0, 1], 1),
    ).not.toThrow();
  });

  it("rejects depth < 1", () => {
    expect(() => new DualPortRAM(0, 4)).toThrow(RangeError);
  });

  it("rejects width < 1", () => {
    expect(() => new DualPortRAM(4, 0)).toThrow(RangeError);
  });
});
