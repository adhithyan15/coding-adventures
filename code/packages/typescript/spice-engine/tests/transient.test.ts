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
  estimatePeriod,
  inductor,
  inductorWithInitialCurrent,
  pssNewtonUpdate,
  pssResidualJacobian,
  pssResidual,
  resistor,
  transient,
  voltageSource,
  voltageSourceWithWaveform,
  waveformPeriod,
} from "../src/index.js";

function expectClose(actual: number | undefined, expected: number): void {
  expect(actual).not.toBeUndefined();
  expect(actual!).toBeCloseTo(expected, 9);
}

describe("transient", () => {
  it("reports periods for periodic source waveforms", () => {
    expectClose(waveformPeriod(new SinWaveform(0.0, 1.0, 2.0)), 0.5);
    expect(waveformPeriod(new SinWaveform(0.0, 1.0, 2.0, 0.0, 1.0))).toBeUndefined();
    expect(waveformPeriod(new SinWaveform(0.0, 1.0, 0.0))).toBeUndefined();
    expectClose(waveformPeriod(new PulseWaveform(0.0, 1.0, 0.0, 0.0, 0.0, 0.5, 2.5)), 2.5);
    expect(waveformPeriod(new PwlWaveform([[0.0, 0.0], [1.0, 1.0]]))).toBeUndefined();
    expect(waveformPeriod(new ExpWaveform())).toBeUndefined();
  });

  it("estimates a harmonic period for periodic independent sources", () => {
    const circuit = new Circuit();
    circuit.add(
      voltageSourceWithWaveform(
        "V1",
        "in",
        "0",
        0.0,
        new SinWaveform(0.0, 1.0, 1_000.0),
      ),
    );
    circuit.add(
      currentSourceWithWaveform(
        "I1",
        "out",
        "0",
        0.0,
        new PulseWaveform(0.0, 1.0e-3, 0.0, 0.0, 0.0, 0.25e-3, 0.5e-3),
      ),
    );
    circuit.add(resistor("R1", "in", "out", 1_000.0));

    expectClose(estimatePeriod(circuit), 1.0e-3);
  });

  it("does not estimate a period for nonperiodic or incommensurate sources", () => {
    const nonPeriodic = new Circuit();
    nonPeriodic.add(
      voltageSourceWithWaveform(
        "V1",
        "in",
        "0",
        0.0,
        new PwlWaveform([[0.0, 0.0], [1.0e-3, 1.0]]),
      ),
    );
    expect(estimatePeriod(nonPeriodic)).toBeUndefined();

    const incommensurate = new Circuit();
    incommensurate.add(
      voltageSourceWithWaveform(
        "V1",
        "in",
        "0",
        0.0,
        new PulseWaveform(0.0, 1.0, 0.0, 0.0, 0.0, 0.25e-3, 1.0e-3),
      ),
    );
    incommensurate.add(
      currentSourceWithWaveform(
        "I1",
        "out",
        "0",
        0.0,
        new PulseWaveform(0.0, 1.0e-3, 0.0, 0.0, 0.0, 0.25e-3, 0.7e-3),
      ),
    );
    expect(estimatePeriod(incommensurate)).toBeUndefined();
  });

  it("reports one-period PSS node closure residuals", () => {
    const circuit = new Circuit();
    circuit.add(
      voltageSourceWithWaveform(
        "V1",
        "in",
        "0",
        0.0,
        new SinWaveform(0.0, 1.0, 1_000.0),
      ),
    );
    circuit.add(resistor("R1", "in", "0", 1_000.0));

    const result = pssResidual(circuit, 32);

    expect(result).not.toBeUndefined();
    expectClose(result!.periodSeconds, 1.0e-3);
    expectClose(result!.timeStepSeconds, 1.0e-3 / 32.0);
    expectClose(result!.residualTolerance, 1.0e-6);
    expect(result!.withinTolerance).toBe(true);
    expectClose(result!.nodeResiduals.get("in"), 0.0);
    expectClose(result!.branchResiduals.get("I(V1)"), 0.0);
    expect(result!.residualVector.map((entry) => [entry.kind, entry.name])).toEqual([
      ["node", "in"],
      ["branch_current", "I(V1)"],
    ]);
    expectClose(result!.residualVector[0].value, 0.0);
    expectClose(result!.residualVector[1].value, 0.0);
    expectClose(result!.maxAbsBranchResidual, 0.0);
    expectClose(result!.maxAbsResidual, 0.0);
    const expectedL2Norm = Math.sqrt(
      result!.residualVector.reduce((sum, entry) => sum + entry.value * entry.value, 0.0),
    );
    expectClose(result!.residualL2Norm, expectedL2Norm);
    expectClose(result!.residualRmsNorm, expectedL2Norm / Math.sqrt(result!.residualVector.length));
  });

  it("reports finite-difference PSS residual Jacobian columns", () => {
    const circuit = new Circuit();
    circuit.add(
      voltageSourceWithWaveform(
        "V1",
        "in",
        "0",
        0.0,
        new SinWaveform(0.0, 1.0, 1_000.0),
      ),
    );
    circuit.add(resistor("R1", "in", "out", 1_000.0));
    circuit.add(capacitorWithInitialVoltage("C1", "out", "0", 1.0e-6, 0.1));

    const result = pssResidualJacobian(circuit, 32, 1.0e-6, 1.0e-5);

    expect(result).not.toBeUndefined();
    expectClose(result!.perturbation, 1.0e-5);
    expect(result!.stateVector).toEqual([
      { kind: "capacitor_voltage", name: "C1", value: 0.1 },
    ]);
    expect(result!.columns[0].state).toEqual(result!.stateVector[0]);
    expect(result!.jacobian).toHaveLength(result!.residual.residualVector.length);
    expect(result!.jacobian.every((row) => row.length === 1)).toBe(true);
    const outDerivative = result!.columns[0].residualDerivatives.find(
      (entry) => entry.name === "out",
    )!.value;
    const outRow = result!.residual.residualVector.findIndex(
      (entry) => entry.name === "out",
    );
    expectClose(result!.jacobian[outRow][0], outDerivative);
    expect(Math.abs(outDerivative)).toBeGreaterThan(0.1);
    expect(result!.jacobian.every((row) => Number.isFinite(row[0]))).toBe(true);
  });

  it("reports least-squares PSS Newton state updates", () => {
    const circuit = new Circuit();
    circuit.add(
      voltageSourceWithWaveform(
        "V1",
        "in",
        "0",
        0.0,
        new SinWaveform(0.0, 1.0, 1_000.0),
      ),
    );
    circuit.add(resistor("R1", "in", "out", 1_000.0));
    circuit.add(capacitorWithInitialVoltage("C1", "out", "0", 1.0e-6, 0.1));

    const result = pssNewtonUpdate(circuit, 32, 1.0e-6, 1.0e-5);

    expect(result).not.toBeUndefined();
    expect(result!.jacobian.stateVector[0].name).toBe("C1");
    expect(result!.stateUpdates[0].kind).toBe("capacitor_voltage");
    expect(result!.stateUpdates[0].name).toBe("C1");
    expectClose(
      result!.nextStateVector[0].value,
      result!.jacobian.stateVector[0].value + result!.stateUpdates[0].value,
    );
    expectClose(result!.updateL2Norm, Math.abs(result!.stateUpdates[0].value));
    expect(Number.isFinite(result!.stateUpdates[0].value)).toBe(true);
  });

  it("does not report a PSS residual without a periodic source period", () => {
    const circuit = new Circuit();
    circuit.add(
      voltageSourceWithWaveform(
        "V1",
        "in",
        "0",
        0.0,
        new PwlWaveform([[0.0, 0.0], [1.0e-3, 1.0]]),
      ),
    );

    expect(pssResidual(circuit)).toBeUndefined();
  });

  it("rejects negative PSS residual tolerances", () => {
    const circuit = new Circuit();
    circuit.add(
      voltageSourceWithWaveform(
        "V1",
        "in",
        "0",
        0.0,
        new SinWaveform(0.0, 1.0, 1_000.0),
      ),
    );

    expect(() => pssResidual(circuit, 32, -1.0)).toThrow(SpiceError);
  });

  it("rejects non-positive PSS residual Jacobian perturbations", () => {
    const circuit = new Circuit();
    circuit.add(
      voltageSourceWithWaveform(
        "V1",
        "in",
        "0",
        0.0,
        new SinWaveform(0.0, 1.0, 1_000.0),
      ),
    );

    expect(() => pssResidualJacobian(circuit, 32, 1.0e-6, 0.0)).toThrow(
      SpiceError,
    );
  });

  it("returns an empty PSS Newton update without reactive state", () => {
    const circuit = new Circuit();
    circuit.add(
      voltageSourceWithWaveform(
        "V1",
        "in",
        "0",
        0.0,
        new SinWaveform(0.0, 1.0, 1_000.0),
      ),
    );
    circuit.add(resistor("R1", "in", "0", 1_000.0));

    const result = pssNewtonUpdate(circuit, 32);

    expect(result).not.toBeUndefined();
    expect(result!.stateUpdates).toEqual([]);
    expect(result!.nextStateVector).toEqual([]);
    expectClose(result!.updateL2Norm, 0.0);
  });

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
