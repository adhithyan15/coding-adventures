/**
 * Tests for the ARM simulator.
 */

import { describe, it, expect } from "vitest";
import { formatPipeline } from "@coding-adventures/cpu-simulator";
import {
  ARMDecoder,
  ARMSimulator,
  assemble,
  encodeAdd,
  encodeHlt,
  encodeMovImm,
  encodeSub,
  COND_AL,
  OPCODE_MOV,
  OPCODE_SUB,
} from "../src/simulator.js";

// ---------------------------------------------------------------------------
// Encoding tests
// ---------------------------------------------------------------------------

describe("TestEncoding", () => {
  it("MOV R0, #1 should encode to 0xE3A00001", () => {
    /**
     * Breakdown: cond=1110 00 I=1 opcode=1101 S=0 Rn=0000 Rd=0000 imm=00000001
     */
    expect(encodeMovImm(0, 1)).toBe(0xe3a00001);
  });

  it("MOV R1, #2 should encode to 0xE3A01002", () => {
    expect(encodeMovImm(1, 2)).toBe(0xe3a01002);
  });

  it("ADD R2, R0, R1 should encode to 0xE0802001", () => {
    /**
     * Breakdown: cond=1110 00 I=0 opcode=0100 S=0 Rn=0000 Rd=0010 Rm=0001
     */
    expect(encodeAdd(2, 0, 1)).toBe(0xe0802001);
  });

  it("SUB R2, R0, R1 should encode to 0xE0402001", () => {
    /**
     * Breakdown: cond=1110 00 I=0 opcode=0010 S=0 Rn=0000 Rd=0010 Rm=0001
     */
    expect(encodeSub(2, 0, 1)).toBe(0xe0402001);
  });

  it("HLT should encode to 0xFFFFFFFF", () => {
    expect(encodeHlt()).toBe(0xffffffff);
  });
});

// ---------------------------------------------------------------------------
// Decoder tests
// ---------------------------------------------------------------------------

describe("TestDecoder", () => {
  it("MOV R0, #1 should decode with rd=0 and imm=1", () => {
    const decoder = new ARMDecoder();
    const result = decoder.decode(0xe3a00001, 0);
    expect(result.mnemonic).toBe("mov");
    expect(result.fields["rd"]).toBe(0);
    expect(result.fields["imm"]).toBe(1);
    expect(result.fields["i_bit"]).toBe(1);
    expect(result.fields["opcode"]).toBe(0b1101);
  });

  it("ADD R2, R0, R1 should decode with rd=2, rn=0, rm=1", () => {
    const decoder = new ARMDecoder();
    const result = decoder.decode(0xe0802001, 0);
    expect(result.mnemonic).toBe("add");
    expect(result.fields["rd"]).toBe(2);
    expect(result.fields["rn"]).toBe(0);
    expect(result.fields["rm"]).toBe(1);
    expect(result.fields["i_bit"]).toBe(0);
  });

  it("SUB R2, R0, R1 should decode with rd=2, rn=0, rm=1", () => {
    const decoder = new ARMDecoder();
    const result = decoder.decode(0xe0402001, 0);
    expect(result.mnemonic).toBe("sub");
    expect(result.fields["rd"]).toBe(2);
    expect(result.fields["rn"]).toBe(0);
    expect(result.fields["rm"]).toBe(1);
  });

  it("HLT (0xFFFFFFFF) should decode to mnemonic 'hlt'", () => {
    const decoder = new ARMDecoder();
    const result = decoder.decode(0xffffffff, 0);
    expect(result.mnemonic).toBe("hlt");
  });

  it("all normal instructions should have condition code AL (0b1110)", () => {
    const decoder = new ARMDecoder();
    const result = decoder.decode(encodeMovImm(0, 42), 0);
    expect(result.fields["cond"]).toBe(0b1110);
  });
});

// ---------------------------------------------------------------------------
// End-to-end simulator tests
// ---------------------------------------------------------------------------

