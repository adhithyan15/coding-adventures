import { describe, expect, it } from "vitest";
import {
  Circuit,
  SpiceError,
  currentSource,
  mcDc,
  resistor,
  vccs,
  voltageSource,
} from "../src/index.js";

function divider(): Circuit {
  const circuit = new Circuit();
  circuit.add(voltageSource("Vin", "in", "0", 10.0));
  circuit.add(resistor("Rtop", "in", "mid", 1_000.0));
  circuit.add(resistor("Rbot", "mid", "0", 1_000.0));
  return circuit;
}

function voltages(result: ReturnType<typeof mcDc>): number[] {
  return result.points.map((point) => point.voltage("mid") ?? Number.NaN);
}

describe("mcDc", () => {
  it("returns trial-indexed operating points and zero spread at zero tolerance", () => {
    const result = mcDc(divider(), "mid", 8, { seed: 7, tolerance: 0.0 });

    expect(result.outputNode).toBe("mid");
    expect(result.nTrials).toBe(8);
    expect(result.points).toHaveLength(8);
    expect(result.points.map((point) => point.trial)).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7,
    ]);
    expect(result.points.every((point) => point.converged)).toBe(true);
    expect(result.mean).toBeCloseTo(5.0, 12);
    expect(result.stdDev).toBe(0.0);
    for (const point of result.points) {
      expect(point.voltage("mid")).toBeCloseTo(5.0, 12);
      expect(point.voltage("0")).toBe(0.0);
      expect(point.branchCurrent("Vin")).toBeDefined();
    }
  });

  it("is reproducible with the same seed", () => {
    const left = mcDc(divider(), "mid", 20, {
      seed: 42,
      tolerance: 0.05,
      distribution: "uniform",
    });
    const right = mcDc(divider(), "mid", 20, {
      seed: 42,
      tolerance: 0.05,
      distribution: "uniform",
    });

    expect(left.mean).toBe(right.mean);
    expect(left.stdDev).toBe(right.stdDev);
    expect(voltages(left)).toEqual(voltages(right));
  });

  it("uses different seeds for different trial vectors", () => {
    const left = mcDc(divider(), "mid", 20, {
      seed: 1,
      tolerance: 0.05,
      distribution: "uniform",
    });
    const right = mcDc(divider(), "mid", 20, {
      seed: 2,
      tolerance: 0.05,
      distribution: "uniform",
    });

    expect(voltages(left)).not.toEqual(voltages(right));
  });

  it("reports a spread near the nominal divider voltage", () => {
    const result = mcDc(divider(), "mid", 200, {
      seed: 3,
      tolerance: 0.05,
      distribution: "gaussian",
    });

    expect(result.points.every((point) => point.converged)).toBe(true);
    expect(result.mean).toBeGreaterThan(4.5);
    expect(result.mean).toBeLessThan(5.5);
    expect(result.stdDev).toBeGreaterThan(0.0);
  });

  it("varies current source and VCCS DC parameters", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vctrl", "ctrl", "0", 1.0));
    circuit.add(vccs("Gm", "0", "out", "ctrl", "0", 1.0e-3));
    circuit.add(currentSource("Ibias", "0", "out", 1.0e-3));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    const result = mcDc(circuit, "out", 40, {
      seed: 9,
      tolerance: 0.05,
      distribution: "uniform",
    });

    expect(result.mean).toBeGreaterThan(1.5);
    expect(result.mean).toBeLessThan(2.5);
    expect(result.stdDev).toBeGreaterThan(0.0);
  });

  it("rejects invalid inputs", () => {
    const circuit = divider();

    expect(() => mcDc(circuit, "missing")).toThrowError(SpiceError);
    expect(() => mcDc(circuit, "missing")).toThrowError(
      "output node was not found in circuit",
    );
    expect(() => mcDc(circuit, "mid", 0)).toThrowError(
      "nTrials must be a positive integer",
    );
    expect(() =>
      mcDc(circuit, "mid", 1, { tolerance: Number.NaN }),
    ).toThrowError("tolerance must be finite and non-negative");
    expect(() =>
      mcDc(circuit, "mid", 1, { distribution: "triangular" as never }),
    ).toThrowError("distribution must be 'gaussian' or 'uniform'");
  });
});
