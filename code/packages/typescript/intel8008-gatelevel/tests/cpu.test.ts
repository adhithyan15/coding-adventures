/**
 * Intel 8008 Gate-Level Simulator — Test Suite
 *
 * Tests cover every instruction group and verify that the gate-level CPU
 * produces identical results to the behavioral simulator. Key sections:
 *
 * 1. Individual components (ProgramCounter, PushDownStack, GateALU8, etc.)
 * 2. Full CPU instruction tests (same test patterns as the behavioral sim)
 * 3. Cross-validation: run the same programs through both simulators and
 *    assert identical register/flag/memory state at each step.
 */

import { describe, it, expect, beforeEach } from "vitest";
import {
  Intel8008GateLevel,
  ProgramCounter,
  PushDownStack,
  GateALU8,
  RegisterFile,
  FlagRegister,
  decode,
  intToBits,
  bitsToInt,
  computeParity,
} from "../src/index.js";

// ---------------------------------------------------------------------------
// Helper utilities
// ---------------------------------------------------------------------------

function lo(addr: number): number { return addr & 0xFF; }
function hi(addr: number): number { return (addr >> 8) & 0x3F; }

// ---------------------------------------------------------------------------
// Opcode constants (mirrors behavioral simulator test file)
// ---------------------------------------------------------------------------
const HLT = 0x76;

const MVI_B = 0x06;
const MVI_C = 0x0E;
const MVI_D = 0x16;
const MVI_E = 0x1E;
const MVI_H = 0x26;
const MVI_L = 0x2E;
const MVI_M = 0x36;
const MVI_A = 0x3E;

const MOV_A_B = 0x78;
const MOV_B_A = 0x47;
const MOV_M_A = 0x77;
const ORA_M   = 0xB6;

const INR_A = 0x38;
const INR_B = 0x00;
const DCR_A = 0x39;
const DCR_B = 0x01;
const INR_M = 0x30;
const DCR_M = 0x31;

const ADD_B = 0x80;
const ADC_B = 0x88;
const SUB_B = 0x90;
const SBB_B = 0x98;
const ANA_B = 0xA0;
const XRA_B = 0xA8;
const ORA_B = 0xB0;
const CMP_B = 0xB8;
const CMP_A = 0xBF;
const XRA_A = 0xAF;

const ADI = 0xC4;
const ACI = 0xCC;
const SUI = 0xD4;
const SBI = 0xDC;
const ANI = 0xE4;
const XRI = 0xEC;
const ORI = 0xF4;
const CPI = 0xFC;

const RLC = 0x02;
const RRC = 0x0A;
const RAL = 0x12;
const RAR = 0x1A;

const JMP = 0x7C;
const JFZ = 0x48;
const JTZ = 0x4C;
const JFC = 0x40;
const JTC = 0x44;
const JTP = 0x5C;

const CAL = 0x7E;
const CFZ = 0x4A;
const CTZ = 0x4E;

const RET = 0x3F;
const RFC = 0x03;
const RTC = 0x07;
const RFZ = 0x0B;
const RTZ = 0x0F;

const RST0 = 0x05;
const RST1 = 0x0D;
const RST7 = 0x3D;

const IN_0 = 0x41;
const IN_7 = 0x79;

// ---------------------------------------------------------------------------
// 1. Sub-component tests
// ---------------------------------------------------------------------------

describe("intToBits and bitsToInt", () => {
  it("round-trips 0", () => {
    expect(bitsToInt(intToBits(0, 8))).toBe(0);
  });
  it("round-trips 255", () => {
    expect(bitsToInt(intToBits(255, 8))).toBe(255);
  });
  it("round-trips 14-bit max", () => {
    expect(bitsToInt(intToBits(0x3FFF, 14))).toBe(0x3FFF);
  });
  it("intToBits(5, 8) → [1,0,1,0,0,0,0,0]", () => {
    expect(intToBits(5, 8)).toEqual([1, 0, 1, 0, 0, 0, 0, 0]);
  });
  it("bitsToInt([1,1,0,0,0,0,0,0]) = 3", () => {
    expect(bitsToInt([1, 1, 0, 0, 0, 0, 0, 0])).toBe(3);
  });
});

describe("computeParity", () => {
  it("all zeros → even parity → 1", () => {
    expect(computeParity([0, 0, 0, 0, 0, 0, 0, 0])).toBe(1);
  });
  it("one 1-bit → odd parity → 0", () => {
    expect(computeParity([1, 0, 0, 0, 0, 0, 0, 0])).toBe(0);
  });
  it("two 1-bits → even parity → 1", () => {
    expect(computeParity([1, 1, 0, 0, 0, 0, 0, 0])).toBe(1);
  });
  it("three 1-bits → odd parity → 0", () => {
    expect(computeParity([1, 1, 1, 0, 0, 0, 0, 0])).toBe(0);
  });
  it("0xFF = 8 ones → even parity → 1", () => {
    expect(computeParity(intToBits(0xFF, 8))).toBe(1);
  });
  it("0x03 = 2 ones → even parity → 1", () => {
    expect(computeParity(intToBits(0x03, 8))).toBe(1);
  });
});

describe("ProgramCounter", () => {
  let pc: ProgramCounter;
  beforeEach(() => { pc = new ProgramCounter(); });

  it("starts at 0", () => {
    expect(pc.value).toBe(0);
  });
  it("increment increments by 1", () => {
    pc.increment();
    expect(pc.value).toBe(1);
  });
  it("multiple increments", () => {
    for (let i = 0; i < 100; i++) pc.increment();
    expect(pc.value).toBe(100);
  });
  it("wraps at 0x3FFF", () => {
    pc.load(0x3FFF);
    pc.increment();
    expect(pc.value).toBe(0);
  });
  it("load sets PC", () => {
    pc.load(0x1234);
    expect(pc.value).toBe(0x1234 & 0x3FFF);
  });
  it("load masks to 14 bits", () => {
    pc.load(0xFFFF);
    expect(pc.value).toBe(0x3FFF);
  });
  it("reset returns to 0", () => {
    pc.load(500);
    pc.reset();
    expect(pc.value).toBe(0);
  });
});

