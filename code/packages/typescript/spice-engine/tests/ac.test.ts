import { describe, expect, it } from "vitest";
import {
  type AcPoint,
  type Complex,
  Circuit,
  SpiceError,
  formatAcTable,
  formatCornerAcTable,
  formatCornerSParameterTable,
  formatDeckAcTable,
  formatMeasurementTable,
  formatSParameterTable,
  acSweep,
  acSweepCorners,
  bjt,
  capacitor,
  cccs,
  ccvs,
  complexAbs,
  complexPhase,
  currentSource,
  currentSourceWithAc,
  deviceModelCapacitanceAuditFixtures,
  diode,
  inductor,
  jfet,
  mosfet,
  mutualInductor,
  resistor,
  measureAcSweepDeck,
  measureAcSweepProbe,
  sParameters,
  sParametersCorners,
  transmissionLine,
  vcvs,
  voltageSource,
  voltageSourceWithAc,
} from "../src/index.js";

const TWO_PI_FOR_TEST = 2.0 * Math.PI;

function acPoint(frequencyHz: number, value: Complex): AcPoint {
  const nodeVoltages = new Map<string, Complex>([["out", value]]);
  const branchCurrents = new Map<string, Complex>();
  return {
    frequencyHz,
    nodeVoltages,
    branchCurrents,
    voltage(node: string): Complex | undefined {
      const normalized = node.toLowerCase();
      return normalized === "0" || normalized === "gnd" ? { real: 0.0, imag: 0.0 } : nodeVoltages.get(node);
    },
    branchCurrent(sourceName: string): Complex | undefined {
      const key = sourceName.startsWith("I(") ? sourceName : `I(${sourceName})`;
      return branchCurrents.get(key);
    },
  };
}

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

  it("solves a large resistor ladder through the sparse complex solver path", () => {
    const circuit = new Circuit();
    circuit.add(voltageSourceWithAc("V1", "n0", "0", 0.0, 1.0, 0.0));
    for (let index = 0; index < 34; index++) {
      circuit.add(resistor(`R${index}`, `n${index}`, `n${index + 1}`, 1_000.0));
    }
    circuit.add(resistor("R34", "n34", "0", 1_000.0));

    const points = acSweep(circuit, 1_000.0, 1_000.0, 1);

    expect(points).toHaveLength(1);
    expectClose(points[0].voltage("n34")!.real, 1.0 / 35.0);
    expectClose(points[0].voltage("n34")!.imag, 0.0);
  });

  it("formats stable text output tables for AC results", () => {
    const circuit = new Circuit();
    circuit.add(voltageSourceWithAc("V1", "in", "0", 0.0, 1.0, 0.0));
    circuit.add(resistor("R1", "in", "out", 1_000.0));
    circuit.add(capacitor("C1", "out", "0", 1.0e-6));

    const corner = 1.0 / (2.0 * Math.PI * 1_000.0 * 1.0e-6);
    const point = acSweep(circuit, corner, corner, 10)[0];

    expect(formatAcTable([point], ["V(out)", "I(V1)"])).toBe(
      "Index\tFrequency\tProbe\tReal\tImaginary\tMagnitude\tPhase\n" +
        "0\t1.591549e+02\tV(out)\t5.000000e-01\t-5.000000e-01\t7.071068e-01\t-4.500000e+01\n" +
        "0\t1.591549e+02\tI(V1)\t-5.000000e-04\t-5.000000e-04\t7.071068e-04\t-1.350000e+02\n",
    );
    expect(formatDeckAcTable([point], ".save V(out)\n.probe ac I(V1)\n.end\n")).toBe(
      "Index\tFrequency\tProbe\tReal\tImaginary\tMagnitude\tPhase\n" +
        "0\t1.591549e+02\tV(out)\t5.000000e-01\t-5.000000e-01\t7.071068e-01\t-4.500000e+01\n" +
        "0\t1.591549e+02\tI(V1)\t-5.000000e-04\t-5.000000e-04\t7.071068e-04\t-1.350000e+02\n",
    );
  });

  it("executes AC probe measurements and parsed cards", () => {
    const points = [
      acPoint(10.0, { real: 1.0, imag: 0.0 }),
      acPoint(100.0, { real: 0.0, imag: 2.0 }),
      acPoint(1_000.0, { real: 0.0, imag: 0.5 }),
    ];

    const peak = measureAcSweepProbe(points, "outPeak", "V(out)", "max", 10.0, 100.0);
    const average = measureAcSweepProbe(points, "outAvg", "V(out)", "avg");

    expectClose(peak.value, 2.0);
    expect(peak.analysis).toBe("ac");
    expectClose(average.value, 1.1666666666666667);
    expect(formatMeasurementTable([peak, average])).toBe(
      "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n" +
        "outPeak\tac\tV(out)\tmax\t1.000000e+01\t1.000000e+02\t2.000000e+00\n" +
        "outAvg\tac\tV(out)\tavg\t\t\t1.166667e+00\n",
    );

    const measurements = measureAcSweepDeck(
      points,
      `
.measure ac outSwing PP V(out) FROM=10 TO=1000
.meas ac outFinal FINAL V(out)
.end
`,
    );

    expect(formatMeasurementTable(measurements)).toBe(
      "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n" +
        "outSwing\tac\tV(out)\tpp\t1.000000e+01\t1.000000e+03\t1.500000e+00\n" +
        "outFinal\tac\tV(out)\tlast\t\t\t5.000000e-01\n",
    );
  });

  it("runs device model capacitance audit fixtures as reference AC points", () => {
    const fixtures = deviceModelCapacitanceAuditFixtures();
    expect(fixtures.map((fixture) => fixture.name)).toStrictEqual([
      "diode-capacitance-ac",
      "bjt-capacitance-ac",
      "jfet-capacitance-ac",
      "mos-level1-capacitance-ac",
    ]);

    for (const fixture of fixtures) {
      const point = acSweep(fixture.circuit, fixture.frequencyHz, fixture.frequencyHz, 1)[0]!;
      const voltage = point.voltage(fixture.probeNode);
      expect(voltage).not.toBeUndefined();
      const magnitude = complexAbs(voltage!);
      expect(magnitude).toBeGreaterThanOrEqual(fixture.expectedMagnitudeMin);
      expect(magnitude).toBeLessThanOrEqual(fixture.expectedMagnitudeMax);
      expect(fixture.deckLines[0]!.startsWith("* device-model capacitance fixture:")).toBe(true);
      expect(fixture.deckLines.some((line) => line.startsWith(".model "))).toBe(true);
      expect(fixture.deckLines.some((line) => line.startsWith(".ac "))).toBe(true);
      expect(fixture.capacitanceBehavior.length).toBeGreaterThan(0);
    }

    const jfetFixture = fixtures.find((fixture) => fixture.kind === "NJF");
    expect(jfetFixture?.capacitanceBehavior).toContain("CGS/CGD");
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

  it("stamps reverse-biased diode junction capacitance", () => {
    const circuit = new Circuit();
    circuit.add(voltageSourceWithAc("Vac", "in", "0", 0.0, 1.0));
    circuit.add(resistor("R1", "in", "node", 1_000.0));
    circuit.add(diode("D1", "0", "node", 1.0e-15, 0.02585, 1.0, undefined, 1.0e-3, 1.0e-6));

    const points = acSweep(circuit, 10.0, 100_000.0, 2);
    const low = complexAbs(points[0].voltage("node")!);
    const high = complexAbs(points[points.length - 1].voltage("node")!);

    expect(low).toBeGreaterThan(0.9);
    expect(high).toBeLessThan(low / 100.0);
  });

  it("reduces diode depletion capacitance with reverse bias", () => {
    const highFrequencyVoltage = (dcBias: number): number => {
      const circuit = new Circuit();
      circuit.add(voltageSourceWithAc("Vac", "in", "0", dcBias, 1.0));
      circuit.add(resistor("R1", "in", "node", 1_000.0));
      circuit.add(
        diode(
          "D1",
          "0",
          "node",
          1.0e-15,
          0.02585,
          1.0,
          undefined,
          1.0e-3,
          1.0e-6,
          0.0,
          1.0,
          0.5,
        ),
      );
      return complexAbs(acSweep(circuit, 100_000.0, 100_000.0, 1)[0].voltage("node")!);
    };

    const zeroBias = highFrequencyVoltage(0.0);
    const reverseBiased = highFrequencyVoltage(4.0);

    expect(reverseBiased).toBeGreaterThan(zeroBias * 1.8);
  });

  it("shapes forward-biased diode depletion capacitance with FC", () => {
    const forwardBiasedVoltage = (coefficient: number): number => {
      const circuit = new Circuit();
      circuit.add(voltageSourceWithAc("Vac", "in", "0", 0.75, 1.0));
      circuit.add(resistor("R1", "in", "node", 1_000.0));
      circuit.add(
        diode(
          "D1",
          "node",
          "0",
          1.0e-30,
          0.02585,
          1.0,
          undefined,
          1.0e-3,
          1.0e-6,
          0.0,
          1.0,
          0.5,
          coefficient,
        ),
      );
      return complexAbs(acSweep(circuit, 1_000.0, 1_000.0, 1)[0].voltage("node")!);
    };

    const earlyTransition = forwardBiasedVoltage(0.2);
    const lateTransition = forwardBiasedVoltage(0.8);

    expect(lateTransition).toBeLessThan(earlyTransition * 0.85);
  });

  it("stamps forward-biased diode transit-time diffusion capacitance", () => {
    const highFrequencyAnode = (transitTime: number): number => {
      const circuit = new Circuit();
      circuit.add(voltageSourceWithAc("Vac", "in", "0", 1.0, 1.0));
      circuit.add(resistor("R1", "in", "anode", 1.0e6));
      circuit.add(diode("D1", "anode", "0", 1.0e-15, 0.02585, 1.0, undefined, 1.0e-3, 0.0, transitTime));
      return complexAbs(acSweep(circuit, 100_000_000.0, 100_000_000.0, 1)[0].voltage("anode")!);
    };

    const withoutTransit = highFrequencyAnode(0.0);
    const withTransit = highFrequencyAnode(1.0e-6);

    expect(withoutTransit).toBeGreaterThan(0.01);
    expect(withTransit).toBeLessThan(withoutTransit / 100.0);
  });

  it("runs frequency sweeps at each named corner", () => {
    const resistance = 1_000.0;
    const capacitance = 1.0e-6;
    const cornerFrequency = 1.0 / (2.0 * Math.PI * resistance * capacitance);

    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 1.0));
    circuit.add(resistor("R1", "in", "out", resistance));
    circuit.add(capacitor("C1", "out", "0", capacitance));

    const result = acSweepCorners(
      circuit,
      cornerFrequency,
      cornerFrequency,
      10,
      [
        { name: "nominal", overrides: [] },
        {
          name: "r-fast",
          overrides: [{ elementName: "R1", parameter: "resistance", value: 500.0 }],
        },
      ],
    );

    expect(result.points.map((point) => point.cornerName)).toEqual(["nominal", "r-fast"]);
    expect(result.points[0].points).toHaveLength(1);
    expect(result.points[0].points[0].frequencyHz).toBeCloseTo(cornerFrequency, 9);
    expect(complexAbs(result.points[0].points[0].voltage("out")!)).toBeCloseTo(
      1.0 / Math.sqrt(2.0),
      9,
    );
    expect(complexAbs(result.points[1].points[0].voltage("out")!)).toBeCloseTo(
      1.0 / Math.sqrt(1.25),
      9,
    );
    expect(formatCornerAcTable(result, ["V(out)", "I(Vin)"])).toBe(
      "Corner\tIndex\tFrequency\tProbe\tReal\tImaginary\tMagnitude\tPhase\n" +
      "nominal\t0\t1.591549e+02\tV(out)\t5.000000e-01\t-5.000000e-01\t7.071068e-01\t-4.500000e+01\n" +
      "nominal\t0\t1.591549e+02\tI(Vin)\t-5.000000e-04\t-5.000000e-04\t7.071068e-04\t-1.350000e+02\n" +
      "r-fast\t0\t1.591549e+02\tV(out)\t8.000000e-01\t-4.000000e-01\t8.944272e-01\t-2.656505e+01\n" +
      "r-fast\t0\t1.591549e+02\tI(Vin)\t-4.000000e-04\t-8.000000e-04\t8.944272e-04\t-1.165651e+02\n",
    );
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

  it("stamps mutual-inductor transformer coupling", () => {
    const primaryL = 1.0e-3;
    const secondaryL = 4.0e-3;
    const coupling = 0.9;
    const load = 1_000.0;
    const frequency = 1_000.0;
    const mutualL = coupling * Math.sqrt(primaryL * secondaryL);
    const denominator = {
      real: 1.0,
      imag: (TWO_PI_FOR_TEST * frequency * secondaryL) / load,
    };
    const numerator = {
      real: 0.0,
      imag: TWO_PI_FOR_TEST * frequency * mutualL,
    };
    const scale = denominator.real ** 2 + denominator.imag ** 2;
    const expected = {
      real: (numerator.real * denominator.real + numerator.imag * denominator.imag) / scale,
      imag: (numerator.imag * denominator.real - numerator.real * denominator.imag) / scale,
    };

    const circuit = new Circuit();
    circuit.add(currentSourceWithAc("Iin", "0", "pri", 0.0, 1.0));
    circuit.add(inductor("Lpri", "pri", "0", primaryL));
    circuit.add(inductor("Lsec", "sec", "0", secondaryL));
    circuit.add(mutualInductor("K1", "Lpri", "Lsec", coupling));
    circuit.add(resistor("Rload", "sec", "0", load));

    const points = acSweep(circuit, frequency, frequency, 10);

    const secondary = points[0].voltage("sec");
    expect(secondary).not.toBeUndefined();
    expectClose(secondary!.real, expected.real);
    expectClose(secondary!.imag, expected.imag);
  });

  it("rejects mutual-inductor missing references", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "pri", "0", 1.0));
    circuit.add(inductor("Lpri", "pri", "0", 1.0e-3));
    circuit.add(mutualInductor("Kbad", "Lpri", "Lmissing", 0.9));

    expect(() => acSweep(circuit, 1_000.0, 1_000.0, 10)).toThrowError(SpiceError);
  });

  it("applies transmission-line matched-load phase delay", () => {
    const frequency = 1_000_000.0;
    const delay = 1.0 / (4.0 * frequency);
    const circuit = new Circuit();
    circuit.add(voltageSourceWithAc("Vin", "src", "0", 0.0, 1.0));
    circuit.add(resistor("Rsrc", "src", "in", 50.0));
    circuit.add(transmissionLine("T1", "in", "0", "out", "0", 50.0, delay));
    circuit.add(resistor("Rload", "out", "0", 50.0));

    const points = acSweep(circuit, frequency, frequency, 10);
    const out = points[0].voltage("out");

    expect(out).not.toBeUndefined();
    expectClose(out!.real, 0.0);
    expectClose(out!.imag, -0.5);
  });

  it("rejects invalid transmission-line AC parameters", () => {
    const circuit = new Circuit();
    circuit.add(voltageSourceWithAc("Vin", "src", "0", 0.0, 1.0));
    circuit.add(transmissionLine("Tbad", "src", "0", "out", "0", 0.0, 1.0e-9));
    circuit.add(resistor("Rload", "out", "0", 50.0));

    expect(() => acSweep(circuit, 1_000.0, 1_000.0, 10)).toThrowError(SpiceError);
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

  it("uses explicit voltage-source AC magnitude and phase separately from DC bias", () => {
    const circuit = new Circuit();
    circuit.add(voltageSourceWithAc("Vin", "vin", "0", 10.0, 2.0, 90.0));
    circuit.add(resistor("R1", "vin", "out", 1_000.0));
    circuit.add(resistor("R2", "out", "0", 1_000.0));

    const points = acSweep(circuit, 1_000.0, 1_000.0, 10);

    expect(points).toHaveLength(1);
    const vin = points[0].voltage("vin");
    const out = points[0].voltage("out");
    expect(vin).not.toBeUndefined();
    expect(out).not.toBeUndefined();
    expectClose(vin!.real, 0.0);
    expectClose(vin!.imag, 2.0);
    expectClose(out!.real, 0.0);
    expectClose(out!.imag, 1.0);
  });

  it("zeros sources without AC specs when any explicit AC source is present", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vbias", "bias", "0", 5.0));
    circuit.add(currentSourceWithAc("Iac", "0", "out", 0.0, 1.0e-3, 90.0));
    circuit.add(resistor("R1", "bias", "out", 1_000.0));
    circuit.add(resistor("R2", "out", "0", 1_000.0));

    const points = acSweep(circuit, 1_000.0, 1_000.0, 10);

    expect(points).toHaveLength(1);
    const bias = points[0].voltage("bias");
    const out = points[0].voltage("out");
    expect(bias).not.toBeUndefined();
    expect(out).not.toBeUndefined();
    expectClose(bias!.real, 0.0);
    expectClose(bias!.imag, 0.0);
    expectClose(out!.real, 0.0);
    expectClose(out!.imag, 0.5);
  });

  it("uses JFET common-source gain from the DC bias point", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vdd", "vdd", "0", 10.0));
    circuit.add(voltageSourceWithAc("Vin", "gate", "0", 0.0, 1.0, 0.0));
    circuit.add(resistor("Rd", "vdd", "drain", 1_000.0));
    circuit.add(jfet("J1", "drain", "gate", "0", "NJF", 1.0e-3, -2.0));

    const points = acSweep(circuit, 1_000.0, 1_000.0, 10);

    expect(points).toHaveLength(1);
    const out = points[0].voltage("drain");
    expect(out).not.toBeUndefined();
    expectClose(out!.real, -4.0);
    expectClose(out!.imag, 0.0);
    expectClose(complexAbs(points[0].voltage("vdd")!), 0.0);
  });

  it("uses JFET gate-source capacitance in AC analysis", () => {
    function gateAmplitude(gateSourceCapacitance: number): number {
      const circuit = new Circuit();
      circuit.add(voltageSourceWithAc("Vac", "in", "0", 0.0, 1.0));
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
      return complexAbs(acSweep(circuit, 100_000.0, 100_000.0, 1)[0].voltage("gate")!);
    }

    const withoutCapacitance = gateAmplitude(0.0);
    const withCapacitance = gateAmplitude(1.0e-6);

    expect(withoutCapacitance).toBeGreaterThan(0.9);
    expect(withCapacitance).toBeLessThan(withoutCapacitance / 100.0);
  });

  it("uses MOSFET overlap capacitance in AC analysis", () => {
    function gateAmplitude(CGSO: number): number {
      const circuit = new Circuit();
      circuit.add(voltageSourceWithAc("Vac", "in", "0", 0.0, 1.0));
      circuit.add(resistor("Rin", "in", "gate", 1_000.0));
      circuit.add(resistor("Rdrain", "drain", "0", 1_000.0));
      circuit.add(mosfet("M1", "drain", "gate", "0", "0", "NMOS", {
        KP: 1.0e-12,
        W: 1.0,
        L: 1.0,
        CGSO,
      }));
      return complexAbs(acSweep(circuit, 100_000.0, 100_000.0, 1)[0].voltage("gate")!);
    }

    const withoutCapacitance = gateAmplitude(0.0);
    const withCapacitance = gateAmplitude(1.0e-6);

    expect(withoutCapacitance).toBeGreaterThan(0.9);
    expect(withCapacitance).toBeLessThan(withoutCapacitance / 100.0);
  });

  it("uses BJT base-emitter capacitance in AC analysis", () => {
    function baseAmplitude(baseEmitterCapacitance: number): number {
      const circuit = new Circuit();
      circuit.add(voltageSourceWithAc("Vac", "in", "0", 0.0, 1.0));
      circuit.add(resistor("Rin", "in", "base", 1_000.0));
      circuit.add(resistor("Rc", "col", "0", 1_000.0));
      circuit.add(bjt("Q1", "col", "base", "0", "NPN", 1.0e-14, 100.0, 0.02585, baseEmitterCapacitance));
      return complexAbs(acSweep(circuit, 100_000.0, 100_000.0, 1)[0].voltage("base")!);
    }

    const withoutCapacitance = baseAmplitude(0.0);
    const withCapacitance = baseAmplitude(1.0e-6);

    expect(withoutCapacitance).toBeGreaterThan(0.9);
    expect(withCapacitance).toBeLessThan(withoutCapacitance / 100.0);
  });

  it("uses BJT forward transit time as diffusion capacitance in AC analysis", () => {
    function baseAmplitude(forwardTransitTime: number): number {
      const circuit = new Circuit();
      circuit.add(voltageSourceWithAc("Vac", "in", "0", 0.0, 1.0));
      circuit.add(resistor("Rin", "in", "base", 1_000.0));
      circuit.add(resistor("Rc", "col", "0", 1_000.0));
      circuit.add(bjt("Q1", "col", "base", "0", "NPN", 25.85e-6, 100.0, 0.02585, 0.0, 0.0, forwardTransitTime));
      return complexAbs(acSweep(circuit, 100_000.0, 100_000.0, 1)[0].voltage("base")!);
    }

    const withoutTransitTime = baseAmplitude(0.0);
    const withTransitTime = baseAmplitude(1.0e-3);

    expect(withoutTransitTime).toBeGreaterThan(0.9);
    expect(withTransitTime).toBeLessThan(withoutTransitTime / 100.0);
  });

  it("uses BJT reverse transit time as base-collector diffusion capacitance in AC analysis", () => {
    function baseAmplitude(reverseTransitTime: number): number {
      const circuit = new Circuit();
      circuit.add(voltageSourceWithAc("Vac", "in", "0", 0.0, 1.0));
      circuit.add(resistor("Rin", "in", "base", 1_000.0));
      circuit.add(resistor("Rc", "col", "0", 1.0));
      circuit.add(bjt("Q1", "col", "base", "0", "NPN", 25.85e-6, 100.0, 0.02585, 0.0, 0.0, 0.0, reverseTransitTime));
      return complexAbs(acSweep(circuit, 100_000.0, 100_000.0, 1)[0].voltage("base")!);
    }

    const withoutTransitTime = baseAmplitude(0.0);
    const withTransitTime = baseAmplitude(1.0e-2);

    expect(withoutTransitTime).toBeGreaterThan(0.9);
    expect(withTransitTime).toBeLessThan(withoutTransitTime / 100.0);
  });

  it("uses BJT reverse emission coefficient to reduce base-collector diffusion capacitance", () => {
    function baseAmplitude(reverseEmissionCoefficient: number): number {
      const circuit = new Circuit();
      circuit.add(voltageSourceWithAc("Vac", "in", "0", 0.0, 1.0));
      circuit.add(resistor("Rin", "in", "base", 1_000.0));
      circuit.add(resistor("Rc", "col", "0", 1.0));
      circuit.add(bjt("Q1", "col", "base", "0", "NPN", 25.85e-6, 100.0, 0.02585, 0.0, 0.0, 0.0, 1.0e-2, 3.0, 1.11, 0.0, 1.0, reverseEmissionCoefficient));
      return complexAbs(acSweep(circuit, 100_000.0, 100_000.0, 1)[0].voltage("base")!);
    }

    expect(baseAmplitude(2.0)).toBeGreaterThan(baseAmplitude(1.0));
  });

  it("shapes BJT base-emitter depletion capacitance under reverse bias", () => {
    function baseAmplitude(baseEmitterGradingCoefficient: number): number {
      const circuit = new Circuit();
      circuit.add(voltageSourceWithAc("Vac", "in", "0", -1.0, 1.0));
      circuit.add(resistor("Rin", "in", "base", 1_000.0));
      circuit.add(bjt("Q1", "0", "base", "0", "NPN", 1.0e-14, 100.0, 0.02585, 1.0e-6, 0.0, 0.0, 0.0, 3.0, 1.11, 0.0, 1.0, 1.0, 0.75, baseEmitterGradingCoefficient));
      return complexAbs(acSweep(circuit, 1_000.0, 1_000.0, 1)[0].voltage("base")!);
    }

    expect(baseAmplitude(0.5)).toBeGreaterThan(baseAmplitude(0.0));
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

  it("applies CCCS current gain in AC analysis", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 1.0));
    circuit.add(resistor("Rsense", "in", "sense", 1_000.0));
    circuit.add(voltageSource("Vsense", "sense", "0", 0.0));
    circuit.add(cccs("F1", "0", "out", "Vsense", 3.0));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    const points = acSweep(circuit, 1_000.0, 1_000.0, 10);

    expect(points).toHaveLength(1);
    const out = points[0].voltage("out");
    expect(out).not.toBeUndefined();
    expectClose(out!.real, 3.0);
    expectClose(out!.imag, 0.0);
  });

  it("applies CCVS transresistance in AC analysis", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 1.0));
    circuit.add(resistor("Rsense", "in", "sense", 1_000.0));
    circuit.add(voltageSource("Vsense", "sense", "0", 0.0));
    circuit.add(ccvs("H1", "out", "0", "Vsense", 3_000.0));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    const points = acSweep(circuit, 1_000.0, 1_000.0, 10);

    expect(points).toHaveLength(1);
    const out = points[0].voltage("out");
    expect(out).not.toBeUndefined();
    expectClose(out!.real, 3.0);
    expectClose(out!.imag, 0.0);
    expectClose(points[0].branchCurrent("H1")?.real, -3.0e-3);
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

