import { describe, expect, it } from "vitest";
import {
  Circuit,
  SpiceError,
  capacitor,
  currentSource,
  noiseAc,
  resistor,
  voltageSource,
} from "../src/index.js";

const BOLTZMANN = 1.380_649e-23;

describe("noiseAc", () => {
  it("computes Johnson noise for a single grounded resistor", () => {
    const circuit = new Circuit();
    circuit.add(currentSource("Iin", "0", "out", 0.0));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    const result = noiseAc(circuit, "out", "Iin", [1_000.0], 300.0);
    const expected = 4.0 * BOLTZMANN * 300.0 * 1_000.0;

    expect(result.outputNode).toBe("out");
    expect(result.inputSource).toBe("Iin");
    expect(result.temperatureKelvin).toBe(300.0);
    expect(result.points).toHaveLength(1);
    expect(result.points[0].frequencyHz).toBe(1_000.0);
    expect(result.points[0].entries).toHaveLength(1);
    expect(result.points[0].entries[0]).toMatchObject({
      elementName: "Rload",
      noiseType: "thermal",
    });
    expect(result.points[0].entries[0].sourcePsd).toBeCloseTo(
      4.0 * BOLTZMANN * 300.0 / 1_000.0,
      30,
    );
    expect(result.points[0].outputPsd).toBeCloseTo(expected, 30);
    expect(result.points[0].inputReferredPsd).toBeCloseTo(
      4.0 * BOLTZMANN * 300.0 / 1_000.0,
      30,
    );
  });

  it("sorts resistor contributions by output noise", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 1.0));
    circuit.add(resistor("Rsource", "in", "out", 1_000.0));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    const point = noiseAc(circuit, "out", "Vin", [1_000.0]).points[0];
    const names = point.entries.map((entry) => entry.elementName);

    expect(names).toEqual(["Rload", "Rsource"]);
    expect(point.entries[0].outputPsd).toBeCloseTo(
      point.entries[1].outputPsd,
      30,
    );
    expect(point.outputPsd).toBeGreaterThan(0.0);
    expect(point.outputPsd).toBeCloseTo(
      point.entries[0].outputPsd + point.entries[1].outputPsd,
      30,
    );
    expect(point.inputReferredPsd).toBeGreaterThan(point.outputPsd);
  });

  it("follows RC low-pass transfer with frequency", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 1.0));
    circuit.add(resistor("R1", "in", "out", 1_000.0));
    circuit.add(capacitor("C1", "out", "0", 1.0e-6));

    const [low, corner, high] = noiseAc(
      circuit,
      "out",
      "Vin",
      [1.0, 1.0 / (2.0 * Math.PI * 1_000.0 * 1.0e-6), 1.0e6],
    ).points;

    expect(low.outputPsd).toBeGreaterThan(corner.outputPsd);
    expect(corner.outputPsd).toBeGreaterThan(high.outputPsd);
    expect(corner.outputPsd).toBeCloseTo(low.outputPsd / 2.0, 1);
    expect(high.outputPsd).toBeLessThan(low.outputPsd * 1.0e-4);
  });

  it("uses default logarithmic frequencies", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 1.0));
    circuit.add(resistor("Rload", "in", "0", 1_000.0));

    const result = noiseAc(circuit, "in", "Vin");

    expect(result.points).toHaveLength(50);
    expect(result.points[0].frequencyHz).toBeCloseTo(1.0, 12);
    expect(result.points[49].frequencyHz).toBeCloseTo(1.0e6, 6);
  });

  it("reports zero output noise at ground while keeping source PSDs", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 1.0));
    circuit.add(resistor("Rload", "in", "0", 1_000.0));

    const point = noiseAc(circuit, "0", "Vin", [1_000.0]).points[0];

    expect(point.outputPsd).toBe(0.0);
    expect(point.inputReferredPsd).toBe(0.0);
    expect(point.entries).toHaveLength(1);
    expect(point.entries[0].sourcePsd).toBeGreaterThan(0.0);
    expect(point.entries[0].outputPsd).toBe(0.0);
  });

  it("rejects invalid inputs", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 1.0));
    circuit.add(resistor("Rload", "in", "0", 1_000.0));

    expect(() => noiseAc(circuit, "missing", "Vin", [1.0])).toThrowError(
      SpiceError,
    );
    expect(() => noiseAc(circuit, "in", "Rload", [1.0])).toThrowError(
      "input element must be an independent voltage or current source",
    );
    expect(() => noiseAc(circuit, "in", "Vin", [0.0])).toThrowError(
      "frequencies must be finite and positive",
    );
    expect(() => noiseAc(circuit, "in", "Vin", [1.0], 0.0)).toThrowError(
      "temperature must be finite and positive",
    );
  });
});
