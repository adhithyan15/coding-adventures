/**
 * Tests for LocalMemory.
 */

import { describe, it, expect } from "vitest";
import { FP32, FP16, BF16, floatToBits, bitsToFloat } from "@coding-adventures/fp-arithmetic";
import { LocalMemory } from "../src/memory.js";

describe("Construction", () => {
  it("default size is 4096 bytes", () => {
    const mem = new LocalMemory();
    expect(mem.size).toBe(4096);
  });

  it("custom size", () => {
    const mem = new LocalMemory(256);
    expect(mem.size).toBe(256);
  });

  it("rejects size < 1", () => {
    expect(() => new LocalMemory(0)).toThrow("positive");
  });

  it("initialized to zero", () => {
    const mem = new LocalMemory(16);
    for (let i = 0; i < 16; i++) {
      expect(mem.readByte(i)).toBe(0);
    }
  });
});

describe("ByteAccess", () => {
  it("read and write byte", () => {
    const mem = new LocalMemory();
    mem.writeByte(0, 0x42);
    expect(mem.readByte(0)).toBe(0x42);
  });

  it("byte masking to 8 bits", () => {
    const mem = new LocalMemory();
    mem.writeByte(0, 0x1ff); // 9 bits
    expect(mem.readByte(0)).toBe(0xff); // truncated to 8
  });

  it("read and write multiple bytes", () => {
    const mem = new LocalMemory();
    const data = new Uint8Array([0x01, 0x02, 0x03, 0x04]);
    mem.writeBytes(0, data);
    const result = mem.readBytes(0, 4);
    expect(Array.from(result)).toEqual([0x01, 0x02, 0x03, 0x04]);
  });

  it("out of bounds read throws", () => {
    const mem = new LocalMemory(8);
    expect(() => mem.readByte(8)).toThrow("out of bounds");
  });

  it("out of bounds write throws", () => {
    const mem = new LocalMemory(8);
    expect(() => mem.writeByte(8, 0)).toThrow("out of bounds");
  });

  it("negative address throws", () => {
    const mem = new LocalMemory();
    expect(() => mem.readByte(-1)).toThrow();
  });

  it("multi-byte out of bounds throws", () => {
    const mem = new LocalMemory(8);
    expect(() => mem.readBytes(6, 4)).toThrow();
  });
});

describe("FloatAccess", () => {
  it("store and load FP32", () => {
    const mem = new LocalMemory();
    const value = floatToBits(3.14, FP32);
    mem.storeFloat(0, value);
    const result = mem.loadFloat(0, FP32);
    expect(bitsToFloat(result)).toBeCloseTo(3.14, 4);
  });

  it("store and load FP16", () => {
    const mem = new LocalMemory();
    const value = floatToBits(1.0, FP16);
    mem.storeFloat(0, value);
    const result = mem.loadFloat(0, FP16);
    expect(bitsToFloat(result)).toBe(1.0);
  });

  it("store and load BF16", () => {
    const mem = new LocalMemory();
    const value = floatToBits(2.0, BF16);
    mem.storeFloat(0, value);
    const result = mem.loadFloat(0, BF16);
    expect(bitsToFloat(result)).toBe(2.0);
  });

  it("FP32 uses 4 bytes", () => {
    const mem = new LocalMemory();
    const value = floatToBits(1.0, FP32);
    mem.storeFloat(0, value);
    const raw = mem.readBytes(0, 4);
    expect(raw.length).toBe(4);
    // 1.0 in FP32 is 0x3F800000, not all zeros
    expect(Array.from(raw).some((b) => b !== 0)).toBe(true);
  });

  it("FP16 uses 2 bytes", () => {
    const mem = new LocalMemory();
    const value = floatToBits(1.0, FP16);
    mem.storeFloat(0, value);
    const raw = mem.readBytes(0, 2);
    expect(raw.length).toBe(2);
    expect(Array.from(raw).some((b) => b !== 0)).toBe(true);
  });

  it("multiple floats at different addresses", () => {
    const mem = new LocalMemory();
    mem.storePythonFloat(0, 1.0);
    mem.storePythonFloat(4, 2.0);
    mem.storePythonFloat(8, 3.0);
    expect(mem.loadFloatAsPython(0)).toBe(1.0);
    expect(mem.loadFloatAsPython(4)).toBe(2.0);
    expect(mem.loadFloatAsPython(8)).toBe(3.0);
  });

  it("store and load negative float", () => {
    const mem = new LocalMemory();
    mem.storePythonFloat(0, -42.5);
    expect(mem.loadFloatAsPython(0)).toBe(-42.5);
  });

  it("store and load zero", () => {
    const mem = new LocalMemory();
    mem.storePythonFloat(0, 0.0);
    expect(mem.loadFloatAsPython(0)).toBe(0.0);
  });

  it("convenience methods with format", () => {
    const mem = new LocalMemory();
    mem.storePythonFloat(0, 2.71828, FP32);
    const result = mem.loadFloatAsPython(0, FP32);
    expect(Math.abs(result - 2.71828)).toBeLessThan(1e-5);
  });

  it("float out of bounds throws", () => {
    const mem = new LocalMemory(8);
    expect(() => mem.loadFloat(6, FP32)).toThrow(); // needs 4 bytes at 6
  });
});

describe("Dump", () => {
  it("dump of fresh memory is all zeros", () => {
    const mem = new LocalMemory(16);
    expect(mem.dump(0, 16)).toEqual(new Array(16).fill(0));
  });

  it("dump reflects written bytes", () => {
    const mem = new LocalMemory();
    mem.writeByte(0, 0xff);
    mem.writeByte(1, 0x42);
    const d = mem.dump(0, 4);
    expect(d[0]).toBe(0xff);
    expect(d[1]).toBe(0x42);
    expect(d[2]).toBe(0);
    expect(d[3]).toBe(0);
  });

  it("toString shows size and non-zero count", () => {
    const mem = new LocalMemory(64);
    expect(mem.toString()).toContain("64 bytes");
    expect(mem.toString()).toContain("0 non-zero");
    mem.writeByte(0, 1);
    expect(mem.toString()).toContain("1 non-zero");
  });
});