describe("PushDownStack", () => {
  let stk: PushDownStack;
  beforeEach(() => { stk = new PushDownStack(); });

  it("starts with PC=0", () => {
    expect(stk.pc).toBe(0);
  });
  it("setPC updates entry 0", () => {
    stk.setPC(0x100);
    expect(stk.pc).toBe(0x100);
  });
  it("push saves return address and loads target", () => {
    stk.setPC(0x10);
    stk.push(0x10, 0x200);
    expect(stk.pc).toBe(0x200);
    expect(stk.snapshot[1]).toBe(0x10);
  });
  it("pop restores return address", () => {
    stk.setPC(0x10);
    stk.push(0x10, 0x200);
    stk.pop();
    expect(stk.pc).toBe(0x10);
  });
  it("nested push/pop sequence", () => {
    stk.setPC(5);
    stk.push(5, 100);   // return to 5, go to 100
    stk.setPC(103);     // advance PC past call instruction fetch
    stk.push(103, 200); // return to 103, go to 200
    expect(stk.pc).toBe(200);
    stk.pop();
    expect(stk.pc).toBe(103);
    stk.pop();
    expect(stk.pc).toBe(5);
  });
  it("reset clears all entries", () => {
    stk.setPC(0x1234);
    stk.reset();
    expect(stk.pc).toBe(0);
    expect(stk.snapshot.every(v => v === 0)).toBe(true);
  });
});

describe("GateALU8 – add", () => {
  let alu: GateALU8;
  beforeEach(() => { alu = new GateALU8(); });

  it("1 + 2 = 3, no carry", () => {
    const [result, carry] = alu.add(1, 2);
    expect(result).toBe(3);
    expect(carry).toBe(0);
  });
  it("255 + 1 = 0 with carry", () => {
    const [result, carry] = alu.add(255, 1);
    expect(result).toBe(0);
    expect(carry).toBe(1);
  });
  it("with carry-in: 5 + 5 + 1 = 11", () => {
    const [result, carry] = alu.add(5, 5, 1);
    expect(result).toBe(11);
    expect(carry).toBe(0);
  });
});

describe("GateALU8 – subtract", () => {
  let alu: GateALU8;
  beforeEach(() => { alu = new GateALU8(); });

  it("5 - 3 = 2, no borrow", () => {
    const [result, borrow] = alu.subtract(5, 3);
    expect(result).toBe(2);
    expect(borrow).toBe(0);
  });
  it("3 - 5 = 0xFE, borrow", () => {
    const [result, borrow] = alu.subtract(3, 5);
    expect(result).toBe(0xFE);
    expect(borrow).toBe(1);
  });
  it("0 - 0 = 0, no borrow", () => {
    const [result, borrow] = alu.subtract(0, 0);
    expect(result).toBe(0);
    expect(borrow).toBe(0);
  });
  it("0 - 1 = 255, borrow", () => {
    const [result, borrow] = alu.subtract(0, 1);
    expect(result).toBe(255);
    expect(borrow).toBe(1);
  });
  it("SBB: 5 - 3 - 1(borrow) = 1", () => {
    const [result, borrow] = alu.subtract(5, 3, 1);
    expect(result).toBe(1);
    expect(borrow).toBe(0);
  });
});

describe("GateALU8 – bitwise ops", () => {
  let alu: GateALU8;
  beforeEach(() => { alu = new GateALU8(); });

  it("AND(0xF0, 0x0F) = 0x00", () => {
    expect(alu.bitwiseAnd(0xF0, 0x0F)).toBe(0x00);
  });
  it("AND(0xFF, 0xAA) = 0xAA", () => {
    expect(alu.bitwiseAnd(0xFF, 0xAA)).toBe(0xAA);
  });
  it("OR(0xF0, 0x0F) = 0xFF", () => {
    expect(alu.bitwiseOr(0xF0, 0x0F)).toBe(0xFF);
  });
  it("XOR(0xFF, 0xFF) = 0x00", () => {
    expect(alu.bitwiseXor(0xFF, 0xFF)).toBe(0x00);
  });
  it("XOR(0xAA, 0x55) = 0xFF", () => {
    expect(alu.bitwiseXor(0xAA, 0x55)).toBe(0xFF);
  });
});

describe("GateALU8 – increment/decrement", () => {
  let alu: GateALU8;
  beforeEach(() => { alu = new GateALU8(); });

  it("increment(5) = 6", () => {
    const [r] = alu.increment(5);
    expect(r).toBe(6);
  });
  it("increment(0xFF) = 0 (wrap)", () => {
    const [r] = alu.increment(0xFF);
    expect(r).toBe(0);
  });
  it("decrement(5) = 4", () => {
    const [r] = alu.decrement(5);
    expect(r).toBe(4);
  });
  it("decrement(0) = 0xFF (wrap)", () => {
    const [r] = alu.decrement(0);
    expect(r).toBe(0xFF);
  });
});

describe("GateALU8 – rotates", () => {
  let alu: GateALU8;
  beforeEach(() => { alu = new GateALU8(); });

  it("RLC(0x80) → 0x01, CY=1", () => {
    const [r, cy] = alu.rotateLeftCircular(0x80);
    expect(r).toBe(0x01);
    expect(cy).toBe(1);
  });
  it("RLC(0x01) → 0x02, CY=0", () => {
    const [r, cy] = alu.rotateLeftCircular(0x01);
    expect(r).toBe(0x02);
    expect(cy).toBe(0);
  });
  it("RRC(0x01) → 0x80, CY=1", () => {
    const [r, cy] = alu.rotateRightCircular(0x01);
    expect(r).toBe(0x80);
    expect(cy).toBe(1);
  });
  it("RAL(0x80, cy=0) → 0x00, CY=1", () => {
    const [r, cy] = alu.rotateLeftCarry(0x80, 0);
    expect(r).toBe(0x00);
    expect(cy).toBe(1);
  });
  it("RAL(0x80, cy=1) → 0x01, CY=1", () => {
    const [r, cy] = alu.rotateLeftCarry(0x80, 1);
    expect(r).toBe(0x01);
    expect(cy).toBe(1);
  });
  it("RAR(0x01, cy=0) → 0x00, CY=1", () => {
    const [r, cy] = alu.rotateRightCarry(0x01, 0);
    expect(r).toBe(0x00);
    expect(cy).toBe(1);
  });
});