describe("sParameters", () => {
  it("extracts a series-resistor two-port from AC port solves", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("P1", "p1", "0", 0.0));
    circuit.add(voltageSource("P2", "p2", "0", 0.0));
    circuit.add(resistor("Rseries", "p1", "p2", 50.0));

    const result = sParameters(circuit, "P1", "P2", [1.0e6], 50.0);
    const point = result.points[0];

    expect(point.s11.real).toBeCloseTo(1.0 / 3.0, 9);
    expect(point.s22.real).toBeCloseTo(1.0 / 3.0, 9);
    expect(point.s21.real).toBeCloseTo(2.0 / 3.0, 9);
    expect(point.s12.real).toBeCloseTo(2.0 / 3.0, 9);
    expect(point.s11.imag).toBeCloseTo(0.0, 12);
    expect(point.s21.imag).toBeCloseTo(0.0, 12);
    expect(formatSParameterTable(result)).toBe(
      "Index\tFrequency\tPort1\tPort2\tParameter\tReal\tImaginary\tMagnitude\tPhase\n" +
        "0\t1.000000e+06\tP1\tP2\tS11\t3.333333e-01\t0.000000e+00\t3.333333e-01\t0.000000e+00\n" +
        "0\t1.000000e+06\tP1\tP2\tS21\t6.666667e-01\t0.000000e+00\t6.666667e-01\t0.000000e+00\n" +
        "0\t1.000000e+06\tP1\tP2\tS12\t6.666667e-01\t0.000000e+00\t6.666667e-01\t0.000000e+00\n" +
        "0\t1.000000e+06\tP1\tP2\tS22\t3.333333e-01\t0.000000e+00\t3.333333e-01\t0.000000e+00\n",
    );
  });

  it("runs two-port extraction at each named corner and formats stable tables", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("P1", "p1", "0", 0.0));
    circuit.add(voltageSource("P2", "p2", "0", 0.0));
    circuit.add(resistor("Rseries", "p1", "p2", 50.0));

    const result = sParametersCorners(
      circuit,
      "P1",
      "P2",
      [1.0e6],
      [
        { name: "nominal", overrides: [] },
        {
          name: "series-high",
          overrides: [{ elementName: "Rseries", parameter: "resistance", value: 100.0 }],
        },
      ],
      50.0,
    );

    expect(result.port1Source).toBe("P1");
    expect(result.port2Source).toBe("P2");
    expect(result.referenceImpedanceOhms).toBe(50.0);
    expect(result.points.map((point) => point.cornerName)).toEqual([
      "nominal",
      "series-high",
    ]);
    expect(result.points[0].result.points[0].s21.real).toBeCloseTo(2.0 / 3.0, 9);
    expect(result.points[1].result.points[0].s21.real).toBeCloseTo(0.5, 9);
    expect(formatCornerSParameterTable(result)).toBe(
      "Corner\tIndex\tFrequency\tPort1\tPort2\tParameter\tReal\tImaginary\tMagnitude\tPhase\n" +
        "nominal\t0\t1.000000e+06\tP1\tP2\tS11\t3.333333e-01\t0.000000e+00\t3.333333e-01\t0.000000e+00\n" +
        "nominal\t0\t1.000000e+06\tP1\tP2\tS21\t6.666667e-01\t0.000000e+00\t6.666667e-01\t0.000000e+00\n" +
        "nominal\t0\t1.000000e+06\tP1\tP2\tS12\t6.666667e-01\t0.000000e+00\t6.666667e-01\t0.000000e+00\n" +
        "nominal\t0\t1.000000e+06\tP1\tP2\tS22\t3.333333e-01\t0.000000e+00\t3.333333e-01\t0.000000e+00\n" +
        "series-high\t0\t1.000000e+06\tP1\tP2\tS11\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\n" +
        "series-high\t0\t1.000000e+06\tP1\tP2\tS21\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\n" +
        "series-high\t0\t1.000000e+06\tP1\tP2\tS12\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\n" +
        "series-high\t0\t1.000000e+06\tP1\tP2\tS22\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\n",
    );
  });
});
