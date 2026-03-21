/**
 * Tests for the GPUCore -- the main processing element simulator.
 */

import { describe, it, expect } from "vitest";
import { FP16, FP32 } from "@coding-adventures/fp-arithmetic";
import { GPUCore } from "../src/core.js";
import { GenericISA } from "../src/generic-isa.js";
import { fadd, fmul, halt, jmp, limm, nop } from "../src/opcodes.js";
import { formatTrace } from "../src/trace.js";

describe("Construction", () => {
  it("default: GenericISA, FP32, 32 regs, 4KB memory", () => {
    const core = new GPUCore();
    expect(core.isa.name).toBe("Generic");
    expect(core.fmt).toEqual(FP32);
    expect(core.registers.numRegisters).toBe(32);
    expect(core.memory.size).toBe(4096);
    expect(core.pc).toBe(0);
    expect(core.halted).toBe(false);
  });

  it("custom ISA", () => {
    const isa = new GenericISA();
    const core = new GPUCore({ isa });
    expect(core.isa).toBe(isa);
  });

  it("custom register count", () => {
    const core = new GPUCore({ numRegisters: 255 });
    expect(core.registers.numRegisters).toBe(255);
  });

  it("custom format", () => {
    const core = new GPUCore({ fmt: FP16 });
    expect(core.fmt).toEqual(FP16);
  });

  it("custom memory size", () => {
    const core = new GPUCore({ memorySize: 1024 });
    expect(core.memory.size).toBe(1024);
  });

  it("toString shows ISA, register count, format, status", () => {
    const core = new GPUCore();
    const s = core.toString();
    expect(s).toContain("Generic");
    expect(s).toContain("running");
  });
});

describe("LoadProgram", () => {
  it("loading a program resets PC and cycle", () => {
    const core = new GPUCore();
    core.loadProgram([limm(0, 1.0), halt()]);
    expect(core.pc).toBe(0);
    expect(core.cycle).toBe(0);
    expect(core.halted).toBe(false);
  });

  it("loading replaces the old program", () => {
    const core = new GPUCore();
    core.loadProgram([limm(0, 1.0), halt()]);
    core.run();
    expect(core.halted).toBe(true);
    core.loadProgram([limm(0, 2.0), halt()]);
    expect(core.halted).toBe(false);
    expect(core.pc).toBe(0);
  });
});

describe("Step", () => {
  it("step through LIMM", () => {
    const core = new GPUCore();
    core.loadProgram([limm(0, 42.0), halt()]);
    const trace = core.step();
    expect(trace.pc).toBe(0);
    expect(trace.cycle).toBe(1);
    expect(core.registers.readFloat(0)).toBe(42.0);
    expect(core.pc).toBe(1);
  });

  it("step through FADD", () => {
    const core = new GPUCore();
    core.loadProgram([limm(0, 1.0), limm(1, 2.0), fadd(2, 0, 1), halt()]);
    core.step(); // limm R0, 1.0
    core.step(); // limm R1, 2.0
    const trace = core.step(); // fadd R2, R0, R1
    expect(core.registers.readFloat(2)).toBe(3.0);
    expect(trace.description).toContain("3");
  });

  it("step into HALT sets halted", () => {
    const core = new GPUCore();
    core.loadProgram([halt()]);
    const trace = core.step();
    expect(trace.halted).toBe(true);
    expect(core.halted).toBe(true);
  });

  it("stepping halted core throws", () => {
    const core = new GPUCore();
    core.loadProgram([halt()]);
    core.step();
    expect(() => core.step()).toThrow("halted");
  });

  it("stepping past program end throws", () => {
    const core = new GPUCore();
    core.loadProgram([nop()]);
    core.step(); // PC now 1
    expect(() => core.step()).toThrow("PC=1 out of program range");
  });

  it("each step increments cycle", () => {
    const core = new GPUCore();
    core.loadProgram([nop(), nop(), halt()]);
    core.step();
    expect(core.cycle).toBe(1);
    core.step();
    expect(core.cycle).toBe(2);
  });
});

describe("Run", () => {
  it("simple program", () => {
    const core = new GPUCore();
    core.loadProgram([limm(0, 3.0), limm(1, 4.0), fmul(2, 0, 1), halt()]);
    const traces = core.run();
    expect(traces.length).toBe(4);
    expect(core.registers.readFloat(2)).toBe(12.0);
    expect(core.halted).toBe(true);
  });

  it("infinite loop hits max_steps", () => {
    const core = new GPUCore();
    core.loadProgram([jmp(0)]);
    expect(() => core.run(100)).toThrow("Execution limit");
  });

  it("empty program throws", () => {
    const core = new GPUCore();
    core.loadProgram([]);
    expect(() => core.run()).toThrow("out of program range");
  });
});

describe("Reset", () => {
  it("clears registers", () => {
    const core = new GPUCore();
    core.loadProgram([limm(0, 42.0), halt()]);
    core.run();
    core.reset();
    expect(core.registers.readFloat(0)).toBe(0.0);
  });

  it("clears PC", () => {
    const core = new GPUCore();
    core.loadProgram([nop(), halt()]);
    core.run();
    core.reset();
    expect(core.pc).toBe(0);
  });

  it("clears halted", () => {
    const core = new GPUCore();
    core.loadProgram([halt()]);
    core.run();
    expect(core.halted).toBe(true);
    core.reset();
    expect(core.halted).toBe(false);
  });

  it("preserves program", () => {
    const core = new GPUCore();
    core.loadProgram([limm(0, 99.0), halt()]);
    core.run();
    core.reset();
    core.run();
    expect(core.registers.readFloat(0)).toBe(99.0);
  });

  it("clears memory", () => {
    const core = new GPUCore();
    core.memory.storePythonFloat(0, 42.0);
    core.reset();
    expect(core.memory.loadFloatAsPython(0)).toBe(0.0);
  });

  it("clears cycle", () => {
    const core = new GPUCore();
    core.loadProgram([nop(), halt()]);
    core.run();
    expect(core.cycle).toBeGreaterThan(0);
    core.reset();
    expect(core.cycle).toBe(0);
  });
});

describe("Traces", () => {
  it("trace has all expected fields", () => {
    const core = new GPUCore();
    core.loadProgram([limm(0, 1.0), halt()]);
    const trace = core.step();
    expect(trace.cycle).toBe(1);
    expect(trace.pc).toBe(0);
    expect(trace.nextPc).toBe(1);
    expect(trace.halted).toBe(false);
    expect(trace.description).not.toBe("");
  });

  it("formatTrace returns readable string", () => {
    const core = new GPUCore();
    core.loadProgram([limm(0, 1.0), halt()]);
    const trace = core.step();
    const formatted = formatTrace(trace);
    expect(formatted).toContain("[Cycle 1]");
    expect(formatted).toContain("PC=0");
  });

  it("halt trace shows HALTED", () => {
    const core = new GPUCore();
    core.loadProgram([halt()]);
    const trace = core.step();
    expect(formatTrace(trace)).toContain("HALTED");
  });

  it("trace records registers changed", () => {
    const core = new GPUCore();
    core.loadProgram([limm(5, 3.14), halt()]);
    const trace = core.step();
    expect("R5" in trace.registersChanged).toBe(true);
  });
});
