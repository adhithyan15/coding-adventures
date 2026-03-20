/**
 * Tests for the GenericISA instruction set implementation.
 */

import { describe, it, expect } from "vitest";
import { GenericISA } from "../src/generic-isa.js";
import { LocalMemory } from "../src/memory.js";
import { FPRegisterFile } from "../src/registers.js";
import {
  fadd,
  fsub,
  fmul,
  ffma,
  fneg,
  fabsOp,
  load,
  store,
  mov,
  limm,
  beq,
  blt,
  bne,
  jmp,
  nop,
  halt,
} from "../src/opcodes.js";

/** Fresh ISA, register file, and memory for each test. */
function setup() {
  return {
    isa: new GenericISA(),
    regs: new FPRegisterFile(),
    mem: new LocalMemory(),
  };
}

describe("ProtocolCompliance", () => {
  it("has name 'Generic'", () => {
    expect(new GenericISA().name).toBe("Generic");
  });
});

describe("Arithmetic", () => {
  it("FADD", () => {
    const { isa, regs, mem } = setup();
    regs.writeFloat(0, 1.0);
    regs.writeFloat(1, 2.0);
    const result = isa.execute(fadd(2, 0, 1), regs, mem);
    expect(regs.readFloat(2)).toBe(3.0);
    expect(result.registersChanged).toEqual({ R2: 3.0 });
  });

  it("FADD negative", () => {
    const { isa, regs, mem } = setup();
    regs.writeFloat(0, 1.0);
    regs.writeFloat(1, -3.0);
    isa.execute(fadd(2, 0, 1), regs, mem);
    expect(regs.readFloat(2)).toBe(-2.0);
  });

  it("FSUB", () => {
    const { isa, regs, mem } = setup();
    regs.writeFloat(0, 5.0);
    regs.writeFloat(1, 3.0);
    isa.execute(fsub(2, 0, 1), regs, mem);
    expect(regs.readFloat(2)).toBe(2.0);
  });

  it("FMUL", () => {
    const { isa, regs, mem } = setup();
    regs.writeFloat(0, 3.0);
    regs.writeFloat(1, 4.0);
    isa.execute(fmul(2, 0, 1), regs, mem);
    expect(regs.readFloat(2)).toBe(12.0);
  });

  it("FMUL by zero", () => {
    const { isa, regs, mem } = setup();
    regs.writeFloat(0, 42.0);
    regs.writeFloat(1, 0.0);
    isa.execute(fmul(2, 0, 1), regs, mem);
    expect(regs.readFloat(2)).toBe(0.0);
  });

  it("FFMA: Rd = Rs1 * Rs2 + Rs3", () => {
    const { isa, regs, mem } = setup();
    regs.writeFloat(0, 2.0);
    regs.writeFloat(1, 3.0);
    regs.writeFloat(2, 1.0);
    const result = isa.execute(ffma(3, 0, 1, 2), regs, mem);
    expect(regs.readFloat(3)).toBe(7.0);
    expect(result.registersChanged).toBeTruthy();
    expect(result.registersChanged!["R3"]).toBeDefined();
  });

  it("FNEG", () => {
    const { isa, regs, mem } = setup();
    regs.writeFloat(0, 5.0);
    isa.execute(fneg(1, 0), regs, mem);
    expect(regs.readFloat(1)).toBe(-5.0);
  });

  it("FNEG double negation", () => {
    const { isa, regs, mem } = setup();
    regs.writeFloat(0, 3.0);
    isa.execute(fneg(1, 0), regs, mem);
    isa.execute(fneg(2, 1), regs, mem);
    expect(regs.readFloat(2)).toBe(3.0);
  });

  it("FABS positive", () => {
    const { isa, regs, mem } = setup();
    regs.writeFloat(0, 5.0);
    isa.execute(fabsOp(1, 0), regs, mem);
    expect(regs.readFloat(1)).toBe(5.0);
  });

  it("FABS negative", () => {
    const { isa, regs, mem } = setup();
    regs.writeFloat(0, -5.0);
    isa.execute(fabsOp(1, 0), regs, mem);
    expect(regs.readFloat(1)).toBe(5.0);
  });
});

describe("Memory", () => {
  it("store and load", () => {
    const { isa, regs, mem } = setup();
    regs.writeFloat(0, 0.0); // base address
    regs.writeFloat(1, 3.14);
    isa.execute(store(0, 1, 0.0), regs, mem);
    isa.execute(load(2, 0, 0.0), regs, mem);
    expect(regs.readFloat(2)).toBeCloseTo(3.14, 4);
  });

  it("store with offset", () => {
    const { isa, regs, mem } = setup();
    regs.writeFloat(0, 0.0);
    regs.writeFloat(1, 42.0);
    isa.execute(store(0, 1, 8.0), regs, mem);
    isa.execute(load(2, 0, 8.0), regs, mem);
    expect(regs.readFloat(2)).toBe(42.0);
  });

  it("store returns memoryChanged", () => {
    const { isa, regs, mem } = setup();
    regs.writeFloat(0, 0.0);
    regs.writeFloat(1, 5.0);
    const result = isa.execute(store(0, 1, 0.0), regs, mem);
    expect(result.memoryChanged).toBeTruthy();
    expect(result.memoryChanged![0]).toBeDefined();
  });

  it("load returns registersChanged", () => {
    const { isa, regs, mem } = setup();
    mem.storePythonFloat(0, 7.0);
    regs.writeFloat(0, 0.0);
    const result = isa.execute(load(1, 0, 0.0), regs, mem);
    expect(result.registersChanged).toBeTruthy();
    expect(result.registersChanged!["R1"]).toBeDefined();
  });
});

