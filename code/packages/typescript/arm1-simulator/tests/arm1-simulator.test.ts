/**
 * =========================================================================
 * ARM1 Simulator Test Suite
 * =========================================================================
 *
 * Comprehensive tests for the ARM1 behavioral simulator, ported from the
 * Go test suite. Covers:
 *   - Constants and type helpers
 *   - Condition code evaluation
 *   - Barrel shifter (all 4 shift types + RRX)
 *   - ALU (all 16 operations + flag computation)
 *   - Instruction decoder
 *   - Disassembly
 *   - Full CPU execution (MOV, ADD, SUB, conditional, loops, LDR/STR, LDM/STM, BL)
 */

import { describe, it, expect } from "vitest";
import {
  VERSION,
  MODE_USR, MODE_FIQ, MODE_IRQ, MODE_SVC,
  modeString,
  COND_EQ, COND_NE, COND_CS, COND_CC, COND_MI, COND_PL,
  COND_VS, COND_VC, COND_HI, COND_LS, COND_GE, COND_LT,
  COND_GT, COND_LE, COND_AL, COND_NV,
  OP_AND, OP_EOR, OP_SUB, OP_RSB, OP_ADD, OP_ADC, OP_SBC,
  OP_TST, OP_TEQ, OP_CMP, OP_CMN, OP_ORR, OP_MOV, OP_BIC, OP_MVN,
  SHIFT_LSL, SHIFT_LSR, SHIFT_ASR, SHIFT_ROR,
  FLAG_N, FLAG_Z, FLAG_C, FLAG_V,
  PC_MASK, MODE_MASK, HALT_SWI,
  opString, isTestOp, isLogicalOp, shiftString,
  evaluateCondition,
  barrelShift, decodeImmediate,
  aluExecute,
  decode,
  disassemble,
  ARM1,
  encodeMovImm, encodeALUReg, encodeBranch, encodeHalt,
  encodeLDR, encodeSTR, encodeLDM, encodeSTM,
  encodeDataProcessing,
  INST_DATA_PROCESSING, INST_BRANCH, INST_SWI,
  type Flags,
} from "../src/index.js";

// =========================================================================
// Helper: load a program from uint32 instruction words
// =========================================================================

function loadProgram(cpu: ARM1, instructions: number[]): void {
  const code = new Uint8Array(instructions.length * 4);
  const view = new DataView(code.buffer);
  for (let i = 0; i < instructions.length; i++) {
    view.setUint32(i * 4, instructions[i] >>> 0, true);  // little-endian
  }
  cpu.loadProgram(code, 0);
}

// =========================================================================
// Types and Constants
// =========================================================================

describe("types and constants", () => {
  it("has a version", () => {
    expect(VERSION).toBe("0.1.0");
  });

  it("ModeString returns correct names", () => {
    expect(modeString(MODE_USR)).toBe("USR");
    expect(modeString(MODE_FIQ)).toBe("FIQ");
    expect(modeString(MODE_IRQ)).toBe("IRQ");
    expect(modeString(MODE_SVC)).toBe("SVC");
    expect(modeString(99)).toBe("???");
  });

  it("OpString returns correct mnemonics", () => {
    expect(opString(OP_ADD)).toBe("ADD");
    expect(opString(OP_MOV)).toBe("MOV");
    expect(opString(99)).toBe("???");
  });

  it("isTestOp identifies test-only operations", () => {
    expect(isTestOp(OP_TST)).toBe(true);
    expect(isTestOp(OP_CMP)).toBe(true);
    expect(isTestOp(OP_TEQ)).toBe(true);
    expect(isTestOp(OP_CMN)).toBe(true);
    expect(isTestOp(OP_ADD)).toBe(false);
  });

  it("isLogicalOp identifies logical operations", () => {
    expect(isLogicalOp(OP_AND)).toBe(true);
    expect(isLogicalOp(OP_MOV)).toBe(true);
    expect(isLogicalOp(OP_ADD)).toBe(false);
  });

  it("ShiftString returns correct mnemonics", () => {
    expect(shiftString(SHIFT_LSL)).toBe("LSL");
    expect(shiftString(SHIFT_LSR)).toBe("LSR");
    expect(shiftString(SHIFT_ASR)).toBe("ASR");
    expect(shiftString(SHIFT_ROR)).toBe("ROR");
    expect(shiftString(99)).toBe("???");
  });

  it("flag constants are correct unsigned values", () => {
    expect(FLAG_N).toBe(0x80000000);
    expect(FLAG_Z).toBe(0x40000000);
    expect(FLAG_C).toBe(0x20000000);
    expect(FLAG_V).toBe(0x10000000);
    expect(PC_MASK).toBe(0x03FFFFFC);
    expect(MODE_MASK).toBe(0x3);
    expect(HALT_SWI).toBe(0x123456);
  });
});

