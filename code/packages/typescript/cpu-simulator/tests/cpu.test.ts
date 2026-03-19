/**
 * Tests for the CPU with a mock instruction set.
 *
 * To test the CPU independently of any real ISA, we create a tiny mock
 * instruction set with just 3 instructions:
 *
 *     0x01 XX: LOAD_IMM -- load immediate value XX into register 0
 *     0x02 XX: ADD_IMM  -- add immediate value XX to register 0
 *     0x00 00: HALT     -- stop execution
 *
 * Each instruction is 4 bytes (padded with zeros for alignment).
 */

import { describe, expect, it } from "vitest";
import { CPU } from "../src/cpu.js";
import type { InstructionDecoder, InstructionExecutor } from "../src/cpu.js";
import { Memory } from "../src/memory.js";
import type { DecodeResult, ExecuteResult } from "../src/pipeline.js";
import { formatPipeline } from "../src/pipeline.js";
import { RegisterFile } from "../src/registers.js";

// --- Mock ISA ---

/**
 * Decodes our tiny 3-instruction mock ISA.
 */
class MockDecoder implements InstructionDecoder {
  decode(rawInstruction: number, _pc: number): DecodeResult {
    const opcode = rawInstruction & 0xff;
    const arg = (rawInstruction >>> 8) & 0xff;

    if (opcode === 0x00) {
      return { mnemonic: "HALT", fields: {}, rawInstruction };
    } else if (opcode === 0x01) {
      return {
        mnemonic: "LOAD_IMM",
        fields: { value: arg },
        rawInstruction,
      };
    } else if (opcode === 0x02) {
      return {
        mnemonic: "ADD_IMM",
        fields: { value: arg },
        rawInstruction,
      };
    } else {
      return {
        mnemonic: "UNKNOWN",
        fields: { opcode },
        rawInstruction,
      };
    }
  }
}

/**
 * Executes our tiny 3-instruction mock ISA.
 */
class MockExecutor implements InstructionExecutor {
  execute(
    decoded: DecodeResult,
    registers: RegisterFile,
    _memory: Memory,
    pc: number
  ): ExecuteResult {
    if (decoded.mnemonic === "HALT") {
      return {
        description: "Halt execution",
        registersChanged: {},
        memoryChanged: {},
        nextPc: pc,
        halted: true,
      };
    } else if (decoded.mnemonic === "LOAD_IMM") {
      const value = decoded.fields["value"];
      registers.write(0, value);
      return {
        description: `R0 = ${value}`,
        registersChanged: { R0: value },
        memoryChanged: {},
        nextPc: pc + 4,
        halted: false,
      };
    } else if (decoded.mnemonic === "ADD_IMM") {
      const value = decoded.fields["value"];
      const old = registers.read(0);
      const result = old + value;
      registers.write(0, result);
      return {
        description: `R0 = ${old} + ${value} = ${result}`,
        registersChanged: { R0: result },
        memoryChanged: {},
        nextPc: pc + 4,
        halted: false,
      };
    } else {
      return {
        description: "Unknown instruction",
        registersChanged: {},
        memoryChanged: {},
        nextPc: pc + 4,
        halted: false,
      };
    }
  }
}

// --- Helpers ---

/**
 * Encode a mock instruction as 4 little-endian bytes.
 */
function makeInstruction(opcode: number, arg: number = 0): number[] {
  const value = (opcode | (arg << 8)) >>> 0;
  return [
    value & 0xff,
    (value >>> 8) & 0xff,
    (value >>> 16) & 0xff,
    (value >>> 24) & 0xff,
  ];
}

/**
 * Create a CPU with our mock ISA.
 */
function makeCpu(): CPU {
  return new CPU(new MockDecoder(), new MockExecutor(), 4, 32);
}

// --- Tests ---

