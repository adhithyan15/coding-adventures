/**
 * Tests for the FPRegisterFile.
 */

import { describe, it, expect } from "vitest";
import { FP32, FP16, BF16, floatToBits } from "@coding-adventures/fp-arithmetic";
import { FPRegisterFile } from "../src/registers.js";

describe("Construction", () => {
  it("default: 32 FP32 registers, all zero", () => {
    const rf = new FPRegisterFile();
    expect(rf.numRegisters).toBe(32);
    expect(rf.fmt).toEqual(FP32);
    expect(rf.readFloat(0)).toBe(0.0);
    expect(rf.readFloat(31)).toBe(0.0);
  });

  it("custom register count", () => {
    const rf = new FPRegisterFile(64);
    expect(rf.numRegisters).toBe(64);
    rf.writeFloat(63, 1.0);
    expect(rf.readFloat(63)).toBe(1.0);
  });

  it("NVIDIA scale: 255 registers", () => {
    const rf = new FPRegisterFile(255);
    rf.writeFloat(254, 42.0);
    expect(rf.readFloat(254)).toBe(42.0);
  });

  it("maximum 256 registers", () => {
    const rf = new FPRegisterFile(256);
    expect(rf.numRegisters).toBe(256);
  });

  it("rejects 0 registers", () => {
    expect(() => new FPRegisterFile(0)).toThrow("num_registers must be 1-256");
  });

  it("rejects >256 registers", () => {
    expect(() => new FPRegisterFile(257)).toThrow("num_registers must be 1-256");
  });

  it("FP16 format", () => {
    const rf = new FPRegisterFile(32, FP16);
    expect(rf.fmt).toEqual(FP16);
    rf.writeFloat(0, 1.0);
    expect(rf.readFloat(0)).toBe(1.0);
  });

  it("BF16 format", () => {
    const rf = new FPRegisterFile(32, BF16);
    expect(rf.fmt).toEqual(BF16);
    rf.writeFloat(0, 1.0);
    expect(rf.readFloat(0)).toBe(1.0);
  });
});

describe("ReadWrite", () => {
  it("write and read FloatBits", () => {
    const rf = new FPRegisterFile();
    const value = floatToBits(3.14, FP32);
    rf.write(0, value);
    const result = rf.read(0);
    expect(result).toEqual(value);
  });

  it("write and read float", () => {
    const rf = new FPRegisterFile();
    rf.writeFloat(5, 2.71828);
    const result = rf.readFloat(5);
    expect(Math.abs(result - 2.71828)).toBeLessThan(1e-5);
  });

  it("write negative value", () => {
    const rf = new FPRegisterFile();
    rf.writeFloat(0, -42.0);
    expect(rf.readFloat(0)).toBe(-42.0);
  });

  it("write zero", () => {
    const rf = new FPRegisterFile();
    rf.writeFloat(0, 99.0);
    rf.writeFloat(0, 0.0);
    expect(rf.readFloat(0)).toBe(0.0);
  });

  it("overwrite replaces value", () => {
    const rf = new FPRegisterFile();
    rf.writeFloat(0, 1.0);
    rf.writeFloat(0, 2.0);
    expect(rf.readFloat(0)).toBe(2.0);
  });

  it("independent registers", () => {
    const rf = new FPRegisterFile();
    rf.writeFloat(0, 1.0);
    rf.writeFloat(1, 2.0);
    expect(rf.readFloat(0)).toBe(1.0);
    expect(rf.readFloat(1)).toBe(2.0);
  });

  it("read out of bounds throws", () => {
    const rf = new FPRegisterFile(8);
    expect(() => rf.read(8)).toThrow("Register index 8");
  });

  it("write out of bounds throws", () => {
    const rf = new FPRegisterFile(8);
    expect(() => rf.write(8, floatToBits(1.0, FP32))).toThrow(
      "Register index 8",
    );
  });

  it("negative index throws", () => {
    const rf = new FPRegisterFile();
    expect(() => rf.read(-1)).toThrow();
  });
});

describe("Dump", () => {
  it("dump of all-zero registers returns empty", () => {
    const rf = new FPRegisterFile();
    expect(rf.dump()).toEqual({});
  });

  it("dump includes only non-zero registers", () => {
    const rf = new FPRegisterFile();
    rf.writeFloat(0, 1.0);
    rf.writeFloat(5, 3.14);
    const result = rf.dump();
    expect("R0" in result).toBe(true);
    expect("R5" in result).toBe(true);
    expect(Object.keys(result).length).toBe(2);
  });

  it("dumpAll includes all registers including zeros", () => {
    const rf = new FPRegisterFile(4);
    rf.writeFloat(0, 1.0);
    const result = rf.dumpAll();
    expect(Object.keys(result).length).toBe(4);
    expect(result["R0"]).toBe(1.0);
    expect(result["R1"]).toBe(0.0);
  });

  it("toString shows 'all zero' for fresh register file", () => {
    const rf = new FPRegisterFile();
    expect(rf.toString()).toContain("all zero");
  });

  it("toString shows non-zero register values", () => {
    const rf = new FPRegisterFile();
    rf.writeFloat(0, 3.0);
    expect(rf.toString()).toContain("R0=3");
  });
});