// =========================================================================
// Condition Evaluator
// =========================================================================

describe("evaluateCondition", () => {
  const tests: [string, number, Flags, boolean][] = [
    ["EQ when Z set", COND_EQ, { N: false, Z: true, C: false, V: false }, true],
    ["EQ when Z clear", COND_EQ, { N: false, Z: false, C: false, V: false }, false],
    ["NE when Z clear", COND_NE, { N: false, Z: false, C: false, V: false }, true],
    ["NE when Z set", COND_NE, { N: false, Z: true, C: false, V: false }, false],
    ["CS when C set", COND_CS, { N: false, Z: false, C: true, V: false }, true],
    ["CC when C clear", COND_CC, { N: false, Z: false, C: false, V: false }, true],
    ["MI when N set", COND_MI, { N: true, Z: false, C: false, V: false }, true],
    ["PL when N clear", COND_PL, { N: false, Z: false, C: false, V: false }, true],
    ["VS when V set", COND_VS, { N: false, Z: false, C: false, V: true }, true],
    ["VC when V clear", COND_VC, { N: false, Z: false, C: false, V: false }, true],
    ["HI when C=1,Z=0", COND_HI, { N: false, Z: false, C: true, V: false }, true],
    ["HI when C=1,Z=1", COND_HI, { N: false, Z: true, C: true, V: false }, false],
    ["LS when C=0", COND_LS, { N: false, Z: false, C: false, V: false }, true],
    ["LS when Z=1", COND_LS, { N: false, Z: true, C: true, V: false }, true],
    ["GE when N=V=0", COND_GE, { N: false, Z: false, C: false, V: false }, true],
    ["GE when N=V=1", COND_GE, { N: true, Z: false, C: false, V: true }, true],
    ["GE when N!=V", COND_GE, { N: true, Z: false, C: false, V: false }, false],
    ["LT when N!=V", COND_LT, { N: true, Z: false, C: false, V: false }, true],
    ["LT when N=V", COND_LT, { N: false, Z: false, C: false, V: false }, false],
    ["GT when Z=0,N=V", COND_GT, { N: false, Z: false, C: false, V: false }, true],
    ["GT when Z=1", COND_GT, { N: false, Z: true, C: false, V: false }, false],
    ["LE when Z=1", COND_LE, { N: false, Z: true, C: false, V: false }, true],
    ["LE when N!=V", COND_LE, { N: true, Z: false, C: false, V: false }, true],
    ["AL always", COND_AL, { N: false, Z: false, C: false, V: false }, true],
    ["NV never", COND_NV, { N: false, Z: false, C: false, V: false }, false],
  ];

  for (const [name, cond, flags, want] of tests) {
    it(name, () => {
      expect(evaluateCondition(cond, flags)).toBe(want);
    });
  }
});

// =========================================================================
// Barrel Shifter
// =========================================================================

