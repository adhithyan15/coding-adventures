import { describe, expect, it } from "vitest";
import {
  Circuit,
  capacitor,
  currentSource,
  inductor,
  resistor,
  tf,
  voltageSource,
} from "../src/index.js";

function expectClose(actual: number, expected: number): void {
  expect(actual).toBeCloseTo(expected, 9);
}

describe("tf", () => {
  it("exposes a gain alias on transfer-function results", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "vin", "0", 10.0));
    circuit.add(resistor("R1", "vin", "mid", 1_000.0));
    circuit.add(resistor("R2", "mid", "0", 1_000.0));

    const result = tf(circuit, "mid", "Vin");

    expectClose(result.gain(), 0.5);
  });

  it("reports gain and impedances for a voltage divider", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "vin", "0", 10.0));
    circuit.add(resistor("R1", "vin", "mid", 1_000.0));
    circuit.add(resistor("R2", "mid", "0", 1_000.0));

    const result = tf(circuit, "mid", "Vin");

    expectClose(result.transferRatio, 0.5);
    expectClose(result.inputImpedanceOhms, 2_000.0);
    expectClose(result.outputImpedanceOhms, 500.0);
  });

  it("matches Thevenin values for an unequal divider", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 5.0));
    circuit.add(resistor("Rtop", "in", "out", 1_000.0));
    circuit.add(resistor("Rbot", "out", "0", 3_000.0));

    const result = tf(circuit, "out", "Vin");

    expectClose(result.gain(), 0.75);
    expectClose(result.inputImpedanceOhms, 4_000.0);
    expectClose(result.outputImpedanceOhms, 750.0);
  });

  it("reports current-source transimpedance", () => {
    const circuit = new Circuit();
    circuit.add(currentSource("Iin", "0", "out", 1.0e-3));
    circuit.add(resistor("Rload", "out", "0", 2_000.0));

    const result = tf(circuit, "out", "Iin");

    expectClose(result.transferRatio, 2_000.0);
    expectClose(result.inputImpedanceOhms, 2_000.0);
    expectClose(result.outputImpedanceOhms, 2_000.0);
  });

  it("treats capacitors as open and inductors as shorts at DC small signal", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 1.0));
    circuit.add(capacitor("Cblock", "in", "blocked", 1.0e-6));
    circuit.add(resistor("Rblocked", "blocked", "0", 1_000.0));
    circuit.add(inductor("Lshort", "in", "out", 1.0e-3));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    const result = tf(circuit, "out", "Vin");

    expect(result.gain()).toBeCloseTo(1.0, 9);
    expect(result.outputImpedanceOhms).toBeLessThan(1.0e-6);
  });

  it("rejects missing output nodes", () => {
    const circuit = new Circuit();

    expect(() => tf(circuit, "missing", "Vin")).toThrowError(
      "output node was not found",
    );
  });

  it("rejects non-source input elements", () => {
    const circuit = new Circuit();
    circuit.add(resistor("Rin", "in", "0", 1_000.0));

    expect(() => tf(circuit, "in", "Rin")).toThrowError(
      "input element must be an independent voltage or current source",
    );
  });
});