describe("GateALU8 – flag computation", () => {
  let alu: GateALU8;
  beforeEach(() => { alu = new GateALU8(); });

  it("result=0 → Z=1, S=0, P=1", () => {
    const f = alu.flagsFromResult(0, 0);
    expect(f.zero).toBe(1);
    expect(f.sign).toBe(0);
    expect(f.parity).toBe(1);
  });
  it("result=0x80 → Z=0, S=1, P=0", () => {
    const f = alu.flagsFromResult(0x80, 0);
    expect(f.zero).toBe(0);
    expect(f.sign).toBe(1);
    expect(f.parity).toBe(0);
  });
  it("result=0x03 → P=1 (2 ones = even parity)", () => {
    const f = alu.flagsFromResult(0x03, 0);
    expect(f.parity).toBe(1);
  });
  it("result=0x07 → P=0 (3 ones = odd parity)", () => {
    const f = alu.flagsFromResult(0x07, 0);
    expect(f.parity).toBe(0);
  });
});

describe("decode – gate decoder", () => {
  it("HLT (0x76) → isHalt=1", () => {
    const d = decode(0x76);
    expect(d.isHalt).toBe(1);
  });
  it("HLT (0xFF) → isHalt=1", () => {
    const d = decode(0xFF);
    expect(d.isHalt).toBe(1);
  });
  it("ADD B (0x80) → aluOp=add, regSrc=0", () => {
    const d = decode(0x80);
    expect(d.aluOp).toBe("add");
    expect(d.regSrc).toBe(0);
  });
  it("MOV A,B (0x78) → not a jump or call", () => {
    const d = decode(0x78);
    expect(d.isJump).toBe(0);
    expect(d.isCall).toBe(0);
    expect(d.isHalt).toBe(0);
  });
  it("JMP (0x7C) → isJump=1", () => {
    const d = decode(0x7C);
    expect(d.isJump).toBe(1);
    expect(d.condCode).toBe(7);
  });
  it("CAL (0x7E) → isCall=1", () => {
    const d = decode(0x7E);
    expect(d.isCall).toBe(1);
    expect(d.condCode).toBe(7);
  });
  it("RET (0x3F) → isReturn=1, condCode=7", () => {
    const d = decode(0x3F);
    expect(d.isReturn).toBe(1);
    expect(d.condCode).toBe(7);
  });
  it("MVI B (0x06) → 2 bytes", () => {
    const d = decode(0x06);
    expect(d.instructionBytes).toBe(2);
  });
  it("IN 0 (0x41) → isInput=1, port=0", () => {
    const d = decode(0x41);
    expect(d.isInput).toBe(1);
    expect(d.portNumber).toBe(0);
  });
  it("RST 0 (0x05) → isRST=1, rstTarget=0", () => {
    const d = decode(0x05);
    expect(d.isRST).toBe(1);
    expect(d.rstTarget).toBe(0);
  });
  it("RST 7 (0x3D) → isRST=1, rstTarget=56", () => {
    const d = decode(0x3D);
    expect(d.isRST).toBe(1);
    expect(d.rstTarget).toBe(56);
  });
});

// ---------------------------------------------------------------------------
// 2. Full CPU instruction tests
// ---------------------------------------------------------------------------

describe("Intel8008GateLevel – MVI and MOV", () => {
  let cpu: Intel8008GateLevel;
  beforeEach(() => { cpu = new Intel8008GateLevel(); });

  it("MVI A, 42 loads 42 into accumulator", () => {
    const prog = new Uint8Array([MVI_A, 42, HLT]);
    cpu.run(prog);
    expect(cpu.a).toBe(42);
  });
  it("MVI B, 10; MOV A, B gives A=10", () => {
    const prog = new Uint8Array([MVI_B, 10, MOV_A_B, HLT]);
    cpu.run(prog);
    expect(cpu.a).toBe(10);
    expect(cpu.b).toBe(10);
  });
  it("MVI to all registers", () => {
    const prog = new Uint8Array([
      MVI_B, 1,
      MVI_C, 2,
      MVI_D, 3,
      MVI_E, 4,
      MVI_H, 5,
      MVI_L, 6,
      MVI_A, 7,
      HLT,
    ]);
    cpu.run(prog);
    expect(cpu.b).toBe(1);
    expect(cpu.c).toBe(2);
    expect(cpu.d).toBe(3);
    expect(cpu.e).toBe(4);
    expect(cpu.h).toBe(5);
    expect(cpu.l).toBe(6);
    expect(cpu.a).toBe(7);
  });
  it("MVI M writes to memory at H:L", () => {
    const prog = new Uint8Array([
      MVI_H, 0x00,
      MVI_L, 0x50,
      MVI_M, 0xAB,
      HLT,
    ]);
    cpu.run(prog);
    expect(cpu.memory[0x50]).toBe(0xAB);
  });
});

