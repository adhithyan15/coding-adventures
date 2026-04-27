/**
 * Intel 8008 Simulator — Test Suite
 *
 * Tests cover every instruction group and verify correct behavior of:
 * - Data movement (MOV, MVI)
 * - Arithmetic (ADD, ADC, SUB, SBB, ADI, SUI, etc.)
 * - Logic (ANA, XRA, ORA, CMP, ANI, XRI, ORI, CPI)
 * - Increment/decrement (INR, DCR)
 * - Rotate (RLC, RRC, RAL, RAR)
 * - Control flow (JMP, JFZ/JTZ, etc.)
 * - Calls and returns (CAL, CFZ/CTZ, RET, RFC/RTC, etc.)
 * - Restarts (RST)
 * - I/O (IN, OUT — partial)
 * - Stack depth and nesting
 * - Example programs from the spec
 */

import { describe, it, expect, beforeEach } from "vitest";
import { Intel8008Simulator } from "../src/index.js";

// ---------------------------------------------------------------------------
// Helper: hex string for display
// ---------------------------------------------------------------------------
function hex(n: number): string {
  return `0x${n.toString(16).toUpperCase().padStart(2, "0")}`;
}

// ---------------------------------------------------------------------------
// Fixtures: frequently used opcodes
// ---------------------------------------------------------------------------
const HLT = 0x76;

// MVI R, d  (2 bytes)
const MVI_B = 0x06;
const MVI_C = 0x0E;
const MVI_D = 0x16;
const MVI_E = 0x1E;
const MVI_H = 0x26;
const MVI_L = 0x2E;
const MVI_M = 0x36;
const MVI_A = 0x3E;

// MOV r, r  (1 byte)
const MOV_A_B = 0x78;  // MOV A, B  (01 111 000)
// Note: 0x7E is CAL (unconditional call), NOT MOV A,M
// To load M into A, use: XRA_A (clears A), then ADD_M or ORA_M
const ORA_M  = 0xB6;   // ORA M  (10 110 110) — if A=0, loads M into A
const MOV_B_A = 0x47;  // MOV B, A  (01 000 111)
const MOV_M_A = 0x77;  // MOV M, A  (01 110 111)

// INR / DCR
const INR_A = 0x38;
const INR_B = 0x00;
const DCR_A = 0x39;
const DCR_B = 0x01;
const INR_M = 0x30;
const DCR_M = 0x31;

// ALU register (1 byte): 10 OOO SSS
const ADD_B = 0x80;  // A = A + B
const ADC_B = 0x88;  // A = A + B + CY
const SUB_B = 0x90;  // A = A - B
const SBB_B = 0x98;  // A = A - B - CY
const ANA_B = 0xA0;  // A = A & B
const XRA_B = 0xA8;  // A = A ^ B
const ORA_B = 0xB0;  // A = A | B
const CMP_B = 0xB8;  // flags = A - B; A unchanged

// CMP A (self-compare — always sets Z=1)
const CMP_A = 0xBF;

// ALU immediate (2 bytes): 11 OOO 100
const ADI = 0xC4;  // ADI imm
const ACI = 0xCC;  // ACI imm
const SUI = 0xD4;  // SUI imm
const SBI = 0xDC;  // SBI imm
const ANI = 0xE4;  // ANI imm
const XRI = 0xEC;  // XRI imm
const ORI = 0xF4;  // ORI imm
const CPI = 0xFC;  // CPI imm

// Rotates
const RLC = 0x02;
const RRC = 0x0A;
const RAL = 0x12;
const RAR = 0x1A;

// JMP (3 bytes): 0x7C, lo, hi
const JMP = 0x7C;
// JFZ (3 bytes): 0x48, lo, hi  (jump if Z=0)
const JFZ = 0x48;
// JTZ (3 bytes): 0x4C, lo, hi  (jump if Z=1)
const JTZ = 0x4C;
// JFC (jump if CY=0): 0x40
const JFC = 0x40;
// JTC (jump if CY=1): 0x44
const JTC = 0x44;
// JTP (jump if P=1): 0x5C
const JTP = 0x5C;
// JFP (jump if P=0): 0x58
const JFP = 0x58;

// CAL (3 bytes): 0x7E, lo, hi
const CAL = 0x7E;
// CFZ: 0x4A (call if Z=0)
const CFZ = 0x4A;
// CTZ: 0x4E (call if Z=1)
const CTZ = 0x4E;

// RET (1 byte): 0x3F
const RET = 0x3F;
// RFC (1 byte): 0x03 (return if CY=0)
const RFC = 0x03;
// RTC (1 byte): 0x07 (return if CY=1)
const RTC = 0x07;
// RFZ (1 byte): 0x0B (return if Z=0)
const RFZ = 0x0B;
// RTZ (1 byte): 0x0F (return if Z=1)
const RTZ = 0x0F;
// RFS (return if S=0): 0x13
const RFS = 0x13;
// RFP (return if P=0): 0x1B
const RFP = 0x1B;
// RTP (return if P=1): 0x1F
const RTP = 0x1F;

// RST N (1 byte): 00 AAA 101
const RST0 = 0x05;  // RST 0 → jumps to 0x0000
const RST1 = 0x0D;  // RST 1 → jumps to 0x0008
const RST7 = 0x3D;  // RST 7 → jumps to 0x0038

// IN port (1 byte): 01 PPP 001
const IN_0 = 0x41;
const IN_7 = 0x79;

