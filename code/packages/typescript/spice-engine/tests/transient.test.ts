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
  currentSource,
  currentSourceWithWaveform,
  estimatePeriod,
  fourier,
  inductor,
  inductorWithInitialCurrent,
  mutualInductor,
  pss,
  pssNewtonCandidate,
  pssNewtonIteration,
  pssNewtonSolve,
  pssNewtonUpdate,
  pssResidualJacobian,
  pssResidual,
  resistor,
  transient,
  transientAdaptive,
  transmissionLine,
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

  it("applies PSS Newton updates to a candidate reactive state", () => {
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

    const result = pssNewtonCandidate(circuit, 32, 1.0e-6, 1.0e-5);

    expect(result).not.toBeUndefined();
    expect(result!.update.nextStateVector[0].name).toBe("C1");
    expect(result!.candidateStateVector).toEqual(result!.update.nextStateVector);
    const candidateCap = result!.candidateCircuit.elements().find(
      (element) => element.kind === "capacitor" && element.name === "C1",
    );
    if (candidateCap?.kind !== "capacitor") {
      throw new Error("missing candidate capacitor");
    }
    const originalCap = circuit.elements().find(
      (element) => element.kind === "capacitor" && element.name === "C1",
    );
    if (originalCap?.kind !== "capacitor") {
      throw new Error("missing original capacitor");
    }
    expectClose(originalCap.initialVoltage, 0.1);
    expectClose(
      candidateCap.initialVoltage,
      result!.update.nextStateVector[0].value,
    );
    expectClose(
      result!.candidateResidual.periodSeconds,
      result!.update.jacobian.residual.periodSeconds,
    );
    expect(Number.isFinite(result!.candidateResidual.residualL2Norm)).toBe(true);
  });

  it("accepts improving PSS Newton iteration candidates", () => {
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

    const result = pssNewtonIteration(circuit, 32, 1.0e-6, 1.0e-5);

    expect(result).not.toBeUndefined();
    const baseResidual = result!.candidate.update.jacobian.residual;
    const candidateResidual = result!.candidate.candidateResidual;
    expect(result!.accepted).toBe(true);
    expect(result!.nextCircuit).toBe(result!.candidate.candidateCircuit);
    expect(result!.nextStateVector).toEqual(result!.candidate.candidateStateVector);
    expect(result!.nextResidual).toBe(candidateResidual);
    expect(result!.converged).toBe(candidateResidual.withinTolerance);
    expect(candidateResidual.residualL2Norm).toBeLessThan(baseResidual.residualL2Norm);
    expectClose(
      result!.residualL2Reduction,
      baseResidual.residualL2Norm - candidateResidual.residualL2Norm,
    );
    expectClose(
      result!.residualL2Ratio,
      candidateResidual.residualL2Norm / baseResidual.residualL2Norm,
    );
  });

  it("runs accepted PSS Newton iterations to convergence", () => {
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

    const result = pssNewtonSolve(circuit, 32, 1.0e-3, 1.0e-5, 4);

    expect(result).not.toBeUndefined();
    expect(result!.iterationCount).toBe(result!.iterations.length);
    expect(result!.iterationCount).toBeGreaterThanOrEqual(1);
    expect(result!.iterationCount).toBeLessThanOrEqual(4);
    expect(result!.iterations.every((iteration) => iteration.accepted)).toBe(true);
    expect(result!.converged).toBe(true);
    expect(result!.finalResidual.withinTolerance).toBe(true);
    expect(result!.finalResidual.residualL2Norm).toBeLessThan(
      result!.iterations[0].candidate.update.jacobian.residual.residualL2Norm,
    );
    expect(result!.finalCircuit).toBe(result!.iterations.at(-1)!.nextCircuit);
    expect(result!.finalStateVector).toEqual(result!.iterations.at(-1)!.nextStateVector);
  });

  it("returns a solved PSS steady-state period", () => {
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

    const result = pss(circuit, 32, 1.0e-3, 1.0e-5, 4);

    expect(result).not.toBeUndefined();
    expect(result!.converged).toBe(true);
    expect(result!.solve.converged).toBe(true);
    expect(result!.periodSeconds).toBe(result!.solve.finalResidual.periodSeconds);
    expect(result!.timeStepSeconds).toBe(result!.solve.finalResidual.timeStepSeconds);
    expect(result!.steadyState.length).toBeGreaterThan(0);
    expect(result!.steadyState.at(-1)!.time).toBeCloseTo(result!.periodSeconds, 12);
    expect(
      pssResidual(result!.solve.finalCircuit, 32, 1.0e-3)!.residualL2Norm,
    ).toBeCloseTo(result!.solve.finalResidual.residualL2Norm, 12);
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

  it("uses Gear-2 BDF2 capacitor companions after an Euler bootstrap step", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "in", "0", 1.0));
    circuit.add(resistor("R1", "in", "vc", 1_000.0));
    circuit.add(capacitor("C1", "vc", "0", 1.0e-6));

    const points = transient(circuit, 1.0e-3, 3.0e-3, "gear2");

    expectClose(points[0].voltage("vc"), 0.5);
    expectClose(points[1].voltage("vc"), 0.8);
    expectClose(points[2].voltage("vc"), 0.94);
  });

  it("uses trapezoidal capacitor companions", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "in", "0", 1.0));
    circuit.add(resistor("R1", "in", "vc", 1_000.0));
    circuit.add(capacitor("C1", "vc", "0", 1.0e-6));

    const points = transient(circuit, 1.0e-3, 3.0e-3, "trap");

    expectClose(points[0].voltage("vc"), 1.0 / 3.0);
    expectClose(points[1].voltage("vc"), 7.0 / 9.0);
    expectClose(points[2].voltage("vc"), 25.0 / 27.0);
  });

  it("uses Gear-2 BDF2 inductor companions after an Euler bootstrap step", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "in", "0", 1.0));
    circuit.add(resistor("R1", "in", "out", 1_000.0));
    circuit.add(inductor("L1", "out", "0", 1.0));

    const points = transient(circuit, 1.0e-3, 3.0e-3, "gear2");

    expectClose(points[0].branchCurrent("L1"), 0.5e-3);
    expectClose(points[1].branchCurrent("L1"), 0.8e-3);
    expectClose(points[2].branchCurrent("L1"), 0.94e-3);
  });

  it("uses trapezoidal inductor companions", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "in", "0", 1.0));
    circuit.add(resistor("R1", "in", "out", 1_000.0));
    circuit.add(inductor("L1", "out", "0", 1.0));

    const points = transient(circuit, 1.0e-3, 3.0e-3, "trap");

    expectClose(points[0].branchCurrent("L1"), 1.0e-3 / 3.0);
    expectClose(points[1].branchCurrent("L1"), 7.0e-3 / 9.0);
    expectClose(points[2].branchCurrent("L1"), 25.0e-3 / 27.0);
  });

  it("shows Gear-2 damping a coarse LC oscillator more than trap", () => {
    const circuit = new Circuit();
    circuit.add(capacitorWithInitialVoltage("C1", "tank", "0", 1.0, 1.0));
    circuit.add(inductor("L1", "tank", "0", 1.0));

    const trapPoints = transient(circuit, 1.0, 10.0, "trap");
    const gearPoints = transient(circuit, 1.0, 10.0, "gear2");
    const trapTail = Math.max(...trapPoints.slice(-4).map((point) => Math.abs(point.voltage("tank") ?? 0.0)));
    const gearTail = Math.max(...gearPoints.slice(-4).map((point) => Math.abs(point.voltage("tank") ?? 0.0)));

    expect(gearTail).toBeLessThan(trapTail * 0.75);
  });

  it("matches fixed-step trap when adaptive bounds pin the step", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "in", "0", 1.0));
    circuit.add(resistor("R1", "in", "vc", 1_000.0));
    circuit.add(capacitor("C1", "vc", "0", 1.0e-6));

    const fixed = transient(circuit, 1.0e-3, 3.0e-3, "trap");
    const adaptive = transientAdaptive(circuit, 1.0e-3, 3.0e-3, {
      method: "trap",
      tolerance: 1.0,
      minStep: 1.0e-3,
      maxStep: 1.0e-3,
    });

    expect(adaptive.converged).toBe(true);
    expect(adaptive.stepsRejected).toBe(0);
    expect(adaptive.points.map((point) => point.time)).toEqual(
      fixed.map((point) => point.time),
    );
    expectClose(adaptive.points.at(-1)?.voltage("vc"), fixed.at(-1)?.voltage("vc") ?? 0.0);
  });

  it("uses variable adaptive steps with Gear-2 after the bootstrap step", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "in", "0", 1.0));
    circuit.add(resistor("R1", "in", "vc", 1_000.0));
    circuit.add(capacitor("C1", "vc", "0", 1.0e-6));

    const adaptive = transientAdaptive(circuit, 1.0e-4, 1.0e-3, {
      method: "gear2",
      tolerance: 1.0,
      maxStep: 5.0e-4,
    });

    expect(adaptive.method).toBe("gear2");
    expect(adaptive.converged).toBe(true);
    expect(adaptive.stepsRejected).toBe(0);
    expect(adaptive.points.length).toBeLessThan(transient(circuit, 1.0e-4, 1.0e-3, "gear2").length);
    expectClose(adaptive.points.at(-1)?.time, 1.0e-3);
    expect((adaptive.points.at(-1)?.voltage("vc") ?? 0.0)).toBeGreaterThan(0.0);
  });

  it("couples secondary voltage through a mutual inductor", () => {
    const circuit = new Circuit();
    circuit.add(currentSource("Istep", "0", "pri", 1.0));
    circuit.add(inductor("Lpri", "pri", "0", 1.0));
    circuit.add(inductor("Lsec", "sec", "0", 1.0));
    circuit.add(mutualInductor("K1", "Lpri", "Lsec", 0.5));
    circuit.add(resistor("Rload", "sec", "0", 10.0));

    const points = transient(circuit, 0.1, 0.1);

    expectClose(points[0].voltage("pri"), 8.75);
    expectClose(points[0].voltage("sec"), 2.5);
    expectClose(points[0].branchCurrent("Lsec"), -0.25);
  });

  it("delays a matched transmission-line step", () => {
    const delay = 1.0e-9;
    const circuit = new Circuit();
    circuit.add(voltageSource("VIN", "in", "0", 1.0));
    circuit.add(transmissionLine("T1", "in", "0", "out", "0", 50.0, delay));
    circuit.add(resistor("RL", "out", "0", 50.0));

    const points = transient(circuit, delay / 2.0, 2.0 * delay);

    expectClose(points[0].voltage("out"), 0.0);
    expectClose(points[1].voltage("out"), 1.0);
    expectClose(points[1].branchCurrent("T1:2"), -0.02);
  });

  it("rejects invalid transmission-line transient parameters", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("VIN", "in", "0", 1.0));
    circuit.add(transmissionLine("Tbad", "in", "0", "out", "0", 50.0, 0.0));
    circuit.add(resistor("RL", "out", "0", 50.0));

    expect(() => transient(circuit, 1.0e-9, 1.0e-9)).toThrowError(SpiceError);
    expect(() => transient(circuit, 1.0e-9, 1.0e-9)).toThrowError(
      "invalid element Tbad",
    );
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

  it("extracts Fourier components from a transient sinusoid", () => {
    const freq = 1_000.0;
    const amp = 2.0;
    const offset = 0.25;
    const period = 1.0 / freq;
    const circuit = new Circuit();
    circuit.add(
      voltageSourceWithWaveform(
        "Vin",
        "in",
        "0",
        0.0,
        new SinWaveform(offset, amp, freq),
      ),
    );

    const points = transient(circuit, period / 64.0, 2.0 * period);
    const analysis = fourier(points, freq, ["V(in)"], 5);
    const probe = analysis.probes[0];
    const fundamental = probe.harmonics[0];

    expect(analysis.startTime).toBeCloseTo(period, 12);
    expect(probe.dc).toBeCloseTo(offset, 3);
    expect(fundamental.frequencyHz).toBeCloseTo(freq, 9);
    expect(fundamental.magnitude).toBeCloseTo(amp, 2);
    expect(fundamental.sine).toBeCloseTo(amp, 2);
    expect(Math.abs(fundamental.cosine)).toBeLessThan(2.0e-3);
    expect(probe.totalHarmonicDistortion).toBeLessThan(2.0e-3);
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