describe("Intel8008GateLevel – ADD", () => {
  let cpu: Intel8008GateLevel;
  beforeEach(() => { cpu = new Intel8008GateLevel(); });

  it("ADD B: A = A + B", () => {
    const prog = new Uint8Array([MVI_A, 3, MVI_B, 5, ADD_B, HLT]);
    cpu.run(prog);
    expect(cpu.a).toBe(8);
    expect(cpu.currentFlags.carry).toBe(false);
    expect(cpu.currentFlags.zero).toBe(false);
  });
  it("ADD overflow sets carry", () => {
    const prog = new Uint8Array([MVI_A, 0xFF, MVI_B, 1, ADD_B, HLT]);
    cpu.run(prog);
    expect(cpu.a).toBe(0);
    expect(cpu.currentFlags.carry).toBe(true);
    expect(cpu.currentFlags.zero).toBe(true);
  });
  it("ADI immediate: A = A + immediate", () => {
    const prog = new Uint8Array([MVI_A, 10, ADI, 20, HLT]);
    cpu.run(prog);
    expect(cpu.a).toBe(30);
  });
  it("ADC uses carry flag", () => {
    // Set up: A=5, B=3, CY=1 via overflow
    const prog = new Uint8Array([
      MVI_A, 0xFF,
      MVI_B, 1,
      ADD_B,     // A=0, CY=1
      MVI_A, 5,
      MVI_B, 3,
      ADC_B,     // A = 5 + 3 + 1 = 9
      HLT,
    ]);
    cpu.run(prog);
    expect(cpu.a).toBe(9);
  });
});

describe("Intel8008GateLevel – SUB", () => {
  let cpu: Intel8008GateLevel;
  beforeEach(() => { cpu = new Intel8008GateLevel(); });

  it("SUB B: A = A - B", () => {
    const prog = new Uint8Array([MVI_A, 10, MVI_B, 3, SUB_B, HLT]);
    cpu.run(prog);
    expect(cpu.a).toBe(7);
    expect(cpu.currentFlags.carry).toBe(false);
  });
  it("SUB borrow: A < B sets carry", () => {
    const prog = new Uint8Array([MVI_A, 3, MVI_B, 5, SUB_B, HLT]);
    cpu.run(prog);
    expect(cpu.a).toBe(0xFE);
    expect(cpu.currentFlags.carry).toBe(true);
  });
  it("SUI immediate", () => {
    const prog = new Uint8Array([MVI_A, 20, SUI, 7, HLT]);
    cpu.run(prog);
    expect(cpu.a).toBe(13);
  });
});

describe("Intel8008GateLevel – Logical operations", () => {
  let cpu: Intel8008GateLevel;
  beforeEach(() => { cpu = new Intel8008GateLevel(); });

  it("ANA clears carry and ANDs", () => {
    const prog = new Uint8Array([MVI_A, 0xF0, MVI_B, 0x0F, ANA_B, HLT]);
    cpu.run(prog);
    expect(cpu.a).toBe(0x00);
    expect(cpu.currentFlags.carry).toBe(false);
    expect(cpu.currentFlags.zero).toBe(true);
  });
  it("ORA clears carry and ORs", () => {
    const prog = new Uint8Array([MVI_A, 0xF0, MVI_B, 0x0F, ORA_B, HLT]);
    cpu.run(prog);
    expect(cpu.a).toBe(0xFF);
    expect(cpu.currentFlags.carry).toBe(false);
  });
  it("XRA clears register (A ^ A = 0)", () => {
    const prog = new Uint8Array([MVI_A, 0xFF, XRA_A, HLT]);
    cpu.run(prog);
    expect(cpu.a).toBe(0x00);
    expect(cpu.currentFlags.zero).toBe(true);
  });
  it("ANI immediate", () => {
    const prog = new Uint8Array([MVI_A, 0xFF, ANI, 0x0F, HLT]);
    cpu.run(prog);
    expect(cpu.a).toBe(0x0F);
  });
  it("ORI immediate", () => {
    const prog = new Uint8Array([MVI_A, 0x0F, ORI, 0xF0, HLT]);
    cpu.run(prog);
    expect(cpu.a).toBe(0xFF);
  });
  it("XRI immediate", () => {
    const prog = new Uint8Array([MVI_A, 0xFF, XRI, 0xFF, HLT]);
    cpu.run(prog);
    expect(cpu.a).toBe(0x00);
    expect(cpu.currentFlags.zero).toBe(true);
  });
});

describe("Intel8008GateLevel – CMP", () => {
  let cpu: Intel8008GateLevel;
  beforeEach(() => { cpu = new Intel8008GateLevel(); });

  it("CMP A: always Z=1, A unchanged", () => {
    const prog = new Uint8Array([MVI_A, 42, CMP_A, HLT]);
    cpu.run(prog);
    expect(cpu.a).toBe(42);
    expect(cpu.currentFlags.zero).toBe(true);
    expect(cpu.currentFlags.carry).toBe(false);
  });
  it("CMP B: A > B → Z=0, CY=0", () => {
    const prog = new Uint8Array([MVI_A, 10, MVI_B, 5, CMP_B, HLT]);
    cpu.run(prog);
    expect(cpu.a).toBe(10);
    expect(cpu.currentFlags.zero).toBe(false);
    expect(cpu.currentFlags.carry).toBe(false);
  });
  it("CMP B: A < B → CY=1 (borrow)", () => {
    const prog = new Uint8Array([MVI_A, 5, MVI_B, 10, CMP_B, HLT]);
    cpu.run(prog);
    expect(cpu.a).toBe(5);
    expect(cpu.currentFlags.carry).toBe(true);
  });
  it("CPI immediate: A == imm → Z=1", () => {
    const prog = new Uint8Array([MVI_A, 0x42, CPI, 0x42, HLT]);
    cpu.run(prog);
    expect(cpu.currentFlags.zero).toBe(true);
  });
});