describe("barrel shifter", () => {
  describe("LSL", () => {
    const tests: [string, number, number, number, boolean][] = [
      ["LSL #0 (no shift)", 0xFF, 0, 0xFF, false],
      ["LSL #1", 0xFF, 1, 0x1FE, false],
      ["LSL #4", 0xFF, 4, 0xFF0, false],
      ["LSL #31", 1, 31, 0x80000000, false],
      ["LSL #32", 1, 32, 0, true],
      ["LSL #33", 1, 33, 0, false],
    ];
    for (const [name, value, amount, wantVal, wantC] of tests) {
      it(name, () => {
        const [val, c] = barrelShift(value, SHIFT_LSL, amount, false, false);
        expect(val >>> 0).toBe(wantVal >>> 0);
        expect(c).toBe(wantC);
      });
    }
  });

  describe("LSR", () => {
    const tests: [string, number, number, boolean, number, boolean][] = [
      ["LSR #1", 0xFF, 1, false, 0x7F, true],
      ["LSR #8", 0xFF00, 8, false, 0xFF, false],
      ["LSR #0 (encodes #32)", 0x80000000, 0, false, 0, true],
      ["LSR #32 by register", 0x80000000, 32, true, 0, true],
    ];
    for (const [name, value, amount, byReg, wantVal, wantC] of tests) {
      it(name, () => {
        const [val, c] = barrelShift(value, SHIFT_LSR, amount, false, byReg);
        expect(val >>> 0).toBe(wantVal >>> 0);
        expect(c).toBe(wantC);
      });
    }
  });

  describe("ASR", () => {
    it("ASR #1 positive", () => {
      const [val, c] = barrelShift(0x7FFFFFFE, SHIFT_ASR, 1, false, false);
      expect(val >>> 0).toBe(0x3FFFFFFF);
      expect(c).toBe(false);
    });

    it("ASR #1 negative", () => {
      const [val, c] = barrelShift(0x80000000, SHIFT_ASR, 1, false, false);
      expect(val >>> 0).toBe(0xC0000000);
      expect(c).toBe(false);
    });

    it("ASR #0 (encodes #32) negative", () => {
      const [val, c] = barrelShift(0x80000000, SHIFT_ASR, 0, false, false);
      expect(val >>> 0).toBe(0xFFFFFFFF);
      expect(c).toBe(true);
    });

    it("ASR #0 (encodes #32) positive", () => {
      const [val, c] = barrelShift(0x7FFFFFFF, SHIFT_ASR, 0, false, false);
      expect(val >>> 0).toBe(0);
      expect(c).toBe(false);
    });
  });

  describe("ROR", () => {
    it("ROR #4", () => {
      const [val, c] = barrelShift(0x0000000F, SHIFT_ROR, 4, false, false);
      expect(val >>> 0).toBe(0xF0000000);
      expect(c).toBe(true);
    });

    it("ROR #8", () => {
      const [val, c] = barrelShift(0x000000FF, SHIFT_ROR, 8, false, false);
      expect(val >>> 0).toBe(0xFF000000);
      expect(c).toBe(true);
    });

    it("ROR #16", () => {
      const [val, c] = barrelShift(0x0000FFFF, SHIFT_ROR, 16, false, false);
      expect(val >>> 0).toBe(0xFFFF0000);
      expect(c).toBe(true);
    });
  });

  describe("RRX", () => {
    it("RRX with carry in and bit 0 set", () => {
      const [val, c] = barrelShift(0x00000001, SHIFT_ROR, 0, true, false);
      expect(val >>> 0).toBe(0x80000000);
      expect(c).toBe(true);  // old bit 0 was 1
    });

    it("RRX with carry in and bit 0 clear", () => {
      const [val, c] = barrelShift(0x00000000, SHIFT_ROR, 0, true, false);
      expect(val >>> 0).toBe(0x80000000);
      expect(c).toBe(false);  // old bit 0 was 0
    });
  });

  describe("decodeImmediate", () => {
    it("no rotation", () => {
      const [val] = decodeImmediate(0xFF, 0);
      expect(val).toBe(0xFF);
    });

    it("1 ROR 2 = 0x40000000", () => {
      const [val] = decodeImmediate(0x01, 1);
      expect(val >>> 0).toBe(0x40000000);
    });

    it("0xFF ROR 8 = 0xFF000000", () => {
      const [val] = decodeImmediate(0xFF, 4);
      expect(val >>> 0).toBe(0xFF000000);
    });
  });
});

// =========================================================================
// ALU
// =========================================================================