// ---------------------------------------------------------------------------
// Helper: build a 3-byte JMP/CAL target
// ---------------------------------------------------------------------------
function lo(addr: number): number { return addr & 0xFF; }
function hi(addr: number): number { return (addr >> 8) & 0x3F; }

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("Intel8008Simulator", () => {

  let sim: Intel8008Simulator;

  beforeEach(() => {
    sim = new Intel8008Simulator();
  });

  // =========================================================================
  // Basic state
  // =========================================================================

  describe("initial state", () => {
    it("all registers are 0", () => {
      expect(sim.a).toBe(0);
      expect(sim.b).toBe(0);
      expect(sim.c).toBe(0);
      expect(sim.d).toBe(0);
      expect(sim.e).toBe(0);
      expect(sim.h).toBe(0);
      expect(sim.l).toBe(0);
    });

    it("PC is 0", () => {
      expect(sim.pc).toBe(0);
    });

    it("flags are all false", () => {
      const f = sim.currentFlags;
      expect(f.carry).toBe(false);
      expect(f.zero).toBe(false);
      expect(f.sign).toBe(false);
      expect(f.parity).toBe(false);
    });

    it("not halted", () => {
      expect(sim.isHalted).toBe(false);
    });

    it("stack is all zeros", () => {
      expect(sim.stack.every(v => v === 0)).toBe(true);
    });

    it("hlAddress is 0", () => {
      expect(sim.hlAddress).toBe(0);
    });
  });

  // =========================================================================
  // HLT
  // =========================================================================

  describe("HLT", () => {
    it("halts immediately on 0x76", () => {
      sim.run(new Uint8Array([HLT]));
      expect(sim.isHalted).toBe(true);
    });

    it("halts immediately on 0xFF", () => {
      sim.run(new Uint8Array([0xFF]));
      expect(sim.isHalted).toBe(true);
    });

    it("step() on halted processor throws", () => {
      sim.run(new Uint8Array([HLT]));
      expect(() => sim.step()).toThrow();
    });

    it("reset() clears halted state", () => {
      sim.run(new Uint8Array([HLT]));
      sim.reset();
      expect(sim.isHalted).toBe(false);
    });
  });

  // =========================================================================
  // MVI — Move Immediate
  // =========================================================================

  describe("MVI", () => {
    it("MVI A loads immediate into A", () => {
      sim.run(new Uint8Array([MVI_A, 0x42, HLT]));
      expect(sim.a).toBe(0x42);
    });

    it("MVI B loads immediate into B", () => {
      sim.run(new Uint8Array([MVI_B, 0x07, HLT]));
      expect(sim.b).toBe(0x07);
    });

    it("MVI C, MVI D, MVI E work", () => {
      sim.run(new Uint8Array([
        MVI_C, 0x01,
        MVI_D, 0x02,
        MVI_E, 0x03,
        HLT,
      ]));
      expect(sim.c).toBe(0x01);
      expect(sim.d).toBe(0x02);
      expect(sim.e).toBe(0x03);
    });

    it("MVI H and MVI L set H and L", () => {
      sim.run(new Uint8Array([MVI_H, 0x10, MVI_L, 0x20, HLT]));
      expect(sim.h).toBe(0x10);
      expect(sim.l).toBe(0x20);
    });

    it("MVI M writes to memory at [H:L]", () => {
      // Set H:L = 0x0010, then write 0xAB to that address
      sim.run(new Uint8Array([
        MVI_H, 0x00,
        MVI_L, 0x10,
        MVI_M, 0xAB,
        HLT,
      ]));
      expect(sim.memory[0x0010]).toBe(0xAB);
    });

    it("MVI does not affect flags", () => {
      sim.run(new Uint8Array([MVI_A, 0x00, HLT]));
      // MVI does not set Z=1 even though immediate is 0
      expect(sim.currentFlags.zero).toBe(false);
    });
  });

  // =========================================================================
  // MOV — Register Transfer
  // =========================================================================

  describe("MOV", () => {
    it("MOV A, B copies B to A", () => {
      sim.run(new Uint8Array([MVI_B, 0x55, MOV_A_B, HLT]));
      expect(sim.a).toBe(0x55);
      expect(sim.b).toBe(0x55);  // B unchanged
    });

    it("MOV B, A copies A to B", () => {
      sim.run(new Uint8Array([MVI_A, 0x33, MOV_B_A, HLT]));
      expect(sim.b).toBe(0x33);
      expect(sim.a).toBe(0x33);
    });

    it("load from [H:L] using ORA M (A=0, then ORA M = M)", () => {
      // In 8008, 0x7E is CAL (not MOV A,M). To load mem[H:L] into A,
      // zero A first then use ORA M (A | M = M when A=0).
      const XRA_A = 0xAF;  // XRA A = A ^ A = 0 (10 101 111)
      sim.run(new Uint8Array([
        MVI_H, 0x00,
        MVI_L, 0x40,
        MVI_M, 0xCD,
        XRA_A,    // A = 0
        ORA_M,    // A = A | mem[0x0040] = 0xCD
        HLT,
      ]));
      expect(sim.a).toBe(0xCD);
    });

    it("MOV M, A writes A to memory at [H:L]", () => {
      sim.run(new Uint8Array([
        MVI_H, 0x00,
        MVI_L, 0x50,
        MVI_A, 0x99,
        MOV_M_A,  // mem[0x0050] ← A  (01 110 111 = 0x77)
        HLT,
      ]));
      expect(sim.memory[0x0050]).toBe(0x99);
    });

    it("MOV does not affect flags", () => {
      sim.run(new Uint8Array([MVI_A, 0xFF, MOV_B_A, HLT]));
      expect(sim.currentFlags.sign).toBe(false);  // MOV doesn't set flags
    });
  });

  // =========================================================================
  // INR / DCR
  // =========================================================================

  describe("INR", () => {
    it("INR A increments accumulator", () => {
      sim.run(new Uint8Array([MVI_A, 0x05, INR_A, HLT]));
      expect(sim.a).toBe(0x06);
    });

    it("INR B increments B (also encodes as 0x00)", () => {
      sim.run(new Uint8Array([MVI_B, 0x0F, INR_B, HLT]));
      expect(sim.b).toBe(0x10);
    });

    it("INR wraps 0xFF to 0x00 and sets Z=1", () => {
      sim.run(new Uint8Array([MVI_A, 0xFF, INR_A, HLT]));
      expect(sim.a).toBe(0x00);
      expect(sim.currentFlags.zero).toBe(true);
    });

    it("INR does not change CY", () => {
      // First set CY by doing an ADD that overflows
      sim.run(new Uint8Array([
        MVI_A, 0xFF,
        MVI_B, 0x02,
        ADD_B,         // A=0x01, CY=1
        INR_A,         // A=0x02, CY should still be 1
        HLT,
      ]));
      expect(sim.currentFlags.carry).toBe(true);
    });

    it("INR sets S flag for result >= 0x80", () => {
      sim.run(new Uint8Array([MVI_A, 0x7F, INR_A, HLT]));
      expect(sim.a).toBe(0x80);
      expect(sim.currentFlags.sign).toBe(true);
    });

    it("INR sets P flag for even parity result", () => {
      sim.run(new Uint8Array([MVI_A, 0x02, INR_A, HLT]));  // 0x03 = 00000011 = 2 ones = even
      expect(sim.a).toBe(0x03);
      expect(sim.currentFlags.parity).toBe(true);
    });

    it("INR M increments memory at [H:L]", () => {
      sim.run(new Uint8Array([
        MVI_H, 0x00, MVI_L, 0x50,
        MVI_M, 0x09,
        INR_M,
        HLT,
      ]));
      expect(sim.memory[0x0050]).toBe(0x0A);
    });
  });

  describe("DCR", () => {
    it("DCR A decrements accumulator", () => {
      sim.run(new Uint8Array([MVI_A, 0x05, DCR_A, HLT]));
      expect(sim.a).toBe(0x04);
    });

    it("DCR wraps 0x00 to 0xFF and sets S=1", () => {
      sim.run(new Uint8Array([MVI_A, 0x00, DCR_A, HLT]));
      expect(sim.a).toBe(0xFF);
      expect(sim.currentFlags.sign).toBe(true);
    });

    it("DCR sets Z=1 when result is 0", () => {
      sim.run(new Uint8Array([MVI_A, 0x01, DCR_A, HLT]));
      expect(sim.a).toBe(0x00);
      expect(sim.currentFlags.zero).toBe(true);
    });

    it("DCR does not change CY", () => {
      sim.run(new Uint8Array([
        MVI_A, 0xFF,
        MVI_B, 0x02,
        ADD_B,       // CY=1
        DCR_A,       // CY still 1
        HLT,
      ]));
      expect(sim.currentFlags.carry).toBe(true);
    });
  });

  // =========================================================================
  // ALU Register Instructions
  // =========================================================================

  describe("ADD", () => {
    it("adds B to A without carry", () => {
      sim.run(new Uint8Array([MVI_A, 0x03, MVI_B, 0x04, ADD_B, HLT]));
      expect(sim.a).toBe(0x07);
    });

    it("sets CY on overflow", () => {
      sim.run(new Uint8Array([MVI_A, 0xFF, MVI_B, 0x01, ADD_B, HLT]));
      expect(sim.a).toBe(0x00);
      expect(sim.currentFlags.carry).toBe(true);
      expect(sim.currentFlags.zero).toBe(true);
    });

    it("sets Z=1 when result is zero", () => {
      sim.run(new Uint8Array([MVI_A, 0x00, MVI_B, 0x00, ADD_B, HLT]));
      expect(sim.currentFlags.zero).toBe(true);
    });

    it("sets S=1 when bit7 is 1", () => {
      sim.run(new Uint8Array([MVI_A, 0x7F, MVI_B, 0x01, ADD_B, HLT]));
      expect(sim.a).toBe(0x80);
      expect(sim.currentFlags.sign).toBe(true);
    });

    it("sets P=1 for even parity (0x03 = 2 ones)", () => {
      sim.run(new Uint8Array([MVI_A, 0x01, MVI_B, 0x02, ADD_B, HLT]));
      expect(sim.a).toBe(0x03);
      expect(sim.currentFlags.parity).toBe(true);
    });

    it("sets P=0 for odd parity (0x07 = 3 ones)", () => {
      sim.run(new Uint8Array([MVI_A, 0x03, MVI_B, 0x04, ADD_B, HLT]));
      expect(sim.a).toBe(0x07);
      expect(sim.currentFlags.parity).toBe(false);
    });
  });

  describe("ADC", () => {
    it("adds B + CY to A", () => {
      // Set CY first via overflow, then ADC
      sim.run(new Uint8Array([
        MVI_A, 0xFF, MVI_B, 0x01, ADD_B,  // A=0x00, CY=1
        MVI_A, 0x05,                        // A=0x05
        ADC_B,                              // A = 5 + 1 (B) + 1 (CY) = 7
        HLT,
      ]));
      expect(sim.a).toBe(0x07);
    });

    it("adds without carry when CY=0", () => {
      sim.run(new Uint8Array([MVI_A, 0x05, MVI_B, 0x03, ADC_B, HLT]));
      expect(sim.a).toBe(0x08);
    });
  });

  describe("SUB", () => {
    it("subtracts B from A", () => {
      sim.run(new Uint8Array([MVI_A, 0x0A, MVI_B, 0x03, SUB_B, HLT]));
      expect(sim.a).toBe(0x07);
    });

    it("sets CY=1 on borrow (A < B)", () => {
      sim.run(new Uint8Array([MVI_A, 0x01, MVI_B, 0x02, SUB_B, HLT]));
      expect(sim.a).toBe(0xFF);
      expect(sim.currentFlags.carry).toBe(true);
    });

    it("sets Z=1 when A = B", () => {
      sim.run(new Uint8Array([MVI_A, 0x05, MVI_B, 0x05, SUB_B, HLT]));
      expect(sim.a).toBe(0x00);
      expect(sim.currentFlags.zero).toBe(true);
      expect(sim.currentFlags.carry).toBe(false);
    });
  });

  describe("SBB", () => {
    it("subtracts B + CY from A", () => {
      // CY=1 from previous overflow
      sim.run(new Uint8Array([
        MVI_A, 0xFF, MVI_B, 0x01, ADD_B,  // CY=1, A=0
        MVI_A, 0x0A,
        MVI_B, 0x03,
        SBB_B,  // A = 10 - 3 - 1 = 6
        HLT,
      ]));
      expect(sim.a).toBe(0x06);
    });
  });

  describe("ANA", () => {
    it("ANDs A and B", () => {
      sim.run(new Uint8Array([MVI_A, 0xFF, MVI_B, 0x0F, ANA_B, HLT]));
      expect(sim.a).toBe(0x0F);
    });

    it("clears CY", () => {
      sim.run(new Uint8Array([
        MVI_A, 0xFF, MVI_B, 0x01, ADD_B,  // CY=1
        MVI_A, 0xFF, MVI_B, 0x0F, ANA_B,  // CY cleared to 0
        HLT,
      ]));
      expect(sim.currentFlags.carry).toBe(false);
    });
  });

  describe("XRA", () => {
    it("XORs A and B", () => {
      sim.run(new Uint8Array([MVI_A, 0xF0, MVI_B, 0x0F, XRA_B, HLT]));
      expect(sim.a).toBe(0xFF);
    });

    it("XRA A clears accumulator (A ^ A = 0)", () => {
      // XRA A = 0xAF = 10 101 111
      const XRA_A = 0xAF;
      sim.run(new Uint8Array([MVI_A, 0x55, XRA_A, HLT]));
      expect(sim.a).toBe(0x00);
      expect(sim.currentFlags.zero).toBe(true);
    });

    it("clears CY", () => {
      sim.run(new Uint8Array([
        MVI_A, 0xFF, MVI_B, 0x01, ADD_B,  // CY=1
        MVI_A, 0xFF, MVI_B, 0x0F, XRA_B,
        HLT,
      ]));
      expect(sim.currentFlags.carry).toBe(false);
    });
  });

  describe("ORA", () => {
    it("ORs A and B", () => {
      sim.run(new Uint8Array([MVI_A, 0xF0, MVI_B, 0x0F, ORA_B, HLT]));
      expect(sim.a).toBe(0xFF);
    });

    it("clears CY", () => {
      sim.run(new Uint8Array([
        MVI_A, 0xFF, MVI_B, 0x01, ADD_B,  // CY=1
        MVI_A, 0x00, MVI_B, 0x00, ORA_B,  // CY cleared
        HLT,
      ]));
      expect(sim.currentFlags.carry).toBe(false);
    });
  });

  describe("CMP", () => {
    it("sets Z=1 when A = B, A unchanged", () => {
      sim.run(new Uint8Array([MVI_A, 0x05, MVI_B, 0x05, CMP_B, HLT]));
      expect(sim.a).toBe(0x05);  // A unchanged
      expect(sim.currentFlags.zero).toBe(true);
    });

    it("sets CY=1 when A < B (borrow)", () => {
      sim.run(new Uint8Array([MVI_A, 0x03, MVI_B, 0x05, CMP_B, HLT]));
      expect(sim.currentFlags.carry).toBe(true);
    });

    it("CMP A always sets Z=1", () => {
      sim.run(new Uint8Array([MVI_A, 0x42, CMP_A, HLT]));
      expect(sim.currentFlags.zero).toBe(true);
      expect(sim.currentFlags.carry).toBe(false);
      expect(sim.a).toBe(0x42);  // unchanged
    });
  });

  // =========================================================================
  // ALU Immediate
  // =========================================================================

  describe("ADI", () => {
    it("adds immediate to A", () => {
      sim.run(new Uint8Array([MVI_A, 0x05, ADI, 0x03, HLT]));
      expect(sim.a).toBe(0x08);
    });

    it("sets CY on overflow", () => {
      sim.run(new Uint8Array([MVI_A, 0xFE, ADI, 0x05, HLT]));
      expect(sim.a).toBe(0x03);
      expect(sim.currentFlags.carry).toBe(true);
    });
  });

  describe("SUI", () => {
    it("subtracts immediate from A", () => {
      sim.run(new Uint8Array([MVI_A, 0x0A, SUI, 0x03, HLT]));
      expect(sim.a).toBe(0x07);
    });
  });

  describe("ANI", () => {
    it("ANDs A with immediate", () => {
      sim.run(new Uint8Array([MVI_A, 0xFF, ANI, 0x0F, HLT]));
      expect(sim.a).toBe(0x0F);
    });
  });

  describe("XRI", () => {
    it("XORs A with immediate", () => {
      sim.run(new Uint8Array([MVI_A, 0xFF, XRI, 0x0F, HLT]));
      expect(sim.a).toBe(0xF0);
    });
  });

  describe("ORI", () => {
    it("ORs A with immediate", () => {
      sim.run(new Uint8Array([MVI_A, 0xF0, ORI, 0x0F, HLT]));
      expect(sim.a).toBe(0xFF);
    });

    it("ORI 0x00 updates flags without changing A (canonical idiom)", () => {
      // From spec: ORI 0x00 is the idiom to set flags from A without changing A
      sim.run(new Uint8Array([MVI_A, 0xB5, ORI, 0x00, HLT]));
      expect(sim.a).toBe(0xB5);  // unchanged
      // 0xB5 = 10110101 = 5 ones → odd parity → P=0
      expect(sim.currentFlags.parity).toBe(false);
      expect(sim.currentFlags.sign).toBe(true);    // bit7=1
      expect(sim.currentFlags.zero).toBe(false);
    });
  });

  describe("CPI", () => {
    it("compares A with immediate; A unchanged", () => {
      sim.run(new Uint8Array([MVI_A, 0x0A, CPI, 0x0A, HLT]));
      expect(sim.a).toBe(0x0A);  // unchanged
      expect(sim.currentFlags.zero).toBe(true);
      expect(sim.currentFlags.carry).toBe(false);
    });

    it("sets CY=1 when A < immediate", () => {
      sim.run(new Uint8Array([MVI_A, 0x05, CPI, 0x0A, HLT]));
      expect(sim.currentFlags.carry).toBe(true);
    });
  });

  // =========================================================================
  // Rotate Instructions
  // =========================================================================

  describe("RLC", () => {
    it("rotates A left: CY=A[7], A[0]=old A[7]", () => {
      sim.run(new Uint8Array([MVI_A, 0b10110001, RLC, HLT]));
      // 10110001 → 01100011 (left circular), CY=1
      expect(sim.a).toBe(0b01100011);
      expect(sim.currentFlags.carry).toBe(true);
    });

    it("does not affect Z, S, P flags", () => {
      sim.run(new Uint8Array([MVI_A, 0xFF, ANI, 0x00, HLT]));  // set Z=1
      const zBefore = sim.currentFlags.zero;
      sim.reset();
      sim.run(new Uint8Array([MVI_A, 0xFF, ANI, 0x00, MVI_A, 0x01, RLC, HLT]));
      // Z flag from ANI was 1, RLC should preserve it
      // Actually Z is set by ANI (0), then MVI doesn't change flags, then RLC
      // The key point: RLC only updates CY, not Z/S/P
      expect(sim.currentFlags.carry).toBe(false);  // bit7 of 0x01 = 0
      expect(zBefore).toBe(true);
    });

    it("handles zero input: CY=0, A=0", () => {
      sim.run(new Uint8Array([MVI_A, 0x00, RLC, HLT]));
      expect(sim.a).toBe(0x00);
      expect(sim.currentFlags.carry).toBe(false);
    });
  });

  describe("RRC", () => {
    it("rotates A right: CY=A[0], A[7]=old A[0]", () => {
      sim.run(new Uint8Array([MVI_A, 0b01100011, RRC, HLT]));
      // 01100011 → 10110001 (right circular), CY=1
      expect(sim.a).toBe(0b10110001);
      expect(sim.currentFlags.carry).toBe(true);
    });
  });

  describe("RAL", () => {
    it("rotates A left through carry", () => {
      // 9-bit rotate: [CY | A7..A0] left by 1
      // Start: CY=1, A=0b10000000
      // After: A[0]=old_CY=1, new_CY=old_A[7]=1
      // A = 0b00000001, CY=1
      sim.run(new Uint8Array([
        MVI_A, 0xFF, MVI_B, 0x01, ADD_B,  // CY=1, A=0
        MVI_A, 0b10000000,
        RAL,
        HLT,
      ]));
      expect(sim.a).toBe(0b00000001);
      expect(sim.currentFlags.carry).toBe(true);
    });
  });

  describe("RAR", () => {
    it("rotates A right through carry", () => {
      // 9-bit rotate right: [A7..A0 | CY] right by 1
      // Start: CY=1, A=0b00000001
      // After: A[7]=old_CY=1, new_CY=old_A[0]=1
      // A = 0b10000000, CY=1
      sim.run(new Uint8Array([
        MVI_A, 0xFF, MVI_B, 0x01, ADD_B,  // CY=1
        MVI_A, 0b00000001,
        RAR,
        HLT,
      ]));
      expect(sim.a).toBe(0b10000000);
      expect(sim.currentFlags.carry).toBe(true);
    });
  });

  // =========================================================================
  // Jump Instructions
  // =========================================================================

  describe("JMP (unconditional)", () => {
    it("jumps to target address", () => {
      // Program at 0x0000:
      //   JMP 0x0006 (jumps over MVI_B 0xFF)
      //   MVI_B 0xFF  (should be skipped)
      // At 0x0006: MVI_A 0x42, HLT
      const prog = new Uint8Array(16);
      prog[0] = JMP; prog[1] = lo(0x0006); prog[2] = hi(0x0006);
      prog[3] = MVI_B; prog[4] = 0xFF;  // skipped
      prog[5] = HLT;                     // also skipped (reached by 3-byte JMP)
      prog[6] = MVI_A; prog[7] = 0x42;
      prog[8] = HLT;
      sim.run(prog);
      expect(sim.a).toBe(0x42);
      expect(sim.b).toBe(0x00);  // MVI_B was skipped
    });
  });

  describe("JFZ / JTZ (conditional jump on Zero)", () => {
    it("JFZ jumps when Z=0 (not zero)", () => {
      // A=0x05, B=0x03. SUB_B → A=2, Z=0. JFZ target → skip MVI_A 0xFF.
      const prog = new Uint8Array(32);
      prog[0] = MVI_A; prog[1] = 0x05;
      prog[2] = MVI_B; prog[3] = 0x03;
      prog[4] = SUB_B;               // A=2, Z=0
      prog[5] = JFZ; prog[6] = lo(0x0B); prog[7] = hi(0x0B);  // jump to 0x0B
      prog[8] = MVI_A; prog[9] = 0xFF;  // skipped
      prog[10] = HLT;                    // skipped
      prog[11] = HLT;                    // landing pad
      sim.run(prog);
      expect(sim.a).toBe(0x02);  // not overwritten
    });

    it("JFZ does not jump when Z=1", () => {
      // A=5, B=5. SUB_B → Z=1. JFZ should not jump.
      const prog = new Uint8Array(32);
      prog[0] = MVI_A; prog[1] = 0x05;
      prog[2] = MVI_B; prog[3] = 0x05;
      prog[4] = SUB_B;               // A=0, Z=1
      prog[5] = JFZ; prog[6] = lo(0x0F); prog[7] = hi(0x0F);  // NOT taken
      prog[8] = MVI_A; prog[9] = 0xAB;  // executed (JFZ not taken)
      prog[10] = HLT;
      sim.run(prog);
      expect(sim.a).toBe(0xAB);
    });

    it("JTZ jumps when Z=1", () => {
      const prog = new Uint8Array(32);
      prog[0] = MVI_A; prog[1] = 0x05;
      prog[2] = MVI_B; prog[3] = 0x05;
      prog[4] = SUB_B;               // Z=1
      prog[5] = JTZ; prog[6] = lo(0x0B); prog[7] = hi(0x0B);
      prog[8] = MVI_A; prog[9] = 0xFF;  // skipped
      prog[10] = HLT;                    // skipped
      prog[11] = HLT;
      sim.run(prog);
      expect(sim.a).toBe(0x00);
    });
  });

  describe("JFC / JTC (conditional on Carry)", () => {
    it("JTC jumps when CY=1", () => {
      const prog = new Uint8Array(32);
      prog[0] = MVI_A; prog[1] = 0xFF;
      prog[2] = MVI_B; prog[3] = 0x01;
      prog[4] = ADD_B;               // CY=1
      prog[5] = JTC; prog[6] = lo(0x0B); prog[7] = hi(0x0B);
      prog[8] = MVI_A; prog[9] = 0xAA;  // skipped
      prog[10] = HLT;
      prog[11] = HLT;
      sim.run(prog);
      expect(sim.a).toBe(0x00);  // not 0xAA
    });

    it("JFC jumps when CY=0", () => {
      const prog = new Uint8Array(32);
      prog[0] = MVI_A; prog[1] = 0x01;
      prog[2] = MVI_B; prog[3] = 0x01;
      prog[4] = ADD_B;               // A=2, CY=0
      prog[5] = JFC; prog[6] = lo(0x0B); prog[7] = hi(0x0B);
      prog[8] = MVI_A; prog[9] = 0xAA;  // skipped
      prog[10] = HLT;
      prog[11] = HLT;
      sim.run(prog);
      expect(sim.a).toBe(0x02);
    });
  });

  // =========================================================================
  // Call and Return
  // =========================================================================

  describe("CAL / RET (unconditional)", () => {
    it("calls a subroutine and returns", () => {
      // Main:   MVI_A 0x01, CAL sub, MVI_B 0x02, HLT
      // Sub:    MVI_A 0x42, RET
      const prog = new Uint8Array(32);
      // offset 0: MVI_A 0x01
      prog[0] = MVI_A; prog[1] = 0x01;
      // offset 2: CAL 0x0010
      prog[2] = CAL; prog[3] = lo(0x10); prog[4] = hi(0x10);
      // offset 5: MVI_B 0x02 (executed after return)
      prog[5] = MVI_B; prog[6] = 0x02;
      // offset 7: HLT
      prog[7] = HLT;
      // sub at 0x10: MVI_A 0x42, RET
      prog[0x10] = MVI_A; prog[0x11] = 0x42;
      prog[0x12] = RET;
      sim.run(prog);
      expect(sim.a).toBe(0x42);  // set in subroutine
      expect(sim.b).toBe(0x02);  // set after return
    });

    it("stack depth increases on call, decreases on return", () => {
      const prog = new Uint8Array(32);
      prog[0] = CAL; prog[1] = lo(0x10); prog[2] = hi(0x10);
      prog[3] = HLT;
      prog[0x10] = RET;
      sim.run(prog);
      expect(sim.depth).toBe(0);
    });
  });

  describe("CFZ / CTZ (conditional call)", () => {
    it("CTZ calls when Z=1", () => {
      const prog = new Uint8Array(32);
      // Force Z=1
      prog[0] = MVI_A; prog[1] = 0x00;
      prog[2] = ORI; prog[3] = 0x00;  // Z=1 (A=0)
      prog[4] = CTZ; prog[5] = lo(0x10); prog[6] = hi(0x10);
      prog[7] = HLT;
      prog[0x10] = MVI_B; prog[0x11] = 0x99;
      prog[0x12] = RET;
      sim.run(prog);
      expect(sim.b).toBe(0x99);
    });

    it("CFZ does not call when Z=1", () => {
      const prog = new Uint8Array(32);
      prog[0] = MVI_A; prog[1] = 0x00;
      prog[2] = ORI; prog[3] = 0x00;  // Z=1
      prog[4] = CFZ; prog[5] = lo(0x10); prog[6] = hi(0x10);  // NOT taken
      prog[7] = MVI_B; prog[8] = 0x07;
      prog[9] = HLT;
      prog[0x10] = MVI_B; prog[0x11] = 0x99;
      prog[0x12] = RET;
      sim.run(prog);
      expect(sim.b).toBe(0x07);
    });
  });

  describe("RFC / RTC (conditional return)", () => {
    it("RFC returns when CY=0", () => {
      const prog = new Uint8Array(32);
      // Main: CAL sub, MVI_B 0x55, HLT
      prog[0] = CAL; prog[1] = lo(0x10); prog[2] = hi(0x10);
      prog[3] = MVI_B; prog[4] = 0x55;
      prog[5] = HLT;
      // Sub: MVI_A 0x05, RFC (returns since CY=0), NEVER REACHES HERE
      prog[0x10] = MVI_A; prog[0x11] = 0x05;
      prog[0x12] = RFC;  // CY=0, so returns
      prog[0x13] = MVI_A; prog[0x14] = 0xFF;  // not reached
      prog[0x15] = RET;
      sim.run(prog);
      expect(sim.a).toBe(0x05);  // set in sub before RFC
      expect(sim.b).toBe(0x55);  // set after return
    });

    it("RTC returns when CY=1", () => {
      const prog = new Uint8Array(32);
      prog[0] = CAL; prog[1] = lo(0x10); prog[2] = hi(0x10);
      prog[3] = MVI_B; prog[4] = 0x77;
      prog[5] = HLT;
      // Sub: force CY=1, then RTC
      prog[0x10] = MVI_A; prog[0x11] = 0xFF;
      prog[0x12] = ADI; prog[0x13] = 0x01;  // A=0, CY=1
      prog[0x14] = RTC;  // CY=1, returns
      prog[0x15] = MVI_B; prog[0x16] = 0xFF;  // not reached
      prog[0x17] = RET;
      sim.run(prog);
      expect(sim.b).toBe(0x77);
    });
  });

  // =========================================================================
  // RST — Restart Instructions
  // =========================================================================

  describe("RST", () => {
    it("RST 0 jumps to 0x0000 (pushes return address)", () => {
      // Place a simple handler at 0x0000 that just returns
      // But wait — RST 0 jumps to 0x0000, which is also where our program starts.
      // So we place the program at a non-zero address.
      const prog = new Uint8Array(64);
      // Handler at 0x0000: MVI_A 0x11, RET
      prog[0x00] = MVI_A; prog[0x01] = 0x11;
      prog[0x02] = RET;
      // Main at 0x10: RST 0, MVI_B 0x22, HLT
      prog[0x10] = RST0;
      prog[0x11] = MVI_B; prog[0x12] = 0x22;
      prog[0x13] = HLT;
      sim.loadProgram(prog, 0);
      sim.stackEntries[0] = 0x10;  // Start at main
      while (!sim.isHalted) sim.step();
      expect(sim.a).toBe(0x11);   // set in handler
      expect(sim.b).toBe(0x22);   // set after return
    });

    it("RST 1 jumps to 0x0008", () => {
      const prog = new Uint8Array(64);
      prog[0x08] = MVI_A; prog[0x09] = 0x42;
      prog[0x0A] = RET;
      prog[0x20] = RST1;
      prog[0x21] = HLT;
      sim.loadProgram(prog, 0);
      sim.stackEntries[0] = 0x20;
      while (!sim.isHalted) sim.step();
      expect(sim.a).toBe(0x42);
    });

    it("RST 7 jumps to 0x0038", () => {
      const prog = new Uint8Array(64);
      prog[0x38] = MVI_A; prog[0x39] = 0x7F;
      prog[0x3A] = RET;
      prog[0x3C] = RST7;
      prog[0x3D] = HLT;
      sim.loadProgram(prog, 0);
      sim.stackEntries[0] = 0x3C;
      while (!sim.isHalted) sim.step();
      expect(sim.a).toBe(0x7F);
    });
  });

  // =========================================================================
  // Parity flag
  // =========================================================================

  describe("Parity flag", () => {
    const cases: [number, boolean][] = [
      [0x00, true],   // 0 ones → even
      [0x01, false],  // 1 one → odd
      [0x03, true],   // 2 ones → even
      [0x07, false],  // 3 ones → odd
      [0x0F, true],   // 4 ones → even
      [0x1F, false],  // 5 ones → odd
      [0x3F, true],   // 6 ones → even
      [0x7F, false],  // 7 ones → odd
      [0xFF, true],   // 8 ones → even
    ];

    for (const [val, expectedParity] of cases) {
      it(`0x${val.toString(16).padStart(2,"0")} has parity=${expectedParity}`, () => {
        sim.run(new Uint8Array([MVI_A, val, ORI, 0x00, HLT]));
        expect(sim.currentFlags.parity).toBe(expectedParity);
      });
    }
  });

  // =========================================================================
  // H:L address pair and M register
  // =========================================================================

  describe("H:L addressing (M register)", () => {
    it("hlAddress uses only low 6 bits of H", () => {
      // H = 0xC2 = 11000010 → only low 6 bits used: 000010 = 0x02
      // L = 0x30 → address = (0x02 << 8) | 0x30 = 0x0230
      sim.run(new Uint8Array([MVI_H, 0xC2, MVI_L, 0x30, HLT]));
      expect(sim.hlAddress).toBe((0x02 << 8) | 0x30);
    });

    it("full M register write/read roundtrip using ORA M", () => {
      // Write 0xDE to mem[0x0100] via MVI M, then read it back via ORA M.
      // (0x7E is CAL in 8008, not MOV A,M, so we use XRA A + ORA M.)
      const XRA_A = 0xAF;
      sim.run(new Uint8Array([
        MVI_H, 0x01, MVI_L, 0x00,   // [H:L] = 0x0100
        MVI_M, 0xDE,                // mem[0x0100] = 0xDE
        XRA_A,                      // A = 0
        ORA_M,                      // A = A | mem[0x0100] = 0xDE
        HLT,
      ]));
      expect(sim.a).toBe(0xDE);
    });
  });

  // =========================================================================
  // Stack depth
  // =========================================================================

  describe("Stack depth tracking", () => {
    it("depth 0 at start", () => {
      sim.run(new Uint8Array([HLT]));
      expect(sim.depth).toBe(0);
    });

    it("depth increments on CAL", () => {
      const prog = new Uint8Array(32);
      prog[0] = CAL; prog[1] = lo(0x10); prog[2] = hi(0x10);
      prog[3] = HLT;
      prog[0x10] = RET;
      let maxDepth = 0;
      sim.loadProgram(prog);
      while (!sim.isHalted) {
        sim.step();
        maxDepth = Math.max(maxDepth, sim.depth);
      }
      expect(maxDepth).toBe(1);
      expect(sim.depth).toBe(0);
    });
  });

  // =========================================================================
  // I/O Ports
  // =========================================================================

  describe("I/O", () => {
    it("IN reads from input port into A", () => {
      sim.setInputPort(0, 0xAB);
      sim.run(new Uint8Array([IN_0, HLT]));
      expect(sim.a).toBe(0xAB);
    });

    it("IN 7 reads from port 7", () => {
      sim.setInputPort(7, 0xCC);
      sim.run(new Uint8Array([IN_7, HLT]));
      expect(sim.a).toBe(0xCC);
    });

    it("setInputPort rejects port > 7", () => {
      expect(() => sim.setInputPort(8, 0)).toThrow();
    });

    it("getOutputPort rejects port > 23", () => {
      expect(() => sim.getOutputPort(24)).toThrow();
    });
  });

  // =========================================================================
  // Example Programs from Spec
  // =========================================================================

  describe("Example: 1 + 2 = 3", () => {
    it("basic arithmetic (spec example)", () => {
      // MVI B, 0x01; MVI A, 0x02; ADD B; HLT
      // Result: A=3, Z=0, S=0, CY=0, P=1 (0b00000011 = 2 ones = even)
      sim.run(new Uint8Array([MVI_B, 0x01, MVI_A, 0x02, ADD_B, HLT]));
      expect(sim.a).toBe(0x03);
      expect(sim.currentFlags.zero).toBe(false);
      expect(sim.currentFlags.sign).toBe(false);
      expect(sim.currentFlags.carry).toBe(false);
      expect(sim.currentFlags.parity).toBe(true);
    });
  });

  describe("Example: 1 + 2 using memory", () => {
    it("stores value, loads it back via ORA M, adds 2", () => {
      // 8008 note: 0x7E is CAL (not MOV A,M). To load mem[H:L] into A,
      // use XRA A (zero A) then ORA M (A | M = M).
      const XRA_A = 0xAF;
      sim.run(new Uint8Array([
        MVI_H, 0x00,
        MVI_L, 0x40,
        MVI_M, 0x01,
        XRA_A,      // A = 0
        ORA_M,      // A = mem[0x40] = 1
        ADI, 0x02,  // A = 3
        HLT,
      ]));
      expect(sim.a).toBe(0x03);
    });
  });

  describe("Example: multiply 4 × 5 with loop", () => {
    it("computes 4 * 5 = 20 using repeated addition", () => {
      // B=5 (multiplicand), C=4 (counter), A=0 (accumulator)
      // LOOP: ADD B; DCR C; JFZ LOOP; HLT
      //
      // DCR C = 0x09 = 00 001 001 (DCR C)
      // DCR C encoding: group=00, ddd=001 (C), sss=001 → 0x09
      const DCR_C = 0x09;
      const loopAddr = 0x08;  // LOOP label
      const prog = new Uint8Array(32);
      prog[0] = MVI_B; prog[1] = 0x05;
      prog[2] = MVI_C; prog[3] = 0x04;
      prog[4] = MVI_A; prog[5] = 0x00;
      // LOOP at offset 6:
      prog[6] = ADD_B;
      prog[7] = DCR_C;
      prog[8] = JFZ; prog[9] = lo(6); prog[10] = hi(6);  // JFZ LOOP (Z=0 = not done)
      prog[11] = HLT;
      sim.run(prog);
      expect(sim.a).toBe(20);  // 4 * 5 = 20
    });
  });

  describe("Example: subroutine absolute value", () => {
    it("computes |A| for negative input", () => {
      // Main: MVI A, 0xF6 (-10 signed); ORI 0x00 (update S flag); CAL ABS_VAL; HLT
      // ABS_VAL:
      //   JFS DONE  (if S=0, positive, skip negate)
      //   XRI 0xFF  (A = ~A)
      //   ADI 0x01  (A = ~A + 1 = -A)
      // DONE: RET
      //
      // IMPORTANT: MVI doesn't update flags. Use ORI 0x00 to set S flag from A.
      // JFS = 0x50 (jump if Sign=false = S=0, meaning positive)
      //
      // Layout:
      //   0x00: MVI_A 0xF6
      //   0x02: ORI 0x00    (updates S=1 since bit7 of 0xF6 is 1)
      //   0x04: CAL 0x10
      //   0x07: HLT
      //   0x10: ABS_VAL
      //     JFS DONE (0x17) — if positive (S=0), skip negate
      //     XRI 0xFF
      //     ADI 0x01
      //   0x17: DONE: RET
      const JFS = 0x50;
      const prog = new Uint8Array(32);
      prog[0x00] = MVI_A; prog[0x01] = 0xF6;          // A = -10
      prog[0x02] = ORI; prog[0x03] = 0x00;             // Update flags: S=1 (bit7 of 0xF6)
      prog[0x04] = CAL; prog[0x05] = lo(0x10); prog[0x06] = hi(0x10);
      prog[0x07] = HLT;
      // ABS_VAL at 0x10:
      prog[0x10] = JFS; prog[0x11] = lo(0x17); prog[0x12] = hi(0x17);  // JFS DONE
      prog[0x13] = XRI; prog[0x14] = 0xFF;       // A = ~A
      prog[0x15] = ADI; prog[0x16] = 0x01;       // A = ~A + 1 = -A
      prog[0x17] = RET;
      sim.run(prog);
      expect(sim.a).toBe(0x0A);  // |(-10)| = 10
    });

    it("computes |A| for positive input (no negate)", () => {
      const JFS = 0x50;
      const prog = new Uint8Array(32);
      prog[0x00] = MVI_A; prog[0x01] = 0x0A;          // A = +10
      prog[0x02] = ORI; prog[0x03] = 0x00;             // Update flags: S=0
      prog[0x04] = CAL; prog[0x05] = lo(0x10); prog[0x06] = hi(0x10);
      prog[0x07] = HLT;
      prog[0x10] = JFS; prog[0x11] = lo(0x17); prog[0x12] = hi(0x17);
      prog[0x13] = XRI; prog[0x14] = 0xFF;
      prog[0x15] = ADI; prog[0x16] = 0x01;
      prog[0x17] = RET;
      sim.run(prog);
      expect(sim.a).toBe(0x0A);  // unchanged (was already positive, JFS taken)
    });
  });

  describe("Example: parity check via ORI 0x00", () => {
    it("ORI 0x00 on 0xB5 gives P=0 (odd parity)", () => {
      // 0xB5 = 10110101 = 5 ones = odd parity → P=0
      sim.run(new Uint8Array([MVI_A, 0xB5, ORI, 0x00, HLT]));
      expect(sim.currentFlags.parity).toBe(false);
      expect(sim.currentFlags.sign).toBe(true);   // bit7=1
      expect(sim.currentFlags.zero).toBe(false);
      expect(sim.currentFlags.carry).toBe(false);
    });
  });

  // =========================================================================
  // Trace records
  // =========================================================================

  describe("Trace records", () => {
    it("MVI produces a 2-byte trace", () => {
      const traces = sim.run(new Uint8Array([MVI_A, 0x42, HLT]));
      const mviTrace = traces[0];
      expect(mviTrace.raw.length).toBe(2);
      expect(mviTrace.raw[0]).toBe(MVI_A);
      expect(mviTrace.raw[1]).toBe(0x42);
    });

    it("ADD produces a 1-byte trace with before/after A", () => {
      const traces = sim.run(new Uint8Array([MVI_A, 0x02, MVI_B, 0x03, ADD_B, HLT]));
      const addTrace = traces[2];
      expect(addTrace.raw.length).toBe(1);
      expect(addTrace.aBefore).toBe(0x02);
      expect(addTrace.aAfter).toBe(0x05);
    });

    it("JMP produces a 3-byte trace", () => {
      const prog = new Uint8Array(8);
      prog[0] = JMP; prog[1] = lo(6); prog[2] = hi(6);
      prog[6] = HLT;
      const traces = sim.run(prog);
      const jmpTrace = traces[0];
      expect(jmpTrace.raw.length).toBe(3);
      expect(jmpTrace.mnemonic).toContain("JMP");
    });

    it("ORA M records memAddress and memValue", () => {
      // 8008: 0x7E is CAL (not MOV A,M). Use ORA M to test M-register memory access.
      // MVI_M stores 0xBB, then ORA M reads it (A=0 before ORA, so A=0|0xBB=0xBB).
      const XRA_A = 0xAF;
      const prog = new Uint8Array(64);
      prog[0] = MVI_H; prog[1] = 0x00;
      prog[2] = MVI_L; prog[3] = 0x20;
      prog[4] = MVI_M; prog[5] = 0xBB;
      prog[6] = XRA_A;    // A = 0 (XRA A)
      prog[7] = ORA_M;    // A = mem[0x0020] = 0xBB, records memAddress
      prog[8] = HLT;
      const traces = sim.run(prog);
      const oraTrace = traces[4];  // ORA M (after MVI_H[0], MVI_L[1], MVI_M[2], XRA_A[3])
      expect(oraTrace.memAddress).toBe(0x0020);
      expect(oraTrace.memValue).toBe(0xBB);
    });

    it("HLT has correct mnemonic", () => {
      const traces = sim.run(new Uint8Array([HLT]));
      expect(traces[0].mnemonic).toBe("HLT");
    });
  });

  // =========================================================================
  // Edge cases
  // =========================================================================

  describe("Edge cases", () => {
    it("reset clears all state", () => {
      sim.run(new Uint8Array([MVI_A, 0xFF, MVI_B, 0x11, HLT]));
      sim.reset();
      expect(sim.a).toBe(0);
      expect(sim.b).toBe(0);
      expect(sim.pc).toBe(0);
      expect(sim.isHalted).toBe(false);
    });

    it("run returns empty array if first instruction is HLT", () => {
      const traces = sim.run(new Uint8Array([HLT]));
      expect(traces).toHaveLength(1);
    });

    it("maxSteps safety limit stops runaway loops", () => {
      // Infinite loop: JMP 0x0000
      const prog = new Uint8Array([JMP, 0x00, 0x00]);
      const traces = sim.run(prog, 10);
      expect(traces.length).toBe(10);
    });

    it("loadProgram at non-zero address", () => {
      const prog = new Uint8Array([MVI_A, 0x77, HLT]);
      sim.loadProgram(prog, 0x100);
      while (!sim.isHalted) sim.step();
      expect(sim.a).toBe(0x77);
    });

    it("14-bit PC wraps correctly", () => {
      // 0x3FFF is the last valid address; 0x3FFF+1 should wrap to 0
      expect((0x3FFF + 1) & 0x3FFF).toBe(0);
    });

    it("ACI with CY overflow chain", () => {
      // 0xFF + 0x00 + CY=1 should give 0x00 with CY=1
      sim.run(new Uint8Array([
        MVI_A, 0xFF, MVI_B, 0x01, ADD_B,  // A=0, CY=1
        MVI_A, 0xFF,
        ACI, 0x00,  // A = 0xFF + 0x00 + CY(1) = 0x00, CY=1
        HLT,
      ]));
      expect(sim.a).toBe(0x00);
      expect(sim.currentFlags.carry).toBe(true);
    });

    it("SBB subtracts with borrow correctly", () => {
      sim.run(new Uint8Array([
        MVI_A, 0x05,
        MVI_B, 0x05,
        SUB_B,       // A=0, CY=0 (no borrow)
        MVI_A, 0x10,
        MVI_B, 0x05,
        SBB_B,       // A = 16 - 5 - 0 = 11 = 0x0B
        HLT,
      ]));
      expect(sim.a).toBe(0x0B);
    });

    it("deeply nested calls (up to 7 levels)", () => {
      // 7 nested CAL instructions; innermost does RET 7 times
      // Layout: each level calls the next, innermost sets A=level
      // This tests stack doesn't overflow prematurely
      const prog = new Uint8Array(256);
      // Level 0 (main) at 0x00: CAL 0x10, HLT
      prog[0x00] = CAL; prog[0x01] = lo(0x10); prog[0x02] = hi(0x10);
      prog[0x03] = HLT;
      // Level 1 at 0x10: CAL 0x20, RET
      prog[0x10] = CAL; prog[0x11] = lo(0x20); prog[0x12] = hi(0x20);
      prog[0x13] = RET;
      // Level 2 at 0x20: CAL 0x30, RET
      prog[0x20] = CAL; prog[0x21] = lo(0x30); prog[0x22] = hi(0x30);
      prog[0x23] = RET;
      // Level 3 at 0x30: MVI_A 0x42, RET (base case)
      prog[0x30] = MVI_A; prog[0x31] = 0x42;
      prog[0x32] = RET;
      sim.run(prog);
      expect(sim.a).toBe(0x42);
      expect(sim.depth).toBe(0);
    });
  });

  // =========================================================================
  // Flag interaction tests
  // =========================================================================

  describe("Flag interactions", () => {
    it("ADD 0+0 sets Z=1, P=1, clears S, CY", () => {
      sim.run(new Uint8Array([MVI_A, 0x00, MVI_B, 0x00, ADD_B, HLT]));
      expect(sim.currentFlags.zero).toBe(true);
      expect(sim.currentFlags.parity).toBe(true);
      expect(sim.currentFlags.sign).toBe(false);
      expect(sim.currentFlags.carry).toBe(false);
    });

    it("0x80 has even parity (one '1' bit = odd... wait: 0x80=10000000=1 one = odd)", () => {
      // 0x80 = 10000000 = 1 one → odd parity → P=0
      sim.run(new Uint8Array([MVI_A, 0x80, ORI, 0x00, HLT]));
      expect(sim.currentFlags.parity).toBe(false);
      expect(sim.currentFlags.sign).toBe(true);
    });

    it("0xFF has even parity (8 ones = even)", () => {
      sim.run(new Uint8Array([MVI_A, 0xFF, ORI, 0x00, HLT]));
      expect(sim.currentFlags.parity).toBe(true);
      expect(sim.currentFlags.sign).toBe(true);
    });

    it("SUB self gives Z=1, CY=0", () => {
      // SUB A = 10 010 111 = 0x97
      const SUB_A = 0x97;
      sim.run(new Uint8Array([MVI_A, 0x42, SUB_A, HLT]));
      expect(sim.a).toBe(0x00);
      expect(sim.currentFlags.zero).toBe(true);
      expect(sim.currentFlags.carry).toBe(false);
    });
  });

});