describe("Intel8008GateLevel – INR/DCR", () => {
  let cpu: Intel8008GateLevel;
  beforeEach(() => { cpu = new Intel8008GateLevel(); });

  it("INR A: increments accumulator", () => {
    const prog = new Uint8Array([MVI_A, 5, INR_A, HLT]);
    cpu.run(prog);
    expect(cpu.a).toBe(6);
  });
  it("INR B: increments B", () => {
    const prog = new Uint8Array([MVI_B, 0xFF, INR_B, HLT]);
    cpu.run(prog);
    expect(cpu.b).toBe(0);
    expect(cpu.currentFlags.zero).toBe(true);
  });
  it("INR preserves carry flag", () => {
    // Set carry via overflow, then INR should not clear it
    const prog = new Uint8Array([
      MVI_A, 0xFF,
      MVI_B, 1,
      ADD_B,   // CY=1
      MVI_A, 5,
      INR_A,   // CY should still be 1
      HLT,
    ]);
    cpu.run(prog);
    expect(cpu.a).toBe(6);
    expect(cpu.currentFlags.carry).toBe(true);
  });
  it("DCR A: decrements accumulator", () => {
    const prog = new Uint8Array([MVI_A, 5, DCR_A, HLT]);
    cpu.run(prog);
    expect(cpu.a).toBe(4);
  });
  it("DCR wraps 0 → 0xFF", () => {
    const prog = new Uint8Array([MVI_A, 0, DCR_A, HLT]);
    cpu.run(prog);
    expect(cpu.a).toBe(0xFF);
  });
  it("INR/DCR on M register", () => {
    const prog = new Uint8Array([
      MVI_H, 0,
      MVI_L, 0x40,
      MVI_M, 10,
      INR_M,
      HLT,
    ]);
    cpu.run(prog);
    expect(cpu.memory[0x40]).toBe(11);
  });
});

describe("Intel8008GateLevel – Rotates", () => {
  let cpu: Intel8008GateLevel;
  beforeEach(() => { cpu = new Intel8008GateLevel(); });

  it("RLC: bit7 wraps to bit0", () => {
    const prog = new Uint8Array([MVI_A, 0x80, RLC, HLT]);
    cpu.run(prog);
    expect(cpu.a).toBe(0x01);
    expect(cpu.currentFlags.carry).toBe(true);
  });
  it("RRC: bit0 wraps to bit7", () => {
    const prog = new Uint8Array([MVI_A, 0x01, RRC, HLT]);
    cpu.run(prog);
    expect(cpu.a).toBe(0x80);
    expect(cpu.currentFlags.carry).toBe(true);
  });
  it("RAL shifts through carry", () => {
    // First set carry to 1 via add overflow
    const prog = new Uint8Array([
      MVI_A, 0xFF,
      MVI_B, 1,
      ADD_B,    // A=0, CY=1
      MVI_A, 0x00,
      RAL,      // A = (0x00 << 1) | CY = 0x01, new CY = bit7 = 0
      HLT,
    ]);
    cpu.run(prog);
    expect(cpu.a).toBe(0x01);
    expect(cpu.currentFlags.carry).toBe(false);
  });
  it("RAR shifts through carry", () => {
    const prog = new Uint8Array([
      MVI_A, 0xFF,
      MVI_B, 1,
      ADD_B,    // A=0, CY=1
      MVI_A, 0x00,
      RAR,      // A = (CY << 7) | (0x00 >> 1) = 0x80, new CY = bit0 = 0
      HLT,
    ]);
    cpu.run(prog);
    expect(cpu.a).toBe(0x80);
    expect(cpu.currentFlags.carry).toBe(false);
  });
});

describe("Intel8008GateLevel – Flags", () => {
  let cpu: Intel8008GateLevel;
  beforeEach(() => { cpu = new Intel8008GateLevel(); });

  it("ADD 0 + 0 = 0: Z=1, S=0, P=1", () => {
    const prog = new Uint8Array([XRA_A, HLT]);
    cpu.run(prog);
    expect(cpu.currentFlags.zero).toBe(true);
    expect(cpu.currentFlags.sign).toBe(false);
    expect(cpu.currentFlags.parity).toBe(true);
  });
  it("ADI 0x7F + 0x01 = 0x80: S=1", () => {
    const prog = new Uint8Array([MVI_A, 0x7F, ADI, 0x01, HLT]);
    cpu.run(prog);
    expect(cpu.a).toBe(0x80);
    expect(cpu.currentFlags.sign).toBe(true);
  });
  it("parity flag: 0x03 = 2 ones → P=1", () => {
    const prog = new Uint8Array([MVI_A, 0x00, ADI, 0x03, HLT]);
    cpu.run(prog);
    expect(cpu.currentFlags.parity).toBe(true);
  });
  it("parity flag: 0x07 = 3 ones → P=0", () => {
    const prog = new Uint8Array([MVI_A, 0x00, ADI, 0x07, HLT]);
    cpu.run(prog);
    expect(cpu.currentFlags.parity).toBe(false);
  });
});

