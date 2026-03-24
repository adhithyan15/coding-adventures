/**
 * Tests for pre-assembled ARM1 demo programs.
 *
 * Each test loads a program into an ARM1 CPU, runs it to completion,
 * and verifies the final register state matches the expected result.
 */

import { describe, it, expect } from "vitest";
import { ARM1 } from "@coding-adventures/arm1-simulator";
import { FIBONACCI, SUM_1_TO_10, ARRAY_MAX, BARREL_SHIFTER_DEMO } from "./programs.js";

function runProgram(prog: { code: number[]; data?: number[]; dataAddr?: number }): number[] {
  const cpu = new ARM1(4096);
  cpu.loadProgram(prog.code, 0);
  if (prog.data && prog.dataAddr !== undefined) {
    cpu.loadProgram(prog.data, prog.dataAddr);
  }
  cpu.reset();
  cpu.run(10000);
  const regs: number[] = [];
  for (let i = 0; i < 16; i++) regs.push(cpu.readRegister(i));
  return regs;
}

describe("FIBONACCI", () => {
  it("computes fib(10) = 55 in R0", () => {
    const regs = runProgram(FIBONACCI);
    expect(regs[0]).toBe(55);
  });

  it("has correct byte count (12 instructions × 4 bytes = 48)", () => {
    expect(FIBONACCI.code.length).toBe(48);
  });

  it("encoding is little-endian (first byte is 0x0A for MOV R0,#10)", () => {
    // E3A0000A → bytes [0x0A, 0x00, 0xA0, 0xE3]
    expect(FIBONACCI.code[0]).toBe(0x0A);
    expect(FIBONACCI.code[1]).toBe(0x00);
    expect(FIBONACCI.code[2]).toBe(0xA0);
    expect(FIBONACCI.code[3]).toBe(0xE3);
  });
});

describe("SUM_1_TO_10", () => {
  it("computes 1+2+...+10 = 55 in R1", () => {
    const regs = runProgram(SUM_1_TO_10);
    expect(regs[1]).toBe(55);
  });

  it("R0 is 0 after program (decremented to zero)", () => {
    const regs = runProgram(SUM_1_TO_10);
    expect(regs[0]).toBe(0);
  });

  it("has correct byte count (6 instructions × 4 = 24)", () => {
    expect(SUM_1_TO_10.code.length).toBe(24);
  });
});

describe("ARRAY_MAX", () => {
  it("finds maximum 9 in [5,2,8,1,9,3,7,0] and stores in R1", () => {
    const regs = runProgram(ARRAY_MAX);
    expect(regs[1]).toBe(9);
  });

  it("has data at the expected address (0x200)", () => {
    expect(ARRAY_MAX.dataAddr).toBe(0x200);
  });

  it("data starts with 5 (first array element in little-endian)", () => {
    expect(ARRAY_MAX.data![0]).toBe(5);   // low byte of 0x00000005
    expect(ARRAY_MAX.data![1]).toBe(0);
    expect(ARRAY_MAX.data![2]).toBe(0);
    expect(ARRAY_MAX.data![3]).toBe(0);
  });
});

describe("BARREL_SHIFTER_DEMO", () => {
  // Start: R0 = 0xA5 = 165
  // After LSL #8: R0 = 0x0000A500 = 42240
  // After LSR #4: R1 = 0x00000A50 = 2640
  // After ASR #4: R2 = 0x00000A50 = 2640  (positive, same as LSR)
  // After ROR #8: R3 = 0x000000A5 = 165   (bits rotate back to original position)

  it("LSL #8 gives R0 = 0xA500 = 42240", () => {
    const regs = runProgram(BARREL_SHIFTER_DEMO);
    expect(regs[0]).toBe(0xA500);
  });

  it("LSR #4 gives R1 = 0x0A50 = 2640", () => {
    const regs = runProgram(BARREL_SHIFTER_DEMO);
    expect(regs[1]).toBe(0x0A50);
  });

  it("ASR #4 of positive value gives R2 = 0x0A50 = 2640", () => {
    const regs = runProgram(BARREL_SHIFTER_DEMO);
    expect(regs[2]).toBe(0x0A50);
  });

  it("ROR #8 wraps bits back: R3 = 0xA5 = 165 (original value restored)", () => {
    const regs = runProgram(BARREL_SHIFTER_DEMO);
    expect(regs[3]).toBe(0xA5);
  });

  it("has correct byte count (6 instructions × 4 = 24)", () => {
    expect(BARREL_SHIFTER_DEMO.code.length).toBe(24);
  });
});
