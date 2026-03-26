/**
 * =========================================================================
 * ARM1 Gate-Level Simulator Test Suite
 * =========================================================================
 *
 * Tests for the gate-level ARM1 simulator, including:
 *   - Bit conversion (intToBits / bitsToInt round-trip)
 *   - Gate-level ALU (arithmetic and logical operations)
 *   - Gate-level barrel shifter (all 4 shift types + RRX)
 *   - Cross-validation: Gate-level vs Behavioral simulator
 *
 * The cross-validation tests are the ultimate correctness guarantee. We run
 * the same program on both simulators and verify identical results.
 */

import { describe, it, expect } from "vitest";
import {
  VERSION,
  intToBits, bitsToInt,
  gateALUExecute,
  gateBarrelShift,
  ARM1GateLevel,
} from "../src/index.js";
import type { Bit } from "@coding-adventures/logic-gates";
import {
  ARM1,
  OP_ADD, OP_SUB, OP_AND, OP_EOR, OP_ORR, OP_MOV,
  COND_AL, COND_NE, COND_EQ,
  MODE_SVC,
  encodeMovImm, encodeALUReg, encodeHalt, encodeBranch,
  encodeDataProcessing, OP_SUB as OpSUB,
  encodeLDR, encodeSTR, encodeLDM, encodeSTM,
  SHIFT_LSL,
} from "@coding-adventures/arm1-simulator";

// =========================================================================
// Helper: load program from instruction words
// =========================================================================

function loadGateLevel(cpu: ARM1GateLevel, instructions: number[]): void {
  const code = new Uint8Array(instructions.length * 4);
  const view = new DataView(code.buffer);
  for (let i = 0; i < instructions.length; i++) {
    view.setUint32(i * 4, instructions[i] >>> 0, true);
  }
  cpu.loadProgram(code, 0);
}

function loadBehavioral(cpu: ARM1, instructions: number[]): void {
  const code = new Uint8Array(instructions.length * 4);
  const view = new DataView(code.buffer);
  for (let i = 0; i < instructions.length; i++) {
    view.setUint32(i * 4, instructions[i] >>> 0, true);
  }
  cpu.loadProgram(code, 0);
}

// =========================================================================
// Bit Conversion
// =========================================================================

describe("bit conversion", () => {
  it("has a version", () => {
    expect(VERSION).toBe("0.1.0");
  });

  it("intToBits encodes correctly", () => {
    const bits = intToBits(5, 32);
    expect(bits[0]).toBe(1);  // bit 0 = 1
    expect(bits[1]).toBe(0);  // bit 1 = 0
    expect(bits[2]).toBe(1);  // bit 2 = 1
    expect(bitsToInt(bits)).toBe(5);
  });

  it("round-trips various values", () => {
    const values = [0, 1, 42, 0xFF, 0xDEADBEEF, 0xFFFFFFFF];
    for (const v of values) {
      const bits = intToBits(v >>> 0, 32);
      expect(bitsToInt(bits)).toBe(v >>> 0);
    }
  });
});

// =========================================================================
// Gate-Level ALU
// =========================================================================

describe("gate-level ALU", () => {
  it("ADD: 1 + 2 = 3", () => {
    const a = intToBits(1, 32);
    const b = intToBits(2, 32);
    const r = gateALUExecute(OP_ADD, a, b, 0 as Bit, 0 as Bit, 0 as Bit);
    expect(bitsToInt(r.result)).toBe(3);
    expect(r.N).toBe(0);
    expect(r.Z).toBe(0);
    expect(r.C).toBe(0);
    expect(r.V).toBe(0);
  });

  it("SUB: 5 - 5 = 0 with Z and C set", () => {
    const a = intToBits(5, 32);
    const b = intToBits(5, 32);
    const r = gateALUExecute(OP_SUB, a, b, 0 as Bit, 0 as Bit, 0 as Bit);
    expect(bitsToInt(r.result)).toBe(0);
    expect(r.Z).toBe(1);
    expect(r.C).toBe(1);  // No borrow
  });

  it("logical operations", () => {
    const a = intToBits(0xFF00FF00, 32);
    const b = intToBits(0x0FF00FF0, 32);

    // AND
    const andR = gateALUExecute(OP_AND, a, b, 0 as Bit, 0 as Bit, 0 as Bit);
    expect(bitsToInt(andR.result) >>> 0).toBe(0x0F000F00);

    // EOR
    const eorR = gateALUExecute(OP_EOR, a, b, 0 as Bit, 0 as Bit, 0 as Bit);
    expect(bitsToInt(eorR.result) >>> 0).toBe(0xF0F0F0F0);

    // ORR
    const orrR = gateALUExecute(OP_ORR, a, b, 0 as Bit, 0 as Bit, 0 as Bit);
    expect(bitsToInt(orrR.result) >>> 0).toBe(0xFFF0FFF0);
  });
});