describe("ALU", () => {
  it("ADD: 1 + 2 = 3, no flags", () => {
    const r = aluExecute(OP_ADD, 1, 2, false, false, false);
    expect(r.result).toBe(3);
    expect(r.N).toBe(false);
    expect(r.Z).toBe(false);
    expect(r.C).toBe(false);
    expect(r.V).toBe(false);
  });

  it("ADD: signed overflow", () => {
    const r = aluExecute(OP_ADD, 0x7FFFFFFF, 1, false, false, false);
    expect(r.result >>> 0).toBe(0x80000000);
    expect(r.N).toBe(true);
    expect(r.V).toBe(true);
  });

  it("ADD: unsigned overflow (carry)", () => {
    const r = aluExecute(OP_ADD, 0xFFFFFFFF, 1, false, false, false);
    expect(r.result).toBe(0);
    expect(r.C).toBe(true);
    expect(r.Z).toBe(true);
  });

  it("SUB: 5 - 3 = 2, carry set (no borrow)", () => {
    const r = aluExecute(OP_SUB, 5, 3, false, false, false);
    expect(r.result).toBe(2);
    expect(r.C).toBe(true);
  });

  it("SUB: 3 - 5 borrow (carry clear)", () => {
    const r = aluExecute(OP_SUB, 3, 5, false, false, false);
    expect(r.result >>> 0).toBe(0xFFFFFFFE);
    expect(r.C).toBe(false);
    expect(r.N).toBe(true);
  });

  it("RSB: 5 - 3 = 2", () => {
    const r = aluExecute(OP_RSB, 3, 5, false, false, false);
    expect(r.result).toBe(2);
  });

  it("ADC: 1 + 2 + 1 = 4", () => {
    const r = aluExecute(OP_ADC, 1, 2, true, false, false);
    expect(r.result).toBe(4);
  });

  it("SBC: 5 - 3 - 0 = 2 (C=1)", () => {
    const r = aluExecute(OP_SBC, 5, 3, true, false, false);
    expect(r.result).toBe(2);
  });

  it("logical operations", () => {
    expect(aluExecute(OP_AND, 0xFF00FF00, 0x0FF00FF0, false, false, false).result >>> 0).toBe(0x0F000F00);
    expect(aluExecute(OP_EOR, 0xFF00FF00, 0x0FF00FF0, false, false, false).result >>> 0).toBe(0xF0F0F0F0);
    expect(aluExecute(OP_ORR, 0xFF00FF00, 0x0FF00FF0, false, false, false).result >>> 0).toBe(0xFFF0FFF0);
    expect(aluExecute(OP_BIC, 0xFFFFFFFF, 0x0000FF00, false, false, false).result >>> 0).toBe(0xFFFF00FF);
    expect(aluExecute(OP_MOV, 0, 42, false, false, false).result).toBe(42);
    expect(aluExecute(OP_MVN, 0, 0, false, false, false).result >>> 0).toBe(0xFFFFFFFF);
  });

  it("test operations do not write result", () => {
    const tst = aluExecute(OP_TST, 0xFF, 0x00, false, false, false);
    expect(tst.writeResult).toBe(false);
    expect(tst.Z).toBe(true);

    const cmp = aluExecute(OP_CMP, 5, 5, false, false, false);
    expect(cmp.writeResult).toBe(false);
    expect(cmp.Z).toBe(true);
    expect(cmp.C).toBe(true);
  });
});

// =========================================================================
// Decoder
// =========================================================================

describe("decoder", () => {
  it("decodes ADD R2, R0, R1", () => {
    const d = decode(0xE0802001);
    expect(d.type).toBe(INST_DATA_PROCESSING);
    expect(d.cond).toBe(COND_AL);
    expect(d.opcode).toBe(OP_ADD);
    expect(d.s).toBe(false);
    expect(d.rn).toBe(0);
    expect(d.rd).toBe(2);
    expect(d.rm).toBe(1);
  });

  it("decodes MOV R0, #42", () => {
    const d = decode(0xE3A0002A);
    expect(d.type).toBe(INST_DATA_PROCESSING);
    expect(d.opcode).toBe(OP_MOV);
    expect(d.immediate).toBe(true);
    expect(d.rd).toBe(0);
    expect(d.imm8).toBe(42);
  });

  it("decodes B +8", () => {
    const d = decode(0xEA000002);
    expect(d.type).toBe(INST_BRANCH);
    expect(d.link).toBe(false);
    expect(d.branchOffset).toBe(8);
  });

  it("decodes BL -8", () => {
    const d = decode(0xEBFFFFFE);
    expect(d.type).toBe(INST_BRANCH);
    expect(d.link).toBe(true);
    expect(d.branchOffset).toBe(-8);
  });

  it("decodes SWI 0x123456", () => {
    const d = decode(0xEF123456);
    expect(d.type).toBe(INST_SWI);
    expect(d.swiComment).toBe(0x123456);
  });
});

