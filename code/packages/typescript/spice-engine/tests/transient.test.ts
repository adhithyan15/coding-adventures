import { describe, expect, it } from "vitest";
import {
  Circuit,
  SpiceError,
  capacitor,
  capacitorWithInitialVoltage,
  inductor,
  inductorWithInitialCurrent,
  resistor,
  transient,
  voltageSource,
} from "../src/index.js";

function expectClose(actual: number | undefined, expected: number): void {
  expect(actual).not.toBeUndefined();
  expect(actual!).toBeCloseTo(expected, 9);
}

describe("transient", () => {
  it("uses a backward-Euler capacitor companion for an RC step", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "vin", "0", 1.0));
    circuit.add(resistor("R1", "vin", "out", 1_000.0));
    circuit.add(capacitor("C1", "out", "0", 1.0e-6));

    const points = transient(circuit, 1.0e-3, 3.0e-3);

    expect(points).toHaveLength(3);
    expect(points[0].time).toBeCloseTo(1.0e-3, 12);
    expectClose(points[0].voltage("out"), 0.5);
    expectClose(points[1].voltage("out"), 0.75);
    expectClose(points[2].voltage("out"), 0.875);
  });

  it("respects capacitor initial voltage", () => {
    const circuit = new Circuit();
    circuit.add(resistor("R1", "out", "0", 1_000.0));
    circuit.add(capacitorWithInitialVoltage("C1", "out", "0", 1.0e-6, 1.0));

    const points = transient(circuit, 1.0e-3, 2.0e-3);

    expectClose(points[0].voltage("out"), 0.5);
    expectClose(points[1].voltage("out"), 0.25);
  });

  it("uses a backward-Euler inductor companion for an RL step", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "vin", "0", 1.0));
    circuit.add(resistor("R1", "vin", "out", 1_000.0));
    circuit.add(inductor("L1", "out", "0", 1.0));

    const points = transient(circuit, 1.0e-3, 3.0e-3);

    expect(points).toHaveLength(3);
    expectClose(points[0].voltage("out"), 0.5);
    expectClose(points[0].branchCurrent("L1"), 0.5e-3);
    expectClose(points[1].voltage("out"), 0.25);
    expectClose(points[1].branchCurrent("L1"), 0.75e-3);
    expectClose(points[2].voltage("out"), 0.125);
    expectClose(points[2].branchCurrent("L1"), 0.875e-3);
  });

  it("respects inductor initial current", () => {
    const circuit = new Circuit();
    circuit.add(resistor("R1", "out", "0", 1_000.0));
    circuit.add(inductorWithInitialCurrent("L1", "out", "0", 1.0, 1.0e-3));

    const points = transient(circuit, 1.0e-3, 2.0e-3);

    expectClose(points[0].voltage("out"), -0.5);
    expectClose(points[0].branchCurrent("L1"), 0.5e-3);
    expectClose(points[1].voltage("out"), -0.25);
    expectClose(points[1].branchCurrent("L1"), 0.25e-3);
  });

  it("rejects non-positive capacitance", () => {
    const circuit = new Circuit();
    circuit.add(capacitor("Cbad", "out", "0", 0.0));

    expect(() => transient(circuit, 1.0e-3, 1.0e-3)).toThrowError(SpiceError);
    expect(() => transient(circuit, 1.0e-3, 1.0e-3)).toThrowError(
      "invalid element Cbad",
    );
  });

  it("rejects non-positive inductance", () => {
    const circuit = new Circuit();
    circuit.add(inductor("Lbad", "out", "0", 0.0));

    expect(() => transient(circuit, 1.0e-3, 1.0e-3)).toThrowError(SpiceError);
    expect(() => transient(circuit, 1.0e-3, 1.0e-3)).toThrowError(
      "invalid element Lbad",
    );
  });

  it("rejects non-positive time step", () => {
    const circuit = new Circuit();

    expect(() => transient(circuit, 0.0, 1.0e-3)).toThrowError(SpiceError);
    expect(() => transient(circuit, 0.0, 1.0e-3)).toThrowError(
      "invalid element transient",
    );
  });
});
