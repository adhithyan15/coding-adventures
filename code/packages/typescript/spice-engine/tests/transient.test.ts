import { describe, expect, it } from "vitest";
import {
  Circuit,
  ExpWaveform,
  PulseWaveform,
  PwlWaveform,
  SinWaveform,
  SpiceError,
  capacitor,
  capacitorWithInitialVoltage,
  cccs,
  ccvs,
  currentSourceWithWaveform,
  inductor,
  inductorWithInitialCurrent,
  resistor,
  transient,
  voltageSource,
  voltageSourceWithWaveform,
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

  it("interpolates and clamps PWL waveforms", () => {
    const waveform = new PwlWaveform([
      [0.0, 0.0],
      [0.5, 1.0],
      [1.0, -1.0],
    ]);

    expectClose(waveform.valueAt(-1.0), 0.0);
    expectClose(waveform.valueAt(0.25), 0.5);
    expectClose(waveform.valueAt(0.75), 0.0);
    expectClose(waveform.valueAt(2.0), -1.0);
  });

  it("uses a PWL waveform on a transient voltage source", () => {
    const circuit = new Circuit();
    circuit.add(
      voltageSourceWithWaveform(
        "Vin",
        "in",
        "0",
        0.0,
        new PwlWaveform([
          [0.0, 0.0],
          [0.5, 1.0],
          [1.0, 1.0],
        ]),
      ),
    );
    circuit.add(resistor("Rload", "in", "0", 1_000.0));

    const points = transient(circuit, 0.25, 1.0);

    expect(points).toHaveLength(4);
    expectClose(points[0].voltage("in"), 0.5);
    expectClose(points[1].voltage("in"), 1.0);
    expectClose(points[2].voltage("in"), 1.0);
    expectClose(points[3].voltage("in"), 1.0);
  });

  it("respects SIN waveform delay and damping", () => {
    const waveform = new SinWaveform(1.0, 2.0, 1.0, 0.5, 1.0);

    expectClose(waveform.valueAt(0.25), 1.0);
    expect(waveform.valueAt(0.75)).toBeCloseTo(
      1.0 + 2.0 * Math.exp(-0.25),
      12,
    );
  });

  it("uses a SIN waveform on a transient voltage source", () => {
    const circuit = new Circuit();
    circuit.add(
      voltageSourceWithWaveform(
        "Vin",
        "in",
        "0",
        0.0,
        new SinWaveform(0.0, 2.0, 1.0),
      ),
    );
    circuit.add(resistor("Rload", "in", "0", 1_000.0));

    const points = transient(circuit, 0.25, 0.5);

    expectClose(points[0].voltage("in"), 2.0);
    expectClose(points[1].voltage("in"), 0.0);
  });

  it("updates CCCS output from transient branch current", () => {
    const circuit = new Circuit();
    circuit.add(
      voltageSourceWithWaveform(
        "Vin",
        "in",
        "0",
        0.0,
        new PwlWaveform([
          [0.0, 0.0],
          [0.25, 1.0],
          [0.5, 1.0],
        ]),
      ),
    );
    circuit.add(resistor("Rsense", "in", "sense", 1_000.0));
    circuit.add(voltageSource("Vsense", "sense", "0", 0.0));
    circuit.add(cccs("F1", "0", "out", "Vsense", 2.0));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    const points = transient(circuit, 0.25, 0.5);

    expectClose(points[0].branchCurrent("Vsense"), 1.0e-3);
    expectClose(points[0].voltage("out"), 2.0);
    expectClose(points[1].voltage("out"), 2.0);
  });

  it("updates CCVS output from transient branch current", () => {
    const circuit = new Circuit();
    circuit.add(
      voltageSourceWithWaveform(
        "Vin",
        "in",
        "0",
        0.0,
        new PwlWaveform([
          [0.0, 0.0],
          [0.25, 1.0],
          [0.5, 1.0],
        ]),
      ),
    );
    circuit.add(resistor("Rsense", "in", "sense", 1_000.0));
    circuit.add(voltageSource("Vsense", "sense", "0", 0.0));
    circuit.add(ccvs("H1", "out", "0", "Vsense", 2_000.0));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    const points = transient(circuit, 0.25, 0.5);

    expectClose(points[0].branchCurrent("Vsense"), 1.0e-3);
    expectClose(points[0].voltage("out"), 2.0);
    expectClose(points[1].voltage("out"), 2.0);
  });

  it("repeats PULSE waveforms with edges", () => {
    const waveform = new PulseWaveform(0.0, 5.0, 0.0, 0.2, 0.2, 0.4, 1.0);

    expectClose(waveform.valueAt(0.1), 2.5);
    expectClose(waveform.valueAt(0.3), 5.0);
    expectClose(waveform.valueAt(0.7), 2.5);
    expectClose(waveform.valueAt(1.3), 5.0);
  });

  it("uses a PULSE waveform on a transient current source", () => {
    const circuit = new Circuit();
    circuit.add(
      currentSourceWithWaveform(
        "Iin",
        "0",
        "out",
        0.0,
        new PulseWaveform(0.0, 0.01, 0.0, 0.0, 0.0, 0.5, 1.0),
      ),
    );
    circuit.add(resistor("Rload", "out", "0", 100.0));

    const points = transient(circuit, 0.25, 0.75);

    expectClose(points[0].voltage("out"), 1.0);
    expectClose(points[1].voltage("out"), 0.0);
    expectClose(points[2].voltage("out"), 0.0);
  });

  it("rises and falls EXP waveforms", () => {
    const waveform = new ExpWaveform(0.0, 2.0, 0.0, 0.5, 1.0, 0.5);

    const rising = waveform.valueAt(0.5);
    const falling = waveform.valueAt(2.0);

    expect(rising).toBeGreaterThan(0.0);
    expect(rising).toBeLessThan(2.0);
    expect(falling).toBeLessThan(rising);
  });

  it("uses an EXP waveform on a transient voltage source", () => {
    const circuit = new Circuit();
    circuit.add(
      voltageSourceWithWaveform(
        "Vin",
        "in",
        "0",
        0.0,
        new ExpWaveform(0.0, 1.0, 0.0, 0.5, 10.0, 1.0),
      ),
    );
    circuit.add(resistor("Rload", "in", "0", 1_000.0));

    const points = transient(circuit, 0.5, 1.0);

    expectClose(points[0].voltage("in"), 1.0 - Math.exp(-1.0));
    expectClose(points[1].voltage("in"), 1.0 - Math.exp(-2.0));
  });

  it("rejects invalid PWL waveforms during transient analysis", () => {
    const circuit = new Circuit();
    circuit.add(
      voltageSourceWithWaveform(
        "Vin",
        "in",
        "0",
        0.0,
        new PwlWaveform([
          [0.0, 0.0],
          [0.0, 1.0],
        ]),
      ),
    );
    circuit.add(resistor("Rload", "in", "0", 1_000.0));

    expect(() => transient(circuit, 0.1, 0.1)).toThrowError(SpiceError);
    expect(() => transient(circuit, 0.1, 0.1)).toThrowError(
      "invalid element Vin",
    );
  });
});