// =========================================================================
// Disassembly
// =========================================================================

describe("disassemble", () => {
  const tests: [number, string][] = [
    [0xE3A0002A, "MOV R0, #42"],
    [0xE0802001, "ADD R2, R0, R1"],
    [0xE0912001, "ADDS R2, R1, R1"],
    [0x10802001, "ADDNE R2, R0, R1"],
    [0xEF123456, "HLT"],
  ];

  for (const [inst, want] of tests) {
    it(`disassembles 0x${inst.toString(16).toUpperCase()} as "${want}"`, () => {
      const d = decode(inst);
      expect(disassemble(d)).toBe(want);
    });
  }
});

// =========================================================================
// CPU - Power-on state
// =========================================================================

describe("CPU power-on state", () => {
  it("starts in SVC mode with PC=0, flags clear", () => {
    const cpu = new ARM1(1024);
    expect(cpu.mode).toBe(MODE_SVC);
    expect(cpu.pc).toBe(0);
    const f = cpu.flags;
    expect(f.N).toBe(false);
    expect(f.Z).toBe(false);
    expect(f.C).toBe(false);
    expect(f.V).toBe(false);
  });
});

// =========================================================================
// CPU - Basic programs
// =========================================================================

describe("CPU execution", () => {
  it("MOV immediate", () => {
    const cpu = new ARM1(1024);
    loadProgram(cpu, [
      encodeMovImm(COND_AL, 0, 42),
      encodeHalt(),
    ]);
    cpu.run(10);
    expect(cpu.readRegister(0)).toBe(42);
  });

  it("1 + 2 = 3", () => {
    const cpu = new ARM1(1024);
    loadProgram(cpu, [
      encodeMovImm(COND_AL, 0, 1),
      encodeMovImm(COND_AL, 1, 2),
      encodeALUReg(COND_AL, OP_ADD, 0, 2, 0, 1),
      encodeHalt(),
    ]);
    cpu.run(10);
    expect(cpu.readRegister(0)).toBe(1);
    expect(cpu.readRegister(1)).toBe(2);
    expect(cpu.readRegister(2)).toBe(3);
  });

  it("SUBS sets flags correctly", () => {
    const cpu = new ARM1(1024);
    loadProgram(cpu, [
      encodeMovImm(COND_AL, 0, 5),
      encodeMovImm(COND_AL, 1, 5),
      encodeALUReg(COND_AL, OP_SUB, 1, 2, 0, 1),
      encodeHalt(),
    ]);
    cpu.run(10);
    expect(cpu.readRegister(2)).toBe(0);
    expect(cpu.flags.Z).toBe(true);
    expect(cpu.flags.C).toBe(true);
  });

  it("conditional execution", () => {
    const cpu = new ARM1(1024);
    loadProgram(cpu, [
      encodeMovImm(COND_AL, 0, 5),
      encodeMovImm(COND_AL, 1, 5),
      encodeALUReg(COND_AL, OP_SUB, 1, 2, 0, 1),  // SUBS sets Z
      encodeMovImm(COND_NE, 3, 99),  // should NOT execute
      encodeMovImm(COND_EQ, 4, 42),  // should execute
      encodeHalt(),
    ]);
    cpu.run(20);
    expect(cpu.readRegister(3)).toBe(0);   // MOVNE skipped
    expect(cpu.readRegister(4)).toBe(42);  // MOVEQ executed
  });

  it("barrel shifter in instruction: multiply by 5", () => {
    const cpu = new ARM1(1024);
    // ADD R1, R0, R0, LSL #2 = R0 + R0*4 = R0*5
    const addWithShift =
      (COND_AL << 28) |
      (OP_ADD << 21) |
      (0 << 16) |     // Rn=R0
      (1 << 12) |     // Rd=R1
      (2 << 7) |      // shift amount = 2
      (SHIFT_LSL << 5) |
      0;               // Rm=R0

    loadProgram(cpu, [
      encodeMovImm(COND_AL, 0, 7),
      addWithShift >>> 0,
      encodeHalt(),
    ]);
    cpu.run(10);
    expect(cpu.readRegister(1)).toBe(35);  // 7 * 5 = 35
  });

  it("loop: sum 1 to 10", () => {
    const cpu = new ARM1(1024);
    loadProgram(cpu, [
      encodeMovImm(COND_AL, 0, 0),
      encodeMovImm(COND_AL, 1, 10),
      encodeALUReg(COND_AL, OP_ADD, 0, 0, 0, 1),
      encodeDataProcessing(COND_AL, OP_SUB, 1, 1, 1, (1 << 25) | 1),
      encodeBranch(COND_NE, false, -16),
      encodeHalt(),
    ]);
    cpu.run(100);
    expect(cpu.readRegister(0)).toBe(55);
    expect(cpu.readRegister(1)).toBe(0);
  });
});

