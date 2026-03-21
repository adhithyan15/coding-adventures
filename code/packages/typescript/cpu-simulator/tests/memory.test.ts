/**
 * Tests for memory.
 */

import { describe, expect, it } from "vitest";
import { Memory } from "../src/memory.js";

describe("Memory byte operations", () => {
  it("initial values are zero", () => {
    const mem = new Memory(16);
    for (let i = 0; i < 16; i++) {
      expect(mem.readByte(i)).toBe(0);
    }
  });

  it("write and read byte", () => {
    const mem = new Memory(16);
    mem.writeByte(0, 42);
    expect(mem.readByte(0)).toBe(42);
  });

  it("byte masking (values > 255 are masked to 8 bits)", () => {
    const mem = new Memory(16);
    mem.writeByte(0, 256);
    expect(mem.readByte(0)).toBe(0); // 256 & 0xFF = 0
  });

  it("out of bounds read throws", () => {
    const mem = new Memory(16);
    expect(() => mem.readByte(16)).toThrow(/out of bounds/);
  });

  it("out of bounds write throws", () => {
    const mem = new Memory(16);
    expect(() => mem.writeByte(16, 0)).toThrow(/out of bounds/);
  });
});

describe("Memory word operations", () => {
  it("write and read word", () => {
    const mem = new Memory(16);
    mem.writeWord(0, 0x12345678);
    expect(mem.readWord(0)).toBe(0x12345678);
  });

  it("little-endian byte order (LSB at lowest address)", () => {
    const mem = new Memory(16);
    mem.writeWord(0, 0x12345678);
    expect(mem.readByte(0)).toBe(0x78); // LSB
    expect(mem.readByte(1)).toBe(0x56);
    expect(mem.readByte(2)).toBe(0x34);
    expect(mem.readByte(3)).toBe(0x12); // MSB
  });

  it("word at offset", () => {
    const mem = new Memory(16);
    mem.writeWord(4, 0xdeadbeef);
    expect(mem.readWord(4)).toBe(0xdeadbeef);
    expect(mem.readWord(0)).toBe(0); // First word unaffected
  });

  it("small value stored as 32-bit word", () => {
    const mem = new Memory(16);
    mem.writeWord(0, 3);
    expect(mem.readByte(0)).toBe(3);
    expect(mem.readByte(1)).toBe(0);
    expect(mem.readByte(2)).toBe(0);
    expect(mem.readByte(3)).toBe(0);
  });
});

describe("Memory load", () => {
  it("load bytes", () => {
    const mem = new Memory(16);
    mem.loadBytes(0, [0x01, 0x02, 0x03, 0x04]);
    expect(mem.readByte(0)).toBe(1);
    expect(mem.readByte(1)).toBe(2);
    expect(mem.readByte(2)).toBe(3);
    expect(mem.readByte(3)).toBe(4);
  });

  it("load at offset", () => {
    const mem = new Memory(16);
    mem.loadBytes(4, [0xaa, 0xbb]);
    expect(mem.readByte(4)).toBe(0xaa);
    expect(mem.readByte(5)).toBe(0xbb);
  });

  it("load out of bounds throws", () => {
    const mem = new Memory(4);
    expect(() => mem.loadBytes(2, [0x01, 0x02, 0x03])).toThrow(
      /out of bounds/
    );
  });
});

describe("Memory dump", () => {
  it("dump returns byte values", () => {
    const mem = new Memory(16);
    mem.writeByte(0, 0xab);
    mem.writeByte(1, 0xcd);
    expect(mem.dump(0, 4)).toEqual([0xab, 0xcd, 0, 0]);
  });
});

describe("Memory validation", () => {
  it("zero size throws", () => {
    expect(() => new Memory(0)).toThrow(/at least 1/);
  });
});
