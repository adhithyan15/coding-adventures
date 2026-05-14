import { describe, expect, it } from "vitest";
import {
  Circuit,
  SinWaveform,
  SpiceError,
  currentSource,
  dcOp,
  dcSweep,
  inductor,
  resistor,
  vccs,
  voltageSource,
  voltageSourceWithWaveform,
} from "../src/index.js";

function expectClose(actual: number | undefined, expected: number): void {
  expect(actual).not.toBeUndefined();
  expect(actual!).toBeCloseTo(expected, 9);
}

describe("dcOp", () => {
  it("solves a resistor divider midpoint voltage", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "vin", "0", 10.0));
    circuit.add(resistor("R1", "vin", "mid", 1_000.0));
    circuit.add(resistor("R2", "mid", "0", 1_000.0));

    const result = dcOp(circuit);

    expectClose(result.voltage("vin"), 10.0);
    expectClose(result.voltage("mid"), 5.0);
    expectClose(result.voltage("0"), 0.0);
  });

  it("uses positive-to-negative orientation for current sources", () => {
    const circuit = new Circuit();
    circuit.add(currentSource("I1", "0", "n1", 1.0e-3));
    circuit.add(resistor("R1", "n1", "0", 1_000.0));

    const result = dcOp(circuit);

    expectClose(result.voltage("n1"), 1.0);
  });

  it("reports voltage source branch current", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "n1", "0", 10.0));
    circuit.add(resistor("R1", "n1", "0", 1_000.0));

    const result = dcOp(circuit);

    expectClose(result.branchCurrent("V1"), -10.0e-3);
    expectClose(result.branchCurrent("I(V1)"), -10.0e-3);
  });

  it("recognizes ground aliases as the zero-volt reference", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "n1", "gnd", 3.3));
    circuit.add(resistor("R1", "n1", "GND", 330.0));

    const result = dcOp(circuit);

    expectClose(result.voltage("n1"), 3.3);
    expectClose(result.voltage("gnd"), 0.0);
    expectClose(result.voltage("GND"), 0.0);
  });

  it("treats inductors as ideal shorts in DC", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "vin", "0", 1.0));
    circuit.add(resistor("R1", "vin", "out", 1_000.0));
    circuit.add(inductor("L1", "out", "0", 1.0));

    const result = dcOp(circuit);

    expectClose(result.voltage("out"), 0.0);
    expectClose(result.branchCurrent("L1"), 1.0e-3);
  });

  it("stamps VCCS current from control voltage", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vctrl", "ctrl", "0", 1.0));
    circuit.add(vccs("G1", "0", "out", "ctrl", "0", 1.0e-3));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    const result = dcOp(circuit);

    expectClose(result.voltage("out"), 1.0);
  });

  it("rejects non-finite VCCS transconductance", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vctrl", "ctrl", "0", 1.0));
    circuit.add(vccs("Gbad", "0", "out", "ctrl", "0", Number.NaN));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    expect(() => dcOp(circuit)).toThrowError("transconductance must be finite");
  });

  it("uses static source value when a waveform is present", () => {
    const circuit = new Circuit();
    circuit.add(
      voltageSourceWithWaveform(
        "V1",
        "n1",
        "0",
        3.0,
        new SinWaveform(0.0, 10.0, 1_000.0),
      ),
    );
    circuit.add(resistor("R1", "n1", "0", 1_000.0));

    const result = dcOp(circuit);

    expectClose(result.voltage("n1"), 3.0);
  });

  it("sweeps voltage sources and collects operating points", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "vin", "0", 0.0));
    circuit.add(resistor("R1", "vin", "mid", 1_000.0));
    circuit.add(resistor("R2", "mid", "0", 1_000.0));

    const points = dcSweep(circuit, "V1", 0.0, 2.0, 1.0);

    expect(points).toHaveLength(3);
    expectClose(points[0].value, 0.0);
    expectClose(points[0].result.voltage("mid"), 0.0);
    expectClose(points[1].value, 1.0);
    expectClose(points[1].result.voltage("mid"), 0.5);
    expectClose(points[2].value, 2.0);
    expectClose(points[2].result.voltage("mid"), 1.0);
  });

  it("sweeps current sources and collects operating points", () => {
    const circuit = new Circuit();
    circuit.add(currentSource("I1", "0", "n1", 0.0));
    circuit.add(resistor("R1", "n1", "0", 1_000.0));

    const points = dcSweep(circuit, "I1", 0.0, 2.0e-3, 1.0e-3);

    expect(points).toHaveLength(3);
    expectClose(points[0].result.voltage("n1"), 0.0);
    expectClose(points[1].result.voltage("n1"), 1.0);
    expectClose(points[2].result.voltage("n1"), 2.0);
  });

  it("rejects sweep steps that do not reach the stop value", () => {
    const circuit = new Circuit();

    expect(() => dcSweep(circuit, "V1", 0.0, 1.0, -0.1)).toThrowError(
      "sweep step direction",
    );
  });

  it("rejects missing sweep sources", () => {
    const circuit = new Circuit();

    expect(() => dcSweep(circuit, "Vmissing", 0.0, 1.0, 1.0)).toThrowError(
      "sweep source must be an independent voltage or current source",
    );
  });

  it("returns a singular matrix error for a floating resistor", () => {
    const circuit = new Circuit();
    circuit.add(resistor("R1", "a", "b", 1_000.0));

    expect(() => dcOp(circuit)).toThrowError(SpiceError);
    expect(() => dcOp(circuit)).toThrowError("circuit matrix is singular");
  });

  it("rejects non-positive resistance", () => {
    const circuit = new Circuit();
    circuit.add(resistor("Rbad", "n1", "0", 0.0));

    expect(() => dcOp(circuit)).toThrowError("invalid element Rbad");
  });

  it("rejects duplicate voltage source names", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "n1", "0", 1.0));
    circuit.add(voltageSource("V1", "n2", "0", 2.0));

    expect(() => dcOp(circuit)).toThrowError("duplicate voltage source name");
  });
});