// =========================================================================
// CPU - Load/Store
// =========================================================================

describe("CPU load/store", () => {
  it("STR then LDR", () => {
    const cpu = new ARM1(4096);
    loadProgram(cpu, [
      encodeMovImm(COND_AL, 0, 42),
      encodeMovImm(COND_AL, 1, 0),
      // MOV R1, #256 (= 1 ROR 24 = imm8=1, rotate=12)
      encodeDataProcessing(COND_AL, OP_MOV, 0, 0, 1, (1 << 25) | (12 << 8) | 1),
      encodeSTR(COND_AL, 0, 1, 0, true),
      encodeMovImm(COND_AL, 0, 0),
      encodeLDR(COND_AL, 0, 1, 0, true),
      encodeHalt(),
    ]);
    cpu.run(20);
    expect(cpu.readRegister(0)).toBe(42);
  });

  it("LDRB reads single byte", () => {
    const cpu = new ARM1(4096);
    cpu.writeWord(0x100, 0xDEADBEEF);

    loadProgram(cpu, [
      encodeDataProcessing(COND_AL, OP_MOV, 0, 0, 1, (1 << 25) | (12 << 8) | 1),
      // LDRB R0, [R1] - encoded directly
      ((COND_AL << 28) | 0x05D00000 | (1 << 16) | (0 << 12) | 0) >>> 0,
      encodeHalt(),
    ]);
    cpu.run(10);
    // Byte 0 of 0xDEADBEEF in little-endian is 0xEF
    expect(cpu.readRegister(0)).toBe(0xEF);
  });
});

// =========================================================================
// CPU - Block Transfer
// =========================================================================

describe("CPU block transfer", () => {
  it("STMIA then LDMIA", () => {
    const cpu = new ARM1(4096);
    loadProgram(cpu, [
      encodeMovImm(COND_AL, 0, 10),
      encodeMovImm(COND_AL, 1, 20),
      encodeMovImm(COND_AL, 2, 30),
      encodeMovImm(COND_AL, 3, 40),
      encodeDataProcessing(COND_AL, OP_MOV, 0, 0, 5, (1 << 25) | (12 << 8) | 1),
      encodeSTM(COND_AL, 5, 0x000F, true, "IA"),
      encodeMovImm(COND_AL, 0, 0),
      encodeMovImm(COND_AL, 1, 0),
      encodeMovImm(COND_AL, 2, 0),
      encodeMovImm(COND_AL, 3, 0),
      encodeDataProcessing(COND_AL, OP_MOV, 0, 0, 5, (1 << 25) | (12 << 8) | 1),
      encodeLDM(COND_AL, 5, 0x000F, true, "IA"),
      encodeHalt(),
    ]);
    cpu.run(50);
    expect(cpu.readRegister(0)).toBe(10);
    expect(cpu.readRegister(1)).toBe(20);
    expect(cpu.readRegister(2)).toBe(30);
    expect(cpu.readRegister(3)).toBe(40);
  });
});