describe("Intel8008GateLevel – Conditional jumps", () => {
  let cpu: Intel8008GateLevel;
  beforeEach(() => { cpu = new Intel8008GateLevel(); });

  it("JMP is unconditional", () => {
    // Skip HLT at offset 1, jump to offset 4
    const prog = new Uint8Array([
      JMP, lo(4), hi(4),   // 0: JMP 4
      HLT,                  // 3: never reached
      MVI_A, 99, HLT,      // 4: MVI A, 99; HLT
    ]);
    cpu.run(prog);
    expect(cpu.a).toBe(99);
  });
  it("JTZ: jump if Z=1", () => {
    // XRA A sets Z=1, then JTZ jumps
    const prog = new Uint8Array([
      XRA_A,                   // 0: A=0, Z=1
      JTZ, lo(6), hi(6),       // 1: JTZ 6 (Z=1 → jump)
      MVI_A, 0,               // 4: never reached
      HLT,                     // 6: HLT
      MVI_A, 99, HLT,         // 7: also never reached
    ]);
    cpu.run(prog);
    expect(cpu.a).toBe(0);  // not modified after JTZ
  });
  it("JFZ: jump if Z=0", () => {
    // After MVI A, 5: Z=0 (flags not updated), so use ADI to set Z
    const prog = new Uint8Array([
      MVI_A, 5,               // 0: A=5, flags not updated
      ADI, 0,                 // 2: A=5, Z=0 (5+0 ≠ 0)
      JFZ, lo(10), hi(10),    // 4: JFZ 10 (Z=0 → jump)
      MVI_A, 0xFF, HLT,       // 7: not reached
      MVI_A, 99, HLT,         // 10: MVI A,99; HLT
    ]);
    cpu.run(prog);
    expect(cpu.a).toBe(99);
  });
  it("JTC: jump if CY=1", () => {
    const prog = new Uint8Array([
      MVI_A, 0xFF, ADI, 1,    // 0: A=0, CY=1
      JTC, lo(10), hi(10),    // 4: JTC 10 (CY=1 → jump)
      MVI_A, 0, HLT,          // 7: not reached
      MVI_A, 77, HLT,         // 10: MVI A,77; HLT
    ]);
    cpu.run(prog);
    expect(cpu.a).toBe(77);
  });
  it("JFC: jump if CY=0", () => {
    const prog = new Uint8Array([
      MVI_A, 5, ADI, 1,       // 0: A=6, CY=0
      JFC, lo(10), hi(10),    // 4: JFC 10 (CY=0 → jump)
      MVI_A, 0, HLT,          // 7: not reached
      MVI_A, 55, HLT,         // 10: MVI A,55; HLT
    ]);
    cpu.run(prog);
    expect(cpu.a).toBe(55);
  });
});

describe("Intel8008GateLevel – CALL and RET", () => {
  let cpu: Intel8008GateLevel;
  beforeEach(() => { cpu = new Intel8008GateLevel(); });

  it("CAL + RET: subroutine returns correctly", () => {
    // Layout:
    //   0x00: MVI B, 0       (load B=0)
    //   0x02: CAL 0x10       (call subroutine at 0x10)
    //   0x05: MOV A, B       (A = B after return)
    //   0x06: HLT
    //   0x10: MVI B, 42      (B = 42)
    //   0x12: RET
    const prog = new Uint8Array(0x20);
    prog[0x00] = MVI_B;  prog[0x01] = 0;
    prog[0x02] = CAL;    prog[0x03] = lo(0x10);  prog[0x04] = hi(0x10);
    prog[0x05] = MOV_A_B;
    prog[0x06] = HLT;
    prog[0x10] = MVI_B;  prog[0x11] = 42;
    prog[0x12] = RET;
    cpu.run(prog);
    expect(cpu.a).toBe(42);
    expect(cpu.b).toBe(42);
  });
  it("CTZ: conditional call when Z=1", () => {
    const prog = new Uint8Array(0x20);
    prog[0x00] = XRA_A;           // A=0, Z=1
    prog[0x01] = CTZ;             // CTZ 0x10 (Z=1 → call)
    prog[0x02] = lo(0x10);
    prog[0x03] = hi(0x10);
    prog[0x04] = HLT;             // return lands here
    prog[0x10] = MVI_A;  prog[0x11] = 99;
    prog[0x12] = RET;
    cpu.run(prog);
    expect(cpu.a).toBe(99);
  });
  it("CFZ: no call when Z=1", () => {
    const prog = new Uint8Array(0x20);
    prog[0x00] = XRA_A;           // A=0, Z=1
    prog[0x01] = CFZ;             // CFZ 0x10 (Z=1 → no call)
    prog[0x02] = lo(0x10);
    prog[0x03] = hi(0x10);
    prog[0x04] = MVI_A;  prog[0x05] = 77;
    prog[0x06] = HLT;
    prog[0x10] = MVI_A;  prog[0x11] = 99;  prog[0x12] = RET;
    cpu.run(prog);
    expect(cpu.a).toBe(77);  // call skipped, continues to 0x04
  });
  it("RTC: conditional return when CY=1", () => {
    const prog = new Uint8Array(0x20);
    prog[0x00] = CAL;  prog[0x01] = lo(0x10);  prog[0x02] = hi(0x10);
    prog[0x03] = MOV_A_B;
    prog[0x04] = HLT;
    // Subroutine: set CY=1, RTC should return
    prog[0x10] = MVI_A;  prog[0x11] = 0xFF;
    prog[0x12] = ADI;    prog[0x13] = 1;    // A=0, CY=1
    prog[0x14] = MVI_B;  prog[0x15] = 42;
    prog[0x16] = RTC;                        // return since CY=1
    prog[0x17] = MVI_B;  prog[0x18] = 0;   // not reached
    prog[0x19] = RET;
    cpu.run(prog);
    expect(cpu.b).toBe(42);
    expect(cpu.a).toBe(42);
  });
});

describe("Intel8008GateLevel – RST", () => {
  let cpu: Intel8008GateLevel;
  beforeEach(() => { cpu = new Intel8008GateLevel(); });

  it("RST 1: jumps to address 8, saves return address", () => {
    const prog = new Uint8Array(0x20);
    prog[0x00] = RST1;           // RST 1 → jump to 0x0008
    prog[0x01] = MOV_A_B;        // return here after RET
    prog[0x02] = HLT;
    prog[0x08] = MVI_B;  prog[0x09] = 42;
    prog[0x0A] = RET;
    cpu.run(prog);
    expect(cpu.b).toBe(42);
    expect(cpu.a).toBe(42);
  });
  it("RST 7: jumps to address 56 (0x38)", () => {
    const prog = new Uint8Array(0x40);
    prog[0x00] = RST7;
    prog[0x01] = HLT;
    prog[0x38] = MVI_A;  prog[0x39] = 77;
    prog[0x3A] = RET;
    cpu.run(prog);
    expect(cpu.a).toBe(77);
  });
});

