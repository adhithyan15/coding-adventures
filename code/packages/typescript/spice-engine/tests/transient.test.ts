import { describe, expect, it } from "vitest";
import {
  Circuit,
  type CornerDistortionResult,
  type DistortionResult,
  DigitalLogicLevels,
  DigitalThresholds,
  ExpWaveform,
  type FourierResult,
  type NoiseResult,
  type PoleZeroResult,
  type TransientPoint,
  PulseWaveform,
  PwlWaveform,
  SinWaveform,
  SpiceError,
  capacitor,
  capacitorWithInitialVoltage,
  bjt,
  cccs,
  ccvs,
  currentSource,
  currentSourceWithWaveform,
  dcOp,
  deviceModelChargeAuditFixtures,
  diode,
  digitalEventStreamsToBridgeSchedule,
  digitalEventStreamsToVoltageSources,
  digitalEventsToPwlWaveform,
  digitalEventsToVoltageSource,
  distortionFromFourier,
  distortionFromTransient,
  distortionFromTransientCorners,
  estimatePeriod,
  formatAdaptiveDigitalEventStreamTable,
  formatCornerAdaptiveDigitalEventStreamTable,
  formatCornerDigitalEventStreamTable,
  formatCornerAdaptiveTransientTable,
  formatCornerDistortionTable,
  formatCornerFourierTable,
  formatCornerPoleZeroTable,
  formatCornerPssTable,
  formatCornerTransientTable,
  formatDcTable,
  formatDeckControlPolicyArtifactCsv,
  formatDeckControlPolicyArtifactJson,
  formatDeckControlPolicyArtifactTable,
  formatDeckControlPolicySummaryArtifactCsv,
  formatDeckControlPolicySummaryArtifactJson,
  formatDeckControlPolicySummaryArtifactTable,
  formatDeckNoiseTable,
  formatDeckOpTable,
  deckOutputPlanArtifactRecords,
  formatDeckOutputPlanArtifactCsv,
  formatDeckOutputPlanArtifactJson,
  formatDeckOutputPlanArtifactTable,
  formatDeckRawfileArtifactCsv,
  formatDeckRawfileArtifactJson,
  formatDeckRawfileArtifactTable,
  formatDeckRunArtifactCsv,
  formatDeckRunArtifactJson,
  formatDeckRunArtifactTable,
  deckTableRecords,
  formatDeckTableCsv,
  formatDeckTableJson,
  formatDeckWrdataArtifactCsv,
  formatDeckWrdataArtifactJson,
  formatDeckWrdataArtifactTable,
  formatDeckWrdataAscii,
  formatDeckTransientTable,
  formatDigitalBridgeScheduleTable,
  formatDigitalEventStreamTable,
  formatDigitalEventStreamVcd,
  formatDigitalEventTable,
  formatDistortionTable,
  formatFourierTable,
  formatMeasurementTable,
  formatPoleZeroTable,
  formatPssTable,
  formatTransientTable,
  fourier,
  fourierCorners,
  fourierTransientDeck,
  inductor,
  inductorWithInitialCurrent,
  jfet,
  mosfet,
  mutualInductor,
  pss,
  pssCorners,
  pssNewtonCandidate,
  pssNewtonIteration,
  pssNewtonSolve,
  pssNewtonUpdate,
  poleZeroCorners,
  poleZeroRlcBandpass,
  poleZeroRlcHighpass,
  poleZeroRlcLowpass,
  poleZeroRlcNotch,
  poleZeroRcHighpass,
  poleZeroRcLowpass,
  pssResidualJacobian,
  pssResidual,
  resistor,
  runDeck,
  runDeckAnalysis,
  sampleTransientProbeAsDigitalEvents,
  sampleTransientProbesAsDigitalEventStreams,
  measureTransientDeck,
  measureTransientDelayBetweenProbes,
  measureTransientFindAtProbe,
  measureTransientProbe,
  measureTransientWhenProbe,
  measureTransientWhenProbeCounted,
  transient,
  transientAdaptive,
  transientAdaptiveWithDigitalEventStreams,
  transientAdaptiveWithDigitalEventStreamsCorners,
  transientAdaptiveCorners,
  transientCorners,
  transientWithDigitalEventStreams,
  transientWithDigitalEventStreamsCorners,
  transmissionLine,
  voltageSource,
  voltageSourceWithWaveform,
  waveformPeriod,
} from "../src/index.js";

function expectClose(actual: number | undefined, expected: number): void {
  expect(actual).not.toBeUndefined();
  expect(actual!).toBeCloseTo(expected, 9);
}

function expectRunArtifactTableMatches(execution: {
  readonly runArtifactTable: string;
  readonly runArtifacts: Parameters<typeof formatDeckRunArtifactTable>[0];
}): Record<string, string> {
  expect(execution.runArtifactTable).toBe(formatDeckRunArtifactTable(execution.runArtifacts));
  const records = deckTableRecords(execution.runArtifactTable);
  expect(records).toHaveLength(1);
  return records[0]!;
}

function transientPoint(time: number, nodeVoltages: Record<string, number>): TransientPoint {
  const voltages = new Map(Object.entries(nodeVoltages));
  const currents = new Map<string, number>();
  return {
    time,
    nodeVoltages: voltages,
    branchCurrents: currents,
    voltage(node: string): number | undefined {
      return node === "0" || node.toLowerCase() === "gnd" ? 0.0 : voltages.get(node);
    },
    branchCurrent(sourceName: string): number | undefined {
      const key = sourceName.startsWith("I(") ? sourceName : `I(${sourceName})`;
      return currents.get(key);
    },
  };
}

