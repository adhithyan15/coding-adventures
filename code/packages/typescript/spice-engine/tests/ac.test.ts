import { describe, expect, it } from "vitest";
import {
  Circuit,
  SpiceError,
  acSweep,
  capacitor,
  complexAbs,
  complexPhase,
  currentSource,
  inductor,
  resistor,
  vcvs,
  voltageSource,
} from "../src/index.js";

function expectClose(actual: number | undefined, expected: number): void {
  expect(actual).not.toBeUndefined();
  expect(actual!).toBeCloseTo(expected, 9);
}

describe("acSweep", () => {
  it("keeps a resistive divider frequency-independent", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "vin", "0", 1.0));
    circuit.add(resistor("R1", "vin", "mid", 1_000.0));
    circuit.add(resistor("R2", "mid", "0", 1_000.0));

    const points = acSweep(circuit, 10.0, 1_000.0, 1);

    expect(points).toHaveLength(3);
    for (const point of points) {
      const mid = point.voltage("mid");
      expect(mid).not.toBeUndefined();
      expectClose(mid!.real, 0.5);
      expectClose(mid!.imag, 0.0);
      expectClose(complexAbs(mid!), 0.5);
    }
  });

  it("places an RC low-pass at the minus-three-dB corner", () => {
    const resistance = 1_000.0;
    const capacitance = 1.0e-6;
    const corner = 1.0 / (2.0 * Math.PI * resistance * capacitance);

    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "vin", "0", 1.0));
    circuit.add(resistor("R1", "vin", "out", resistance));
    circuit.add(capacitor("C1", "out", "0", capacitance));

    const points = acSweep(circuit, corner, corner, 10);

    expect(points).toHaveLength(1);
    const out = points[0].voltage("out");
    expect(out).not.toBeUndefined();
    expectClose(complexAbs(out!), 1.0 / Math.sqrt(2.0));
    expectClose(complexPhase(out!), -Math.PI / 4.0);
  });

  it("places an RL high-pass at the minus-three-dB corner", () => {
    const resistance = 1_000.0;
    const inductance = 1.0;
    const corner = resistance / (2.0 * Math.PI * inductance);

    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "vin", "0", 1.0));
    circuit.add(resistor("R1", "vin", "out", resistance));
    circuit.add(inductor("L1", "out", "0", inductance));

    const points = acSweep(circuit, corner, corner, 10);

    expect(points).toHaveLength(1);
    const out = points[0].voltage("out");
    expect(out).not.toBeUndefined();
    expectClose(complexAbs(out!), 1.0 / Math.sqrt(2.0));
    expectClose(complexPhase(out!), Math.PI / 4.0);
  });

  it("injects current-source phasors", () => {
    const circuit = new Circuit();
    circuit.add(currentSource("I1", "0", "n1", 1.0e-3));
    circuit.add(resistor("R1", "n1", "0", 1_000.0));

    const points = acSweep(circuit, 1_000.0, 1_000.0, 10);

    expect(points).toHaveLength(1);
    const n1 = points[0].voltage("n1");
    expect(n1).not.toBeUndefined();
    expectClose(n1!.real, 1.0);
    expectClose(n1!.imag, 0.0);
  });

  it("applies VCVS gain in AC analysis", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 1.0));
    circuit.add(vcvs("E1", "out", "0", "in", "0", 4.0));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    const points = acSweep(circuit, 1_000.0, 1_000.0, 10);

    expect(points).toHaveLength(1);
    const out = points[0].voltage("out");
    expect(out).not.toBeUndefined();
    expectClose(out!.real, 4.0);
    expectClose(out!.imag, 0.0);
    expectClose(points[0].branchCurrent("E1")?.real, -4.0e-3);
  });

  it("rejects invalid frequency bounds", () => {
    const circuit = new Circuit();

    expect(() => acSweep(circuit, 0.0, 1.0, 10)).toThrowError(SpiceError);
    expect(() => acSweep(circuit, 0.0, 1.0, 10)).toThrowError(
      "frequency bounds",
    );
    expect(() => acSweep(circuit, 10.0, 1.0, 10)).toThrowError(
      "stop frequency",
    );
  });
});