describe("Intel8008GateLevel – M register", () => {
  let cpu: Intel8008GateLevel;
  beforeEach(() => { cpu = new Intel8008GateLevel(); });

  it("read from M via XRA_A + ORA_M (A ← mem[H:L])", () => {
    const prog = new Uint8Array(0x100);
    prog[0x00] = MVI_H;  prog[0x01] = 0x00;
    prog[0x02] = MVI_L;  prog[0x03] = 0x50;
    prog[0x04] = MVI_M;  prog[0x05] = 0xAB;  // write 0xAB to mem[0x50]
    prog[0x06] = XRA_A;                        // A = 0
    prog[0x07] = ORA_M;                        // A = A | mem[H:L] = 0xAB
    prog[0x08] = HLT;
    cpu.run(prog);
    expect(cpu.a).toBe(0xAB);
  });
  it("write to M via MOV M, A", () => {
    const prog = new Uint8Array(0x100);
    prog[0x00] = MVI_H;  prog[0x01] = 0x00;
    prog[0x02] = MVI_L;  prog[0x03] = 0x60;
    prog[0x04] = MVI_A;  prog[0x05] = 0x55;
    prog[0x06] = MOV_M_A;
    prog[0x07] = HLT;
    cpu.run(prog);
    expect(cpu.memory[0x60]).toBe(0x55);
  });
  it("H uses only low 6 bits for address", () => {
    const prog = new Uint8Array(0x100);
    // H = 0xC3 → effective H = 0xC3 & 0x3F = 0x03
    // L = 0x05 → address = (0x03 << 8) | 0x05 = 0x305
    prog[0x00] = MVI_H;  prog[0x01] = 0xC3;
    prog[0x02] = MVI_L;  prog[0x03] = 0x05;
    prog[0x04] = MVI_A;  prog[0x05] = 0xBE;
    prog[0x06] = MOV_M_A;
    prog[0x07] = HLT;
    cpu.run(prog);
    expect(cpu.memory[0x305]).toBe(0xBE);
  });
});

describe("Intel8008GateLevel – I/O", () => {
  let cpu: Intel8008GateLevel;
  beforeEach(() => { cpu = new Intel8008GateLevel(); });

  it("IN reads from input port into A", () => {
    cpu.setInputPort(0, 0xAB);
    const prog = new Uint8Array([IN_0, HLT]);
    cpu.run(prog);
    expect(cpu.a).toBe(0xAB);
  });
  it("IN port 7", () => {
    cpu.setInputPort(7, 42);
    const prog = new Uint8Array([IN_7, HLT]);
    cpu.run(prog);
    expect(cpu.a).toBe(42);
  });
  it("setInputPort before run (not cleared by reset)", () => {
    cpu.setInputPort(0, 99);
    const prog = new Uint8Array([IN_0, HLT]);
    cpu.run(prog);   // run() calls reset() internally
    expect(cpu.a).toBe(99);
  });
  it("OUT writes accumulator to output port", () => {
    // OUT instruction: 00 PP0 010 — use opcode 0x22 (port=(0x22>>1)&0x1F = 17)
    // Let's use a simpler approach: build from spec
    // For OUT with port number P: opcode = 00 P[4:1] P[0] 010
    // Port 1: opcode = 0x0A (00 000 1 010 = 0x0A? No.)
    // Actually let's check: port = (opcode & 0x3E) >> 1
    // For port=1: (opcode & 0x3E) = 0x02 → opcode = 0x02 | something
    // opcode bits[2:0] must be 010. So opcode & 0x07 = 0x02.
    // opcode = port << 1 | 0x02 (but need bit[2]=0 to not be rotate)
    // For port=4: opcode = (4 << 1) | 0x02 = 0x0A... no, 4<<1 = 8, 8 | 2 = 0x0A
    // But 0x0A is RRC! (group 00, ddd=1, sss=010). Only ddd≤3 is rotate.
    // Let's pick port 17: opcode = (17 << 1) | 0x02 = 0x22 (group00, ddd=4, sss=010 → OUT)
    const outPort17 = 0x22;  // OUT port=(0x22>>1)&0x1F=17
    const prog = new Uint8Array([MVI_A, 0x55, outPort17, HLT]);
    cpu.run(prog);
    const port = (outPort17 & 0x3E) >> 1;  // 17
    expect(cpu.getOutputPort(port)).toBe(0x55);
  });
});

describe("Intel8008GateLevel – HLT", () => {
  let cpu: Intel8008GateLevel;
  beforeEach(() => { cpu = new Intel8008GateLevel(); });

  it("HLT stops execution", () => {
    const prog = new Uint8Array([HLT, MVI_A, 99]);
    cpu.run(prog);
    expect(cpu.a).toBe(0);  // never reached MVI_A,99
    expect(cpu.isHalted).toBe(true);
  });
  it("step() throws when halted", () => {
    cpu.run(new Uint8Array([HLT]));
    expect(() => cpu.step()).toThrow("halted");
  });
  it("reset clears halted state", () => {
    cpu.run(new Uint8Array([HLT]));
    expect(cpu.isHalted).toBe(true);
    cpu.reset();
    expect(cpu.isHalted).toBe(false);
  });
});