describe("transient", () => {
  it("lets a JFET source follower charge an output capacitor", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vdd", "vdd", "0", 10.0));
    circuit.add(
      voltageSourceWithWaveform(
        "Vg",
        "gate",
        "0",
        0.0,
        new PwlWaveform([[0.0, 0.0], [1.0e-6, 1.0], [2.0e-6, 1.0]]),
      ),
    );
    circuit.add(jfet("J1", "vdd", "gate", "out", "NJF", 1.0e-3, -2.0));
    circuit.add(resistor("Rs", "out", "0", 1_000.0));
    circuit.add(capacitor("Cout", "out", "0", 1.0e-9));

    const points = transient(circuit, 1.0e-7, 2.0e-6);

    const initialOut = points[0].voltage("out");
    const finalOut = points[points.length - 1].voltage("out");
    expect(finalOut).toBeGreaterThan(initialOut! + 1.0);
    expect(finalOut).toBeGreaterThan(1.5);
    expect(finalOut).toBeLessThan(2.0);
  });

  it("runs device model charge audit transient fixtures", () => {
    const fixtures = deviceModelChargeAuditFixtures();
    expect(fixtures.map((fixture) => fixture.name)).toEqual([
      "diode-storage-charge",
      "bjt-storage-charge",
      "jfet-storage-charge",
      "mos-level1-storage-charge",
    ]);

    for (const fixture of fixtures) {
      const points = transient(fixture.circuit, fixture.timeStepSeconds, fixture.stopTimeSeconds);
      expect(points.length).toBeGreaterThan(0);
      const initial = points[0].voltage(fixture.probeNode);
      const final = points[points.length - 1].voltage(fixture.probeNode);
      expect(initial).not.toBeUndefined();
      expect(final).not.toBeUndefined();
      expect(initial!).toBeGreaterThanOrEqual(fixture.expectedInitialMin);
      expect(initial!).toBeLessThanOrEqual(fixture.expectedInitialMax);
      expect(final!).toBeGreaterThanOrEqual(fixture.expectedFinalMin);
      expect(final!).toBeLessThanOrEqual(fixture.expectedFinalMax);
      expect(fixture.storageCapacitanceFarads).toBeGreaterThan(0.0);
      expect(fixture.deckLines[0].startsWith("* device-model charge fixture:")).toBe(true);
      expect(fixture.deckLines.some((line) => line.startsWith(".model "))).toBe(true);
      expect(fixture.deckLines.some((line) => line.startsWith(".tran "))).toBe(true);
      expect(fixture.chargeBehavior.length).toBeGreaterThan(0);
    }

    const jfetFixture = fixtures.find((fixture) => fixture.kind === "NJF");
    expect(jfetFixture?.chargeBehavior).toContain("CGS/CGD");
    const mosFixture = fixtures.find((fixture) => fixture.kind === "NMOS");
    expect(mosFixture?.chargeBehavior).toContain("CGSO/CGDO/CGBO");
    expect(mosFixture?.chargeBehavior).toContain("CBS/CBD");
  });

  it("uses diode junction capacitance during transient current steps", () => {
    function run(junctionCapacitance: number): TransientPoint[] {
      const circuit = new Circuit();
      circuit.add(currentSourceWithWaveform(
        "Istep",
        "0",
        "out",
        0.0,
        new PwlWaveform([
          [0.0, 0.0],
          [1.0e-9, 1.0e-6],
          [5.0e-9, 1.0e-6],
        ]),
      ));
      circuit.add(resistor("Rshunt", "out", "0", 1.0e12));
      circuit.add(diode("D1", "out", "0", 1.0e-15, 0.02585, 1.0, undefined, 1.0e-3, junctionCapacitance));
      return transient(circuit, 1.0e-9, 5.0e-9);
    }

    const unchargedFirst = run(0.0)[0].voltage("out");
    const chargedFirst = run(1.0e-12)[0].voltage("out");
    expect(unchargedFirst).not.toBeUndefined();
    expect(chargedFirst).not.toBeUndefined();
    expect(unchargedFirst!).toBeGreaterThan(0.5);
    expect(chargedFirst!).toBeLessThan(0.01);
    expect(chargedFirst!).toBeLessThan(unchargedFirst!);
  });

  it("uses JFET gate-source capacitance during transient gate steps", () => {
    function run(gateSourceCapacitance: number): TransientPoint[] {
      const circuit = new Circuit();
      circuit.add(voltageSourceWithWaveform(
        "Vstep",
        "in",
        "0",
        0.0,
        new PwlWaveform([
          [0.0, 0.0],
          [1.0e-9, 1.0],
          [5.0e-9, 1.0],
        ]),
      ));
      circuit.add(resistor("Rin", "in", "gate", 1_000.0));
      circuit.add(resistor("Rdrain", "drain", "0", 1_000.0));
      circuit.add(jfet(
        "J1",
        "drain",
        "gate",
        "0",
        "NJF",
        1.0e-12,
        -2.0,
        0.0,
        gateSourceCapacitance,
      ));
      return transient(circuit, 1.0e-9, 5.0e-9, "euler");
    }

    const unchargedFirst = run(0.0)[0].voltage("gate");
    const chargedFirst = run(1.0e-9)[0].voltage("gate");
    expect(unchargedFirst).not.toBeUndefined();
    expect(chargedFirst).not.toBeUndefined();
    expect(unchargedFirst!).toBeGreaterThan(0.5);
    expect(chargedFirst!).toBeLessThan(0.01);
    expect(chargedFirst!).toBeLessThan(unchargedFirst!);
  });

  it("uses MOSFET overlap capacitance during transient gate steps", () => {
    function run(gateSourceOverlapCapacitance: number): TransientPoint[] {
      const circuit = new Circuit();
      circuit.add(voltageSourceWithWaveform(
        "Vstep",
        "in",
        "0",
        0.0,
        new PwlWaveform([
          [0.0, 0.0],
          [1.0e-9, 1.0],
          [5.0e-9, 1.0],
        ]),
      ));
      circuit.add(resistor("Rin", "in", "gate", 1_000.0));
      circuit.add(resistor("Rdrain", "drain", "0", 1_000.0));
      circuit.add(mosfet("M1", "drain", "gate", "0", "0", "NMOS", {
        KP: 1.0e-12,
        W: 1.0,
        L: 1.0,
        CGSO: gateSourceOverlapCapacitance,
      }));
      return transient(circuit, 1.0e-9, 5.0e-9, "euler");
    }

    const unchargedFirst = run(0.0)[0].voltage("gate");
    const chargedFirst = run(1.0e-9)[0].voltage("gate");
    expect(unchargedFirst).not.toBeUndefined();
    expect(chargedFirst).not.toBeUndefined();
    expect(unchargedFirst!).toBeGreaterThan(0.5);
    expect(chargedFirst!).toBeLessThan(0.01);
    expect(chargedFirst!).toBeLessThan(unchargedFirst!);
  });

  it("uses MOSFET bulk junction capacitance during transient drain steps", () => {
    function run(drainBulkCapacitance: number): TransientPoint[] {
      const circuit = new Circuit();
      circuit.add(voltageSourceWithWaveform(
        "Vstep",
        "in",
        "0",
        0.0,
        new PwlWaveform([
          [0.0, 0.0],
          [1.0e-9, 1.0],
          [5.0e-9, 1.0],
        ]),
      ));
      circuit.add(resistor("Rin", "in", "drain", 1_000.0));
      circuit.add(mosfet("M1", "drain", "0", "0", "0", "NMOS", {
        KP: 1.0e-12,
        W: 1.0,
        L: 1.0,
        CBD: drainBulkCapacitance,
      }));
      return transient(circuit, 1.0e-9, 5.0e-9, "euler");
    }

    const unchargedFirst = run(0.0)[0].voltage("drain");
    const chargedFirst = run(1.0e-9)[0].voltage("drain");
    expect(unchargedFirst).not.toBeUndefined();
    expect(chargedFirst).not.toBeUndefined();
    expect(unchargedFirst!).toBeGreaterThan(0.5);
    expect(chargedFirst!).toBeLessThan(0.01);
    expect(chargedFirst!).toBeLessThan(unchargedFirst!);
  });

  it("shapes MOSFET bulk junction transient capacitance under reverse bias", () => {
    function run(gradingCoefficient: number): TransientPoint[] {
      const circuit = new Circuit();
      circuit.add(voltageSourceWithWaveform(
        "Vstep",
        "in",
        "0",
        1.0,
        new PwlWaveform([
          [0.0, 1.0],
          [1.0e-9, 2.0],
          [5.0e-9, 2.0],
        ]),
      ));
      circuit.add(resistor("Rin", "in", "drain", 1_000.0));
      circuit.add(mosfet("M1", "drain", "0", "0", "0", "NMOS", {
        KP: 1.0e-12,
        W: 1.0,
        L: 1.0,
        CBD: 1.0e-12,
        PB: 1.0,
        MJ: gradingCoefficient,
      }));
      return transient(circuit, 1.0e-9, 5.0e-9, "euler");
    }

    const fixedFirst = run(0.0)[0].voltage("drain");
    const shapedFirst = run(0.5)[0].voltage("drain");
    expect(fixedFirst).not.toBeUndefined();
    expect(shapedFirst).not.toBeUndefined();
    expect(fixedFirst!).toBeCloseTo(1.25, 1);
    expect(shapedFirst!).toBeGreaterThan(fixedFirst! + 0.04);
    expect(shapedFirst!).toBeLessThan(1.4);
  });

  it("uses diode transit time to hold forward charge on turnoff", () => {
    function run(transitTime: number): TransientPoint[] {
      const circuit = new Circuit();
      circuit.add(currentSourceWithWaveform(
        "Istep",
        "0",
        "out",
        0.0,
        new PwlWaveform([
          [0.0, 1.0e-3],
          [1.0e-9, 0.0],
          [5.0e-9, 0.0],
        ]),
      ));
      circuit.add(resistor("Rshunt", "out", "0", 1.0e12));
      circuit.add(diode("D1", "out", "0", 1.0e-15, 0.02585, 1.0, undefined, 1.0e-3, 0.0, transitTime));
      return transient(circuit, 1.0e-9, 5.0e-9);
    }

    const noStorage = run(0.0);
    const stored = run(1.0e-9);
    expectClose(noStorage[0].voltage("out"), 0.0);
    expect(stored[0].voltage("out")!).toBeGreaterThan(0.6);
    expect(stored[stored.length - 1].voltage("out")!).toBeLessThan(stored[0].voltage("out")!);
  });

  it("uses BJT base-emitter capacitance during transient base current steps", () => {
    function run(baseEmitterCapacitance: number): TransientPoint[] {
      const circuit = new Circuit();
      circuit.add(voltageSource("Vcc", "collector", "0", 5.0));
      circuit.add(currentSourceWithWaveform(
        "Istep",
        "0",
        "base",
        0.0,
        new PwlWaveform([
          [0.0, 0.0],
          [1.0e-9, 1.0e-6],
          [5.0e-9, 1.0e-6],
        ]),
      ));
      circuit.add(resistor("Rshunt", "base", "0", 1.0e12));
      circuit.add(bjt(
        "Q1",
        "collector",
        "base",
        "0",
        "NPN",
        1.0e-15,
        100.0,
        0.02585,
        baseEmitterCapacitance,
      ));
      return transient(circuit, 1.0e-9, 5.0e-9);
    }

    const unchargedFirst = run(0.0)[0].voltage("base");
    const chargedFirst = run(1.0e-12)[0].voltage("base");
    expect(unchargedFirst).not.toBeUndefined();
    expect(chargedFirst).not.toBeUndefined();
    expect(unchargedFirst!).toBeGreaterThan(0.5);
    expect(chargedFirst!).toBeLessThan(0.01);
    expect(chargedFirst!).toBeLessThan(unchargedFirst!);
  });

  it("shapes BJT base-emitter depletion capacitance during reverse-biased transients", () => {
    function steppedBaseVoltage(baseEmitterGradingCoefficient: number): number {
      const circuit = new Circuit();
      circuit.add(voltageSourceWithWaveform(
        "Vdrive",
        "in",
        "0",
        -1.0,
        new PwlWaveform([
          [0.0, -1.0],
          [1.0e-9, -1.0],
          [2.0e-9, 0.0],
          [5.0e-9, 0.0],
        ]),
      ));
      circuit.add(resistor("Rin", "in", "base", 1_000.0));
      circuit.add(bjt("Q1", "0", "base", "0", "NPN", 1.0e-14, 100.0, 0.02585, 1.0e-12, 0.0, 0.0, 0.0, 3.0, 1.11, 0.0, 1.0, 1.0, 0.75, baseEmitterGradingCoefficient));
      return transient(circuit, 1.0e-9, 5.0e-9)[1].voltage("base")!;
    }

    expect(steppedBaseVoltage(0.5)).toBeGreaterThan(steppedBaseVoltage(0.0));
  });

  it("shapes BJT base-collector depletion capacitance during reverse-biased transients", () => {
    function steppedCollectorVoltage(baseCollectorGradingCoefficient: number): number {
      const circuit = new Circuit();
      circuit.add(voltageSourceWithWaveform(
        "Vdrive",
        "in",
        "0",
        1.0,
        new PwlWaveform([[0.0, 1.0], [1.0e-9, 1.0], [2.0e-9, 0.0], [5.0e-9, 0.0]]),
      ));
      circuit.add(resistor("Rin", "in", "collector", 1_000.0));
      circuit.add(bjt("Q1", "collector", "0", "0", "NPN", 1.0e-14, 100.0, 0.02585, 0.0, 1.0e-12, 0.0, 0.0, 3.0, 1.11, 0.0, 1.0, 1.0, 0.75, 0.33, 0.75, baseCollectorGradingCoefficient));
      return transient(circuit, 1.0e-9, 5.0e-9)[1].voltage("collector")!;
    }

    expect(steppedCollectorVoltage(0.5)).toBeLessThan(steppedCollectorVoltage(0.0));
  });

  it("uses BJT XCJC to partition depletion charge to the external base", () => {
    function steppedBaseVoltage(baseCollectorCapacitanceFraction: number): number {
      const circuit = new Circuit();
      circuit.add(voltageSourceWithWaveform(
        "Vdrive",
        "in",
        "0",
        0.0,
        new PwlWaveform([[0.0, 0.0], [1.0e-9, 0.0], [2.0e-9, 1.0], [5.0e-9, 1.0]]),
      ));
      circuit.add(resistor("Rin", "in", "base", 1_000.0));
      circuit.add({
        ...bjt("Q1", "0", "base", "0"),
        saturationCurrent: 1.0e-30,
        baseCollectorCapacitance: 1.0e-12,
        baseResistance: 10_000.0,
        baseCollectorCapacitanceFraction,
      });
      return transient(circuit, 1.0e-9, 5.0e-9)[1].voltage("base")!;
    }

    expect(steppedBaseVoltage(1.0)).toBeGreaterThan(steppedBaseVoltage(0.0));
  });

  it("uses BJT FC to shape both forward-biased transient charge companions", () => {
    function heldVoltage(coefficient: number, baseEmitter: boolean): number {
      const circuit = new Circuit();
      circuit.add(voltageSourceWithWaveform(
        "Vdrive",
        "in",
        "0",
        0.6,
        new PwlWaveform([[0.0, 0.6], [1.0e-9, 0.6], [2.0e-9, 0.0], [5.0e-9, 0.0]]),
      ));
      circuit.add(resistor("Rin", "in", "base", 1_000.0));
      circuit.add(bjt("Q1", "0", "base", "0", "NPN", 1.0e-30, 100.0, 0.02585, baseEmitter ? 1.0e-12 : 0.0, baseEmitter ? 0.0 : 1.0e-12, 0.0, 0.0, 3.0, 1.11, 0.0, 1.0, 1.0, 0.75, 0.33, 0.75, 0.33, coefficient));
      return transient(circuit, 1.0e-9, 5.0e-9)[1].voltage("base")!;
    }

    for (const baseEmitter of [true, false]) {
      expect(heldVoltage(0.8, baseEmitter)).toBeGreaterThan(heldVoltage(0.2, baseEmitter));
    }
  });

  it("uses BJT forward transit time to hold base charge on turnoff", () => {
    function run(
      forwardTransitTime: number,
      forwardTransitTimeBiasCoefficient = 0.0,
      forwardTransitTimeCurrent = 0.0,
      forwardTransitTimeVoltage = 0.0,
      collectorVoltage = 5.0,
    ): TransientPoint[] {
      const circuit = new Circuit();
      circuit.add(voltageSource("Vcc", "collector", "0", collectorVoltage));
      circuit.add(currentSourceWithWaveform(
        "Istep",
        "0",
        "base",
        0.0,
        new PwlWaveform([
          [0.0, 1.0e-3],
          [1.0e-9, 0.0],
          [5.0e-9, 0.0],
        ]),
      ));
      circuit.add(resistor("Rshunt", "base", "0", 1.0e12));
      circuit.add({
        ...bjt(
          "Q1",
          "collector",
          "base",
          "0",
          "NPN",
          1.0e-15,
          100.0,
          0.02585,
          0.0,
          0.0,
          forwardTransitTime,
        ),
        forwardTransitTimeBiasCoefficient,
        forwardTransitTimeCurrent,
        forwardTransitTimeVoltage,
      });
      return transient(circuit, 1.0e-9, 5.0e-9);
    }

    const noStorage = run(0.0);
    const stored = run(1.0e-9);
    const biasScaled = run(1.0e-9, 9.0);
    const currentLimited = run(1.0e-9, 9.0, 1.0);
    const voltageLimited = run(1.0e-9, 9.0, 0.0, 0.5, 10.0);
    expectClose(noStorage[0].voltage("base"), 0.0);
    expect(stored[0].voltage("base")!).toBeGreaterThan(0.6);
    expect(stored[stored.length - 1].voltage("base")!).toBeLessThan(stored[0].voltage("base")!);
    expect(Math.abs(
      biasScaled[biasScaled.length - 1].voltage("base")! -
        stored[stored.length - 1].voltage("base")!,
    )).toBeGreaterThan(1.0e-12);
    expect(Math.abs(
      currentLimited[currentLimited.length - 1].voltage("base")! -
        stored[stored.length - 1].voltage("base")!,
    )).toBeLessThan(Math.abs(
      biasScaled[biasScaled.length - 1].voltage("base")! -
        stored[stored.length - 1].voltage("base")!,
    ));
    expect(Math.abs(
      voltageLimited[voltageLimited.length - 1].voltage("base")! -
        stored[stored.length - 1].voltage("base")!,
    )).toBeLessThan(Math.abs(
      biasScaled[biasScaled.length - 1].voltage("base")! -
        stored[stored.length - 1].voltage("base")!,
    ));
  });

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

  it("runs PSS per corner and formats stable text tables", () => {
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

    const nominal = pss(circuit, 4, 1.0e-9, 1.0e-5, 2);
    const result = pssCorners(circuit, [
      { name: "nominal", overrides: [] },
      {
        name: "rload-high",
        overrides: [{ elementName: "R1", parameter: "resistance", value: 2_000.0 }],
      },
    ], 4, 1.0e-9, 1.0e-5, 2);

    expect(nominal).not.toBeUndefined();
    expect(result).not.toBeUndefined();
    expect(result!.points.map((point) => point.cornerName)).toEqual(["nominal", "rload-high"]);
    expect(result!.points.every((point) => point.result.converged)).toBe(true);
    expectClose(result!.points[0].result.periodSeconds, 1.0e-3);
    expectClose(result!.points[1].result.steadyState[0].branchCurrent("V1"), -5.0e-4);
    expect(formatPssTable(nominal!, ["V(in)", "I(V1)"])).toBe(
      "Index\tPeriod\tTimeStep\tConverged\tIterations\tResidualL2\tTime\tV(in)\tI(V1)\n" +
        "0\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449295e-16\t2.500000e-04\t1.000000e+00\t-1.000000e-03\n" +
        "1\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449295e-16\t5.000000e-04\t1.224647e-16\t-1.224647e-19\n" +
        "2\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449295e-16\t7.500000e-04\t-1.000000e+00\t1.000000e-03\n" +
        "3\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449295e-16\t1.000000e-03\t-2.449294e-16\t2.449294e-19\n",
    );
    expect(formatCornerPssTable(result!, ["V(in)", "I(V1)"])).toBe(
      "Corner\tIndex\tPeriod\tTimeStep\tConverged\tIterations\tResidualL2\tTime\tV(in)\tI(V1)\n" +
        "nominal\t0\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449295e-16\t2.500000e-04\t1.000000e+00\t-1.000000e-03\n" +
        "nominal\t1\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449295e-16\t5.000000e-04\t1.224647e-16\t-1.224647e-19\n" +
        "nominal\t2\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449295e-16\t7.500000e-04\t-1.000000e+00\t1.000000e-03\n" +
        "nominal\t3\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449295e-16\t1.000000e-03\t-2.449294e-16\t2.449294e-19\n" +
        "rload-high\t0\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449294e-16\t2.500000e-04\t1.000000e+00\t-5.000000e-04\n" +
        "rload-high\t1\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449294e-16\t5.000000e-04\t1.224647e-16\t-6.123234e-20\n" +
        "rload-high\t2\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449294e-16\t7.500000e-04\t-1.000000e+00\t5.000000e-04\n" +
        "rload-high\t3\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449294e-16\t1.000000e-03\t-2.449294e-16\t1.224647e-19\n",
    );
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

  it("routes parsed .four cards into transient Fourier analyses", () => {
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
    const analyses = fourierTransientDeck(
      points,
      `
.tran 15.625u 2m
.four 1k V(in) HARMONICS=5 FROM=1m
.end
`,
    );
    const analysis = analyses[0];
    const probe = analysis.probes[0];
    const fundamental = probe.harmonics[0];

    expect(analyses).toHaveLength(1);
    expect(probe.probe).toBe("V(in)");
    expect(probe.harmonics).toHaveLength(5);
    expect(analysis.startTime).toBeCloseTo(period, 12);
    expect(probe.dc).toBeCloseTo(offset, 3);
    expect(fundamental.frequencyHz).toBeCloseTo(freq, 9);
    expect(fundamental.magnitude).toBeCloseTo(amp, 2);
  });

  it("runs Fourier analysis for each named corner and formats the table", () => {
    const circuit = new Circuit();
    circuit.add(
      voltageSourceWithWaveform(
        "Vin",
        "in",
        "0",
        0.0,
        new SinWaveform(0.0, 1.0, 1_000.0),
      ),
    );
    circuit.add(resistor("R1", "in", "out", 1_000.0));
    circuit.add(resistor("R2", "out", "0", 1_000.0));

    const result = fourierCorners(
      circuit,
      2.5e-4,
      2.0e-3,
      1_000.0,
      ["V(out)"],
      [
        { name: "nominal", overrides: [] },
        { name: "r2-high", overrides: [{ elementName: "R2", parameter: "resistance", value: 2_000.0 }] },
      ],
      2,
    );

    expect(result.fundamentalFrequencyHz).toBeCloseTo(1_000.0, 9);
    expect(result.points[0].cornerName).toBe("nominal");
    expect(result.points[1].cornerName).toBe("r2-high");
    expect(result.points[0].result.probes[0].harmonics[0].magnitude).toBeCloseTo(0.5, 9);
    expect(result.points[1].result.probes[0].harmonics[0].magnitude).toBeCloseTo(2.0 / 3.0, 9);
    expect(formatCornerFourierTable(result)).toBe(
      "Corner\tProbe\tHarmonic\tFrequency\tCosine\tSine\tMagnitude\tPhase\tDC\tTHD\n" +
        "nominal\tV(out)\t1\t1.000000e+03\t6.018531e-33\t5.000000e-01\t5.000000e-01\t6.896729e-31\t0.000000e+00\t1.224647e-16\n" +
        "nominal\tV(out)\t2\t2.000000e+03\t0.000000e+00\t-6.123234e-17\t6.123234e-17\t1.800000e+02\t0.000000e+00\t1.224647e-16\n" +
        "r2-high\tV(out)\t1\t1.000000e+03\t7.523164e-33\t6.666667e-01\t6.666667e-01\t6.465683e-31\t1.355253e-17\t1.290373e-16\n" +
        "r2-high\tV(out)\t2\t2.000000e+03\t2.710505e-17\t-8.164312e-17\t8.602490e-17\t1.616341e+02\t1.355253e-17\t1.290373e-16\n",
    );
  });

  it("models pole-zero result shapes for a simple RC pole fixture", () => {
    const resistance = 1_000.0;
    const capacitance = 1.0e-6;
    const poleRadPerSecond = -1.0 / (resistance * capacitance);
    const result: PoleZeroResult = {
      inputSource: "Vin",
      outputNode: "out",
      entries: [
        {
          kind: "pole",
          real: poleRadPerSecond,
          imaginary: 0.0,
          frequencyHz: Math.abs(poleRadPerSecond) / (2.0 * Math.PI),
          damping: 1.0,
        },
      ],
    };

    expect(result.entries[0].kind).toBe("pole");
    expect(result.entries[0].frequencyHz).toBeCloseTo(
      1.0 / (2.0 * Math.PI * resistance * capacitance),
      9,
    );
  });

  it("computes the pole for a simple RC low-pass fixture", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 1.0));
    circuit.add(resistor("R1", "in", "out", 1_000.0));
    circuit.add(capacitor("C1", "out", "0", 1.0e-6));

    const result = poleZeroRcLowpass(circuit, "Vin", "out");

    expect(result).toEqual({
      inputSource: "Vin",
      outputNode: "out",
      entries: [
        {
          kind: "pole",
          real: -1.0e3,
          imaginary: 0.0,
          frequencyHz: 1.0e3 / (2.0 * Math.PI),
          damping: 1.0,
        },
      ],
    });
  });

  it("runs selected pole-zero topology for each named corner and formats the table", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 1.0));
    circuit.add(resistor("R1", "in", "out", 1_000.0));
    circuit.add(capacitor("C1", "out", "0", 1.0e-6));

    const result = poleZeroCorners(circuit, "Vin", "out", "rc-lowpass", [
      { name: "nominal", overrides: [] },
      { name: "cap-high", overrides: [{ elementName: "C1", parameter: "capacitance", value: 2.0e-6 }] },
    ]);

    expect(result.inputSource).toBe("Vin");
    expect(result.outputNode).toBe("out");
    expect(result.topology).toBe("rc-lowpass");
    expect(result.points[0].cornerName).toBe("nominal");
    expect(result.points[1].cornerName).toBe("cap-high");
    expect(result.points[0].result.entries[0].real).toBeCloseTo(-1.0e3, 9);
    expect(result.points[1].result.entries[0].real).toBeCloseTo(-5.0e2, 9);
    expect(formatCornerPoleZeroTable(result)).toBe(
      "Corner\tIndex\tKind\tReal\tImaginary\tFrequency\tDamping\n" +
        "nominal\t0\tpole\t-1.000000e+03\t0.000000e+00\t1.591549e+02\t1.000000e+00\n" +
        "cap-high\t0\tpole\t-5.000000e+02\t0.000000e+00\t7.957747e+01\t1.000000e+00\n",
    );
  });

  it("computes the zero and pole for a simple RC high-pass fixture", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 1.0));
    circuit.add(capacitor("C1", "in", "out", 1.0e-6));
    circuit.add(resistor("R1", "out", "0", 1_000.0));

    const result = poleZeroRcHighpass(circuit, "Vin", "out");

    expect(result).toEqual({
      inputSource: "Vin",
      outputNode: "out",
      entries: [
        {
          kind: "zero",
          real: 0.0,
          imaginary: 0.0,
          frequencyHz: 0.0,
          damping: 1.0,
        },
        {
          kind: "pole",
          real: -1.0e3,
          imaginary: 0.0,
          frequencyHz: 1.0e3 / (2.0 * Math.PI),
          damping: 1.0,
        },
      ],
    });
  });

  it("computes complex conjugate poles for a series RLC low-pass fixture", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 1.0));
    circuit.add(resistor("R1", "in", "mid", 10.0));
    circuit.add(inductor("L1", "mid", "out", 1.0e-3));
    circuit.add(capacitor("C1", "out", "0", 1.0e-6));

    const result = poleZeroRlcLowpass(circuit, "Vin", "out");

    const alpha = 10.0 / (2.0 * 1.0e-3);
    const omega0 = 1.0 / Math.sqrt(1.0e-3 * 1.0e-6);
    const imaginary = Math.sqrt(omega0 * omega0 - alpha * alpha);
    expect(result).toEqual({
      inputSource: "Vin",
      outputNode: "out",
      entries: [
        {
          kind: "pole",
          real: -alpha,
          imaginary,
          frequencyHz: omega0 / (2.0 * Math.PI),
          damping: alpha / omega0,
        },
        {
          kind: "pole",
          real: -alpha,
          imaginary: -imaginary,
          frequencyHz: omega0 / (2.0 * Math.PI),
          damping: alpha / omega0,
        },
      ],
    });
  });

  it("computes origin zeros and complex conjugate poles for a series RLC high-pass fixture", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 1.0));
    circuit.add(resistor("R1", "in", "mid", 10.0));
    circuit.add(capacitor("C1", "mid", "out", 1.0e-6));
    circuit.add(inductor("L1", "out", "0", 1.0e-3));

    const result = poleZeroRlcHighpass(circuit, "Vin", "out");

    const alpha = 10.0 / (2.0 * 1.0e-3);
    const omega0 = 1.0 / Math.sqrt(1.0e-3 * 1.0e-6);
    const imaginary = Math.sqrt(omega0 * omega0 - alpha * alpha);
    expect(result).toEqual({
      inputSource: "Vin",
      outputNode: "out",
      entries: [
        {
          kind: "zero",
          real: 0.0,
          imaginary: 0.0,
          frequencyHz: 0.0,
          damping: 1.0,
        },
        {
          kind: "zero",
          real: 0.0,
          imaginary: 0.0,
          frequencyHz: 0.0,
          damping: 1.0,
        },
        {
          kind: "pole",
          real: -alpha,
          imaginary,
          frequencyHz: omega0 / (2.0 * Math.PI),
          damping: alpha / omega0,
        },
        {
          kind: "pole",
          real: -alpha,
          imaginary: -imaginary,
          frequencyHz: omega0 / (2.0 * Math.PI),
          damping: alpha / omega0,
        },
      ],
    });
  });

  it("computes an origin zero and complex conjugate poles for a series RLC band-pass fixture", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 1.0));
    circuit.add(inductor("L1", "in", "mid", 1.0e-3));
    circuit.add(capacitor("C1", "mid", "out", 1.0e-6));
    circuit.add(resistor("R1", "out", "0", 10.0));

    const result = poleZeroRlcBandpass(circuit, "Vin", "out");

    const alpha = 10.0 / (2.0 * 1.0e-3);
    const omega0 = 1.0 / Math.sqrt(1.0e-3 * 1.0e-6);
    const imaginary = Math.sqrt(omega0 * omega0 - alpha * alpha);
    expect(result).toEqual({
      inputSource: "Vin",
      outputNode: "out",
      entries: [
        {
          kind: "zero",
          real: 0.0,
          imaginary: 0.0,
          frequencyHz: 0.0,
          damping: 1.0,
        },
        {
          kind: "pole",
          real: -alpha,
          imaginary,
          frequencyHz: omega0 / (2.0 * Math.PI),
          damping: alpha / omega0,
        },
        {
          kind: "pole",
          real: -alpha,
          imaginary: -imaginary,
          frequencyHz: omega0 / (2.0 * Math.PI),
          damping: alpha / omega0,
        },
      ],
    });
  });

  it("computes notch zeros and complex conjugate poles for a series RLC notch fixture", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 1.0));
    circuit.add(resistor("R1", "in", "out", 10.0));
    circuit.add(inductor("L1", "out", "mid", 1.0e-3));
    circuit.add(capacitor("C1", "mid", "0", 1.0e-6));

    const result = poleZeroRlcNotch(circuit, "Vin", "out");

    const alpha = 10.0 / (2.0 * 1.0e-3);
    const omega0 = 1.0 / Math.sqrt(1.0e-3 * 1.0e-6);
    const imaginary = Math.sqrt(omega0 * omega0 - alpha * alpha);
    expect(result).toEqual({
      inputSource: "Vin",
      outputNode: "out",
      entries: [
        {
          kind: "zero",
          real: 0.0,
          imaginary: omega0,
          frequencyHz: omega0 / (2.0 * Math.PI),
          damping: 0.0,
        },
        {
          kind: "zero",
          real: 0.0,
          imaginary: -omega0,
          frequencyHz: omega0 / (2.0 * Math.PI),
          damping: 0.0,
        },
        {
          kind: "pole",
          real: -alpha,
          imaginary,
          frequencyHz: omega0 / (2.0 * Math.PI),
          damping: alpha / omega0,
        },
        {
          kind: "pole",
          real: -alpha,
          imaginary: -imaginary,
          frequencyHz: omega0 / (2.0 * Math.PI),
          damping: alpha / omega0,
        },
      ],
    });
  });

  it("models distortion result shapes for a nonlinear-device smoke fixture", () => {
    const result: DistortionResult = {
      inputSource: "Vin",
      outputProbe: "V(out)",
      points: [
        {
          frequencyHz: 1.0e3,
          fundamentalMagnitude: 1.0,
          harmonics: [
            {
              harmonic: 2,
              frequencyHz: 2.0e3,
              magnitude: 0.025,
              phaseDegrees: -12.0,
            },
          ],
          totalHarmonicDistortion: 0.025,
        },
      ],
    };

    expect(result.points[0].harmonics[0].harmonic).toBe(2);
    expect(result.points[0].totalHarmonicDistortion).toBeCloseTo(0.025, 9);
  });

  it("projects Fourier probe harmonics into the distortion result shape", () => {
    const result = distortionFromFourier(
      {
        fundamentalFrequencyHz: 1.0e3,
        startTime: 0.0,
        endTime: 1.0e-3,
        probes: [
          {
            probe: "V(out)",
            dc: 0.0,
            harmonics: [
              {
                harmonic: 1,
                frequencyHz: 1.0e3,
                cosine: 0.0,
                sine: 1.0,
                magnitude: 1.0,
                phaseDegrees: 0.0,
              },
              {
                harmonic: 2,
                frequencyHz: 2.0e3,
                cosine: 0.0,
                sine: 0.025,
                magnitude: 0.025,
                phaseDegrees: -12.0,
              },
            ],
            totalHarmonicDistortion: 0.025,
          },
        ],
      },
      "Vin",
      "V(out)",
    );

    expect(result).toEqual({
      inputSource: "Vin",
      outputProbe: "V(out)",
      points: [
        {
          frequencyHz: 1.0e3,
          fundamentalMagnitude: 1.0,
          harmonics: [
            {
              harmonic: 2,
              frequencyHz: 2.0e3,
              magnitude: 0.025,
              phaseDegrees: -12.0,
            },
          ],
          totalHarmonicDistortion: 0.025,
        },
      ],
    });
  });

  it("extracts distortion harmonic content from transient samples", () => {
    const freq = 1.0e3;
    const period = 1.0 / freq;
    const points = Array.from({ length: 129 }, (_, index) => {
      const time = (index * period) / 64.0;
      const value = Math.sin(2.0 * Math.PI * freq * time) + 0.1 * Math.sin(4.0 * Math.PI * freq * time);
      return {
        time,
        nodeVoltages: new Map([["out", value]]),
        branchCurrents: new Map<string, number>(),
        voltage(node: string): number | undefined {
          return node === "0" || node.toLowerCase() === "gnd" ? 0.0 : this.nodeVoltages.get(node);
        },
        branchCurrent(sourceName: string): number | undefined {
          return this.branchCurrents.get(sourceName.startsWith("I(") ? sourceName : `I(${sourceName})`);
        },
      };
    });

    const result = distortionFromTransient(points, freq, "Vin", "V(out)", 3);

    expect(result.inputSource).toBe("Vin");
    expect(result.outputProbe).toBe("V(out)");
    expect(result.points[0].frequencyHz).toBeCloseTo(freq, 9);
    expect(result.points[0].fundamentalMagnitude).toBeCloseTo(1.0, 3);
    expect(result.points[0].harmonics[0].harmonic).toBe(2);
    expect(result.points[0].harmonics[0].magnitude).toBeCloseTo(0.1, 3);
    expect(result.points[0].totalHarmonicDistortion).toBeCloseTo(0.1, 3);
  });

  it("projects transient distortion for each named corner", () => {
    const freq = 1.0e3;
    const period = 1.0 / freq;
    const circuit = new Circuit();
    circuit.add(
      voltageSourceWithWaveform(
        "Vin",
        "in",
        "0",
        0.0,
        new SinWaveform(0.0, 1.0, freq),
      ),
    );
    circuit.add(resistor("Rtop", "in", "out", 1_000.0));
    circuit.add(resistor("Rbot", "out", "0", 1_000.0));

    const result = distortionFromTransientCorners(
      circuit,
      period / 64.0,
      2.0 * period,
      freq,
      "Vin",
      "V(out)",
      [
        { name: "nominal", overrides: [] },
        { name: "rbot-high", overrides: [{ elementName: "Rbot", parameter: "resistance", value: 3_000.0 }] },
      ],
      3,
    );

    expect(result.inputSource).toBe("Vin");
    expect(result.outputProbe).toBe("V(out)");
    expect(result.points[0].cornerName).toBe("nominal");
    expect(result.points[1].cornerName).toBe("rbot-high");
    expect(result.points[0].result.points[0].fundamentalMagnitude).toBeCloseTo(0.5, 3);
    expect(result.points[1].result.points[0].fundamentalMagnitude).toBeCloseTo(0.75, 3);
    expect(result.points[0].result.points[0].totalHarmonicDistortion).toBeLessThan(2.0e-3);
    expect(result.points[1].result.points[0].totalHarmonicDistortion).toBeLessThan(2.0e-3);
  });

  it("formats stable text output tables for DC and transient results", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "vin", "0", 10.0));
    circuit.add(resistor("R1", "vin", "mid", 1_000.0));
    circuit.add(resistor("R2", "mid", "0", 1_000.0));
    const dcResult = dcOp(circuit);

    expect(formatDcTable(dcResult)).toBe(
      "Index\tV(mid)\tV(vin)\tI(V1)\n" +
        "0\t5.000000e+00\t1.000000e+01\t-5.000000e-03\n",
    );
    expect(formatDcTable(dcResult, ["V(vin, mid)", "I(V1)"])).toBe(
      "Index\tV(vin, mid)\tI(V1)\n" +
        "0\t5.000000e+00\t-5.000000e-03\n",
    );
    expect(formatDeckOpTable(dcResult, ".save V(mid)\n.probe dc I(V1)\n.end\n")).toBe(
      "Index\tV(mid)\n" +
        "0\t5.000000e+00\n",
    );

    const points = transient(circuit, 1.0e-3, 2.0e-3);
    expect(formatTransientTable(points, ["V(vin)", "V(mid)", "I(V1)"])).toBe(
      "Index\tTime\tV(vin)\tV(mid)\tI(V1)\n" +
        "0\t1.000000e-03\t1.000000e+01\t5.000000e+00\t-5.000000e-03\n" +
        "1\t2.000000e-03\t1.000000e+01\t5.000000e+00\t-5.000000e-03\n",
    );
    expect(
      formatDeckTransientTable(
        points,
        ".save V(mid)\n.probe tran V(vin)\n.print tran I(V1)\n.plot tran V(vin)\n.end\n",
      ),
    ).toBe(
      "Index\tTime\tV(mid)\tV(vin)\tI(V1)\n" +
        "0\t1.000000e-03\t5.000000e+00\t1.000000e+01\t-5.000000e-03\n" +
        "1\t2.000000e-03\t5.000000e+00\t1.000000e+01\t-5.000000e-03\n",
    );
  });

  it("selects marker probe columns in WRDATA ASCII output", () => {
    const table =
      "Index\tV(in)\tI(V1)\n" +
      "0\t1.000000e+00\t-1.000000e-03\n" +
      "1\t2.000000e+00\t-2.000000e-03\n";

    expect(
      formatDeckWrdataAscii(
        table,
        ["I(V1)"],
        ["set wr_vecnames", "set wr_singlescale"],
      ),
    ).toBe(
      "# SPICE deck wrdata artifact\n" +
        "Probes: I(V1)\n" +
        "Options: set wr_vecnames;set wr_singlescale\n" +
        "VectorNames: Index;I(V1)\n" +
        "Scale: Index\n" +
        "Index\tI(V1)\n" +
        "0\t-1.000000e-03\n" +
        "1\t-2.000000e-03\n",
    );
  });

  it("routes selected deck analysis plans into solver executions", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "vin", "0", 1.0));
    circuit.add(resistor("R1", "vin", "mid", 1_000.0));
    circuit.add(resistor("R2", "mid", "0", 1_000.0));
    const netlist = `
.save V(mid)
.probe dc I(V1)
.print dc V(mid)
.plot ac V(mid)
.op
.dc V1 0 1 1
.ac dec 1 1k 1k
.tran 1m 1m
.tf V(mid) V1
.sens V(mid)
.noise V(mid) V1 lin 1 1k 1k
.measure dc mid_avg avg V(mid)
.measure ac mid_peak max V(mid)
.measure tran mid_final final V(mid)
.end
`;
    const netlistLines = netlist.split(/\r?\n/u);
    const directiveLine = (prefix: string) =>
      netlistLines.findIndex((line) => line.trimStart().startsWith(prefix)) + 1;
    const saveLine = directiveLine(".save");
    const probeDcLine = directiveLine(".probe dc");
    const printDcLine = directiveLine(".print dc");
    const plotAcLine = directiveLine(".plot ac");

    const opExecution = runDeckAnalysis(circuit, netlist, "op");
    expect(opExecution.plan.analysis).toBe("op");
    expect(opExecution.outputProbes).toEqual(["V(mid)"]);
    expect(opExecution.outputDirectives).toEqual([".save"]);
    expect(opExecution.analysisDirectives).toEqual([".op"]);
    expect(opExecution.tableCount).toBe(3);
    expect(opExecution.tables).toEqual(["result", "output-plan", "run-artifact"]);
    expect(opExecution.tableArtifacts.map((artifact) => artifact.name)).toEqual(
      opExecution.tables,
    );
    expect(opExecution.measurements).toEqual([]);
    expect(opExecution.measurementTable).toBe("Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n");
    expect(opExecution.table).toBe("Index\tV(mid)\n0\t5.000000e-01\n");
    expect(formatDeckTableCsv(opExecution.table)).toBe("Index,V(mid)\n0,5.000000e-01\n");
    expect(deckTableRecords(opExecution.table)).toEqual([
      { Index: "0", "V(mid)": "5.000000e-01" },
    ]);
    expect(JSON.parse(formatDeckTableJson(opExecution.table))).toEqual([
      { Index: "0", "V(mid)": "5.000000e-01" },
    ]);
    expect(opExecution.tableArtifacts[0]).toMatchObject({
      name: "result",
      table: opExecution.table,
      csv: formatDeckTableCsv(opExecution.table),
      json: formatDeckTableJson(opExecution.table),
      records: deckTableRecords(opExecution.table),
    });
    const expectedOutputPlanColumns = [
      "Analysis",
      "Directive",
      "Line",
      "SourceName",
      "OutputNode",
      "SweepKind",
      "StartValue",
      "StopValue",
      "StepValue",
      "PointCount",
      "StartFrequencyHz",
      "StopFrequencyHz",
      "StepTime",
      "StopTime",
      "StartTime",
      "MaxStep",
      "UseInitialConditions",
      "ResultRows",
      "ResultColumns",
      "ResultColumnList",
      "OutputProbes",
      "OutputProbeList",
      "OutputProbeLines",
      "OutputProbeLineList",
      "OutputDirectives",
      "OutputDirectiveList",
      "OutputDirectiveKinds",
      "OutputDirectiveKindList",
      "OutputDirectiveAnalysisKinds",
      "OutputDirectiveAnalysisKindList",
      "OutputDirectiveLines",
      "OutputDirectiveLineList",
      "Tables",
      "TableList",
    ];
    const expectedOutputPlanRow = [
      "op",
      ".op",
      String(opExecution.plan.lineNumber),
      "",
      "",
      "",
      "",
      "",
      "",
      "",
      "",
      "",
      "",
      "",
      "",
      "",
      "",
      "1",
      "2",
      "Index;V(mid)",
      "1",
      "V(mid)",
      "1",
      String(saveLine),
      "1",
      ".save",
      "1",
      "save",
      "1",
      "global",
      "1",
      String(saveLine),
      "3",
      "result;output-plan;run-artifact",
    ];
    const expectedOutputPlanRecords = [
      Object.fromEntries(
        expectedOutputPlanColumns.map((column, index) => [
          column,
          expectedOutputPlanRow[index] ?? "",
        ]),
      ),
    ];
    expect(opExecution.outputPlanArtifactCount).toBe(1);
    expect(opExecution.outputPlanArtifacts).toHaveLength(1);
    expect(opExecution.outputPlanArtifacts[0]).toMatchObject({
      analysis: "op",
      directive: ".op",
      lineNumber: opExecution.plan.lineNumber,
      sourceName: undefined,
      outputNode: undefined,
      sweepKind: undefined,
      startValue: undefined,
      stopValue: undefined,
      stepValue: undefined,
      pointCount: undefined,
      startFrequencyHz: undefined,
      stopFrequencyHz: undefined,
      stepTime: undefined,
      stopTime: undefined,
      startTime: undefined,
      maxStep: undefined,
      useInitialConditions: undefined,
      resultRowCount: 1,
      resultColumnCount: 2,
      resultColumns: ["Index", "V(mid)"],
      outputProbeCount: 1,
      outputProbes: ["V(mid)"],
      outputProbeLineCount: 1,
      outputProbeLines: [saveLine],
      outputDirectiveCount: 1,
      outputDirectives: [".save"],
      outputDirectiveKindCount: 1,
      outputDirectiveKinds: ["save"],
      outputDirectiveAnalysisKindCount: 1,
      outputDirectiveAnalysisKinds: ["global"],
      outputDirectiveLineCount: 1,
      outputDirectiveLines: [saveLine],
      tableCount: 3,
      tables: ["result", "output-plan", "run-artifact"],
    });
    expect(opExecution.outputPlanArtifactTable).toBe(
      `${expectedOutputPlanColumns.join("\t")}\n${expectedOutputPlanRow.join("\t")}\n`,
    );
    expect(opExecution.outputPlanArtifactTable).toBe(
      formatDeckOutputPlanArtifactTable(opExecution.outputPlanArtifacts),
    );
    expect(opExecution.outputPlanArtifactCsv).toBe(
      `${expectedOutputPlanColumns.join(",")}\n${expectedOutputPlanRow.join(",")}\n`,
    );
    expect(opExecution.outputPlanArtifactCsv).toBe(
      formatDeckOutputPlanArtifactCsv(opExecution.outputPlanArtifacts),
    );
    expect(JSON.parse(opExecution.outputPlanArtifactJson)).toEqual(expectedOutputPlanRecords);
    expect(opExecution.outputPlanArtifactJson).toBe(
      formatDeckOutputPlanArtifactJson(opExecution.outputPlanArtifacts),
    );
    expect(opExecution.outputPlanArtifactRecords).toEqual(expectedOutputPlanRecords);
    expect(opExecution.outputPlanArtifactRecords).toEqual(
      deckOutputPlanArtifactRecords(opExecution.outputPlanArtifacts),
    );
    expect(opExecution.runArtifacts[0]?.resultRows).toBe(1);
    expect(opExecution.runArtifacts[0]?.resultColumnCount).toBe(2);
    expect(opExecution.runArtifacts[0]?.resultColumns).toEqual(["Index", "V(mid)"]);
    expect(opExecution.runArtifacts[0]?.tableCount).toBe(3);
    expect(opExecution.runArtifacts[0]?.tables).toEqual(["result", "output-plan", "run-artifact"]);
    expect(opExecution.runArtifacts[0]?.sourceName).toBeUndefined();
    expect(opExecution.runArtifacts[0]?.outputNode).toBeUndefined();
    expect(opExecution.runArtifacts[0]?.sweepKind).toBeUndefined();
    expect(opExecution.runArtifacts[0]?.pointCount).toBeUndefined();
    expect(opExecution.runArtifacts[0]?.stepTime).toBeUndefined();
    expect(opExecution.runArtifacts[0]?.useInitialConditions).toBeUndefined();
    expect(opExecution.runArtifacts[0]?.outputProbes).toEqual(["V(mid)"]);
    expect(opExecution.runArtifacts[0]?.outputDirectives).toEqual([".save"]);
    expect(opExecution.runArtifacts[0]?.analysisDirectiveCount).toBe(1);
    expect(opExecution.runArtifacts[0]?.analysisDirectives).toEqual([".op"]);
    expect(opExecution.runArtifacts[0]?.measurementNames).toEqual([]);
    expect(opExecution.runArtifacts[0]?.fourierProbes).toEqual([]);
    expect(opExecution.runArtifacts[0]?.controlLineCount).toBe(0);
    expect(opExecution.runArtifacts[0]?.controlLines).toEqual([]);
    expect(opExecution.diagnosticCount).toBe(0);
    expect(opExecution.diagnosticCodes).toEqual([]);
    expect(opExecution.runArtifacts[0]?.diagnosticCount).toBe(0);
    expect(opExecution.runArtifacts[0]?.diagnosticCodes).toEqual([]);
    const opRunArtifactRecord = expectRunArtifactTableMatches(opExecution);
    expect(opRunArtifactRecord.Analysis).toBe("op");
    expect(opRunArtifactRecord.DeckAnalysisKinds).toBe("7");
    expect(opRunArtifactRecord.DeckAnalysisKindList).toBe("op;dc;ac;tran;tf;sens;noise");
    expect(opRunArtifactRecord.DeckAnalysisDirectives).toBe("7");
    expect(opExecution.tableArtifacts[1]?.name).toBe("output-plan");
    expect(opExecution.tableArtifacts[1]?.table).toBe(opExecution.outputPlanArtifactTable);
    expect(opExecution.tableArtifacts[1]?.csv).toBe(opExecution.outputPlanArtifactCsv);
    expect(opExecution.tableArtifacts[1]?.json).toBe(opExecution.outputPlanArtifactJson);
    expect(opExecution.tableArtifacts[1]?.records).toEqual(
      opExecution.outputPlanArtifactRecords,
    );
    expect(opExecution.tableArtifacts[2]?.name).toBe("run-artifact");
    expect(opExecution.tableArtifacts[2]?.table).toBe(opExecution.runArtifactTable);
    expect(opExecution.tableArtifacts[2]?.records).toEqual(
      deckTableRecords(opExecution.runArtifactTable),
    );
    expect(formatDeckTableCsv(opExecution.runArtifactTable)).toBe(
      formatDeckRunArtifactCsv(opExecution.runArtifacts),
    );
    expect(formatDeckTableJson(opExecution.runArtifactTable)).toBe(
      formatDeckRunArtifactJson(opExecution.runArtifacts),
    );
    expect(deckTableRecords(opExecution.runArtifactTable)).toEqual(
      JSON.parse(formatDeckRunArtifactJson(opExecution.runArtifacts)),
    );
    expect(formatDeckRunArtifactCsv(opExecution.runArtifacts)).toBe(
      formatDeckTableCsv(opExecution.runArtifactTable),
    );
    expect(formatDeckTableCsv('Name\tValue\nprobe\tSPICE,"QUOTED"\n')).toBe(
      'Name,Value\nprobe,"SPICE,""QUOTED"""\n',
    );
    expect(formatDeckTableJson('Name\tValue\nprobe\tSPICE,"QUOTED"\n')).toBe(
      '[{"Name":"probe","Value":"SPICE,\\"QUOTED\\""}]\n',
    );
    expect(deckTableRecords('Name\tValue\nprobe\tSPICE,"QUOTED"\n')).toEqual([
      { Name: "probe", Value: 'SPICE,"QUOTED"' },
    ]);
    const artifactJson = formatDeckRunArtifactJson(opExecution.runArtifacts);
    const artifactRecords = JSON.parse(artifactJson) as Array<Record<string, string>>;
    expect(Object.keys(artifactRecords[0]!)).toEqual([
      "Analysis",
      "Directive",
      "AnalysisDirectives",
      "AnalysisDirectiveList",
      "Line",
      "SourceName",
      "OutputNode",
      "SweepKind",
      "StartValue",
      "StopValue",
      "StepValue",
      "PointCount",
      "StartFrequencyHz",
      "StopFrequencyHz",
      "StepTime",
      "StopTime",
      "StartTime",
      "MaxStep",
      "UseInitialConditions",
      "ResultRows",
      "ResultColumns",
      "ResultColumnList",
      "Tables",
      "TableList",
      "OutputProbes",
      "OutputProbeList",
      "OutputDirectives",
      "OutputDirectiveList",
      "Measurements",
      "MeasurementList",
      "Fourier",
      "FourierList",
      "ControlLines",
      "ControlLineList",
      "WriteMarkers",
      "WriteMarkerList",
      "RawfileOptions",
      "RawfileOptionList",
      "ControlPolicyArtifacts",
      "ControlPolicyCategoryList",
      "ControlPolicyCodeList",
      "ControlPolicySeverityList",
      "Diagnostics",
      "DiagnosticCodeList",
      "DeckAnalysisKinds",
      "DeckAnalysisKindList",
      "DeckAnalysisDirectives",
      "DeckAnalysisDirectiveList",
    ]);
    expect(artifactRecords).toEqual([
      {
        Analysis: "op",
        Directive: ".op",
        AnalysisDirectives: "1",
        AnalysisDirectiveList: ".op",
        Line: String(opExecution.plan.lineNumber),
        SourceName: "",
        OutputNode: "",
        SweepKind: "",
        StartValue: "",
        StopValue: "",
        StepValue: "",
        PointCount: "",
        StartFrequencyHz: "",
        StopFrequencyHz: "",
        StepTime: "",
        StopTime: "",
        StartTime: "",
        MaxStep: "",
        UseInitialConditions: "",
        ResultRows: "1",
        ResultColumns: "2",
        ResultColumnList: "Index;V(mid)",
        Tables: "3",
        TableList: "result;output-plan;run-artifact",
        OutputProbes: "1",
        OutputProbeList: "V(mid)",
        OutputDirectives: "1",
        OutputDirectiveList: ".save",
        Measurements: "0",
        MeasurementList: "",
        Fourier: "0",
        FourierList: "",
        ControlLines: "0",
        ControlLineList: "",
        WriteMarkers: "0",
        WriteMarkerList: "",
        RawfileOptions: "0",
        RawfileOptionList: "",
        ControlPolicyArtifacts: "0",
        ControlPolicyCategoryList: "",
        ControlPolicyCodeList: "",
        ControlPolicySeverityList: "",
        Diagnostics: "0",
        DiagnosticCodeList: "",
        DeckAnalysisKinds: "7",
        DeckAnalysisKindList: "op;dc;ac;tran;tf;sens;noise",
        DeckAnalysisDirectives: "7",
        DeckAnalysisDirectiveList: ".op;.dc;.ac;.tran;.tf;.sens;.noise",
      },
    ]);
    const diagnosticArtifact = {
      ...opExecution.runArtifacts[0]!,
      diagnosticCount: 2,
      diagnosticCodes: ["SPICE_DECK_ANALYSIS_TOKEN", "SPICE_DECK_ANALYSIS_RANGE"],
    };
    const diagnosticRecord = deckTableRecords(formatDeckRunArtifactTable([diagnosticArtifact]))[0]!;
    expect(diagnosticRecord.Diagnostics).toBe("2");
    expect(diagnosticRecord.DiagnosticCodeList).toBe(
      "SPICE_DECK_ANALYSIS_TOKEN;SPICE_DECK_ANALYSIS_RANGE",
    );
    const quotedDiagnosticArtifact = {
      ...opExecution.runArtifacts[0]!,
      diagnosticCount: 2,
      diagnosticCodes: ["SPICE_DECK_ANALYSIS_TOKEN", 'SPICE,"QUOTED"'],
    };
    expect(formatDeckRunArtifactCsv([quotedDiagnosticArtifact])).toMatch(
      /"SPICE_DECK_ANALYSIS_TOKEN;SPICE,""QUOTED""",7,op;dc;ac;tran;tf;sens;noise,7,.op;.dc;.ac;.tran;.tf;.sens;.noise\n$/u,
    );
    expect(
      (JSON.parse(formatDeckRunArtifactJson([quotedDiagnosticArtifact])) as Array<Record<string, string>>)[0]?.[
        "DiagnosticCodeList"
      ],
    ).toBe('SPICE_DECK_ANALYSIS_TOKEN;SPICE,"QUOTED"');

    const dcExecution = runDeckAnalysis(circuit, netlist, "dc");
    expect(dcExecution.plan.sourceName).toBe("V1");
    expect(dcExecution.outputProbes).toEqual(["V(mid)", "I(V1)"]);
    expect(dcExecution.outputDirectives).toEqual([".save", ".probe", ".print"]);
    expect(dcExecution.outputPlanArtifacts[0]?.lineNumber).toBe(dcExecution.plan.lineNumber);
    expect(dcExecution.outputPlanArtifacts[0]?.sourceName).toBe("V1");
    expect(dcExecution.outputPlanArtifacts[0]?.outputNode).toBeUndefined();
    expect(dcExecution.outputPlanArtifacts[0]?.sweepKind).toBeUndefined();
    expect(dcExecution.outputPlanArtifacts[0]?.startValue).toBeCloseTo(0.0, 12);
    expect(dcExecution.outputPlanArtifacts[0]?.stopValue).toBeCloseTo(1.0, 12);
    expect(dcExecution.outputPlanArtifacts[0]?.stepValue).toBeCloseTo(1.0, 12);
    expect(dcExecution.outputPlanArtifacts[0]?.pointCount).toBeUndefined();
    expect(dcExecution.outputPlanArtifacts[0]?.startFrequencyHz).toBeUndefined();
    expect(dcExecution.outputPlanArtifacts[0]?.stopFrequencyHz).toBeUndefined();
    expect(dcExecution.outputPlanArtifacts[0]?.stepTime).toBeUndefined();
    expect(dcExecution.outputPlanArtifacts[0]?.useInitialConditions).toBeUndefined();
    expect(dcExecution.outputPlanArtifactRecords[0]?.Line).toBe(
      String(dcExecution.plan.lineNumber),
    );
    expect(dcExecution.outputPlanArtifactRecords[0]?.SourceName).toBe("V1");
    expect(dcExecution.outputPlanArtifactRecords[0]?.OutputNode).toBe("");
    expect(dcExecution.outputPlanArtifactRecords[0]?.SweepKind).toBe("");
    expect(dcExecution.outputPlanArtifactRecords[0]?.StartValue).toBe("0.000000e+00");
    expect(dcExecution.outputPlanArtifactRecords[0]?.StopValue).toBe("1.000000e+00");
    expect(dcExecution.outputPlanArtifactRecords[0]?.StepValue).toBe("1.000000e+00");
    expect(dcExecution.outputPlanArtifactRecords[0]?.PointCount).toBe("");
    expect(dcExecution.outputPlanArtifactRecords[0]?.StartFrequencyHz).toBe("");
    expect(dcExecution.outputPlanArtifactRecords[0]?.StopFrequencyHz).toBe("");
    expect(dcExecution.outputPlanArtifactRecords[0]?.StepTime).toBe("");
    expect(dcExecution.outputPlanArtifactRecords[0]?.UseInitialConditions).toBe("");
    expect(dcExecution.outputPlanArtifacts[0]?.outputDirectiveKinds).toEqual([
      "save",
      "probe",
      "print",
    ]);
    expect(dcExecution.outputPlanArtifactRecords[0]?.OutputDirectiveKindList).toBe(
      "save;probe;print",
    );
    expect(dcExecution.outputPlanArtifacts[0]?.outputDirectiveAnalysisKinds).toEqual([
      "global",
      "dc",
    ]);
    const dcOutputDirectiveLines = [saveLine, probeDcLine, printDcLine];
    const dcOutputProbeLines = [saveLine, probeDcLine];
    expect(dcExecution.outputPlanArtifacts[0]?.outputProbeLines).toEqual(
      dcOutputProbeLines,
    );
    expect(dcExecution.outputPlanArtifacts[0]?.outputDirectiveLines).toEqual(
      dcOutputDirectiveLines,
    );
    expect(dcExecution.outputPlanArtifactRecords[0]?.OutputDirectiveAnalysisKindList).toBe(
      "global;dc",
    );
    expect(dcExecution.outputPlanArtifactRecords[0]?.OutputProbeLineList).toBe(
      dcOutputProbeLines.join(";"),
    );
    expect(dcExecution.outputPlanArtifactRecords[0]?.OutputDirectiveLineList).toBe(
      dcOutputDirectiveLines.join(";"),
    );
    expect(dcExecution.analysisDirectives).toEqual([".dc"]);
    expect(dcExecution.tableCount).toBe(4);
    expect(dcExecution.tables).toEqual(["result", "measurement", "output-plan", "run-artifact"]);
    expect(dcExecution.tableArtifacts.map((artifact) => artifact.name)).toEqual(
      dcExecution.tables,
    );
    expect(dcExecution.measurements.map((measurement) => measurement.name)).toEqual(["mid_avg"]);
    expect(dcExecution.measurementTable).toBe(
      "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n" +
        "mid_avg\tdc\tV(mid)\tavg\t\t\t2.500000e-01\n",
    );
    expect(dcExecution.tableArtifacts[1]).toMatchObject({
      name: "measurement",
      table: dcExecution.measurementTable,
      csv: formatDeckTableCsv(dcExecution.measurementTable),
      json: formatDeckTableJson(dcExecution.measurementTable),
      records: deckTableRecords(dcExecution.measurementTable),
    });
    expect(Array.isArray(dcExecution.result)).toBe(true);
    expect(dcExecution.table).toBe(
      "Index\tSource\tValue\tV(mid)\tI(V1)\n" +
        "0\tV1\t0.000000e+00\t0.000000e+00\t0.000000e+00\n" +
        "1\tV1\t1.000000e+00\t5.000000e-01\t-5.000000e-04\n",
    );
    expect(dcExecution.runArtifacts[0]?.analysis).toBe("dc");
    expect(dcExecution.runArtifacts[0]?.sourceName).toBe("V1");
    expect(dcExecution.runArtifacts[0]?.outputNode).toBeUndefined();
    expect(dcExecution.runArtifacts[0]?.startValue).toBeCloseTo(0.0, 12);
    expect(dcExecution.runArtifacts[0]?.stopValue).toBeCloseTo(1.0, 12);
    expect(dcExecution.runArtifacts[0]?.stepValue).toBeCloseTo(1.0, 12);
    expect(dcExecution.runArtifacts[0]?.resultColumnCount).toBe(5);
    expect(dcExecution.runArtifacts[0]?.resultColumns).toEqual([
      "Index",
      "Source",
      "Value",
      "V(mid)",
      "I(V1)",
    ]);
    expect(dcExecution.runArtifacts[0]?.tableCount).toBe(4);
    expect(dcExecution.runArtifacts[0]?.tables).toEqual([
      "result",
      "measurement",
      "output-plan",
      "run-artifact",
    ]);
    expect(dcExecution.runArtifacts[0]?.stepTime).toBeUndefined();
    expect(dcExecution.runArtifacts[0]?.useInitialConditions).toBeUndefined();
    expect(dcExecution.runArtifacts[0]?.outputProbes).toEqual(["V(mid)", "I(V1)"]);
    expect(dcExecution.runArtifacts[0]?.outputDirectives).toEqual([
      ".save",
      ".probe",
      ".print",
    ]);
    expect(dcExecution.runArtifacts[0]?.analysisDirectives).toEqual([".dc"]);
    expect(dcExecution.runArtifacts[0]?.measurementNames).toEqual(["mid_avg"]);
    expect(dcExecution.runArtifacts[0]?.fourierProbes).toEqual([]);
    const dcRunArtifactRecord = expectRunArtifactTableMatches(dcExecution);
    expect(dcRunArtifactRecord.Analysis).toBe("dc");
    expect(dcRunArtifactRecord.DeckAnalysisKinds).toBe("7");
    expect(dcRunArtifactRecord.DeckAnalysisKindList).toBe("op;dc;ac;tran;tf;sens;noise");

    const acExecution = runDeckAnalysis(circuit, netlist, "ac");
    expect(acExecution.outputProbes).toEqual(["V(mid)"]);
    expect(acExecution.outputDirectives).toEqual([".save", ".plot"]);
    expect(acExecution.outputPlanArtifacts[0]?.outputNode).toBeUndefined();
    expect(acExecution.outputPlanArtifacts[0]?.sweepKind).toBe("dec");
    expect(acExecution.outputPlanArtifacts[0]?.pointCount).toBe(1);
    expect(acExecution.outputPlanArtifacts[0]?.startFrequencyHz).toBeCloseTo(1.0e3, 9);
    expect(acExecution.outputPlanArtifacts[0]?.stopFrequencyHz).toBeCloseTo(1.0e3, 9);
    expect(acExecution.outputPlanArtifacts[0]?.startValue).toBeUndefined();
    expect(acExecution.outputPlanArtifacts[0]?.stepTime).toBeUndefined();
    expect(acExecution.outputPlanArtifacts[0]?.useInitialConditions).toBeUndefined();
    expect(acExecution.outputPlanArtifactRecords[0]?.SweepKind).toBe("dec");
    expect(acExecution.outputPlanArtifactRecords[0]?.PointCount).toBe("1");
    expect(acExecution.outputPlanArtifactRecords[0]?.StartFrequencyHz).toBe("1.000000e+03");
    expect(acExecution.outputPlanArtifactRecords[0]?.StopFrequencyHz).toBe("1.000000e+03");
    expect(acExecution.outputPlanArtifactRecords[0]?.StepTime).toBe("");
    expect(acExecution.outputPlanArtifacts[0]?.outputDirectiveKinds).toEqual([
      "save",
      "plot",
    ]);
    expect(acExecution.outputPlanArtifactRecords[0]?.OutputDirectiveKindList).toBe(
      "save;plot",
    );
    expect(acExecution.outputPlanArtifacts[0]?.outputDirectiveAnalysisKinds).toEqual([
      "global",
      "ac",
    ]);
    const acOutputDirectiveLines = [saveLine, plotAcLine];
    expect(acExecution.outputPlanArtifacts[0]?.outputProbeLines).toEqual([saveLine]);
    expect(acExecution.outputPlanArtifacts[0]?.outputDirectiveLines).toEqual(
      acOutputDirectiveLines,
    );
    expect(acExecution.outputPlanArtifactRecords[0]?.OutputDirectiveAnalysisKindList).toBe(
      "global;ac",
    );
    expect(acExecution.outputPlanArtifactRecords[0]?.OutputDirectiveLineList).toBe(
      acOutputDirectiveLines.join(";"),
    );
    expect(acExecution.analysisDirectives).toEqual([".ac"]);
    expect(acExecution.tableCount).toBe(4);
    expect(acExecution.tables).toEqual(["result", "measurement", "output-plan", "run-artifact"]);
    expect(acExecution.measurements.map((measurement) => measurement.name)).toEqual(["mid_peak"]);
    expect(acExecution.measurementTable).toBe(
      "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n" +
        "mid_peak\tac\tV(mid)\tmax\t\t\t5.000000e-01\n",
    );
    expect(Array.isArray(acExecution.result)).toBe(true);
    expect(acExecution.table).toBe(
      "Index\tFrequency\tProbe\tReal\tImaginary\tMagnitude\tPhase\n" +
        "0\t1.000000e+03\tV(mid)\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\n",
    );
    expect(acExecution.runArtifacts[0]?.outputProbes).toEqual(["V(mid)"]);
    expect(acExecution.runArtifacts[0]?.sourceName).toBeUndefined();
    expect(acExecution.runArtifacts[0]?.outputNode).toBeUndefined();
    expect(acExecution.runArtifacts[0]?.sweepKind).toBe("dec");
    expect(acExecution.runArtifacts[0]?.pointCount).toBe(1);
    expect(acExecution.runArtifacts[0]?.startFrequencyHz).toBeCloseTo(1.0e3, 9);
    expect(acExecution.runArtifacts[0]?.stopFrequencyHz).toBeCloseTo(1.0e3, 9);
    expect(acExecution.runArtifacts[0]?.resultColumnCount).toBe(7);
    expect(acExecution.runArtifacts[0]?.resultColumns).toEqual([
      "Index",
      "Frequency",
      "Probe",
      "Real",
      "Imaginary",
      "Magnitude",
      "Phase",
    ]);
    expect(acExecution.runArtifacts[0]?.tableCount).toBe(4);
    expect(acExecution.runArtifacts[0]?.tables).toEqual([
      "result",
      "measurement",
      "output-plan",
      "run-artifact",
    ]);
    expect(acExecution.runArtifacts[0]?.stepTime).toBeUndefined();
    expect(acExecution.runArtifacts[0]?.useInitialConditions).toBeUndefined();
    expect(acExecution.runArtifacts[0]?.outputDirectives).toEqual([".save", ".plot"]);
    expect(acExecution.runArtifacts[0]?.measurementNames).toEqual(["mid_peak"]);
    expect(acExecution.runArtifacts[0]?.fourierProbes).toEqual([]);
    const acRunArtifactRecord = expectRunArtifactTableMatches(acExecution);
    expect(acRunArtifactRecord.Analysis).toBe("ac");
    expect(acRunArtifactRecord.DeckAnalysisKinds).toBe("7");
    expect(acRunArtifactRecord.DeckAnalysisKindList).toBe("op;dc;ac;tran;tf;sens;noise");

    const tranExecution = runDeckAnalysis(circuit, netlist, "tran");
    expect(tranExecution.outputProbes).toEqual(["V(mid)"]);
    expect(tranExecution.outputDirectives).toEqual([".save"]);
    expect(tranExecution.outputPlanArtifacts[0]?.stepTime).toBeCloseTo(1.0e-3, 12);
    expect(tranExecution.outputPlanArtifacts[0]?.stopTime).toBeCloseTo(1.0e-3, 12);
    expect(tranExecution.outputPlanArtifacts[0]?.startTime).toBeUndefined();
    expect(tranExecution.outputPlanArtifacts[0]?.maxStep).toBeUndefined();
    expect(tranExecution.outputPlanArtifacts[0]?.useInitialConditions).toBe(false);
    expect(tranExecution.outputPlanArtifactRecords[0]?.StepTime).toBe("1.000000e-03");
    expect(tranExecution.outputPlanArtifactRecords[0]?.StopTime).toBe("1.000000e-03");
    expect(tranExecution.outputPlanArtifactRecords[0]?.StartTime).toBe("");
    expect(tranExecution.outputPlanArtifactRecords[0]?.MaxStep).toBe("");
    expect(tranExecution.outputPlanArtifactRecords[0]?.UseInitialConditions).toBe("false");
    expect(tranExecution.outputPlanArtifacts[0]?.outputProbeLines).toEqual([saveLine]);
    expect(tranExecution.outputPlanArtifacts[0]?.outputDirectiveLines).toEqual([saveLine]);
    expect(tranExecution.analysisDirectives).toEqual([".tran"]);
    expect(tranExecution.tableCount).toBe(4);
    expect(tranExecution.tables).toEqual(["result", "measurement", "output-plan", "run-artifact"]);
    expect(tranExecution.measurements.map((measurement) => measurement.name)).toEqual(["mid_final"]);
    expect(tranExecution.measurementTable).toBe(
      "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n" +
        "mid_final\ttran\tV(mid)\tlast\t\t\t5.000000e-01\n",
    );
    expect(Array.isArray(tranExecution.result)).toBe(true);
    expect(tranExecution.table).toBe(
      "Index\tTime\tV(mid)\n" +
        "0\t1.000000e-03\t5.000000e-01\n",
    );
    expect(tranExecution.runArtifacts[0]?.outputProbes).toEqual(["V(mid)"]);
    expect(tranExecution.runArtifacts[0]?.sourceName).toBeUndefined();
    expect(tranExecution.runArtifacts[0]?.outputNode).toBeUndefined();
    expect(tranExecution.runArtifacts[0]?.stepTime).toBeCloseTo(1.0e-3, 12);
    expect(tranExecution.runArtifacts[0]?.stopTime).toBeCloseTo(1.0e-3, 12);
    expect(tranExecution.runArtifacts[0]?.resultColumnCount).toBe(3);
    expect(tranExecution.runArtifacts[0]?.resultColumns).toEqual(["Index", "Time", "V(mid)"]);
    expect(tranExecution.runArtifacts[0]?.tableCount).toBe(4);
    expect(tranExecution.runArtifacts[0]?.tables).toEqual([
      "result",
      "measurement",
      "output-plan",
      "run-artifact",
    ]);
    expect(tranExecution.runArtifacts[0]?.startTime).toBeUndefined();
    expect(tranExecution.runArtifacts[0]?.maxStep).toBeUndefined();
    expect(tranExecution.runArtifacts[0]?.useInitialConditions).toBe(false);
    expect(tranExecution.runArtifacts[0]?.outputDirectives).toEqual([".save"]);
    expect(tranExecution.runArtifacts[0]?.measurementNames).toEqual(["mid_final"]);
    expect(tranExecution.runArtifacts[0]?.fourierProbes).toEqual([]);
    expect(tranExecution.runArtifacts[0]?.diagnosticCount).toBe(0);
    expect(tranExecution.runArtifacts[0]?.diagnosticCodes).toEqual([]);
    const tranRunArtifactRecord = expectRunArtifactTableMatches(tranExecution);
    expect(tranRunArtifactRecord.Analysis).toBe("tran");
    expect(tranRunArtifactRecord.DeckAnalysisKinds).toBe("7");
    expect(tranRunArtifactRecord.DeckAnalysisKindList).toBe("op;dc;ac;tran;tf;sens;noise");

    const tfExecution = runDeckAnalysis(circuit, netlist, "tf");
    expect(tfExecution.plan.outputNode).toBe("mid");
    expect(tfExecution.plan.sourceName).toBe("V1");
    const tfResult = tfExecution.result as {
      readonly transferRatio: number;
      readonly inputImpedanceOhms: number;
      readonly outputImpedanceOhms: number;
    };
    expect(tfResult.transferRatio).toBeCloseTo(0.5, 9);
    expect(tfResult.inputImpedanceOhms).toBeCloseTo(2_000.0, 9);
    expect(tfResult.outputImpedanceOhms).toBeCloseTo(500.0, 9);
    expect(tfExecution.outputProbes).toEqual(["V(mid)"]);
    expect(tfExecution.outputDirectives).toEqual([]);
    expect(tfExecution.outputPlanArtifacts[0]?.outputNode).toBe("mid");
    expect(tfExecution.outputPlanArtifactRecords[0]?.OutputNode).toBe("mid");
    expect(tfExecution.analysisDirectives).toEqual([".tf"]);
    expect(tfExecution.tableCount).toBe(3);
    expect(tfExecution.tables).toEqual(["result", "output-plan", "run-artifact"]);
    expect(tfExecution.measurements).toEqual([]);
    expect(tfExecution.measurementTable).toBe("Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n");
    expect(tfExecution.table).toBe(
      "TransferRatio\tInputImpedance\tOutputImpedance\n" +
        "5.000000e-01\t2.000000e+03\t5.000000e+02\n",
    );
    expect(tfExecution.runArtifacts[0]?.analysis).toBe("tf");
    expect(tfExecution.runArtifacts[0]?.sourceName).toBe("V1");
    expect(tfExecution.runArtifacts[0]?.outputNode).toBe("mid");
    expect(tfExecution.runArtifacts[0]?.resultRows).toBe(1);
    expect(tfExecution.runArtifacts[0]?.resultColumnCount).toBe(3);
    expect(tfExecution.runArtifacts[0]?.resultColumns).toEqual([
      "TransferRatio",
      "InputImpedance",
      "OutputImpedance",
    ]);
    expect(tfExecution.runArtifacts[0]?.tableCount).toBe(3);
    expect(tfExecution.runArtifacts[0]?.tables).toEqual(["result", "output-plan", "run-artifact"]);
    expect(tfExecution.runArtifacts[0]?.stepTime).toBeUndefined();
    expect(tfExecution.runArtifacts[0]?.useInitialConditions).toBeUndefined();
    expect(tfExecution.runArtifacts[0]?.outputProbes).toEqual(["V(mid)"]);
    expect(tfExecution.runArtifacts[0]?.outputDirectives).toEqual([]);
    expect(tfExecution.runArtifacts[0]?.measurementNames).toEqual([]);
    expect(tfExecution.runArtifacts[0]?.fourierProbes).toEqual([]);
    const tfRunArtifactRecord = expectRunArtifactTableMatches(tfExecution);
    expect(tfRunArtifactRecord.Analysis).toBe("tf");
    expect(tfRunArtifactRecord.DeckAnalysisKinds).toBe("7");
    expect(tfRunArtifactRecord.DeckAnalysisKindList).toBe("op;dc;ac;tran;tf;sens;noise");

    const sensExecution = runDeckAnalysis(circuit, netlist, "sens");
    expect(sensExecution.plan.outputNode).toBe("mid");
    expect(sensExecution.plan.sourceName).toBeUndefined();
    const sensResult = sensExecution.result as {
      readonly outputNode: string;
      readonly entries: readonly unknown[];
    };
    expect(sensResult.outputNode).toBe("mid");
    expect(sensResult.entries).toHaveLength(3);
    expect(sensExecution.outputProbes).toEqual(["V(mid)"]);
    expect(sensExecution.outputPlanArtifacts[0]?.outputNode).toBe("mid");
    expect(sensExecution.outputPlanArtifactRecords[0]?.OutputNode).toBe("mid");
    expect(sensExecution.analysisDirectives).toEqual([".sens"]);
    expect(sensExecution.tableCount).toBe(3);
    expect(sensExecution.tables).toEqual(["result", "output-plan", "run-artifact"]);
    expect(sensExecution.measurements).toEqual([]);
    expect(sensExecution.measurementTable).toBe("Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n");
    expect(sensExecution.table.startsWith(
      "OutputNode\tNominalVoltage\tElement\tParameter\tNominalValue\tSensitivity\tRelativeSensitivity\n",
    )).toBe(true);
    expect(sensExecution.runArtifacts[0]?.analysis).toBe("sens");
    expect(sensExecution.runArtifacts[0]?.sourceName).toBeUndefined();
    expect(sensExecution.runArtifacts[0]?.outputNode).toBe("mid");
    expect(sensExecution.runArtifacts[0]?.resultRows).toBe(1);
    expect(sensExecution.runArtifacts[0]?.resultColumnCount).toBe(7);
    expect(sensExecution.runArtifacts[0]?.resultColumns).toEqual([
      "OutputNode",
      "NominalVoltage",
      "Element",
      "Parameter",
      "NominalValue",
      "Sensitivity",
      "RelativeSensitivity",
    ]);
    expect(sensExecution.runArtifacts[0]?.tableCount).toBe(3);
    expect(sensExecution.runArtifacts[0]?.tables).toEqual(["result", "output-plan", "run-artifact"]);
    expect(sensExecution.runArtifacts[0]?.stepTime).toBeUndefined();
    expect(sensExecution.runArtifacts[0]?.useInitialConditions).toBeUndefined();
    expect(sensExecution.runArtifacts[0]?.outputProbes).toEqual(["V(mid)"]);
    expect(sensExecution.runArtifacts[0]?.outputDirectives).toEqual([]);
    expect(sensExecution.runArtifacts[0]?.measurementNames).toEqual([]);
    expect(sensExecution.runArtifacts[0]?.fourierProbes).toEqual([]);
    const sensRunArtifactRecord = expectRunArtifactTableMatches(sensExecution);
    expect(sensRunArtifactRecord.Analysis).toBe("sens");
    expect(sensRunArtifactRecord.DeckAnalysisKinds).toBe("7");
    expect(sensRunArtifactRecord.DeckAnalysisKindList).toBe("op;dc;ac;tran;tf;sens;noise");

    const noiseExecution = runDeckAnalysis(circuit, netlist, "noise");
    expect(noiseExecution.plan.outputNode).toBe("mid");
    expect(noiseExecution.plan.sourceName).toBe("V1");
    expect(noiseExecution.plan.sweepKind).toBe("lin");
    expect(noiseExecution.plan.pointCount).toBe(1);
    expect(noiseExecution.plan.startFrequencyHz).toBeCloseTo(1.0e3, 9);
    expect(noiseExecution.plan.stopFrequencyHz).toBeCloseTo(1.0e3, 9);
    const noiseResult = noiseExecution.result as NoiseResult;
    expect(noiseResult.outputNode).toBe("mid");
    expect(noiseResult.inputSource).toBe("V1");
    expect(noiseResult.points).toHaveLength(1);
    expect(noiseExecution.outputProbes).toEqual(["V(mid)"]);
    expect(noiseExecution.outputPlanArtifacts[0]?.sourceName).toBe("V1");
    expect(noiseExecution.outputPlanArtifacts[0]?.outputNode).toBe("mid");
    expect(noiseExecution.outputPlanArtifacts[0]?.sweepKind).toBe("lin");
    expect(noiseExecution.outputPlanArtifacts[0]?.pointCount).toBe(1);
    expect(noiseExecution.outputPlanArtifacts[0]?.startFrequencyHz).toBeCloseTo(1.0e3, 9);
    expect(noiseExecution.outputPlanArtifacts[0]?.stopFrequencyHz).toBeCloseTo(1.0e3, 9);
    expect(noiseExecution.outputPlanArtifactRecords[0]?.OutputNode).toBe("mid");
    expect(noiseExecution.outputPlanArtifactRecords[0]?.SweepKind).toBe("lin");
    expect(noiseExecution.outputPlanArtifactRecords[0]?.PointCount).toBe("1");
    expect(noiseExecution.outputPlanArtifactRecords[0]?.StartFrequencyHz).toBe("1.000000e+03");
    expect(noiseExecution.outputPlanArtifactRecords[0]?.StopFrequencyHz).toBe("1.000000e+03");
    expect(noiseExecution.analysisDirectives).toEqual([".noise"]);
    expect(noiseExecution.tableCount).toBe(3);
    expect(noiseExecution.tables).toEqual(["result", "output-plan", "run-artifact"]);
    expect(noiseExecution.measurements).toEqual([]);
    expect(noiseExecution.measurementTable).toBe("Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n");
    expect(noiseExecution.table).toBe(formatDeckNoiseTable(noiseResult));
    expect(noiseExecution.table.startsWith(
      "Index\tFrequency\tOutputNode\tInputSource\tOutputPSD\tInputReferredPSD\tElement\tType\tSourcePSD\tContributionPSD\n",
    )).toBe(true);
    expect(noiseExecution.runArtifacts[0]?.analysis).toBe("noise");
    expect(noiseExecution.runArtifacts[0]?.sourceName).toBe("V1");
    expect(noiseExecution.runArtifacts[0]?.outputNode).toBe("mid");
    expect(noiseExecution.runArtifacts[0]?.sweepKind).toBe("lin");
    expect(noiseExecution.runArtifacts[0]?.pointCount).toBe(1);
    expect(noiseExecution.runArtifacts[0]?.startFrequencyHz).toBeCloseTo(1.0e3, 9);
    expect(noiseExecution.runArtifacts[0]?.stopFrequencyHz).toBeCloseTo(1.0e3, 9);
    expect(noiseExecution.runArtifacts[0]?.resultRows).toBe(1);
    expect(noiseExecution.runArtifacts[0]?.resultColumnCount).toBe(10);
    expect(noiseExecution.runArtifacts[0]?.resultColumns).toEqual([
      "Index",
      "Frequency",
      "OutputNode",
      "InputSource",
      "OutputPSD",
      "InputReferredPSD",
      "Element",
      "Type",
      "SourcePSD",
      "ContributionPSD",
    ]);
    expect(noiseExecution.runArtifacts[0]?.tableCount).toBe(3);
    expect(noiseExecution.runArtifacts[0]?.tables).toEqual(["result", "output-plan", "run-artifact"]);
    expect(noiseExecution.runArtifacts[0]?.stepTime).toBeUndefined();
    expect(noiseExecution.runArtifacts[0]?.useInitialConditions).toBeUndefined();
    expect(noiseExecution.runArtifacts[0]?.outputProbes).toEqual(["V(mid)"]);
    expect(noiseExecution.runArtifacts[0]?.outputDirectives).toEqual([]);
    expect(noiseExecution.runArtifacts[0]?.measurementNames).toEqual([]);
    expect(noiseExecution.runArtifacts[0]?.fourierProbes).toEqual([]);
    const noiseRunArtifactRecord = expectRunArtifactTableMatches(noiseExecution);
    expect(noiseRunArtifactRecord.Analysis).toBe("noise");
    expect(noiseRunArtifactRecord.DeckAnalysisKinds).toBe("7");
    expect(noiseRunArtifactRecord.DeckAnalysisKindList).toBe("op;dc;ac;tran;tf;sens;noise");

    const tranWindowExecution = runDeckAnalysis(
      circuit,
      ".save V(mid)\n.tran 2m 6m 2m 1m uic\n.end\n",
    );
    expect(tranWindowExecution.plan.startTime).toBeCloseTo(2.0e-3, 12);
    expect(tranWindowExecution.plan.maxStep).toBeCloseTo(1.0e-3, 12);
    expect(tranWindowExecution.plan.useInitialConditions).toBe(true);
    expect(tranWindowExecution.runArtifacts[0]?.stepTime).toBeCloseTo(2.0e-3, 12);
    expect(tranWindowExecution.runArtifacts[0]?.stopTime).toBeCloseTo(6.0e-3, 12);
    expect(tranWindowExecution.runArtifacts[0]?.startTime).toBeCloseTo(2.0e-3, 12);
    expect(tranWindowExecution.runArtifacts[0]?.maxStep).toBeCloseTo(1.0e-3, 12);
    expect(tranWindowExecution.runArtifacts[0]?.useInitialConditions).toBe(true);
    expect(tranWindowExecution.runArtifacts[0]?.resultColumnCount).toBe(3);
    expect(tranWindowExecution.runArtifacts[0]?.resultColumns).toEqual(["Index", "Time", "V(mid)"]);
    expect(tranWindowExecution.runArtifacts[0]?.tableCount).toBe(3);
    expect(tranWindowExecution.runArtifacts[0]?.tables).toEqual([
      "result",
      "output-plan",
      "run-artifact",
    ]);
    expect(tranWindowExecution.tableCount).toBe(3);
    expect(tranWindowExecution.tables).toEqual(["result", "output-plan", "run-artifact"]);
    expect(tranWindowExecution.outputProbes).toEqual(["V(mid)"]);
    const tranWindowPoints = tranWindowExecution.result as { time: number }[];
    expect(tranWindowPoints).toHaveLength(3);
    [
      2.0e-3,
      4.0e-3,
      6.0e-3,
    ].forEach((expectedTime, index) => {
      expect(tranWindowPoints[index]?.time).toBeCloseTo(expectedTime, 12);
    });
    expect(tranWindowExecution.table).toBe(
      "Index\tTime\tV(mid)\n" +
        "0\t2.000000e-03\t5.000000e-01\n" +
        "1\t4.000000e-03\t5.000000e-01\n" +
        "2\t6.000000e-03\t5.000000e-01\n",
    );
    const tranWindowRunArtifactRecord = expectRunArtifactTableMatches(tranWindowExecution);
    expect(tranWindowRunArtifactRecord.Analysis).toBe("tran");
    expect(tranWindowRunArtifactRecord.DeckAnalysisKinds).toBe("1");
    expect(tranWindowRunArtifactRecord.DeckAnalysisKindList).toBe("tran");

    expect(() => runDeckAnalysis(circuit, netlist)).toThrow(/multiple analysis cards/);

    const linExecution = runDeckAnalysis(circuit, ".save V(mid)\n.ac lin 3 1 3\n.end\n");
    expect(linExecution.outputProbes).toEqual(["V(mid)"]);
    const linPoints = linExecution.result as { frequencyHz: number }[];
    expect(linPoints.map((point) => point.frequencyHz)).toEqual([1.0, 2.0, 3.0]);
    expect(linExecution.table).toBe(
      "Index\tFrequency\tProbe\tReal\tImaginary\tMagnitude\tPhase\n" +
        "0\t1.000000e+00\tV(mid)\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\n" +
        "1\t2.000000e+00\tV(mid)\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\n" +
        "2\t3.000000e+00\tV(mid)\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\n",
    );

    const octExecution = runDeckAnalysis(circuit, ".save V(mid)\n.ac oct 1 1 4\n.end\n");
    expect(octExecution.outputProbes).toEqual(["V(mid)"]);
    const octPoints = octExecution.result as { frequencyHz: number }[];
    expect(octPoints.map((point) => point.frequencyHz)).toEqual([1.0, 2.0, 4.0]);
    expect(octExecution.table).toBe(
      "Index\tFrequency\tProbe\tReal\tImaginary\tMagnitude\tPhase\n" +
        "0\t1.000000e+00\tV(mid)\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\n" +
        "1\t2.000000e+00\tV(mid)\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\n" +
        "2\t4.000000e+00\tV(mid)\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\n",
    );
  });

  it("executes every deck analysis card in source order", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "in", "0", 1.0));
    circuit.add(resistor("R1", "in", "0", 1_000.0));
    const netlist = ".save V(in)\n.op\n.dc V1 0 1 1\n.op\n.end\n";

    expect(() => runDeckAnalysis(circuit, netlist)).toThrow(/multiple analysis cards/);

    const execution = runDeck(circuit, netlist);

    expect(execution.executionCount).toBe(3);
    expect(execution.analysisOrder).toEqual(["op", "dc", "op"]);
    expect(execution.analysisDirectives).toEqual([".op", ".dc", ".op"]);
    expect(execution.executions.map((item) => item.plan.analysis)).toEqual(["op", "dc", "op"]);
    expect(execution.runArtifactCount).toBe(3);
    expect(execution.runArtifacts.map((artifact) => artifact.analysis)).toEqual([
      "op",
      "dc",
      "op",
    ]);
    expect(execution.runArtifactRecords).toEqual(deckTableRecords(execution.runArtifactTable));
    expect(execution.runArtifactRecords[1]?.Analysis).toBe("dc");
    expect(execution.runArtifactRecords[1]?.DeckAnalysisKinds).toBe("2");
    expect(execution.runArtifactRecords[1]?.DeckAnalysisKindList).toBe("op;dc");
    expect(execution.runArtifactRecords[1]?.DeckAnalysisDirectives).toBe("3");
    expect(execution.runArtifactRecords[1]?.DeckAnalysisDirectiveList).toBe(".op;.dc;.op");
  });

  it("surfaces control diagnostics in selected run artifacts", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "in", "0", 1.0));
    circuit.add(resistor("R1", "in", "0", 1_000.0));
    const netlist = `
.save V(in)
.control
save V(in)
probe V(in)
set filetype=ascii
set wr_vecnames
set wr_singlescale
set appendwrite
.set WR_VECNAMES
write out.raw V(in) V(missing)
wrdata out.dat V(in) V(missing)
source other.cir
cd /tmp
if v(in) > 0
let gain = 2
.endc
.op
.end
`;

    const execution = runDeckAnalysis(circuit, netlist, "op");
    const expectedCodes = [
      "SPICE_DECK_CONTROL_SCRIPT_COMMAND",
      "SPICE_DECK_CONTROL_WORKDIR_COMMAND",
      "SPICE_DECK_CONTROL_FLOW_COMMAND",
      "SPICE_DECK_CONTROL_VARIABLE_COMMAND",
    ];
    const codeList = expectedCodes.join(";");
    const expectedControlLines = [".save V(in)", ".probe V(in)"];
    const controlLineList = expectedControlLines.join(";");
    const expectedWriteMarkers = [
      "write out.raw V(in) V(missing)",
      "wrdata out.dat V(in) V(missing)",
    ];
    const writeMarkerList = expectedWriteMarkers.join(";");
    const expectedRawfileOptions = [
      "set filetype=ascii",
      "set wr_vecnames",
      "set wr_singlescale",
      "set appendwrite",
      "set wr_vecnames",
    ];
    const rawfileOptionList = expectedRawfileOptions.join(";");
    const expectedPolicyLines = [13, 14, 15, 16];
    const expectedPolicyCategories = ["script", "workdir", "control-flow", "variable"];
    const expectedPolicyCommands = [
      "source other.cir",
      "cd /tmp",
      "if v(in) > 0",
      "let gain = 2",
    ];
    const expectedTableNames = [
      "result",
      "control-policy",
      "control-policy-summary",
      "output-plan",
      "run-artifact",
    ];

    expect(execution.controlLineCount).toBe(expectedControlLines.length);
    expect(execution.controlLines).toEqual(expectedControlLines);
    expect(execution.writeMarkerCount).toBe(expectedWriteMarkers.length);
    expect(execution.writeMarkers).toEqual(expectedWriteMarkers);
    expect(execution.rawfileOptionCount).toBe(expectedRawfileOptions.length);
    expect(execution.rawfileOptions).toEqual(expectedRawfileOptions);
    expect(execution.rawfileArtifactCount).toBe(1);
    expect(execution.rawfileArtifacts[0]?.target).toBe("out.raw");
    expect(execution.rawfileArtifacts[0]?.marker).toBe("write out.raw V(in) V(missing)");
    expect(execution.rawfileArtifacts[0]?.probeCount).toBe(2);
    expect(execution.rawfileArtifacts[0]?.probes).toEqual(["V(in)", "V(missing)"]);
    expect(execution.rawfileArtifacts[0]?.matchedProbeCount).toBe(1);
    expect(execution.rawfileArtifacts[0]?.matchedProbes).toEqual(["V(in)"]);
    expect(execution.rawfileArtifacts[0]?.unmatchedProbeCount).toBe(1);
    expect(execution.rawfileArtifacts[0]?.unmatchedProbes).toEqual(["V(missing)"]);
    expect(execution.rawfileArtifacts[0]?.optionCount).toBe(expectedRawfileOptions.length);
    expect(execution.rawfileArtifacts[0]?.options).toEqual(expectedRawfileOptions);
    expect(execution.rawfileArtifacts[0]?.rawfile).toContain("Title: SPICE deck op result\n");
    expect(execution.rawfileArtifacts[0]?.rawfile).toContain("No. Variables: 2\n");
    expect(execution.rawfileArtifacts[0]?.rawfile).toContain(`Options: ${rawfileOptionList}\n`);
    expect(execution.rawfileArtifacts[0]?.rawfile).toContain("0\t0\t1.000000e+00\n");
    const rawfileRecord = execution.rawfileArtifactRecords[0]!;
    expect(rawfileRecord["Target"]).toBe("out.raw");
    expect(rawfileRecord["Marker"]).toBe("write out.raw V(in) V(missing)");
    expect(rawfileRecord["Probes"]).toBe("2");
    expect(rawfileRecord["ProbeList"]).toBe("V(in);V(missing)");
    expect(rawfileRecord["MatchedProbes"]).toBe("1");
    expect(rawfileRecord["MatchedProbeList"]).toBe("V(in)");
    expect(rawfileRecord["UnmatchedProbes"]).toBe("1");
    expect(rawfileRecord["UnmatchedProbeList"]).toBe("V(missing)");
    expect(rawfileRecord["Options"]).toBe(String(expectedRawfileOptions.length));
    expect(rawfileRecord["RawfileOptionList"]).toBe(rawfileOptionList);
    expect(rawfileRecord["Bytes"]).toBe(String(execution.rawfileArtifacts[0]?.rawfile.length));
    expect(execution.rawfileArtifactTable).toBe(
      formatDeckRawfileArtifactTable(execution.rawfileArtifacts),
    );
    expect(execution.rawfileArtifactCsv).toBe(
      formatDeckRawfileArtifactCsv(execution.rawfileArtifacts),
    );
    expect(execution.rawfileArtifactJson).toBe(
      formatDeckRawfileArtifactJson(execution.rawfileArtifacts),
    );
    expect(JSON.parse(execution.rawfileArtifactJson)[0].RawfileOptionList).toBe(
      rawfileOptionList,
    );
    const rawfileJson = JSON.parse(execution.rawfileArtifactJson)[0];
    expect(rawfileJson.ProbeList).toBe("V(in);V(missing)");
    expect(rawfileJson.MatchedProbeList).toBe("V(in)");
    expect(rawfileJson.UnmatchedProbeList).toBe("V(missing)");
    expect(execution.wrdataArtifactCount).toBe(1);
    expect(execution.wrdataArtifacts[0]?.target).toBe("out.dat");
    expect(execution.wrdataArtifacts[0]?.marker).toBe(
      "wrdata out.dat V(in) V(missing)",
    );
    expect(execution.wrdataArtifacts[0]?.probeCount).toBe(2);
    expect(execution.wrdataArtifacts[0]?.probes).toEqual(["V(in)", "V(missing)"]);
    expect(execution.wrdataArtifacts[0]?.matchedProbeCount).toBe(1);
    expect(execution.wrdataArtifacts[0]?.matchedProbes).toEqual(["V(in)"]);
    expect(execution.wrdataArtifacts[0]?.unmatchedProbeCount).toBe(1);
    expect(execution.wrdataArtifacts[0]?.unmatchedProbes).toEqual(["V(missing)"]);
    expect(execution.wrdataArtifacts[0]?.optionCount).toBe(expectedRawfileOptions.length);
    expect(execution.wrdataArtifacts[0]?.options).toEqual(expectedRawfileOptions);
    expect(execution.wrdataArtifacts[0]?.datafile).toContain(
      "# SPICE deck wrdata artifact\n",
    );
    expect(execution.wrdataArtifacts[0]?.datafile).toContain(
      "Probes: V(in);V(missing)\n",
    );
    expect(execution.wrdataArtifacts[0]?.datafile).toContain(
      `Options: ${rawfileOptionList}\n`,
    );
    expect(execution.wrdataArtifacts[0]?.datafile).toContain(
      "VectorNames: Index;V(in)\n",
    );
    expect(execution.wrdataArtifacts[0]?.datafile).toContain("Scale: Index\n");
    expect(execution.wrdataArtifacts[0]?.datafile).toContain("Index\tV(in)\n");
    expect(execution.wrdataArtifacts[0]?.datafile).toContain("0\t1.000000e+00\n");
    const wrdataRecord = execution.wrdataArtifactRecords[0]!;
    expect(wrdataRecord["Target"]).toBe("out.dat");
    expect(wrdataRecord["Marker"]).toBe("wrdata out.dat V(in) V(missing)");
    expect(wrdataRecord["Probes"]).toBe("2");
    expect(wrdataRecord["ProbeList"]).toBe("V(in);V(missing)");
    expect(wrdataRecord["MatchedProbes"]).toBe("1");
    expect(wrdataRecord["MatchedProbeList"]).toBe("V(in)");
    expect(wrdataRecord["UnmatchedProbes"]).toBe("1");
    expect(wrdataRecord["UnmatchedProbeList"]).toBe("V(missing)");
    expect(wrdataRecord["Options"]).toBe(String(expectedRawfileOptions.length));
    expect(wrdataRecord["RawfileOptionList"]).toBe(rawfileOptionList);
    expect(wrdataRecord["Bytes"]).toBe(String(execution.wrdataArtifacts[0]?.datafile.length));
    expect(execution.wrdataArtifactTable).toBe(
      formatDeckWrdataArtifactTable(execution.wrdataArtifacts),
    );
    expect(execution.wrdataArtifactCsv).toBe(
      formatDeckWrdataArtifactCsv(execution.wrdataArtifacts),
    );
    expect(execution.wrdataArtifactJson).toBe(
      formatDeckWrdataArtifactJson(execution.wrdataArtifacts),
    );
    const wrdataJson = JSON.parse(execution.wrdataArtifactJson)[0];
    expect(wrdataJson.ProbeList).toBe("V(in);V(missing)");
    expect(wrdataJson.MatchedProbeList).toBe("V(in)");
    expect(wrdataJson.UnmatchedProbeList).toBe("V(missing)");
    expect(wrdataJson.RawfileOptionList).toBe(rawfileOptionList);
    expect(execution.controlPolicyArtifactCount).toBe(expectedCodes.length);
    expect(execution.controlPolicyArtifacts.map((artifact) => artifact.lineNumber)).toEqual(
      expectedPolicyLines,
    );
    expect(execution.controlPolicyArtifacts.map((artifact) => artifact.category)).toEqual(
      expectedPolicyCategories,
    );
    expect(execution.controlPolicyArtifacts.map((artifact) => artifact.command)).toEqual(
      expectedPolicyCommands,
    );
    expect(execution.controlPolicyArtifacts.map((artifact) => artifact.code)).toEqual(
      expectedCodes,
    );
    expect(execution.controlPolicyArtifacts.map((artifact) => artifact.severity)).toEqual(
      Array(expectedCodes.length).fill("error"),
    );
    expect(execution.controlPolicyArtifacts[0]?.message).toContain(
      "external script and shell commands are disabled",
    );
    const policyRecord = execution.controlPolicyArtifactRecords[0]!;
    expect(policyRecord["Line"]).toBe("13");
    expect(policyRecord["Category"]).toBe("script");
    expect(policyRecord["Command"]).toBe("source other.cir");
    expect(policyRecord["Code"]).toBe("SPICE_DECK_CONTROL_SCRIPT_COMMAND");
    expect(policyRecord["Severity"]).toBe("error");
    expect(execution.controlPolicyArtifactTable).toBe(
      formatDeckControlPolicyArtifactTable(execution.controlPolicyArtifacts),
    );
    expect(execution.controlPolicyArtifactCsv).toBe(
      formatDeckControlPolicyArtifactCsv(execution.controlPolicyArtifacts),
    );
    expect(execution.controlPolicyArtifactJson).toBe(
      formatDeckControlPolicyArtifactJson(execution.controlPolicyArtifacts),
    );
    const policyJson = JSON.parse(execution.controlPolicyArtifactJson);
    expect(policyJson[2].Category).toBe("control-flow");
    expect(policyJson[3].Command).toBe("let gain = 2");
    expect(execution.controlPolicySummaryArtifactCount).toBe(expectedPolicyCategories.length);
    expect(execution.controlPolicySummaryArtifacts.map((artifact) => artifact.category)).toEqual(
      expectedPolicyCategories,
    );
    expect(execution.controlPolicySummaryArtifacts.map((artifact) => artifact.artifactCount)).toEqual(
      [1, 1, 1, 1],
    );
    expect(execution.controlPolicySummaryArtifacts.map((artifact) => artifact.lineNumbers)).toEqual(
      expectedPolicyLines.map((lineNumber) => [lineNumber]),
    );
    expect(execution.controlPolicySummaryArtifacts.map((artifact) => artifact.commands)).toEqual(
      expectedPolicyCommands.map((command) => [command]),
    );
    expect(execution.controlPolicySummaryArtifacts.map((artifact) => artifact.codes)).toEqual(
      expectedCodes.map((code) => [code]),
    );
    const summaryRecord = execution.controlPolicySummaryArtifactRecords[0]!;
    expect(summaryRecord["Category"]).toBe("script");
    expect(summaryRecord["Artifacts"]).toBe("1");
    expect(summaryRecord["LineList"]).toBe("13");
    expect(summaryRecord["CommandList"]).toBe("source other.cir");
    expect(summaryRecord["CodeList"]).toBe("SPICE_DECK_CONTROL_SCRIPT_COMMAND");
    expect(summaryRecord["SeverityList"]).toBe("error");
    expect(execution.controlPolicySummaryArtifactTable).toBe(
      formatDeckControlPolicySummaryArtifactTable(execution.controlPolicySummaryArtifacts),
    );
    expect(execution.controlPolicySummaryArtifactCsv).toBe(
      formatDeckControlPolicySummaryArtifactCsv(execution.controlPolicySummaryArtifacts),
    );
    expect(execution.controlPolicySummaryArtifactJson).toBe(
      formatDeckControlPolicySummaryArtifactJson(execution.controlPolicySummaryArtifacts),
    );
    const summaryJson = JSON.parse(execution.controlPolicySummaryArtifactJson);
    expect(summaryJson[2].Category).toBe("control-flow");
    expect(summaryJson[3].CommandList).toBe("let gain = 2");
    expect(execution.diagnosticCount).toBe(expectedCodes.length);
    expect(execution.diagnosticCodes).toEqual(expectedCodes);
    expect(execution.tableCount).toBe(expectedTableNames.length);
    expect(execution.tables).toEqual(expectedTableNames);
    expect(execution.tableArtifacts.map((artifact) => artifact.name)).toEqual(
      expectedTableNames,
    );
    const policyTableArtifact = execution.tableArtifacts.at(-4)!;
    expect(policyTableArtifact.name).toBe("control-policy");
    expect(policyTableArtifact.table).toBe(execution.controlPolicyArtifactTable);
    expect(policyTableArtifact.csv).toBe(execution.controlPolicyArtifactCsv);
    expect(policyTableArtifact.json).toBe(execution.controlPolicyArtifactJson);
    expect(policyTableArtifact.records).toEqual(execution.controlPolicyArtifactRecords);
    const summaryTableArtifact = execution.tableArtifacts.at(-3)!;
    expect(summaryTableArtifact.name).toBe("control-policy-summary");
    expect(summaryTableArtifact.table).toBe(execution.controlPolicySummaryArtifactTable);
    expect(summaryTableArtifact.csv).toBe(execution.controlPolicySummaryArtifactCsv);
    expect(summaryTableArtifact.json).toBe(execution.controlPolicySummaryArtifactJson);
    expect(summaryTableArtifact.records).toEqual(
      execution.controlPolicySummaryArtifactRecords,
    );
    const outputPlanTableArtifact = execution.tableArtifacts.at(-2)!;
    expect(outputPlanTableArtifact.name).toBe("output-plan");
    expect(outputPlanTableArtifact.table).toBe(execution.outputPlanArtifactTable);
    expect(outputPlanTableArtifact.csv).toBe(execution.outputPlanArtifactCsv);
    expect(outputPlanTableArtifact.json).toBe(execution.outputPlanArtifactJson);
    expect(outputPlanTableArtifact.records).toEqual(execution.outputPlanArtifactRecords);
    expect(execution.runArtifacts[0]?.controlLineCount).toBe(expectedControlLines.length);
    expect(execution.runArtifacts[0]?.controlLines).toEqual(expectedControlLines);
    expect(execution.runArtifacts[0]?.writeMarkerCount).toBe(expectedWriteMarkers.length);
    expect(execution.runArtifacts[0]?.writeMarkers).toEqual(expectedWriteMarkers);
    expect(execution.runArtifacts[0]?.rawfileOptionCount).toBe(expectedRawfileOptions.length);
    expect(execution.runArtifacts[0]?.rawfileOptions).toEqual(expectedRawfileOptions);
    expect(execution.runArtifacts[0]?.controlPolicyArtifactCount).toBe(expectedCodes.length);
    expect(execution.runArtifacts[0]?.controlPolicyCategories).toEqual(
      expectedPolicyCategories,
    );
    expect(execution.runArtifacts[0]?.controlPolicyCodes).toEqual(expectedCodes);
    expect(execution.runArtifacts[0]?.controlPolicySeverities).toEqual(["error"]);
    expect(execution.runArtifacts[0]?.tableCount).toBe(expectedTableNames.length);
    expect(execution.runArtifacts[0]?.tables).toEqual(expectedTableNames);
    expect(execution.runArtifacts[0]?.diagnosticCount).toBe(expectedCodes.length);
    expect(execution.runArtifacts[0]?.diagnosticCodes).toEqual(expectedCodes);
    const record = deckTableRecords(execution.runArtifactTable)[0]!;
    expect(record["Tables"]).toBe(String(expectedTableNames.length));
    expect(record["TableList"]).toBe(expectedTableNames.join(";"));
    expect(record["ControlLines"]).toBe(String(expectedControlLines.length));
    expect(record["ControlLineList"]).toBe(controlLineList);
    expect(record["WriteMarkers"]).toBe(String(expectedWriteMarkers.length));
    expect(record["WriteMarkerList"]).toBe(writeMarkerList);
    expect(record["RawfileOptions"]).toBe(String(expectedRawfileOptions.length));
    expect(record["RawfileOptionList"]).toBe(rawfileOptionList);
    expect(record["ControlPolicyArtifacts"]).toBe(String(expectedCodes.length));
    expect(record["ControlPolicyCategoryList"]).toBe(expectedPolicyCategories.join(";"));
    expect(record["ControlPolicyCodeList"]).toBe(codeList);
    expect(record["ControlPolicySeverityList"]).toBe("error");
    expect(record["Diagnostics"]).toBe(String(expectedCodes.length));
    expect(record["DiagnosticCodeList"]).toBe(codeList);
    const runArtifact = execution.tableArtifacts[execution.tableArtifacts.length - 1]!;
    expect(runArtifact.name).toBe("run-artifact");
    expect(runArtifact.records[0]?.["ControlLineList"]).toBe(controlLineList);
    expect(runArtifact.records[0]?.["WriteMarkerList"]).toBe(writeMarkerList);
    expect(runArtifact.records[0]?.["RawfileOptionList"]).toBe(rawfileOptionList);
    expect(runArtifact.records[0]?.["ControlPolicyCategoryList"]).toBe(
      expectedPolicyCategories.join(";"),
    );
    expect(runArtifact.records[0]?.["ControlPolicyCodeList"]).toBe(codeList);
    expect(runArtifact.records[0]?.["ControlPolicySeverityList"]).toBe("error");
    expect(runArtifact.records[0]?.["DiagnosticCodeList"]).toBe(codeList);
    expect(runArtifact.csv).toBe(formatDeckRunArtifactCsv(execution.runArtifacts));
    expect(runArtifact.json).toBe(formatDeckRunArtifactJson(execution.runArtifacts));
  });

  it("exposes selected Fourier artifacts from deck transient execution", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "vin", "0", 1.0));
    circuit.add(resistor("R1", "vin", "mid", 1_000.0));
    circuit.add(resistor("R2", "mid", "0", 1_000.0));
    const netlist = `
.save V(mid)
.op
.tran 0.5m 1m
.four 2k V(mid) harmonics=1
.end
`;

    const opExecution = runDeckAnalysis(circuit, netlist, "op");
    expect(opExecution.fourier).toEqual([]);
    expect(opExecution.fourierTable).toBe("");
    expect(opExecution.tableCount).toBe(3);
    expect(opExecution.tables).toEqual(["result", "output-plan", "run-artifact"]);

    const tranExecution = runDeckAnalysis(circuit, netlist, "tran");
    expect(tranExecution.fourier).toHaveLength(1);
    expect(tranExecution.tableCount).toBe(4);
    expect(tranExecution.tables).toEqual(["result", "fourier", "output-plan", "run-artifact"]);
    expect(tranExecution.tableArtifacts.map((artifact) => artifact.name)).toEqual(
      tranExecution.tables,
    );
    const result = tranExecution.fourier[0]!;
    expect(result.fundamentalFrequencyHz).toBeCloseTo(2_000.0, 12);
    expect(result.probes[0]?.probe).toBe("V(mid)");
    expect(result.probes[0]?.harmonics).toHaveLength(1);
    expect(tranExecution.fourierTable).toBe(formatFourierTable(result));
    expect(tranExecution.tableArtifacts[1]).toMatchObject({
      name: "fourier",
      table: tranExecution.fourierTable,
      csv: formatDeckTableCsv(tranExecution.fourierTable),
      json: formatDeckTableJson(tranExecution.fourierTable),
      records: deckTableRecords(tranExecution.fourierTable),
    });
    expect(tranExecution.runArtifacts[0]?.fourierCount).toBe(1);
    expect(tranExecution.runArtifacts[0]?.sourceName).toBeUndefined();
    expect(tranExecution.runArtifacts[0]?.outputNode).toBeUndefined();
    expect(tranExecution.runArtifacts[0]?.stepTime).toBeCloseTo(5.0e-4, 12);
    expect(tranExecution.runArtifacts[0]?.stopTime).toBeCloseTo(1.0e-3, 12);
    expect(tranExecution.runArtifacts[0]?.resultColumnCount).toBe(3);
    expect(tranExecution.runArtifacts[0]?.resultColumns).toEqual(["Index", "Time", "V(mid)"]);
    expect(tranExecution.runArtifacts[0]?.tableCount).toBe(4);
    expect(tranExecution.runArtifacts[0]?.tables).toEqual([
      "result",
      "fourier",
      "output-plan",
      "run-artifact",
    ]);
    expect(tranExecution.runArtifacts[0]?.startTime).toBeUndefined();
    expect(tranExecution.runArtifacts[0]?.maxStep).toBeUndefined();
    expect(tranExecution.runArtifacts[0]?.useInitialConditions).toBe(false);
    expect(tranExecution.runArtifacts[0]?.outputProbes).toEqual(["V(mid)"]);
    expect(tranExecution.runArtifacts[0]?.outputDirectives).toEqual([".save"]);
    expect(tranExecution.runArtifacts[0]?.measurementNames).toEqual([]);
    expect(tranExecution.runArtifacts[0]?.fourierProbes).toEqual(["V(mid)"]);
    const tranRunArtifactRecord = expectRunArtifactTableMatches(tranExecution);
    expect(tranRunArtifactRecord.Analysis).toBe("tran");
    expect(tranRunArtifactRecord.Fourier).toBe("1");
    expect(tranRunArtifactRecord.FourierList).toBe("V(mid)");
    expect(tranRunArtifactRecord.DeckAnalysisKindList).toBe("op;tran");
  });

  it("formats stable transient probe measurements", () => {
    const points = [
      transientPoint(0.0, { in: 0.0, out: 0.0 }),
      transientPoint(1.0e-3, { in: 1.0, out: 1.25 }),
      transientPoint(2.0e-3, { in: 1.0, out: -0.25 }),
      transientPoint(3.0e-3, { in: 1.0, out: 0.75 }),
    ];

    const peakToPeak = measureTransientProbe(
      points,
      "swing",
      "V(out)",
      "peak-to-peak",
      1.0e-3,
      3.0e-3,
    );
    const finalValue = measureTransientProbe(points, "settled", "V(out)", "final");
    const midpoint = measureTransientFindAtProbe(points, "midpoint", "V(out)", 1.5e-3);
    const crossing = measureTransientWhenProbe(
      points,
      "crossing",
      "V(out)",
      0.5,
      1.0e-3,
      3.0e-3,
    );
    const secondCrossing = measureTransientWhenProbeCounted(
      points,
      "second_crossing",
      "V(out)",
      0.5,
      "cross",
      2,
      1.0e-3,
      3.0e-3,
    );
    const propagationDelay = measureTransientDelayBetweenProbes(
      points,
      "prop_delay",
      "V(in)",
      0.5,
      "rise",
      1,
      "V(out)",
      0.5,
      "fall",
      1,
      0.0,
      3.0e-3,
    );

    expect(peakToPeak.value).toBeCloseTo(1.5, 9);
    expect(peakToPeak.mode).toBe("pp");
    expect(finalValue.value).toBeCloseTo(0.75, 9);
    expect(finalValue.mode).toBe("last");
    expect(midpoint.value).toBeCloseTo(0.5, 9);
    expect(midpoint.mode).toBe("find");
    expect(crossing.value).toBeCloseTo(1.5e-3, 9);
    expect(crossing.mode).toBe("when");
    expect(secondCrossing.value).toBeCloseTo(2.75e-3, 9);
    expect(secondCrossing.mode).toBe("when");
    expect(propagationDelay.value).toBeCloseTo(1.0e-3, 9);
    expect(propagationDelay.probe).toBe("V(in)->V(out)");
    expect(propagationDelay.mode).toBe("delay");
    expect(formatMeasurementTable([
      peakToPeak,
      finalValue,
      midpoint,
      crossing,
      secondCrossing,
      propagationDelay,
    ])).toBe(
      "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n" +
        "swing\ttran\tV(out)\tpp\t1.000000e-03\t3.000000e-03\t1.500000e+00\n" +
        "settled\ttran\tV(out)\tlast\t\t\t7.500000e-01\n" +
        "midpoint\ttran\tV(out)\tfind\t1.500000e-03\t1.500000e-03\t5.000000e-01\n" +
        "crossing\ttran\tV(out)\twhen\t1.000000e-03\t3.000000e-03\t1.500000e-03\n" +
        "second_crossing\ttran\tV(out)\twhen\t1.000000e-03\t3.000000e-03\t2.750000e-03\n" +
        "prop_delay\ttran\tV(in)->V(out)\tdelay\t0.000000e+00\t3.000000e-03\t1.000000e-03\n",
    );
  });

  it("executes parsed transient .measure cards", () => {
    const points = [
      transientPoint(0.0, { in: 0.0, out: 0.0 }),
      transientPoint(1.0e-3, { in: 1.0, out: 1.25 }),
      transientPoint(2.0e-3, { in: 1.0, out: -0.25 }),
      transientPoint(3.0e-3, { in: 1.0, out: 0.75 }),
    ];

    const measurements = measureTransientDeck(
      points,
      `
.measure tran swing pp V(out) FROM=1m TO=3m
.measure tran midpoint FIND V(out) AT=1.5m
.measure tran crossing WHEN V(out)=0.5 FROM=1m TO=3m
.measure tran second_cross WHEN V(out)=0.5 FROM=1m TO=3m CROSS=2
.measure tran falling WHEN V(out)=0.5 FROM=1m TO=3m FALL=1
.measure tran rising WHEN V(out)=0.5 FROM=1m TO=3m RISE=1
.measure tran prop_delay TRIG V(in) VAL=0.5 RISE=1 TARG V(out) VAL=0.5 FALL=1 FROM=0 TO=3m
.meas transient mean avg V(out)
.end
`,
    );

    expect(measurements.map(({ name, mode, value, fromValue, toValue }) => [
      name,
      mode,
      value,
      fromValue,
      toValue,
    ])).toStrictEqual([
      ["swing", "pp", 1.5, 0.001, 0.003],
      ["midpoint", "find", 0.5, 0.0015, 0.0015],
      ["crossing", "when", 0.0015, 0.001, 0.003],
      ["second_cross", "when", 0.00275, 0.001, 0.003],
      ["falling", "when", 0.0015, 0.001, 0.003],
      ["rising", "when", 0.00275, 0.001, 0.003],
      ["prop_delay", "delay", 0.001, 0, 0.003],
      ["mean", "avg", 0.4375, undefined, undefined],
    ]);
    expect(formatMeasurementTable(measurements)).toBe(
      "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n" +
        "swing\ttran\tV(out)\tpp\t1.000000e-03\t3.000000e-03\t1.500000e+00\n" +
        "midpoint\ttran\tV(out)\tfind\t1.500000e-03\t1.500000e-03\t5.000000e-01\n" +
        "crossing\ttran\tV(out)\twhen\t1.000000e-03\t3.000000e-03\t1.500000e-03\n" +
        "second_cross\ttran\tV(out)\twhen\t1.000000e-03\t3.000000e-03\t2.750000e-03\n" +
        "falling\ttran\tV(out)\twhen\t1.000000e-03\t3.000000e-03\t1.500000e-03\n" +
        "rising\ttran\tV(out)\twhen\t1.000000e-03\t3.000000e-03\t2.750000e-03\n" +
        "prop_delay\ttran\tV(in)->V(out)\tdelay\t0.000000e+00\t3.000000e-03\t1.000000e-03\n" +
        "mean\ttran\tV(out)\tavg\t\t\t4.375000e-01\n",
    );
  });

  it("runs transient waveforms per corner and formats stable text tables", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "vin", "0", 10.0));
    circuit.add(resistor("R1", "vin", "mid", 1_000.0));
    circuit.add(resistor("R2", "mid", "0", 1_000.0));

    const result = transientCorners(circuit, 1.0e-3, 2.0e-3, [
      { name: "nominal", overrides: [] },
      {
        name: "r2-high",
        overrides: [{ elementName: "R2", parameter: "resistance", value: 2_000.0 }],
      },
    ]);

    expect(result.points.map((point) => point.cornerName)).toEqual(["nominal", "r2-high"]);
    expectClose(result.points[0].points.at(-1)?.voltage("mid"), 5.0);
    expectClose(result.points[1].points.at(-1)?.voltage("mid"), 20.0 / 3.0);
    expect(formatCornerTransientTable(result, ["V(vin)", "V(mid)", "I(V1)"])).toBe(
      "Corner\tIndex\tTime\tV(vin)\tV(mid)\tI(V1)\n" +
        "nominal\t0\t1.000000e-03\t1.000000e+01\t5.000000e+00\t-5.000000e-03\n" +
        "nominal\t1\t2.000000e-03\t1.000000e+01\t5.000000e+00\t-5.000000e-03\n" +
        "r2-high\t0\t1.000000e-03\t1.000000e+01\t6.666667e+00\t-3.333333e-03\n" +
        "r2-high\t1\t2.000000e-03\t1.000000e+01\t6.666667e+00\t-3.333333e-03\n",
    );
  });

  it("runs adaptive transient waveforms per corner and formats stable text tables", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "vin", "0", 1.0));
    circuit.add(resistor("R1", "vin", "out", 1_000.0));
    circuit.add(capacitor("C1", "out", "0", 1.0e-6));

    const result = transientAdaptiveCorners(circuit, 1.0e-3, 2.0e-3, [
      { name: "nominal", overrides: [] },
      {
        name: "r1-high",
        overrides: [{ elementName: "R1", parameter: "resistance", value: 2_000.0 }],
      },
    ], { method: "trap", tolerance: 1.0, minStep: 1.0e-3, maxStep: 1.0e-3 });

    expect(result.points.map((point) => point.cornerName)).toEqual(["nominal", "r1-high"]);
    expectClose(result.points[0].result.points.at(-1)?.voltage("out"), 7.777777777777778e-1);
    expectClose(result.points[1].result.points.at(-1)?.voltage("out"), 5.2e-1);
    expect(formatCornerAdaptiveTransientTable(result, ["V(vin)", "V(out)", "I(V1)"])).toBe(
      "Corner\tMethod\tStepsRejected\tConverged\tIndex\tTime\tV(vin)\tV(out)\tI(V1)\n" +
        "nominal\ttrap\t0\ttrue\t0\t1.000000e-03\t1.000000e+00\t3.333333e-01\t-6.666667e-04\n" +
        "nominal\ttrap\t0\ttrue\t1\t2.000000e-03\t1.000000e+00\t7.777778e-01\t-2.222222e-04\n" +
        "r1-high\ttrap\t0\ttrue\t0\t1.000000e-03\t1.000000e+00\t2.000000e-01\t-4.000000e-04\n" +
        "r1-high\ttrap\t0\ttrue\t1\t2.000000e-03\t1.000000e+00\t5.200000e-01\t-2.400000e-04\n",
    );
  });

  it("builds digital bridge sources schedules and VCD output", () => {
    const streams = [
      {
        signalName: "clk",
        events: [
          { timeSeconds: 0.0, state: "low" as const },
          { timeSeconds: 0.5e-9, state: "high" as const },
          { timeSeconds: 1.0e-9, state: "low" as const },
        ],
      },
      {
        signalName: "enable",
        events: [
          { timeSeconds: 0.25e-9, state: "low" as const },
          { timeSeconds: 0.75e-9, state: "high" as const },
        ],
      },
    ];
    const levels = DigitalLogicLevels.cmos1v8(0.25e-9);

    const waveform = digitalEventsToPwlWaveform(streams[0].events, levels);
    const source = digitalEventsToVoltageSource("Vclk", "clk", "0", streams[0].events, levels);
    const sources = digitalEventStreamsToVoltageSources(streams, "0", levels);
    const schedule = digitalEventStreamsToBridgeSchedule(streams, levels);

    expect(waveform.points.length).toBe(5);
    expect(waveform.points[2][0]).toBeCloseTo(0.75e-9, 18);
    expect(waveform.points[2][1]).toBeCloseTo(1.8, 9);
    expect(source.name).toBe("Vclk");
    expect(sources.map((candidate) => candidate.name)).toEqual(["Vclk", "Venable"]);
    expect(formatDigitalBridgeScheduleTable(schedule)).toBe(
      "Index\tTime\tStopTime\n" +
        "0\t0.000000e+00\t1.250000e-09\n" +
        "1\t2.500000e-10\t1.250000e-09\n" +
        "2\t5.000000e-10\t1.250000e-09\n" +
        "3\t7.500000e-10\t1.250000e-09\n" +
        "4\t1.000000e-09\t1.250000e-09\n" +
        "5\t1.250000e-09\t1.250000e-09\n",
    );
    expect(formatDigitalEventStreamVcd(streams)).toBe(
      "$version coding-adventures spice-engine mixed-signal bridge $end\n" +
        "$timescale 1ps $end\n" +
        "$scope module spice_bridge $end\n" +
        "$var wire 1 s0 clk $end\n" +
        "$var wire 1 s1 enable $end\n" +
        "$upscope $end\n" +
        "$enddefinitions $end\n" +
        "$dumpvars\n" +
        "0s0\n" +
        "0s1\n" +
        "$end\n" +
        "#0\n" +
        "0s0\n" +
        "#250\n" +
        "0s1\n" +
        "#500\n" +
        "1s0\n" +
        "#750\n" +
        "1s1\n" +
        "#1000\n" +
        "0s0\n",
    );
  });

  it("samples transient probes back to digital streams", () => {
    const levels = DigitalLogicLevels.cmos1v8(0.25e-9);
    const events = [
      { timeSeconds: 0.0, state: "low" as const },
      { timeSeconds: 0.5e-9, state: "high" as const },
      { timeSeconds: 1.25e-9, state: "low" as const },
    ];
    const circuit = new Circuit();
    circuit.add(digitalEventsToVoltageSource("Vdin", "din", "0", events, levels));
    circuit.add(resistor("Rload", "din", "0", 1_000.0));

    const points = transient(circuit, 0.25e-9, 1.5e-9);
    const sampled = sampleTransientProbeAsDigitalEvents(points, "V(din)", DigitalThresholds.cmos1v8());
    const streams = sampleTransientProbesAsDigitalEventStreams(
      points,
      [["din", "V(din)"]],
      DigitalThresholds.cmos1v8(),
    );

    expect(formatDigitalEventTable(sampled)).toBe(
      "Index\tTime\tState\n" +
        "0\t2.500000e-10\tlow\n" +
        "1\t7.500000e-10\thigh\n" +
        "2\t1.500000e-09\tlow\n",
    );
    expect(formatDigitalEventStreamTable(streams)).toBe(
      "Signal\tIndex\tTime\tState\n" +
        "din\t0\t2.500000e-10\tlow\n" +
        "din\t1\t7.500000e-10\thigh\n" +
        "din\t2\t1.500000e-09\tlow\n",
    );
  });

  it("runs digital bridge inputs through transient and corner outputs", () => {
    const inputStreams = [
      {
        signalName: "din",
        events: [
          { timeSeconds: 0.0, state: "low" as const },
          { timeSeconds: 0.5e-9, state: "high" as const },
          { timeSeconds: 1.25e-9, state: "low" as const },
        ],
      },
    ];
    const circuit = new Circuit();
    circuit.add(resistor("Rload", "din", "0", 1_000.0));

    const result = transientWithDigitalEventStreams(
      circuit,
      inputStreams,
      "0",
      DigitalLogicLevels.cmos1v8(0.25e-9),
      0.25e-9,
      1.5e-9,
      [["dout", "V(din)"]],
      DigitalThresholds.cmos1v8(),
    );
    const cornerResult = transientWithDigitalEventStreamsCorners(
      circuit,
      inputStreams,
      "0",
      DigitalLogicLevels.cmos1v8(0.25e-9),
      0.25e-9,
      1.5e-9,
      [["dout", "V(din)"]],
      DigitalThresholds.cmos1v8(),
      [
        { name: "nominal", overrides: [] },
        { name: "load-high", overrides: [{ elementName: "Rload", parameter: "resistance", value: 2_000.0 }] },
      ],
    );

    expect(formatDigitalEventStreamTable(result.outputStreams)).toBe(
      "Signal\tIndex\tTime\tState\n" +
        "dout\t0\t2.500000e-10\tlow\n" +
        "dout\t1\t7.500000e-10\thigh\n" +
        "dout\t2\t1.500000e-09\tlow\n",
    );
    expect(formatCornerDigitalEventStreamTable(cornerResult)).toBe(
      "Corner\tSignal\tIndex\tTime\tState\n" +
        "nominal\tdout\t0\t2.500000e-10\tlow\n" +
        "nominal\tdout\t1\t7.500000e-10\thigh\n" +
        "nominal\tdout\t2\t1.500000e-09\tlow\n" +
        "load-high\tdout\t0\t2.500000e-10\tlow\n" +
        "load-high\tdout\t1\t7.500000e-10\thigh\n" +
        "load-high\tdout\t2\t1.500000e-09\tlow\n",
    );
  });

  it("runs adaptive digital bridge outputs with metadata and corners", () => {
    const inputStreams = [
      {
        signalName: "din",
        events: [
          { timeSeconds: 0.0, state: "low" as const },
          { timeSeconds: 0.5e-9, state: "high" as const },
          { timeSeconds: 1.25e-9, state: "low" as const },
        ],
      },
    ];
    const circuit = new Circuit();
    circuit.add(resistor("Rload", "din", "0", 1_000.0));
    const options = { method: "trap" as const, tolerance: 1.0, minStep: 0.25e-9, maxStep: 0.25e-9 };

    const result = transientAdaptiveWithDigitalEventStreams(
      circuit,
      inputStreams,
      "0",
      DigitalLogicLevels.cmos1v8(0.25e-9),
      0.25e-9,
      1.5e-9,
      options,
      [["dout", "V(din)"]],
      DigitalThresholds.cmos1v8(),
    );
    const cornerResult = transientAdaptiveWithDigitalEventStreamsCorners(
      circuit,
      inputStreams,
      "0",
      DigitalLogicLevels.cmos1v8(0.25e-9),
      0.25e-9,
      1.5e-9,
      options,
      [["dout", "V(din)"]],
      DigitalThresholds.cmos1v8(),
      [{ name: "nominal", overrides: [] }],
    );

    expect(formatAdaptiveDigitalEventStreamTable(result)).toBe(
      "Method\tStepsRejected\tConverged\tSignal\tIndex\tTime\tState\n" +
        "trap\t0\ttrue\tdout\t0\t2.500000e-10\tlow\n" +
        "trap\t0\ttrue\tdout\t1\t7.500000e-10\thigh\n" +
        "trap\t0\ttrue\tdout\t2\t1.500000e-09\tlow\n",
    );
    expect(formatCornerAdaptiveDigitalEventStreamTable(cornerResult)).toBe(
      "Corner\tMethod\tStepsRejected\tConverged\tSignal\tIndex\tTime\tState\n" +
        "nominal\ttrap\t0\ttrue\tdout\t0\t2.500000e-10\tlow\n" +
        "nominal\ttrap\t0\ttrue\tdout\t1\t7.500000e-10\thigh\n" +
        "nominal\ttrap\t0\ttrue\tdout\t2\t1.500000e-09\tlow\n",
    );
  });

  it("formats stable text output tables for pole-zero results", () => {
    const result: PoleZeroResult = {
      inputSource: "Vin",
      outputNode: "out",
      entries: [
        {
          kind: "zero",
          real: 0.0,
          imaginary: 1.0e3,
          frequencyHz: 1.0e3 / (2.0 * Math.PI),
          damping: 0.0,
        },
        {
          kind: "pole",
          real: -5.0,
          imaginary: -999.987499921874,
          frequencyHz: 1.0e3 / (2.0 * Math.PI),
          damping: 5.0e-3,
        },
      ],
    };

    expect(formatPoleZeroTable(result)).toBe(
      "Index\tKind\tReal\tImaginary\tFrequency\tDamping\n" +
        "0\tzero\t0.000000e+00\t1.000000e+03\t1.591549e+02\t0.000000e+00\n" +
        "1\tpole\t-5.000000e+00\t-9.999875e+02\t1.591549e+02\t5.000000e-03\n",
    );
  });

  it("formats stable text output tables for distortion results", () => {
    const result: DistortionResult = {
      inputSource: "Vin",
      outputProbe: "V(out)",
      points: [
        {
          frequencyHz: 1000.0,
          fundamentalMagnitude: 1.0,
          harmonics: [
            {
              harmonic: 1,
              frequencyHz: 1000.0,
              magnitude: 1.0,
              phaseDegrees: 0.0,
            },
            {
              harmonic: 2,
              frequencyHz: 2000.0,
              magnitude: 0.025,
              phaseDegrees: -1.5707963267948966,
            },
          ],
          totalHarmonicDistortion: 0.025,
        },
      ],
    };

    expect(formatDistortionTable(result)).toBe(
      "Frequency\tInput\tOutput\tHarmonic\tMagnitude\tPhase\tTHD\n" +
        "1.000000e+03\tVin\tV(out)\t1\t1.000000e+00\t0.000000e+00\t2.500000e-02\n" +
        "1.000000e+03\tVin\tV(out)\t2\t2.500000e-02\t-1.570796e+00\t2.500000e-02\n",
    );
  });

  it("formats stable text output tables for corner distortion results", () => {
    const result: CornerDistortionResult = {
      inputSource: "Vin",
      outputProbe: "V(out)",
      points: [
        {
          cornerName: "nominal",
          result: {
            inputSource: "Vin",
            outputProbe: "V(out)",
            points: [
              {
                frequencyHz: 1000.0,
                fundamentalMagnitude: 1.0,
                harmonics: [
                  {
                    harmonic: 1,
                    frequencyHz: 1000.0,
                    magnitude: 1.0,
                    phaseDegrees: 0.0,
                  },
                  {
                    harmonic: 2,
                    frequencyHz: 2000.0,
                    magnitude: 0.025,
                    phaseDegrees: -1.5707963267948966,
                  },
                ],
                totalHarmonicDistortion: 0.025,
              },
            ],
          },
        },
        {
          cornerName: "slow",
          result: {
            inputSource: "Vin",
            outputProbe: "V(out)",
            points: [
              {
                frequencyHz: 1000.0,
                fundamentalMagnitude: 0.8,
                harmonics: [
                  {
                    harmonic: 2,
                    frequencyHz: 2000.0,
                    magnitude: 0.04,
                    phaseDegrees: 12.5,
                  },
                ],
                totalHarmonicDistortion: 0.05,
              },
            ],
          },
        },
      ],
    };

    expect(formatCornerDistortionTable(result)).toBe(
      "Corner\tFrequency\tInput\tOutput\tHarmonic\tMagnitude\tPhase\tTHD\n" +
        "nominal\t1.000000e+03\tVin\tV(out)\t1\t1.000000e+00\t0.000000e+00\t2.500000e-02\n" +
        "nominal\t1.000000e+03\tVin\tV(out)\t2\t2.500000e-02\t-1.570796e+00\t2.500000e-02\n" +
        "slow\t1.000000e+03\tVin\tV(out)\t2\t4.000000e-02\t1.250000e+01\t5.000000e-02\n",
    );
  });

  it("formats stable text output tables for Fourier results", () => {
    const result: FourierResult = {
      fundamentalFrequencyHz: 1000.0,
      startTime: 0.0,
      endTime: 0.001,
      probes: [
        {
          probe: "V(out)",
          dc: 0.1,
          harmonics: [
            {
              harmonic: 1,
              frequencyHz: 1000.0,
              cosine: 1.0,
              sine: 0.0,
              magnitude: 1.0,
              phaseDegrees: 0.0,
            },
            {
              harmonic: 2,
              frequencyHz: 2000.0,
              cosine: 0.0,
              sine: -0.025,
              magnitude: 0.025,
              phaseDegrees: -90.0,
            },
          ],
          totalHarmonicDistortion: 0.025,
        },
      ],
    };

    expect(formatFourierTable(result)).toBe(
      "Probe\tHarmonic\tFrequency\tCosine\tSine\tMagnitude\tPhase\tDC\tTHD\n" +
        "V(out)\t1\t1.000000e+03\t1.000000e+00\t0.000000e+00\t1.000000e+00\t0.000000e+00\t1.000000e-01\t2.500000e-02\n" +
        "V(out)\t2\t2.000000e+03\t0.000000e+00\t-2.500000e-02\t2.500000e-02\t-9.000000e+01\t1.000000e-01\t2.500000e-02\n",
    );
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