describe("DataMovement", () => {
  it("MOV", () => {
    const { isa, regs, mem } = setup();
    regs.writeFloat(0, 42.0);
    isa.execute(mov(1, 0), regs, mem);
    expect(regs.readFloat(1)).toBe(42.0);
  });

  it("LIMM", () => {
    const { isa, regs, mem } = setup();
    isa.execute(limm(0, 3.14), regs, mem);
    expect(Math.abs(regs.readFloat(0) - 3.14)).toBeLessThan(0.01);
  });

  it("LIMM negative", () => {
    const { isa, regs, mem } = setup();
    isa.execute(limm(0, -99.0), regs, mem);
    expect(regs.readFloat(0)).toBe(-99.0);
  });

  it("LIMM zero", () => {
    const { isa, regs, mem } = setup();
    isa.execute(limm(0, 0.0), regs, mem);
    expect(regs.readFloat(0)).toBe(0.0);
  });
});

describe("ControlFlow", () => {
  it("BEQ taken", () => {
    const { isa, regs, mem } = setup();
    regs.writeFloat(0, 5.0);
    regs.writeFloat(1, 5.0);
    const result = isa.execute(beq(0, 1, 3), regs, mem);
    expect(result.nextPcOffset).toBe(3);
  });

  it("BEQ not taken", () => {
    const { isa, regs, mem } = setup();
    regs.writeFloat(0, 5.0);
    regs.writeFloat(1, 3.0);
    const result = isa.execute(beq(0, 1, 3), regs, mem);
    expect(result.nextPcOffset).toBe(1);
  });

  it("BLT taken", () => {
    const { isa, regs, mem } = setup();
    regs.writeFloat(0, 2.0);
    regs.writeFloat(1, 5.0);
    const result = isa.execute(blt(0, 1, 4), regs, mem);
    expect(result.nextPcOffset).toBe(4);
  });

  it("BLT not taken", () => {
    const { isa, regs, mem } = setup();
    regs.writeFloat(0, 5.0);
    regs.writeFloat(1, 2.0);
    const result = isa.execute(blt(0, 1, 4), regs, mem);
    expect(result.nextPcOffset).toBe(1);
  });

  it("BNE taken", () => {
    const { isa, regs, mem } = setup();
    regs.writeFloat(0, 1.0);
    regs.writeFloat(1, 2.0);
    const result = isa.execute(bne(0, 1, 2), regs, mem);
    expect(result.nextPcOffset).toBe(2);
  });

  it("BNE not taken", () => {
    const { isa, regs, mem } = setup();
    regs.writeFloat(0, 5.0);
    regs.writeFloat(1, 5.0);
    const result = isa.execute(bne(0, 1, 2), regs, mem);
    expect(result.nextPcOffset).toBe(1);
  });

  it("JMP sets absolute PC", () => {
    const { isa, regs, mem } = setup();
    const result = isa.execute(jmp(10), regs, mem);
    expect(result.nextPcOffset).toBe(10);
    expect(result.absoluteJump).toBe(true);
  });

  it("NOP advances PC", () => {
    const { isa, regs, mem } = setup();
    const result = isa.execute(nop(), regs, mem);
    expect(result.nextPcOffset).toBe(1);
    expect(result.halted).toBe(false);
  });

  it("HALT sets halted flag", () => {
    const { isa, regs, mem } = setup();
    const result = isa.execute(halt(), regs, mem);
    expect(result.halted).toBe(true);
  });
});

describe("Descriptions", () => {
  it("FADD description includes values", () => {
    const { isa, regs, mem } = setup();
    regs.writeFloat(0, 1.0);
    regs.writeFloat(1, 2.0);
    const result = isa.execute(fadd(2, 0, 1), regs, mem);
    expect(result.description).toContain("1");
    expect(result.description).toContain("2");
    expect(result.description).toContain("3");
  });

  it("FFMA description includes result", () => {
    const { isa, regs, mem } = setup();
    regs.writeFloat(0, 2.0);
    regs.writeFloat(1, 3.0);
    regs.writeFloat(2, 1.0);
    const result = isa.execute(ffma(3, 0, 1, 2), regs, mem);
    expect(result.description).toContain("7");
  });

  it("branch description when taken", () => {
    const { isa, regs, mem } = setup();
    regs.writeFloat(0, 5.0);
    regs.writeFloat(1, 5.0);
    const result = isa.execute(beq(0, 1, 3), regs, mem);
    expect(result.description.toLowerCase()).toContain("branch");
  });

  it("branch description when not taken", () => {
    const { isa, regs, mem } = setup();
    regs.writeFloat(0, 1.0);
    regs.writeFloat(1, 2.0);
    const result = isa.execute(beq(0, 1, 3), regs, mem);
    expect(result.description.toLowerCase()).toContain("fall through");
  });
});
