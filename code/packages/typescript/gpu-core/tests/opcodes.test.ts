/**
 * Tests for opcodes and instruction construction.
 */

import { describe, it, expect } from "vitest";
import {
  Opcode,
  formatInstruction,
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

describe("Opcode", () => {
  it("all 16 opcodes exist", () => {
    const values = Object.values(Opcode);
    expect(values.length).toBe(16);
  });

  it("opcode values are lowercase strings", () => {
    expect(Opcode.FADD).toBe("fadd");
    expect(Opcode.HALT).toBe("halt");
  });
});

describe("Instruction", () => {
  it("instructions are frozen (readonly)", () => {
    const inst = fadd(2, 0, 1);
    // TypeScript's readonly enforces this at compile time;
    // at runtime, Object.freeze is not used, but the interface
    // discourages mutation.
    expect(inst.opcode).toBe(Opcode.FADD);
    expect(inst.rd).toBe(2);
  });

  it("defaults are zero", () => {
    const inst = nop();
    expect(inst.rd).toBe(0);
    expect(inst.rs1).toBe(0);
    expect(inst.rs2).toBe(0);
    expect(inst.rs3).toBe(0);
    expect(inst.immediate).toBe(0);
  });

  it("formatInstruction FADD shows assembly syntax", () => {
    expect(formatInstruction(fadd(2, 0, 1))).toBe("FADD R2, R0, R1");
  });

  it("formatInstruction FFMA shows four registers", () => {
    expect(formatInstruction(ffma(3, 0, 1, 2))).toBe(
      "FFMA R3, R0, R1, R2",
    );
  });

  it("formatInstruction LIMM shows immediate", () => {
    const s = formatInstruction(limm(0, 3.14));
    expect(s).toContain("3.14");
  });

  it("formatInstruction LOAD shows memory syntax", () => {
    const s = formatInstruction(load(0, 1, 4.0));
    expect(s).toContain("LOAD");
    expect(s).toContain("[R1+");
  });

  it("formatInstruction STORE shows memory syntax", () => {
    const s = formatInstruction(store(1, 2, 8.0));
    expect(s).toContain("STORE");
  });

  it("formatInstruction BEQ shows branch offset", () => {
    const s = formatInstruction(beq(0, 1, 3));
    expect(s).toContain("BEQ");
    expect(s).toContain("+3");
  });

  it("formatInstruction BEQ negative offset", () => {
    const s = formatInstruction(beq(0, 1, -2));
    expect(s).toContain("-2");
  });

  it("formatInstruction HALT", () => {
    expect(formatInstruction(halt())).toBe("HALT");
  });

  it("formatInstruction NOP", () => {
    expect(formatInstruction(nop())).toBe("NOP");
  });

  it("formatInstruction JMP shows target", () => {
    const s = formatInstruction(jmp(5));
    expect(s).toContain("JMP");
    expect(s).toContain("5");
  });
});

describe("HelperConstructors", () => {
  it("fadd", () => {
    const inst = fadd(2, 0, 1);
    expect(inst.opcode).toBe(Opcode.FADD);
    expect(inst.rd).toBe(2);
    expect(inst.rs1).toBe(0);
    expect(inst.rs2).toBe(1);
  });

  it("fsub", () => {
    expect(fsub(2, 0, 1).opcode).toBe(Opcode.FSUB);
  });

  it("fmul", () => {
    expect(fmul(2, 0, 1).opcode).toBe(Opcode.FMUL);
  });

  it("ffma", () => {
    const inst = ffma(3, 0, 1, 2);
    expect(inst.opcode).toBe(Opcode.FFMA);
    expect(inst.rs3).toBe(2);
  });

  it("fneg", () => {
    const inst = fneg(1, 0);
    expect(inst.opcode).toBe(Opcode.FNEG);
    expect(inst.rd).toBe(1);
    expect(inst.rs1).toBe(0);
  });

  it("fabsOp", () => {
    expect(fabsOp(1, 0).opcode).toBe(Opcode.FABS);
  });

  it("load", () => {
    const inst = load(0, 1, 4.0);
    expect(inst.opcode).toBe(Opcode.LOAD);
    expect(inst.rd).toBe(0);
    expect(inst.rs1).toBe(1);
    expect(inst.immediate).toBe(4.0);
  });

  it("load default offset", () => {
    expect(load(0, 1).immediate).toBe(0);
  });

  it("store", () => {
    const inst = store(1, 2, 8.0);
    expect(inst.opcode).toBe(Opcode.STORE);
    expect(inst.rs1).toBe(1);
    expect(inst.rs2).toBe(2);
    expect(inst.immediate).toBe(8.0);
  });

  it("mov", () => {
    expect(mov(1, 0).opcode).toBe(Opcode.MOV);
  });

  it("limm", () => {
    const inst = limm(0, 3.14);
    expect(inst.opcode).toBe(Opcode.LIMM);
    expect(inst.immediate).toBe(3.14);
  });

  it("beq", () => {
    const inst = beq(0, 1, 3);
    expect(inst.opcode).toBe(Opcode.BEQ);
    expect(inst.immediate).toBe(3);
  });

  it("blt", () => {
    const inst = blt(0, 1, -2);
    expect(inst.opcode).toBe(Opcode.BLT);
    expect(inst.immediate).toBe(-2);
  });

  it("bne", () => {
    expect(bne(0, 1, 5).opcode).toBe(Opcode.BNE);
  });

  it("jmp", () => {
    const inst = jmp(10);
    expect(inst.opcode).toBe(Opcode.JMP);
    expect(inst.immediate).toBe(10);
  });

  it("nop", () => {
    expect(nop().opcode).toBe(Opcode.NOP);
  });

  it("halt", () => {
    expect(halt().opcode).toBe(Opcode.HALT);
  });
});