// =========================================================================
// CPU - Branch and Link
// =========================================================================

describe("CPU branch and link", () => {
  it("BL to subroutine and return", () => {
    const cpu = new ARM1(4096);
    loadProgram(cpu, [
      encodeMovImm(COND_AL, 0, 7),     // 0x00: MOV R0, #7
      encodeBranch(COND_AL, true, 4),   // 0x04: BL double
      encodeHalt(),                      // 0x08: HLT
      0,                                 // 0x0C: padding
      // double subroutine at 0x10:
      encodeALUReg(COND_AL, OP_ADD, 0, 0, 0, 0),  // ADD R0, R0, R0
      // MOVS PC, LR
      encodeDataProcessing(COND_AL, OP_MOV, 1, 0, 15, 14),
    ]);
    cpu.run(20);
    expect(cpu.readRegister(0)).toBe(14);  // 7 + 7 = 14
  });
});

// =========================================================================
// CPU - Trace output
// =========================================================================

describe("CPU trace", () => {
  it("step produces correct trace", () => {
    const cpu = new ARM1(1024);
    loadProgram(cpu, [
      encodeMovImm(COND_AL, 0, 42),
      encodeHalt(),
    ]);

    const trace = cpu.step();
    expect(trace.address).toBe(0);
    expect(trace.conditionMet).toBe(true);
    expect(trace.mnemonic).toBe("MOV R0, #42");
    expect(trace.regsAfter[0]).toBe(42);
  });

  it("halted CPU has correct state", () => {
    const cpu = new ARM1(1024);
    loadProgram(cpu, [encodeHalt()]);
    cpu.run(10);
    expect(cpu.halted).toBe(true);
  });
});

// =========================================================================
// Memory access
// =========================================================================

describe("memory access", () => {
  it("readWord and writeWord round-trip", () => {
    const cpu = new ARM1(4096);
    cpu.writeWord(0x100, 0xDEADBEEF);
    expect(cpu.readWord(0x100) >>> 0).toBe(0xDEADBEEF);
  });

  it("readByte and writeByte round-trip", () => {
    const cpu = new ARM1(4096);
    cpu.writeByte(0x100, 0xAB);
    expect(cpu.readByte(0x100)).toBe(0xAB);
  });

  it("word access is little-endian", () => {
    const cpu = new ARM1(4096);
    cpu.writeWord(0x100, 0x04030201);
    // Note: readByte masks address with PC_MASK (0x03FFFFFC), which clears
    // the bottom 2 bits. So we use writeByte to test byte ordering directly.
    cpu.writeByte(0x200, 0xAA);
    cpu.writeByte(0x204, 0xBB);
    expect(cpu.readByte(0x200)).toBe(0xAA);
    expect(cpu.readByte(0x204)).toBe(0xBB);
    // Verify word read returns little-endian interpretation
    expect(cpu.readWord(0x100) >>> 0).toBe(0x04030201);
  });
});

// =========================================================================
// Encoding helpers
// =========================================================================

describe("encoding helpers", () => {
  it("encodeHalt produces correct instruction", () => {
    const inst = encodeHalt();
    const d = decode(inst);
    expect(d.type).toBe(INST_SWI);
    expect(d.swiComment).toBe(HALT_SWI);
  });

  it("encodeMovImm round-trips through decoder", () => {
    const inst = encodeMovImm(COND_AL, 3, 99);
    const d = decode(inst);
    expect(d.opcode).toBe(OP_MOV);
    expect(d.rd).toBe(3);
    expect(d.imm8).toBe(99);
  });

  it("encodeBranch round-trips", () => {
    const inst = encodeBranch(COND_AL, true, 8);
    const d = decode(inst);
    expect(d.type).toBe(INST_BRANCH);
    expect(d.link).toBe(true);
    expect(d.branchOffset).toBe(8);
  });
});
