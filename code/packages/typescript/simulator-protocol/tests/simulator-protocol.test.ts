import { describe, expect, it } from "vitest";

import {
  ExecutionResult,
  type Simulator,
  StepTrace,
} from "../src/simulator-protocol.js";

interface MockState {
  readonly accumulator: number;
  readonly pc: number;
  readonly halted: boolean;
}

class MockSimulator implements Simulator<MockState, StepTrace> {
  private accumulator = 0;
  private pc = 0;
  private halted = false;
  private program = new Uint8Array(0);

  load(program: Uint8Array): void {
    this.program = program;
    this.accumulator = 0;
    this.pc = 0;
    this.halted = false;
  }

  step(): StepTrace {
    if (this.halted) {
      throw new Error("simulator halted");
    }

    const pcBefore = this.pc;
    const opcode = this.program[this.pc] ?? 0x01;
    this.pc += 1;

    if (opcode === 0x01) {
      this.halted = true;
      return new StepTrace(pcBefore, this.pc, "HLT", "halt");
    }

    this.accumulator = opcode & 0x0f;
    return new StepTrace(
      pcBefore,
      this.pc,
      `LDM ${this.accumulator}`,
      `load ${this.accumulator}`
    );
  }

  execute(program: Uint8Array, maxSteps: number = 100_000): ExecutionResult<MockState> {
    this.reset();
    this.load(program);
    const traces: StepTrace[] = [];
    let steps = 0;

    while (!this.halted && steps < maxSteps) {
      traces.push(this.step());
      steps += 1;
    }

    return new ExecutionResult({
      halted: this.halted,
      steps,
      finalState: this.getState(),
      error: this.halted ? null : `max_steps (${maxSteps}) exceeded`,
      traces,
    });
  }

  getState(): MockState {
    return Object.freeze({
      accumulator: this.accumulator,
      pc: this.pc,
      halted: this.halted,
    });
  }

  reset(): void {
    this.accumulator = 0;
    this.pc = 0;
    this.halted = false;
    this.program = new Uint8Array(0);
  }
}

describe("StepTrace", () => {
  it("captures the normalized execution surface", () => {
    const trace = new StepTrace(0, 1, "NOP", "do nothing");
    expect(trace.pcBefore).toBe(0);
    expect(trace.pcAfter).toBe(1);
    expect(trace.mnemonic).toBe("NOP");
    expect(trace.description).toBe("do nothing");
    expect(Object.isFrozen(trace)).toBe(true);
  });
});

describe("ExecutionResult", () => {
  it("reports ok only for clean halts", () => {
    const success = new ExecutionResult({
      halted: true,
      steps: 1,
      finalState: Object.freeze({ value: 1 }),
      error: null,
      traces: [new StepTrace(0, 1, "HLT", "halt")],
    });
    const failure = new ExecutionResult({
      halted: false,
      steps: 2,
      finalState: Object.freeze({ value: 0 }),
      error: "max_steps (2) exceeded",
      traces: [],
    });

    expect(success.ok).toBe(true);
    expect(failure.ok).toBe(false);
    expect(Object.isFrozen(success.traces)).toBe(true);
  });
});

describe("Simulator", () => {
  it("supports end-to-end execution through the shared interface", () => {
    const sim: Simulator<MockState, StepTrace> = new MockSimulator();
    const result = sim.execute(new Uint8Array([0xd7, 0x01]));

    expect(result.ok).toBe(true);
    expect(result.steps).toBe(2);
    expect(result.finalState.accumulator).toBe(7);
    expect(result.traces.map((trace) => trace.mnemonic)).toEqual(["LDM 7", "HLT"]);
  });

  it("returns max-step failures in the shared result type", () => {
    const sim = new MockSimulator();
    const result = sim.execute(new Uint8Array([0xd1, 0xd1, 0xd1]), 2);

    expect(result.ok).toBe(false);
    expect(result.error).toMatch(/max_steps/);
    expect(result.steps).toBe(2);
  });
});