describe("TestARMSimulator", () => {
  it("x = 1 + 2 -> R2 should be 3", () => {
    /**
     * Program:
     *     MOV R0, #1       ; R0 = 1
     *     MOV R1, #2       ; R1 = 2
     *     ADD R2, R0, R1   ; R2 = R0 + R1 = 3
     *     HLT              ; halt
     */
    const sim = new ARMSimulator();
    const program = assemble([
      encodeMovImm(0, 1),
      encodeMovImm(1, 2),
      encodeAdd(2, 0, 1),
      encodeHlt(),
    ]);
    const traces = sim.run(program);

    expect(traces.length).toBe(4);
    expect(sim.cpu.registers.read(0)).toBe(1);
    expect(sim.cpu.registers.read(1)).toBe(2);
    expect(sim.cpu.registers.read(2)).toBe(3);
    expect(sim.cpu.halted).toBe(true);
  });

  it("SUB R2, R0, R1 with R0=10, R1=3 should give R2=7", () => {
    const sim = new ARMSimulator();
    const program = assemble([
      encodeMovImm(0, 10),
      encodeMovImm(1, 3),
      encodeSub(2, 0, 1),
      encodeHlt(),
    ]);
    sim.run(program);
    expect(sim.cpu.registers.read(2)).toBe(7);
  });

  it("ADD 100 + 200 = 300", () => {
    const sim = new ARMSimulator();
    const program = assemble([
      encodeMovImm(0, 100),
      encodeMovImm(1, 200),
      encodeAdd(2, 0, 1),
      encodeHlt(),
    ]);
    sim.run(program);
    expect(sim.cpu.registers.read(2)).toBe(300);
  });

  it("each step should produce a visible pipeline trace", () => {
    const sim = new ARMSimulator();
    const program = assemble([encodeMovImm(0, 7), encodeHlt()]);
    sim.cpu.loadProgram(program);
    const trace = sim.step();

    expect(trace.fetch.pc).toBe(0);
    expect(trace.decode.mnemonic).toBe("mov");
    expect(trace.decode.fields["imm"]).toBe(7);
    expect(trace.execute.registersChanged).toHaveProperty("R0");
    expect(trace.execute.registersChanged["R0"]).toBe(7);
  });

  it("the pipeline format should show all three stages", () => {
    const sim = new ARMSimulator();
    const program = assemble([encodeMovImm(0, 1), encodeHlt()]);
    sim.cpu.loadProgram(program);
    const trace = sim.step();
    const output = formatPipeline(trace);
    expect(output).toContain("FETCH");
    expect(output).toContain("DECODE");
    expect(output).toContain("EXECUTE");
    expect(output).toContain("mov");
  });

  it("ARM should have 16 registers available", () => {
    const sim = new ARMSimulator();
    expect(sim.cpu.registers.numRegisters).toBe(16);
  });
});

// ---------------------------------------------------------------------------
// Go test ports -- additional coverage from the Go test suite
// ---------------------------------------------------------------------------

describe("Go test ports", () => {
  it("full program with SUB: R0=1, R1=2, R2=R0+R1=3, R3=R2-R0=2", () => {
    const sim = new ARMSimulator();
    const program = assemble([
      encodeMovImm(0, 1),
      encodeMovImm(1, 2),
      encodeAdd(2, 0, 1),
      encodeSub(3, 2, 0),
      encodeHlt(),
    ]);

    const traces = sim.run(program);
    expect(traces.length).toBe(5);
    expect(sim.cpu.registers.read(0)).toBe(1);
    expect(sim.cpu.registers.read(1)).toBe(2);
    expect(sim.cpu.registers.read(2)).toBe(3);
    expect(sim.cpu.registers.read(3)).toBe(2);
  });

  it("rotate decode: imm=1 rotate=1 should produce 0x40000000", () => {
    /**
     * Create an instruction with a rotate to test the rotate decode logic.
     * imm = 1, rotate = 1 -> shifted right by 2 positions = 0x40000000
     */
    const sim = new ARMSimulator();
    const cond = COND_AL;
    const raw =
      (((cond << 28) |
        (1 << 25) |
        (OPCODE_MOV << 21) |
        (1 << 12) |
        (1 << 8) |
        1) >>>
        0);
    const program = assemble([raw, encodeHlt()]);

    sim.run(program);
    const val = sim.cpu.registers.read(1);
    expect(val).toBe(0x40000000);
  });

  it("unknown opcode should not produce blank mnemonic", () => {
    const sim = new ARMSimulator();
    const program = assemble([
      ((COND_AL << 28) | (0xf << 21)) >>> 0,
      encodeHlt(),
    ]);
    const traces = sim.run(program);
    expect(traces[0].decode.mnemonic).not.toBe("");
  });
});