describe("CPU step", () => {
  it("single load sets R0", () => {
    // LOAD_IMM 5 should set R0 = 5.
    const cpu = makeCpu();
    const program = [...makeInstruction(0x01, 5), ...makeInstruction(0x00)];
    cpu.loadProgram(program);

    const trace = cpu.step();
    expect(trace.decode.mnemonic).toBe("LOAD_IMM");
    expect(trace.execute.registersChanged).toEqual({ R0: 5 });
    expect(cpu.registers.read(0)).toBe(5);
    expect(cpu.pc).toBe(4);
  });

  it("load and add", () => {
    // LOAD_IMM 1, ADD_IMM 2 should give R0 = 3.
    const cpu = makeCpu();
    const program = [
      ...makeInstruction(0x01, 1), // R0 = 1
      ...makeInstruction(0x02, 2), // R0 = 1 + 2 = 3
      ...makeInstruction(0x00), // HALT
    ];
    cpu.loadProgram(program);

    const trace1 = cpu.step();
    expect(trace1.decode.mnemonic).toBe("LOAD_IMM");
    expect(cpu.registers.read(0)).toBe(1);

    const trace2 = cpu.step();
    expect(trace2.decode.mnemonic).toBe("ADD_IMM");
    expect(trace2.execute.description).toBe("R0 = 1 + 2 = 3");
    expect(cpu.registers.read(0)).toBe(3);
  });

  it("halt stops execution", () => {
    const cpu = makeCpu();
    cpu.loadProgram(makeInstruction(0x00));
    const trace = cpu.step();
    expect(trace.execute.halted).toBe(true);
    expect(cpu.halted).toBe(true);
  });

  it("step after halt throws", () => {
    const cpu = makeCpu();
    cpu.loadProgram(makeInstruction(0x00));
    cpu.step(); // Execute HALT
    expect(() => cpu.step()).toThrow(/halted/);
  });
});

describe("CPU run", () => {
  it("run simple program", () => {
    // Run: LOAD 1, ADD 2, HALT -> R0 should be 3.
    const cpu = makeCpu();
    const program = [
      ...makeInstruction(0x01, 1),
      ...makeInstruction(0x02, 2),
      ...makeInstruction(0x00),
    ];
    cpu.loadProgram(program);

    const traces = cpu.run();
    expect(traces.length).toBe(3); // LOAD, ADD, HALT
    expect(cpu.registers.read(0)).toBe(3);
    expect(cpu.halted).toBe(true);
  });

  it("run with max_steps stops without halt", () => {
    // Run with maxSteps should stop even without HALT.
    const cpu = makeCpu();
    // Infinite loop: just LOAD_IMM 1 repeated (no HALT)
    const program: number[] = [];
    for (let i = 0; i < 100; i++) {
      program.push(...makeInstruction(0x01, 1));
    }
    cpu.loadProgram(program);

    const traces = cpu.run(5);
    expect(traces.length).toBe(5);
    expect(cpu.halted).toBe(false);
  });
});

describe("Pipeline trace", () => {
  it("trace has all stages", () => {
    const cpu = makeCpu();
    cpu.loadProgram([
      ...makeInstruction(0x01, 42),
      ...makeInstruction(0x00),
    ]);
    const trace = cpu.step();

    expect(trace.fetch.pc).toBe(0);
    expect(trace.fetch.rawInstruction).not.toBe(0);
    expect(trace.decode.mnemonic).toBe("LOAD_IMM");
    expect(trace.execute.description).toBe("R0 = 42");
    expect(trace.cycle).toBe(0);
  });

  it("format pipeline contains all stage names", () => {
    const cpu = makeCpu();
    cpu.loadProgram([
      ...makeInstruction(0x01, 1),
      ...makeInstruction(0x00),
    ]);
    const trace = cpu.step();
    const output = formatPipeline(trace);
    expect(output).toContain("FETCH");
    expect(output).toContain("DECODE");
    expect(output).toContain("EXECUTE");
    expect(output).toContain("Cycle 0");
  });

  it("register snapshot captures state after execution", () => {
    const cpu = makeCpu();
    cpu.loadProgram([
      ...makeInstruction(0x01, 7),
      ...makeInstruction(0x00),
    ]);
    const trace = cpu.step();
    expect(trace.registerSnapshot["R0"]).toBe(7);
  });
});

describe("CPU state", () => {
  it("initial state", () => {
    const cpu = makeCpu();
    const state = cpu.state;
    expect(state.pc).toBe(0);
    expect(state.halted).toBe(false);
    expect(state.cycle).toBe(0);
    expect(
      Object.values(state.registers).every((v) => v === 0)
    ).toBe(true);
  });

  it("state after execution", () => {
    const cpu = makeCpu();
    cpu.loadProgram([
      ...makeInstruction(0x01, 5),
      ...makeInstruction(0x00),
    ]);
    cpu.step();
    const state = cpu.state;
    expect(state.pc).toBe(4);
    expect(state.registers["R0"]).toBe(5);
    expect(state.cycle).toBe(1);
  });
});