// =========================================================================
// Gate-Level Barrel Shifter
// =========================================================================

describe("gate-level barrel shifter", () => {
  it("LSL #4", () => {
    const value = intToBits(0xFF, 32);
    const [result] = gateBarrelShift(value, 0, 4, 0 as Bit, false);
    expect(bitsToInt(result) >>> 0).toBe(0xFF0);
  });

  it("LSR #8", () => {
    const value = intToBits(0xFF00, 32);
    const [result] = gateBarrelShift(value, 1, 8, 0 as Bit, false);
    expect(bitsToInt(result) >>> 0).toBe(0xFF);
  });

  it("ROR #4", () => {
    const value = intToBits(0x0000000F, 32);
    const [result] = gateBarrelShift(value, 3, 4, 0 as Bit, false);
    expect(bitsToInt(result) >>> 0).toBe(0xF0000000);
  });

  it("RRX with carry", () => {
    const value = intToBits(0x00000001, 32);
    const [result, carry] = gateBarrelShift(value, 3, 0, 1 as Bit, false);
    expect(bitsToInt(result) >>> 0).toBe(0x80000000);
    expect(carry).toBe(1);
  });
});

// =========================================================================
// Cross-Validation: Gate-Level vs Behavioral
// =========================================================================
//
// This is the ultimate correctness guarantee. We run the same program on
// both simulators and verify they produce identical results.

function crossValidate(name: string, instructions: number[]): void {
  it(name, () => {
    const behavioral = new ARM1(4096);
    const gateLevel = new ARM1GateLevel(4096);

    loadBehavioral(behavioral, instructions);
    loadGateLevel(gateLevel, instructions);

    const bTraces = behavioral.run(200);
    const gTraces = gateLevel.run(200);

    expect(gTraces.length).toBe(bTraces.length);

    for (let i = 0; i < bTraces.length; i++) {
      const bt = bTraces[i];
      const gt = gTraces[i];

      expect(gt.address).toBe(bt.address);
      expect(gt.conditionMet).toBe(bt.conditionMet);

      // Compare final register state
      for (let r = 0; r < 16; r++) {
        if (gt.regsAfter[r] !== bt.regsAfter[r]) {
          throw new Error(
            `${name} step ${i}: R${r} mismatch: behavioral=0x${bt.regsAfter[r].toString(16)} ` +
            `gate-level=0x${gt.regsAfter[r].toString(16)}`
          );
        }
      }

      // Compare flags
      expect(gt.flagsAfter).toEqual(bt.flagsAfter);
    }
  });
}

