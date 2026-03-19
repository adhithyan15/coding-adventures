/**
 * Tests for the RISC-V RV32I simulator.
 */

import { describe, it, expect } from "vitest";
import { formatPipeline } from "@coding-adventures/cpu-simulator";
import {
  RiscVDecoder,
  RiscVSimulator,
  assemble,
  encodeAdd,
  encodeAddi,
  encodeEcall,
} from "../src/simulator.js";

describe("TestEncoding", () => {
  /** Verify instruction encoding matches known RISC-V binary values. */

  it("encode addi x1, x0, 1 should encode to 0x00100093", () => {
    /** addi x1, x0, 1 should encode to 0x00100093. */
    expect(encodeAddi(1, 0, 1)).toBe(0x00100093);
  });

  it("encode addi x2, x0, 2 should encode to 0x00200113", () => {
    /** addi x2, x0, 2 should encode to 0x00200113. */
    expect(encodeAddi(2, 0, 2)).toBe(0x00200113);
  });

  it("encode add x3, x1, x2 should encode to 0x002081B3", () => {
    /** add x3, x1, x2 should encode to 0x002081B3. */
    expect(encodeAdd(3, 1, 2)).toBe(0x002081b3);
  });

  it("encode ecall should encode to 0x00000073", () => {
    /** ecall should encode to 0x00000073. */
    expect(encodeEcall()).toBe(0x00000073);
  });
});

describe("TestDecoder", () => {
  /** Verify the decoder correctly extracts fields from binary instructions. */

  it("decode addi", () => {
    const decoder = new RiscVDecoder();
    const result = decoder.decode(0x00100093, 0);
    expect(result.mnemonic).toBe("addi");
    expect(result.fields["rd"]).toBe(1);
    expect(result.fields["rs1"]).toBe(0);
    expect(result.fields["imm"]).toBe(1);
  });

  it("decode add", () => {
    const decoder = new RiscVDecoder();
    const result = decoder.decode(0x002081b3, 0);
    expect(result.mnemonic).toBe("add");
    expect(result.fields["rd"]).toBe(3);
    expect(result.fields["rs1"]).toBe(1);
    expect(result.fields["rs2"]).toBe(2);
  });

  it("decode ecall", () => {
    const decoder = new RiscVDecoder();
    const result = decoder.decode(0x00000073, 0);
    expect(result.mnemonic).toBe("ecall");
  });

  it("decode negative immediate", () => {
    /** addi x1, x0, -1 should have imm = -1 (sign-extended). */
    const decoder = new RiscVDecoder();
    const instr = encodeAddi(1, 0, -1);
    const result = decoder.decode(instr, 0);
    expect(result.fields["imm"]).toBe(-1);
  });
});

describe("TestRiscVSimulator", () => {
  /** End-to-end tests running actual RISC-V programs. */

  it("x equals 1 plus 2", () => {
    /**
     * The target program: x = 1 + 2 -> x3 should be 3.
     *
     * Program:
     *     addi x1, x0, 1    # x1 = 1
     *     addi x2, x0, 2    # x2 = 2
     *     add  x3, x1, x2   # x3 = x1 + x2 = 3
     *     ecall              # halt
     */
    const sim = new RiscVSimulator();
    const program = assemble([
      encodeAddi(1, 0, 1),
      encodeAddi(2, 0, 2),
      encodeAdd(3, 1, 2),
      encodeEcall(),
    ]);
    const traces = sim.run(program);

    expect(traces.length).toBe(4);
    expect(sim.cpu.registers.read(1)).toBe(1);
    expect(sim.cpu.registers.read(2)).toBe(2);
    expect(sim.cpu.registers.read(3)).toBe(3);
    expect(sim.cpu.halted).toBe(true);
  });

  it("x0 stays zero", () => {
    /** Writing to x0 should be ignored -- x0 is always 0. */
    const sim = new RiscVSimulator();
    const program = assemble([
      encodeAddi(0, 0, 42), // Try to write 42 to x0
      encodeEcall(),
    ]);
    sim.run(program);
    expect(sim.cpu.registers.read(0)).toBe(0);
  });

  it("pipeline trace visible", () => {
    /** Each step should produce a visible pipeline trace. */
    const sim = new RiscVSimulator();
    const program = assemble([encodeAddi(1, 0, 7), encodeEcall()]);
    sim.cpu.loadProgram(program);
    const trace = sim.step();

    expect(trace.fetch.pc).toBe(0);
    expect(trace.decode.mnemonic).toBe("addi");
    expect(trace.decode.fields["imm"]).toBe(7);
    expect(trace.execute.registersChanged).toHaveProperty("x1");
    expect(trace.execute.registersChanged["x1"]).toBe(7);
  });

  it("pipeline format", () => {
    /** The pipeline format should show all three stages. */
    const sim = new RiscVSimulator();
    const program = assemble([encodeAddi(1, 0, 1), encodeEcall()]);
    sim.cpu.loadProgram(program);
    const trace = sim.step();
    const output = formatPipeline(trace);
    expect(output).toContain("FETCH");
    expect(output).toContain("DECODE");
    expect(output).toContain("EXECUTE");
    expect(output).toContain("addi");
  });

  it("add large numbers", () => {
    /** Add 100 + 200 = 300. */
    const sim = new RiscVSimulator();
    const program = assemble([
      encodeAddi(1, 0, 100),
      encodeAddi(2, 0, 200),
      encodeAdd(3, 1, 2),
      encodeEcall(),
    ]);
    sim.run(program);
    expect(sim.cpu.registers.read(3)).toBe(300);
  });

  it("negative immediate", () => {
    /** addi x1, x0, -5 should set x1 to -5 (as unsigned 0xFFFFFFFB). */
    const sim = new RiscVSimulator();
    const program = assemble([encodeAddi(1, 0, -5), encodeEcall()]);
    sim.run(program);
    // In 32-bit unsigned, -5 = 0xFFFFFFFB = 4294967291
    expect(sim.cpu.registers.read(1)).toBe(0xfffffffb);
  });
});
