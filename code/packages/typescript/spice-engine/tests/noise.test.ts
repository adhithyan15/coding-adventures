import { describe, expect, it } from "vitest";
import {
  Circuit,
  SpiceError,
  capacitor,
  bjt,
  currentSource,
  deviceModelNoiseAuditFixtures,
  formatCornerNoiseTable,
  formatNoiseTable,
  mosfet,
  noiseAc,
  noiseAcCorners,
  resistor,
  voltageSource,
} from "../src/index.js";

const BOLTZMANN = 1.380_649e-23;
const MOSFET_CHANNEL_NOISE_GAMMA = 2.0 / 3.0;

describe("noiseAc", () => {
  it("uses BJT forward beta roll-off to reduce shot noise", () => {
    function sourcePsd(forwardBetaRolloffCurrent: number): number {
      const circuit = new Circuit();
      circuit.add(voltageSource("Vcc", "vcc", "0", 5.0));
      circuit.add(voltageSource("Vbase", "base", "0", 0.65));
      circuit.add(resistor("Rload", "vcc", "out", 1_000.0));
      circuit.add({ ...bjt("Q1", "out", "base", "0"), forwardBetaRolloffCurrent });
      const entry = noiseAc(circuit, "out", "Vbase", [1_000.0], 300.0).points[0]?.entries
        .find((candidate) => candidate.elementName === "Q1");
      return entry!.sourcePsd;
    }

    expect(sourcePsd(1.0e-4)).toBeLessThan(sourcePsd(0.0));
  });
  it("uses BJT base-emitter leakage to increase shot noise", () => {
    function sourcePsd(baseEmitterLeakageSaturationCurrent: number): number {
      const circuit = new Circuit();
      circuit.add(voltageSource("Vbase", "base", "0", 0.65));
      circuit.add(resistor("Rload", "out", "0", 1_000.0));
      circuit.add({
        ...bjt("Q1", "out", "base", "0"),
        baseEmitterLeakageSaturationCurrent,
        baseEmitterLeakageEmissionCoefficient: 1.5,
      });
      const entry = noiseAc(circuit, "out", "Vbase", [1_000.0], 300.0).points[0]?.entries
        .find((candidate) => candidate.elementName === "Q1");
      return entry!.sourcePsd;
    }

    expect(sourcePsd(1.0e-10)).toBeGreaterThan(sourcePsd(0.0));
  });
  it("uses BJT base-collector leakage to increase shot noise", () => {
    function sourcePsd(baseCollectorLeakageSaturationCurrent: number): number {
      const circuit = new Circuit();
      circuit.add(voltageSource("Vbase", "base", "0", 0.65));
      circuit.add(resistor("Rload", "out", "0", 1_000.0));
      circuit.add({
        ...bjt("Q1", "out", "base", "base"),
        baseCollectorLeakageSaturationCurrent,
        baseCollectorLeakageEmissionCoefficient: 1.5,
      });
      const entry = noiseAc(circuit, "out", "Vbase", [1_000.0], 300.0).points[0]?.entries
        .find((candidate) => candidate.elementName === "Q1");
      return entry!.sourcePsd;
    }

    expect(sourcePsd(1.0e-10)).toBeGreaterThan(sourcePsd(0.0));
  });
  it("adds thermal noise for BJT emitter resistance", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vbase", "base", "0", 0.65));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));
    circuit.add({ ...bjt("Q1", "out", "base", "0"), emitterResistance: 100.0 });

    const entry = noiseAc(circuit, "out", "Vbase", [1_000.0], 300.0).points[0]!.entries
      .find((candidate) => candidate.elementName === "Q1:RE")!;

    expect(entry.noiseType).toBe("thermal");
    expect(entry.sourcePsd).toBeGreaterThan(0.0);
  });
  it("adds thermal noise for BJT collector resistance", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vbase", "base", "0", 0.65));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));
    circuit.add({ ...bjt("Q1", "out", "base", "0"), collectorResistance: 100.0 });

    const entry = noiseAc(circuit, "out", "Vbase", [1_000.0], 300.0).points[0]!.entries
      .find((candidate) => candidate.elementName === "Q1:RC")!;

    expect(entry.noiseType).toBe("thermal");
    expect(entry.sourcePsd).toBeGreaterThan(0.0);
  });
  it("uses BJT KF for inverse-frequency flicker noise", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vcc", "vcc", "0", 5.0));
    circuit.add(voltageSource("Vbase", "base", "0", 0.7));
    circuit.add(resistor("Rload", "vcc", "out", 1_000.0));
    circuit.add({
      ...bjt("Q1", "out", "base", "0"),
      flickerNoiseCoefficient: 1.0e-12,
    });

    const result = noiseAc(circuit, "out", "Vbase", [10.0, 1_000.0], 300.0);
    const flickerPsds = result.points.map((point) =>
      point.entries.find(
        (entry) => entry.elementName === "Q1" && entry.noiseType === "flicker",
      )!.sourcePsd,
    );

    expect(flickerPsds[0]).toBeGreaterThan(0.0);
    expect(flickerPsds[0] / flickerPsds[1]).toBeCloseTo(100.0, 12);
  });
  it("uses BJT AF as the base-current exponent", () => {
    function sourcePsd(exponent: number): number {
      const circuit = new Circuit();
      circuit.add(voltageSource("Vcc", "vcc", "0", 5.0));
      circuit.add(voltageSource("Vbase", "base", "0", 0.7));
      circuit.add(resistor("Rload", "vcc", "out", 1_000.0));
      circuit.add({
        ...bjt("Q1", "out", "base", "0"),
        flickerNoiseCoefficient: 1.0e-12,
        flickerNoiseExponent: exponent,
      });
      return noiseAc(circuit, "out", "Vbase", [1_000.0], 300.0).points[0]!.entries
        .find((entry) => entry.elementName === "Q1" && entry.noiseType === "flicker")!
        .sourcePsd;
    }

    expect(sourcePsd(2.0)).toBeLessThan(sourcePsd(1.0));
  });
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
    expect(formatNoiseTable(result)).toBe(
      "Index\tFrequency\tOutputNode\tInputSource\tOutputPSD\tInputReferredPSD\tElement\tType\tSourcePSD\tContributionPSD\n" +
        "0\t1.000000e+03\tout\tIin\t1.656779e-17\t1.656779e-23\tRload\tthermal\t1.656779e-23\t1.656779e-17\n",
    );
  });

  it("adds thermal noise for BJT base resistance", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vbase", "base", "0", 0.65));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));
    circuit.add({ ...bjt("Q1", "out", "base", "0"), baseResistance: 100.0 });
    const result = noiseAc(circuit, "out", "Vbase", [1_000.0]);
    const baseResistance = result.points[0]!.entries
      .find((candidate) => candidate.elementName === "Q1:RB")!;

    expect(baseResistance.noiseType).toBe("thermal");
    expect(baseResistance.sourcePsd).toBeGreaterThan(0.0);
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

  it("includes MOSFET channel thermal noise", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vdd", "vdd", "0", 5.0));
    circuit.add(voltageSource("Vgate", "gate", "0", 3.0));
    circuit.add(resistor("Rload", "vdd", "out", 1_000.0));
    circuit.add(mosfet("M1", "out", "gate", "0", "0", "NMOS", {
      VT0: 1.0,
      KP: 1.0e-3,
      LAMBDA: 0.0,
      GAMMA: 0.0,
      W: 1.0,
      L: 1.0,
    }));

    const point = noiseAc(circuit, "out", "Vgate", [1_000.0], 300.0).points[0];
    const entry = point.entries.find((candidate) => candidate.elementName === "M1");
    const gm = 1.0e-3 * (3.0 - 1.0);
    const expectedSourcePsd =
      4.0 * BOLTZMANN * 300.0 * MOSFET_CHANNEL_NOISE_GAMMA * gm;

    expect(entry).toBeDefined();
    expect(entry?.noiseType).toBe("thermal");
    expect(entry?.sourcePsd).toBeCloseTo(expectedSourcePsd, 30);
    expect(entry?.outputPsd).toBeCloseTo(expectedSourcePsd * 1_000.0 ** 2, 30);
  });

  it("runs device model noise audit fixtures as reference noise points", () => {
    const fixtures = deviceModelNoiseAuditFixtures();
    expect(fixtures.map((fixture) => fixture.name)).toStrictEqual([
      "diode-shot-noise",
      "bjt-shot-noise",
      "jfet-channel-noise",
      "mos-level1-channel-noise",
    ]);

    for (const fixture of fixtures) {
      const result = noiseAc(
        fixture.circuit,
        fixture.outputNode,
        fixture.inputSource,
        [fixture.frequencyHz],
        300.0,
      );
      const entry = result.points[0]?.entries.find(
        (candidate) => candidate.elementName === fixture.expectedNoiseElement,
      );
      expect(entry).toBeDefined();
      expect(entry?.noiseType).toBe(fixture.expectedNoiseType);
      expect(entry!.sourcePsd).toBeGreaterThanOrEqual(fixture.expectedSourcePsdMin);
      expect(entry!.sourcePsd).toBeLessThanOrEqual(fixture.expectedSourcePsdMax);
      expect(entry!.outputPsd).toBeGreaterThanOrEqual(fixture.expectedOutputPsdMin);
      expect(entry!.outputPsd).toBeLessThanOrEqual(fixture.expectedOutputPsdMax);
      expect(fixture.deckLines[0]!.startsWith("* device-model noise fixture:")).toBe(true);
      expect(fixture.deckLines.some((line) => line.startsWith(".model "))).toBe(true);
      expect(fixture.deckLines.some((line) => line.startsWith(".noise "))).toBe(true);
      expect(fixture.noiseBehavior.length).toBeGreaterThan(0);
    }
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

  it("runs noise analysis at each named corner and formats stable tables", () => {
    const circuit = new Circuit();
    circuit.add(currentSource("Iin", "0", "out", 0.0));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    const result = noiseAcCorners(
      circuit,
      "out",
      "Iin",
      [
        { name: "nominal", overrides: [] },
        {
          name: "rload-high",
          overrides: [{ elementName: "Rload", parameter: "resistance", value: 2_000.0 }],
        },
      ],
      [1_000.0],
      300.0,
    );

    expect(result.outputNode).toBe("out");
    expect(result.inputSource).toBe("Iin");
    expect(result.points.map((point) => point.cornerName)).toEqual([
      "nominal",
      "rload-high",
    ]);
    expect(result.points[0].result.points[0].outputPsd).toBeCloseTo(1.6567788e-17, 24);
    expect(result.points[1].result.points[0].outputPsd).toBeCloseTo(3.3135576e-17, 24);
    expect(formatCornerNoiseTable(result)).toBe(
      "Corner\tIndex\tFrequency\tOutputNode\tInputSource\tOutputPSD\tInputReferredPSD\tElement\tType\tSourcePSD\tContributionPSD\n" +
        "nominal\t0\t1.000000e+03\tout\tIin\t1.656779e-17\t1.656779e-23\tRload\tthermal\t1.656779e-23\t1.656779e-17\n" +
        "rload-high\t0\t1.000000e+03\tout\tIin\t3.313558e-17\t8.283894e-24\tRload\tthermal\t8.283894e-24\t3.313558e-17\n",
    );
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