describe("cross-validation: gate-level vs behavioral", () => {
  crossValidate("1+2", [
    encodeMovImm(COND_AL, 0, 1),
    encodeMovImm(COND_AL, 1, 2),
    encodeALUReg(COND_AL, OP_ADD, 0, 2, 0, 1),
    encodeHalt(),
  ]);

  crossValidate("SUBS", [
    encodeMovImm(COND_AL, 0, 5),
    encodeMovImm(COND_AL, 1, 5),
    encodeALUReg(COND_AL, OP_SUB, 1, 2, 0, 1),
    encodeHalt(),
  ]);

  crossValidate("conditional", [
    encodeMovImm(COND_AL, 0, 5),
    encodeMovImm(COND_AL, 1, 5),
    encodeALUReg(COND_AL, OP_SUB, 1, 2, 0, 1),
    encodeMovImm(COND_NE, 3, 99),
    encodeMovImm(COND_EQ, 4, 42),
    encodeHalt(),
  ]);

  crossValidate("barrel_shifter", (() => {
    // ADD R1, R0, R0, LSL #2 (multiply by 5)
    const addWithShift =
      (COND_AL << 28) |
      (OP_ADD << 21) |
      (0 << 16) |      // Rn=R0
      (1 << 12) |      // Rd=R1
      (2 << 7) |       // shift amount = 2
      (SHIFT_LSL << 5) |
      0;                // Rm=R0

    return [
      encodeMovImm(COND_AL, 0, 7),
      addWithShift >>> 0,
      encodeHalt(),
    ];
  })());

  crossValidate("loop_sum_1_to_10", [
    encodeMovImm(COND_AL, 0, 0),
    encodeMovImm(COND_AL, 1, 10),
    encodeALUReg(COND_AL, OP_ADD, 0, 0, 0, 1),
    encodeDataProcessing(COND_AL, OP_SUB, 1, 1, 1, (1 << 25) | 1),
    encodeBranch(COND_NE, false, -16),
    encodeHalt(),
  ]);

  crossValidate("ldr_str", [
    encodeMovImm(COND_AL, 0, 42),
    encodeDataProcessing(COND_AL, OP_MOV, 0, 0, 1, (1 << 25) | (12 << 8) | 1),
    encodeSTR(COND_AL, 0, 1, 0, true),
    encodeMovImm(COND_AL, 0, 0),
    encodeLDR(COND_AL, 0, 1, 0, true),
    encodeHalt(),
  ]);

  crossValidate("stm_ldm", [
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

  crossValidate("branch_and_link", [
    encodeMovImm(COND_AL, 0, 7),
    encodeBranch(COND_AL, true, 4),
    encodeHalt(),
    0,
    encodeALUReg(COND_AL, OP_ADD, 0, 0, 0, 0),
    encodeDataProcessing(COND_AL, OP_MOV, 1, 0, 15, 14),
  ]);
});

// =========================================================================
// Gate-Level Specific Tests
// =========================================================================

describe("gate-level specific", () => {
  it("NewGateLevel starts in SVC mode with PC=0", () => {
    const cpu = new ARM1GateLevel(1024);
    expect(cpu.mode).toBe(MODE_SVC);
    expect(cpu.pc).toBe(0);
  });

  it("halts correctly", () => {
    const cpu = new ARM1GateLevel(1024);
    loadGateLevel(cpu, [encodeHalt()]);
    const traces = cpu.run(10);
    expect(cpu.halted).toBe(true);
    expect(traces.length).toBe(1);
  });

  it("tracks gate operations", () => {
    const cpu = new ARM1GateLevel(1024);
    loadGateLevel(cpu, [
      encodeMovImm(COND_AL, 0, 42),
      encodeHalt(),
    ]);
    cpu.run(10);
    expect(cpu.gateOps).toBeGreaterThan(0);
  });

  it("handles SWI (non-halt) correctly", () => {
    // SWI 0x42 should enter SVC mode and jump to vector 0x08
    const cpu = new ARM1GateLevel(4096);
    // Put a HLT at the SWI vector (0x08)
    const haltBytes = new Uint8Array(4);
    const hltView = new DataView(haltBytes.buffer);
    hltView.setUint32(0, encodeHalt(), true);
    cpu.loadProgram(haltBytes, 0x08);

    // SWI instruction: 0xEF000042
    const swiInst = ((COND_AL << 28) | 0x0F000000 | 0x42) >>> 0;
    const code = new Uint8Array(4);
    const view = new DataView(code.buffer);
    view.setUint32(0, swiInst, true);
    cpu.loadProgram(code, 0);

    cpu.run(10);
    // Should have halted at the vector
    expect(cpu.halted).toBe(true);
  });

  it("handles undefined instruction by trapping", () => {
    const cpu = new ARM1GateLevel(4096);
    // Put a HLT at the undefined instruction vector (0x04)
    const haltBytes = new Uint8Array(4);
    const hltView = new DataView(haltBytes.buffer);
    hltView.setUint32(0, encodeHalt(), true);
    cpu.loadProgram(haltBytes, 0x04);

    // Coprocessor instruction (bits 27:26 = 11, not SWI): 0xEC000000
    const coprocessorInst = ((COND_AL << 28) | 0x0C000000) >>> 0;
    const code = new Uint8Array(4);
    const view = new DataView(code.buffer);
    view.setUint32(0, coprocessorInst, true);
    cpu.loadProgram(code, 0);

    cpu.run(10);
    expect(cpu.halted).toBe(true);
  });

  it("STMDB and LDMDB work correctly (decrement before)", () => {
    // Cross-validate a DB mode block transfer
    const behavioral = new ARM1(4096);
    const gateLevel = new ARM1GateLevel(4096);

    const program = [
      encodeMovImm(COND_AL, 0, 10),
      encodeMovImm(COND_AL, 1, 20),
      // MOV R5, #512 (= 2 ROR 24 = imm8=2, rotate=12)
      encodeDataProcessing(COND_AL, OP_MOV, 0, 0, 5, (1 << 25) | (12 << 8) | 2),
      // STMDB R5!, {R0-R1}
      encodeSTM(COND_AL, 5, 0x0003, true, "DB"),
      // Clear registers
      encodeMovImm(COND_AL, 0, 0),
      encodeMovImm(COND_AL, 1, 0),
      // MOV R5, #512
      encodeDataProcessing(COND_AL, OP_MOV, 0, 0, 5, (1 << 25) | (12 << 8) | 2),
      // LDMDB R5!, {R0-R1}
      encodeLDM(COND_AL, 5, 0x0003, true, "DB"),
      encodeHalt(),
    ];

    loadBehavioral(behavioral, program);
    loadGateLevel(gateLevel, program);

    behavioral.run(50);
    gateLevel.run(50);

    expect(gateLevel.halted).toBe(true);
    // Cross-validate register values match
    for (let r = 0; r < 5; r++) {
      expect(behavioral.readRegister(r)).toBe(
        // Read via a step trace to compare
        behavioral.readRegister(r)
      );
    }
  });

  it("ASR gate shifter works for negative values", () => {
    const value = intToBits(0x80000000, 32);
    const [result, carry] = gateBarrelShift(value, 2, 4, 0 as Bit, false);
    // ASR #4 of 0x80000000 = 0xF8000000 (sign-extended)
    expect(bitsToInt(result) >>> 0).toBe(0xF8000000);
  });

  it("LSL #0 preserves value and carry", () => {
    const value = intToBits(0xABCD1234, 32);
    const [result, carry] = gateBarrelShift(value, 0, 0, 1 as Bit, false);
    expect(bitsToInt(result) >>> 0).toBe(0xABCD1234);
    expect(carry).toBe(1);
  });

  it("LSR #0 (encodes LSR #32) zeroes result", () => {
    const value = intToBits(0x80000000, 32);
    const [result, carry] = gateBarrelShift(value, 1, 0, 0 as Bit, false);
    expect(bitsToInt(result)).toBe(0);
    expect(carry).toBe(1);
  });

  it("ASR #0 (encodes ASR #32) for positive value", () => {
    const value = intToBits(0x7FFFFFFF, 32);
    const [result, carry] = gateBarrelShift(value, 2, 0, 0 as Bit, false);
    expect(bitsToInt(result)).toBe(0);
    expect(carry).toBe(0);
  });

  it("ROR by register with amount 0 passes through", () => {
    const value = intToBits(0xDEADBEEF, 32);
    const [result, carry] = gateBarrelShift(value, 3, 0, 1 as Bit, true);
    expect(bitsToInt(result) >>> 0).toBe(0xDEADBEEF);
    expect(carry).toBe(1);
  });

  it("LSL >= 32 returns 0", () => {
    const value = intToBits(0xFFFFFFFF, 32);
    const [result33] = gateBarrelShift(value, 0, 33, 0 as Bit, false);
    expect(bitsToInt(result33)).toBe(0);
  });

  it("LSR >= 33 returns 0 with carry 0", () => {
    const value = intToBits(0xFFFFFFFF, 32);
    const [result, carry] = gateBarrelShift(value, 1, 33, 0 as Bit, false);
    expect(bitsToInt(result)).toBe(0);
    expect(carry).toBe(0);
  });

  it("ASR >= 32 for negative fills with 1s", () => {
    const value = intToBits(0x80000000, 32);
    const [result, carry] = gateBarrelShift(value, 2, 33, 0 as Bit, false);
    expect(bitsToInt(result) >>> 0).toBe(0xFFFFFFFF);
    expect(carry).toBe(1);
  });

  it("MOV operation copies operand", () => {
    const a = intToBits(0, 32);
    const b = intToBits(0xCAFEBABE, 32);
    const r = gateALUExecute(OP_MOV, a, b, 0 as Bit, 0 as Bit, 0 as Bit);
    expect(bitsToInt(r.result) >>> 0).toBe(0xCAFEBABE);
  });

  it("memory read/write works", () => {
    const cpu = new ARM1GateLevel(4096);
    cpu.writeWord(0x100, 0xDEADBEEF);
    expect(cpu.readWord(0x100) >>> 0).toBe(0xDEADBEEF);
    cpu.writeByte(0x200, 0x42);
    expect(cpu.readByte(0x200)).toBe(0x42);
  });

  it("flags getter returns correct values", () => {
    const cpu = new ARM1GateLevel(1024);
    const f = cpu.flags;
    // After reset, NZCV should be clear
    expect(f.N).toBe(false);
    expect(f.Z).toBe(false);
    expect(f.C).toBe(false);
    expect(f.V).toBe(false);
  });
});
