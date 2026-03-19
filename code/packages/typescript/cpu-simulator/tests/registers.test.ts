/**
 * Tests for the register file.
 */

import { describe, expect, it } from "vitest";
import { RegisterFile } from "../src/registers.js";

describe("RegisterFile", () => {
  it("initial values are zero", () => {
    const regs = new RegisterFile(4);
    for (let i = 0; i < 4; i++) {
      expect(regs.read(i)).toBe(0);
    }
  });

  it("write and read", () => {
    const regs = new RegisterFile(4);
    regs.write(1, 42);
    expect(regs.read(1)).toBe(42);
  });

  it("write does not affect other registers", () => {
    const regs = new RegisterFile(4);
    regs.write(2, 99);
    expect(regs.read(0)).toBe(0);
    expect(regs.read(1)).toBe(0);
    expect(regs.read(2)).toBe(99);
    expect(regs.read(3)).toBe(0);
  });

  it("bit width masking (8-bit)", () => {
    // Values exceeding bit width should wrap around.
    const regs = new RegisterFile(4, 8);
    regs.write(0, 256); // 256 = 0x100, doesn't fit in 8 bits
    expect(regs.read(0)).toBe(0); // wraps to 0
  });

  it("bit width masking (32-bit)", () => {
    const regs = new RegisterFile(4, 32);
    regs.write(0, 0xffffffff);
    expect(regs.read(0)).toBe(0xffffffff);
    // 33 bits -- should be masked to 32 bits
    // In JS, bitwise AND with 0xFFFFFFFF will handle this
    regs.write(0, 0x1ffffffff);
    expect(regs.read(0)).toBe(0xffffffff);
  });

  it("read out of range throws", () => {
    const regs = new RegisterFile(4);
    expect(() => regs.read(4)).toThrow(/out of range/);
  });

  it("write out of range throws", () => {
    const regs = new RegisterFile(4);
    expect(() => regs.write(4, 0)).toThrow(/out of range/);
  });

  it("negative index throws", () => {
    const regs = new RegisterFile(4);
    expect(() => regs.read(-1)).toThrow();
  });

  it("dump returns all register values", () => {
    const regs = new RegisterFile(4);
    regs.write(1, 5);
    regs.write(3, 10);
    expect(regs.dump()).toEqual({ R0: 0, R1: 5, R2: 0, R3: 10 });
  });
});
