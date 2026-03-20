/**
 * Tests for MACArrayEngine -- scheduled MAC array execution (NPU style).
 */

import { describe, it, expect } from "vitest";
import {
  MACArrayEngine,
  makeMACArrayConfig,
  makeMACScheduleEntry,
  MACOperation,
  ActivationFunction,
  ExecutionModel,
} from "../src/index.js";

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

describe("MAC enums", () => {
  it("MACOperation values", () => {
    expect(MACOperation.LOAD_INPUT).toBe("load_input");
    expect(MACOperation.LOAD_WEIGHTS).toBe("load_weights");
    expect(MACOperation.MAC).toBe("mac");
    expect(MACOperation.REDUCE).toBe("reduce");
    expect(MACOperation.ACTIVATE).toBe("activate");
    expect(MACOperation.STORE_OUTPUT).toBe("store_output");
  });

  it("ActivationFunction values", () => {
    expect(ActivationFunction.NONE).toBe("none");
    expect(ActivationFunction.RELU).toBe("relu");
    expect(ActivationFunction.SIGMOID).toBe("sigmoid");
    expect(ActivationFunction.TANH).toBe("tanh");
  });
});

// ---------------------------------------------------------------------------
// MACScheduleEntry
// ---------------------------------------------------------------------------

describe("MACScheduleEntry", () => {
  it("creation", () => {
    const entry = makeMACScheduleEntry({
      cycle: 1,
      operation: MACOperation.MAC,
      inputIndices: [0, 1],
      weightIndices: [0, 1],
      outputIndex: 0,
    });
    expect(entry.cycle).toBe(1);
    expect(entry.operation).toBe(MACOperation.MAC);
    expect(entry.inputIndices).toEqual([0, 1]);
    expect(entry.outputIndex).toBe(0);
  });

  it("defaults", () => {
    const entry = makeMACScheduleEntry({
      cycle: 0,
      operation: MACOperation.MAC,
    });
    expect(entry.inputIndices).toEqual([]);
    expect(entry.weightIndices).toEqual([]);
    expect(entry.outputIndex).toBe(0);
    expect(entry.activation).toBe("none");
  });
});

// ---------------------------------------------------------------------------
// MACArrayConfig
// ---------------------------------------------------------------------------