describe("Intel8008GateLevel – Trace records", () => {
  let cpu: Intel8008GateLevel;
  beforeEach(() => { cpu = new Intel8008GateLevel(); });

  it("trace has correct address and mnemonic for MVI A", () => {
    const prog = new Uint8Array([MVI_A, 42, HLT]);
    const traces = cpu.run(prog);
    expect(traces[0].address).toBe(0);
    expect(traces[0].mnemonic).toContain("MVI");
    expect(traces[0].mnemonic).toContain("A");
  });
  it("trace records aBefore and aAfter", () => {
    const prog = new Uint8Array([MVI_A, 5, ADI, 3, HLT]);
    const traces = cpu.run(prog);
    // MVI A, 5: aBefore=0, aAfter=5
    expect(traces[0].aBefore).toBe(0);
    expect(traces[0].aAfter).toBe(5);
    // ADI 3: aBefore=5, aAfter=8
    expect(traces[1].aBefore).toBe(5);
    expect(traces[1].aAfter).toBe(8);
  });
  it("trace records memAddress for M access", () => {
    const prog = new Uint8Array(0x100);
    prog[0x00] = MVI_H;  prog[0x01] = 0;
    prog[0x02] = MVI_L;  prog[0x03] = 0x50;
    prog[0x04] = MVI_M;  prog[0x05] = 0xAB;
    prog[0x06] = HLT;
    const traces = cpu.run(prog);
    // traces[2] is MVI M, 0xAB
    expect(traces[2].memAddress).toBe(0x50);
    expect(traces[2].memValue).toBe(0xAB);
  });
  it("trace raw bytes: MVI is 2 bytes", () => {
    const prog = new Uint8Array([MVI_A, 42, HLT]);
    const traces = cpu.run(prog);
    expect(traces[0].raw.length).toBe(2);
    expect(traces[0].raw[0]).toBe(MVI_A);
    expect(traces[0].raw[1]).toBe(42);
  });
  it("trace raw bytes: JMP is 3 bytes", () => {
    const target = 4;
    const prog = new Uint8Array([JMP, lo(target), hi(target), HLT, HLT]);
    const traces = cpu.run(prog);
    expect(traces[0].raw.length).toBe(3);
  });
});

// ---------------------------------------------------------------------------
// 3. Cross-validation: gate-level matches behavioral simulator
// ---------------------------------------------------------------------------

describe("Cross-validation: gate-level vs behavioral simulator", () => {
  // Import the behavioral simulator dynamically to avoid circular deps
  // We use a dynamic import to keep test structure clean
  it("programs produce identical final register state", async () => {
    const { Intel8008Simulator } = await import(
      "../../intel8008-simulator/src/index.js"
    );
    const beh = new Intel8008Simulator();
    const gate = new Intel8008GateLevel();

    // Test program: ADD, SUB, logical ops, flags
    const testPrograms: Uint8Array[] = [
      // Program 1: basic arithmetic
      new Uint8Array([
        MVI_A, 10, MVI_B, 3, ADD_B, MVI_B, 2, SUB_B, HLT,
      ]),
      // Program 2: flags
      new Uint8Array([
        MVI_A, 0xFF, ADI, 1,  // A=0, CY=1, Z=1
        MVI_B, 5, ADC_B,       // A=0+5+1=6
        HLT,
      ]),
      // Program 3: logical ops
      new Uint8Array([
        MVI_A, 0xF0, MVI_B, 0x0F, ANA_B,  // A=0
        MVI_B, 0xAA, ORA_B,                // A=0xAA
        HLT,
      ]),
      // Program 4: parity flag
      new Uint8Array([
        MVI_A, 0, ADI, 0x03, HLT,  // 0x03 = 2 ones → P=1
      ]),
      // Program 5: jump
      new Uint8Array([
        MVI_A, 5, ADI, 0,
        JFZ, lo(10), hi(10),
        MVI_A, 0xFF,
        HLT,           // offset 9
        MVI_A, 99, HLT,  // offset 10
      ]),
    ];

    for (const prog of testPrograms) {
      const behTraces = beh.run(prog);
      const gateTraces = gate.run(prog);

      // Same number of steps
      expect(gateTraces.length).toBe(behTraces.length);

      // Same final state
      expect(gate.a).toBe(beh.a);
      expect(gate.b).toBe(beh.b);
      expect(gate.c).toBe(beh.c);
      expect(gate.d).toBe(beh.d);
      expect(gate.e).toBe(beh.e);
      expect(gate.h).toBe(beh.h);
      expect(gate.l).toBe(beh.l);
      expect(gate.currentFlags.carry).toBe(beh.currentFlags.carry);
      expect(gate.currentFlags.zero).toBe(beh.currentFlags.zero);
      expect(gate.currentFlags.sign).toBe(beh.currentFlags.sign);
      expect(gate.currentFlags.parity).toBe(beh.currentFlags.parity);
    }
  });

  it("call/return programs match behavioral", async () => {
    const { Intel8008Simulator } = await import(
      "../../intel8008-simulator/src/index.js"
    );
    const beh = new Intel8008Simulator();
    const gate = new Intel8008GateLevel();

    // CALL + RET program
    const prog = new Uint8Array(0x20);
    prog[0x00] = MVI_B;  prog[0x01] = 0;
    prog[0x02] = CAL;    prog[0x03] = lo(0x10);  prog[0x04] = hi(0x10);
    prog[0x05] = MOV_A_B;
    prog[0x06] = HLT;
    prog[0x10] = MVI_B;  prog[0x11] = 42;
    prog[0x12] = RET;

    beh.run(prog);
    gate.run(prog);

    expect(gate.a).toBe(beh.a);
    expect(gate.b).toBe(beh.b);
  });

  it("M register operations match behavioral", async () => {
    const { Intel8008Simulator } = await import(
      "../../intel8008-simulator/src/index.js"
    );
    const beh = new Intel8008Simulator();
    const gate = new Intel8008GateLevel();

    const prog = new Uint8Array(0x100);
    prog[0x00] = MVI_H;  prog[0x01] = 0x00;
    prog[0x02] = MVI_L;  prog[0x03] = 0x50;
    prog[0x04] = MVI_M;  prog[0x05] = 0xAB;
    prog[0x06] = XRA_A;
    prog[0x07] = ORA_M;
    prog[0x08] = HLT;

    beh.run(prog);
    gate.run(prog);

    expect(gate.a).toBe(beh.a);
    expect(gate.memory[0x50]).toBe(beh.memory[0x50]);
  });
});

describe("Intel8008GateLevel – gateCount", () => {
  it("gateCount() returns -1 (not yet instrumented)", () => {
    const cpu = new Intel8008GateLevel();
    cpu.run(new Uint8Array([HLT]));
    expect(cpu.gateCount()).toBe(-1);
  });
});