describe("MACArrayConfig", () => {
  it("has sensible defaults", () => {
    const config = makeMACArrayConfig();
    expect(config.numMacs).toBe(8);
    expect(config.inputBufferSize).toBe(1024);
    expect(config.weightBufferSize).toBe(4096);
    expect(config.outputBufferSize).toBe(1024);
    expect(config.hasActivationUnit).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// MACArrayEngine -- basic properties
// ---------------------------------------------------------------------------

describe("MACArrayEngine properties", () => {
  it("has correct name", () => {
    const engine = new MACArrayEngine(makeMACArrayConfig({ numMacs: 4 }));
    expect(engine.name).toBe("MACArrayEngine");
  });

  it("has correct width", () => {
    const engine = new MACArrayEngine(makeMACArrayConfig({ numMacs: 8 }));
    expect(engine.width).toBe(8);
  });

  it("reports SCHEDULED_MAC execution model", () => {
    const engine = new MACArrayEngine(makeMACArrayConfig({ numMacs: 4 }));
    expect(engine.executionModel).toBe(ExecutionModel.SCHEDULED_MAC);
  });

  it("starts not halted", () => {
    const engine = new MACArrayEngine(makeMACArrayConfig({ numMacs: 4 }));
    expect(engine.halted).toBe(false);
  });

  it("exposes config", () => {
    const config = makeMACArrayConfig({ numMacs: 4 });
    const engine = new MACArrayEngine(config);
    expect(engine.config).toBe(config);
  });

  it("has readable toString", () => {
    const engine = new MACArrayEngine(makeMACArrayConfig({ numMacs: 4 }));
    const r = engine.toString();
    expect(r).toContain("MACArrayEngine");
    expect(r).toContain("num_macs=4");
  });
});

// ---------------------------------------------------------------------------
// MACArrayEngine -- data loading
// ---------------------------------------------------------------------------

describe("MACArrayEngine loading", () => {
  it("loadInputs", () => {
    const engine = new MACArrayEngine(makeMACArrayConfig({ numMacs: 4 }));
    engine.loadInputs([1.0, 2.0, 3.0, 4.0]);
    const outputs = engine.readOutputs();
    expect(outputs[0]).toBe(0.0);
  });

  it("loadWeights", () => {
    const engine = new MACArrayEngine(makeMACArrayConfig({ numMacs: 4 }));
    engine.loadWeights([0.5, 0.5, 0.5, 0.5]);
    // No error = success
  });
});

// ---------------------------------------------------------------------------
// MACArrayEngine -- execution
// ---------------------------------------------------------------------------

describe("MACArrayEngine execution", () => {
  it("dot product", () => {
    const engine = new MACArrayEngine(makeMACArrayConfig({ numMacs: 4 }));
    engine.loadInputs([1.0, 2.0, 3.0, 4.0]);
    engine.loadWeights([1.0, 1.0, 1.0, 1.0]);

    engine.loadSchedule([
      makeMACScheduleEntry({
        cycle: 1,
        operation: MACOperation.MAC,
        inputIndices: [0, 1, 2, 3],
        weightIndices: [0, 1, 2, 3],
        outputIndex: 0,
      }),
      makeMACScheduleEntry({
        cycle: 2,
        operation: MACOperation.REDUCE,
        outputIndex: 0,
      }),
      makeMACScheduleEntry({
        cycle: 3,
        operation: MACOperation.STORE_OUTPUT,
        outputIndex: 0,
      }),
    ]);
    engine.run();

    // 1*1 + 2*1 + 3*1 + 4*1 = 10.0
    expect(Math.abs(engine.readOutputs()[0] - 10.0)).toBeLessThan(0.01);
  });

  it("weighted sum", () => {
    const engine = new MACArrayEngine(makeMACArrayConfig({ numMacs: 4 }));
    engine.loadInputs([2.0, 3.0, 4.0, 5.0]);
    engine.loadWeights([0.5, 0.25, 0.125, 0.0625]);

    engine.loadSchedule([
      makeMACScheduleEntry({
        cycle: 1,
        operation: MACOperation.MAC,
        inputIndices: [0, 1, 2, 3],
        weightIndices: [0, 1, 2, 3],
        outputIndex: 0,
      }),
      makeMACScheduleEntry({
        cycle: 2,
        operation: MACOperation.REDUCE,
        outputIndex: 0,
      }),
      makeMACScheduleEntry({
        cycle: 3,
        operation: MACOperation.STORE_OUTPUT,
        outputIndex: 0,
      }),
    ]);
    engine.run();

    const expected =
      2.0 * 0.5 + 3.0 * 0.25 + 4.0 * 0.125 + 5.0 * 0.0625;
    expect(Math.abs(engine.readOutputs()[0] - expected)).toBeLessThan(0.01);
  });

  it("ReLU activation", () => {
    const engine = new MACArrayEngine(makeMACArrayConfig({ numMacs: 2 }));
    engine.loadInputs([3.0, -5.0]);
    engine.loadWeights([1.0, 1.0]);

    engine.loadSchedule([
      makeMACScheduleEntry({
        cycle: 1,
        operation: MACOperation.MAC,
        inputIndices: [0, 1],
        weightIndices: [0, 1],
        outputIndex: 0,
      }),
      makeMACScheduleEntry({
        cycle: 2,
        operation: MACOperation.REDUCE,
        outputIndex: 0,
      }),
      makeMACScheduleEntry({
        cycle: 3,
        operation: MACOperation.ACTIVATE,
        outputIndex: 0,
        activation: "relu",
      }),
      makeMACScheduleEntry({
        cycle: 4,
        operation: MACOperation.STORE_OUTPUT,
        outputIndex: 0,
      }),
    ]);
    engine.run();

    // 3*1 + (-5)*1 = -2 -> ReLU(-2) = 0
    expect(engine.readOutputs()[0]).toBe(0.0);
  });

  it("sigmoid activation", () => {
    const engine = new MACArrayEngine(makeMACArrayConfig({ numMacs: 1 }));
    engine.loadInputs([0.0]);
    engine.loadWeights([1.0]);

    engine.loadSchedule([
      makeMACScheduleEntry({
        cycle: 1,
        operation: MACOperation.MAC,
        inputIndices: [0],
        weightIndices: [0],
        outputIndex: 0,
      }),
      makeMACScheduleEntry({
        cycle: 2,
        operation: MACOperation.REDUCE,
        outputIndex: 0,
      }),
      makeMACScheduleEntry({
        cycle: 3,
        operation: MACOperation.ACTIVATE,
        outputIndex: 0,
        activation: "sigmoid",
      }),
    ]);
    engine.run();

    // sigmoid(0) = 0.5
    expect(Math.abs(engine.readOutputs()[0] - 0.5)).toBeLessThan(0.01);
  });

  it("tanh activation", () => {
    const engine = new MACArrayEngine(makeMACArrayConfig({ numMacs: 1 }));
    engine.loadInputs([1.0]);
    engine.loadWeights([1.0]);

    engine.loadSchedule([
      makeMACScheduleEntry({
        cycle: 1,
        operation: MACOperation.MAC,
        inputIndices: [0],
        weightIndices: [0],
        outputIndex: 0,
      }),
      makeMACScheduleEntry({
        cycle: 2,
        operation: MACOperation.REDUCE,
        outputIndex: 0,
      }),
      makeMACScheduleEntry({
        cycle: 3,
        operation: MACOperation.ACTIVATE,
        outputIndex: 0,
        activation: "tanh",
      }),
    ]);
    engine.run();

    expect(
      Math.abs(engine.readOutputs()[0] - Math.tanh(1.0)),
    ).toBeLessThan(0.01);
  });

  it("no activation unit skips ACTIVATE", () => {
    const engine = new MACArrayEngine(
      makeMACArrayConfig({ numMacs: 1, hasActivationUnit: false }),
    );
    engine.loadInputs([5.0]);
    engine.loadWeights([1.0]);

    engine.loadSchedule([
      makeMACScheduleEntry({
        cycle: 1,
        operation: MACOperation.MAC,
        inputIndices: [0],
        weightIndices: [0],
        outputIndex: 0,
      }),
      makeMACScheduleEntry({
        cycle: 2,
        operation: MACOperation.REDUCE,
        outputIndex: 0,
      }),
      makeMACScheduleEntry({
        cycle: 3,
        operation: MACOperation.ACTIVATE,
        outputIndex: 0,
        activation: "relu",
      }),
    ]);
    engine.run();

    // Activation skipped, value should remain 5.0
    expect(Math.abs(engine.readOutputs()[0] - 5.0)).toBeLessThan(0.01);
  });

  it("LOAD_INPUT operation in schedule", () => {
    const engine = new MACArrayEngine(makeMACArrayConfig({ numMacs: 4 }));
    engine.loadInputs([1.0, 2.0]);

    engine.loadSchedule([
      makeMACScheduleEntry({
        cycle: 1,
        operation: MACOperation.LOAD_INPUT,
        inputIndices: [0, 1],
      }),
    ]);
    const traces = engine.run();
    expect(traces.length).toBeGreaterThanOrEqual(1);
    expect(traces[0].description).toContain("LOAD_INPUT");
  });

  it("LOAD_WEIGHTS operation in schedule", () => {
    const engine = new MACArrayEngine(makeMACArrayConfig({ numMacs: 4 }));
    engine.loadWeights([1.0, 2.0]);

    engine.loadSchedule([
      makeMACScheduleEntry({
        cycle: 1,
        operation: MACOperation.LOAD_WEIGHTS,
        weightIndices: [0, 1],
      }),
    ]);
    const traces = engine.run();
    expect(traces[0].description).toContain("LOAD_WEIGHTS");
  });
});

// ---------------------------------------------------------------------------
// MACArrayEngine -- halting and idle cycles
// ---------------------------------------------------------------------------

describe("MACArrayEngine halting", () => {
  it("halts after schedule", () => {
    const engine = new MACArrayEngine(makeMACArrayConfig({ numMacs: 4 }));
    engine.loadSchedule([
      makeMACScheduleEntry({
        cycle: 1,
        operation: MACOperation.MAC,
        inputIndices: [0],
        weightIndices: [0],
      }),
    ]);
    engine.run();
    expect(engine.halted).toBe(true);
  });

  it("idle cycles produce idle traces", () => {
    const engine = new MACArrayEngine(makeMACArrayConfig({ numMacs: 4 }));
    engine.loadSchedule([
      makeMACScheduleEntry({
        cycle: 3,
        operation: MACOperation.MAC,
        inputIndices: [0],
        weightIndices: [0],
      }),
    ]);

    // Cycles 1 and 2 should be idle
    const trace1 = engine.step({ cycle: 1 });
    expect(trace1.description).toContain("No operation");
    expect(trace1.activeCount).toBe(0);
  });

  it("halted step returns schedule complete", () => {
    const engine = new MACArrayEngine(makeMACArrayConfig({ numMacs: 4 }));
    engine.loadSchedule([
      makeMACScheduleEntry({
        cycle: 1,
        operation: MACOperation.MAC,
        inputIndices: [0],
        weightIndices: [0],
      }),
    ]);
    engine.run();

    const trace = engine.step({ cycle: 99 });
    expect(trace.description.toLowerCase()).toContain("complete");
  });
});

// ---------------------------------------------------------------------------
// MACArrayEngine -- reset
// ---------------------------------------------------------------------------

describe("MACArrayEngine reset", () => {
  it("resets to initial state", () => {
    const engine = new MACArrayEngine(makeMACArrayConfig({ numMacs: 4 }));
    engine.loadInputs([1.0, 2.0]);
    engine.loadWeights([0.5, 0.5]);
    engine.loadSchedule([
      makeMACScheduleEntry({
        cycle: 1,
        operation: MACOperation.MAC,
        inputIndices: [0, 1],
        weightIndices: [0, 1],
      }),
      makeMACScheduleEntry({
        cycle: 2,
        operation: MACOperation.REDUCE,
        outputIndex: 0,
      }),
    ]);
    engine.run();

    engine.reset();
    expect(engine.halted).toBe(false);
    expect(engine.readOutputs()[0]).toBe(0.0);
  });
});
