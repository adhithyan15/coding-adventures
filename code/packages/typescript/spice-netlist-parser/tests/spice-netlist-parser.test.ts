import {
  acSweep,
  dcOp,
  mcDc,
  noiseAc,
  sensDc,
  tf,
  transientAdaptive,
} from "@coding-adventures/spice-engine";
import { describe, expect, it } from "vitest";
import {
  NetlistParseError,
  buildAnalysisPlan,
  parseNetlist,
  parseValue,
  runNetlist,
} from "../src/index.js";

describe("parseNetlist", () => {
  it("parses a linear operating-point netlist into a circuit", () => {
    const parsed = parseNetlist(`
* resistor divider
V1 vin 0 DC 10
R1 vin mid 1k
R2 mid 0 1k
.op
.end
`);

    expect(parsed.title).toBe("resistor divider");
    expect(parsed.circuit.elements().map((element) => element.kind)).toEqual([
      "voltage-source",
      "resistor",
      "resistor",
    ]);
    expect(parsed.opCards()).toEqual([{ kind: "op" }]);

    const result = dcOp(parsed.circuit);
    expect(result.voltage("mid")).toBeCloseTo(5.0, 9);
  });

  it("builds and runs core analysis plans", () => {
    const deck = `
V1 in 0 DC 1 AC 1
R1 in out 1k
R2 out 0 1k
C1 out 0 1u IC=0
.options method=trap
.op
.dc V1 0 1 0.5
.ac dec 1 1k 1k
.tran 1m 1m
.end
`;
    const parsed = parseNetlist(deck);

    const plan = parsed.analysisPlan();
    expect(plan).toEqual(buildAnalysisPlan(parsed));
    expect(plan.map((step) => [step.index, step.kind])).toEqual([
      [1, "op"],
      [2, "dc"],
      [3, "ac"],
      [4, "tran"],
    ]);

    const results = parsed.runAnalysisPlan();
    expect(results.map((result) => result.kind)).toEqual(["op", "dc", "ac", "tran"]);
    expect((results[0].result as { voltage(node: string): number | undefined }).voltage("out"))
      .toBeCloseTo(0.5, 9);
    const dcPoints = results[1].result as readonly {
      result: { voltage(node: string): number | undefined };
    }[];
    expect(dcPoints).toHaveLength(3);
    expect(dcPoints.at(-1)?.result.voltage("out")).toBeCloseTo(0.5, 9);
    const acPoints = results[2].result as readonly {
      voltage(node: string): { real: number; imag: number } | undefined;
    }[];
    expect(acPoints).toHaveLength(1);
    expect(acPoints[0].voltage("out")?.real ?? 0.0).toBeGreaterThan(0.0);
    const transientPoints = results[3].result as readonly {
      voltage(node: string): number | undefined;
    }[];
    expect(transientPoints).toHaveLength(1);
    expect(transientPoints[0].voltage("out")).toBeGreaterThan(0.0);

    expect(runNetlist(deck)).toHaveLength(4);
  });

  it("parses reactive elements, VCCS, source waveforms, and analysis cards", () => {
    const parsed = parseNetlist(`
Vstep in 0 PULSE(0 1 0 1n 1n 10n 20n)
I1 out 0 1m
Rload in out 2.2k
Cload out 0 10p IC=2.5
L1 out 0 1u IC=3m
G1 out 0 in 0 2m
.tran 1n 20n
.dc Vstep 0 1 0.5
.ac dec 10 1k 1meg
`);

    const elements = parsed.circuit.elements();
    expect(elements.map((element) => element.kind)).toEqual([
      "voltage-source",
      "current-source",
      "resistor",
      "capacitor",
      "inductor",
      "vccs",
    ]);
    expect(elements[0]).toMatchObject({ kind: "voltage-source", waveform: expect.any(Object) });
    expect(elements[3]).toMatchObject({ kind: "capacitor", initialVoltage: 2.5 });
    expect(elements[4]).toMatchObject({ kind: "inductor", initialCurrent: 3.0e-3 });
    expect(parsed.analyses).toEqual([
      { kind: "tran", timeStep: 1.0e-9, stopTime: 20.0e-9 },
      { kind: "dc", sourceName: "Vstep", start: 0.0, stop: 1.0, step: 0.5 },
      { kind: "ac", mode: "dec", points: 10, startHz: 1.0e3, stopHz: 1.0e6 },
    ]);
  });

  it("parses mutual-inductor K cards", () => {
    const parsed = parseNetlist(`
Lpri p 0 10m
Lsec s 0 40m
Kcouple Lpri Lsec 0.75
`);

    expect(parsed.circuit.elements()[2]).toMatchObject({
      kind: "mutual-inductor",
      name: "Kcouple",
      primary: "Lpri",
      secondary: "Lsec",
      coupling: 0.75,
    });
  });

  it("rejects mutual-inductor cards with missing referenced inductors", () => {
    expect(() => parseNetlist(`
Lpri p 0 10m
Kbad Lpri Lmissing 0.75
`)).toThrow(NetlistParseError);
  });

  it("rejects mutual-inductor cards with non-finite coupling", () => {
    expect(() => parseNetlist(`
Lpri p 0 10m
Lsec s 0 40m
Kbad Lpri Lsec 1e999
`)).toThrow(NetlistParseError);
  });

  it("parses transmission-line T cards", () => {
    const parsed = parseNetlist(`
Tdelay in 0 out 0 Z0=50 TD=1n
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "transmission-line",
      name: "Tdelay",
      n1: "in",
      n2: "0",
      n3: "out",
      n4: "0",
      characteristicImpedanceOhms: 50.0,
      delaySeconds: 1.0e-9,
    });
  });

  it("rejects unsupported transmission-line positional forms", () => {
    expect(() => parseNetlist("Tdelay in 0 out 0 50 1n")).toThrow(
      /invalid transmission line parameter syntax/,
    );
  });

  it("rejects transmission-line cards with missing parameters", () => {
    expect(() => parseNetlist("Tdelay in 0 out 0 Z0=50")).toThrow(/requires TD/);
  });

  it("rejects transmission-line cards with non-positive parameters", () => {
    expect(() => parseNetlist("Tdelay in 0 out 0 Z0=0 TD=1n")).toThrow(
      /characteristic impedance must be positive/,
    );
    expect(() => parseNetlist("Tdelay in 0 out 0 Z0=50 TD=0")).toThrow(
      /delay must be positive/,
    );
  });

  it("parses .options analysis cards", () => {
    const parsed = parseNetlist(`
.options reltol=1m abstol=1n gmin=1p method=trap noopiter
`);

    const card = {
      kind: "options",
      values: new Map<string, number | string | boolean>([
        ["reltol", 1.0e-3],
        ["abstol", 1.0e-9],
        ["gmin", 1.0e-12],
        ["method", "trap"],
        ["noopiter", true],
      ]),
    };
    expect(parsed.analyses).toEqual([card]);
    expect(parsed.optionsCards()).toEqual([card]);
  });

  it("builds engine call options from .options cards", () => {
    const parsed = parseNetlist(`
V1 vin 0 DC 10
R1 vin mid 1k
R2 mid 0 1k
.options reltol=1u itl1=7 gmin=1p method=gear2 trtol=2m minstep=1n maxstep=5n
.op
.tran 1n 2n
`);
    const tran = parsed.tranCards()[0];

    expect(parsed.dcOpOptions()).toEqual({
      tolerance: 1.0e-6,
      maxIterations: 7,
      pseudoTransientConductance: 1.0e-12,
    });
    const result = dcOp(parsed.circuit, parsed.dcOpOptions());
    expect(result.voltage("mid")).toBeCloseTo(5.0, 9);

    expect(parsed.adaptiveTransientOptions(tran)).toEqual({
      method: "gear2",
      tolerance: 2.0e-3,
      minStep: 1.0e-9,
      maxStep: 5.0e-9,
    });
    const transient = transientAdaptive(
      parsed.circuit,
      tran.timeStep,
      tran.stopTime,
      parsed.adaptiveTransientOptions(tran),
    );
    expect(transient.converged).toBe(true);
    expect(transient.method).toBe("gear2");
  });

  it("parses .temp analysis cards", () => {
    const parsed = parseNetlist(".temp 27 75 -40");
    const card = { kind: "temp", temperaturesCelsius: [27, 75, -40] };

    expect(parsed.analyses).toEqual([card]);
    expect(parsed.tempCards()).toEqual([card]);
    expect(parsed.operatingTemperatureKelvin()).toBeCloseTo(300.15, 12);
    expect(parsed.operatingTemperatureKelvin(1)).toBeCloseTo(348.15, 12);
  });

  it("defaults operating temperature without .temp cards", () => {
    const parsed = parseNetlist("R1 in out 1k");

    expect(parsed.operatingTemperatureKelvin(0, 301.0)).toBe(301.0);
    expect(() => parseNetlist(".temp 27").operatingTemperatureKelvin(3)).toThrow(
      /temperature index 3 exceeds \.temp entries/,
    );
  });

  it("rejects .temp cards without temperatures", () => {
    expect(() => parseNetlist(".temp")).toThrow(/\.temp expects at least 2 fields/);
  });

  it("parses .print and .plot output cards", () => {
    const parsed = parseNetlist(`
.print TRAN V(out) I(Vin)
.plot ac V(in) V(out)
`);

    const printCard = {
      kind: "print",
      analysis: "tran",
      probes: [
        { kind: "voltage", target: "out" },
        { kind: "current", target: "Vin" },
      ],
    };
    const plotCard = {
      kind: "plot",
      analysis: "ac",
      probes: [
        { kind: "voltage", target: "in" },
        { kind: "voltage", target: "out" },
      ],
    };
    expect(parsed.analyses).toEqual([printCard, plotCard]);
    expect(parsed.printCards()).toEqual([printCard]);
    expect(parsed.plotCards()).toEqual([plotCard]);
  });

  it("parses .save, .probe, and .measure cards", () => {
    const parsed = parseNetlist(`
.save V(out) I(Vin)
.probe tran V(out)
.measure tran peak MAX V(out) FROM=0 TO=1m
`);

    const saveCard = {
      kind: "save",
      probes: [
        { kind: "voltage", target: "out" },
        { kind: "current", target: "Vin" },
      ],
    };
    const probeCard = {
      kind: "probe",
      analysis: "tran",
      probes: [{ kind: "voltage", target: "out" }],
    };
    const measureCard = {
      kind: "measure",
      analysis: "tran",
      name: "peak",
      operation: "max",
      probe: { kind: "voltage", target: "out" },
      start: 0,
      stop: 1.0e-3,
    };

    expect(parsed.analyses).toEqual([saveCard, probeCard, measureCard]);
    expect(parsed.saveCards()).toEqual([saveCard]);
    expect(parsed.probeCards()).toEqual([probeCard]);
    expect(parsed.measureCards()).toEqual([measureCard]);
  });

  it("rejects output cards with missing or unknown probes", () => {
    expect(() => parseNetlist(".print tran")).toThrow(/\.print expects at least 3 fields/);
    expect(() => parseNetlist(".plot tran P(out)")).toThrow(
      /\.plot probe must be V\(node\) or I\(source\)/,
    );
    expect(() => parseNetlist(".save P(out)")).toThrow(
      /\.save probe must be V\(node\) or I\(source\)/,
    );
    expect(() => parseNetlist(".probe tran")).toThrow(
      /\.probe probe must be V\(node\) or I\(source\)/,
    );
    expect(() => parseNetlist(".measure tran final FIND V(out)")).toThrow(
      /\.measure FIND requires AT=<value>/,
    );
    expect(() => parseNetlist(".measure tran peak PEAK V(out) AT=1m")).toThrow(
      /\.measure operation must be FIND/,
    );
  });

  it("selects outputs and evaluates .measure results from analysis plans", () => {
    const deck = `
V1 in 0 DC 1 AC 1
R1 in out 1k
R2 out 0 1k
C1 out 0 1u IC=0
.save V(out)
.print dc V(in)
.probe tran I(V1)
.measure dc half FIND V(out) AT=1
.measure tran final FIND V(out) AT=1m
.measure tran average AVG V(out)
.op
.dc V1 0 1 0.5
.ac dec 1 1k 1k
.tran 1m 1m
.end
`;
    const parsed = parseNetlist(deck);
    const results = parsed.runAnalysisPlan();

    const outputs = parsed.selectOutputs(results);
    expect(outputs.map((output) => output.kind)).toEqual(["op", "dc", "ac", "tran"]);
    expect(outputs[0].rows[0].values.get("V(out)") as number).toBeCloseTo(0.5, 9);
    expect(Array.from(outputs[1].rows.at(-1)!.values.keys())).toEqual(["V(out)", "V(in)"]);
    expect(outputs[1].rows.at(-1)!.values.get("V(in)") as number).toBeCloseTo(1.0, 9);
    expect(outputs[2].rows[0].values.get("V(out)")).toMatchObject({ real: expect.any(Number) });
    expect(outputs[3].rows.at(-1)!.values.has("I(V1)")).toBe(true);

    const measures = parsed.measureResults(results);
    expect(measures.map((measure) => measure.name)).toEqual(["half", "final", "average"]);
    expect(measures[0].value).toBeCloseTo(0.5, 9);
    expect(measures[1].value).toBeCloseTo(outputs[3].rows.at(-1)!.values.get("V(out)") as number, 9);
    expect(measures[2].value).toBeGreaterThan(0.0);
    expect(measures[2].value).toBeLessThanOrEqual(outputs[3].rows.at(-1)!.values.get("V(out)") as number);
  });

  it("parses .four analysis cards", () => {
    const parsed = parseNetlist(".four 1k V(out) I(Vin)");
    const card = {
      kind: "four",
      frequencyHz: 1000,
      probes: [
        { kind: "voltage", target: "out" },
        { kind: "current", target: "Vin" },
      ],
    };

    expect(parsed.analyses).toEqual([card]);
    expect(parsed.fourCards()).toEqual([card]);
  });

  it("rejects .four cards with missing or unknown probes", () => {
    expect(() => parseNetlist(".four 1k")).toThrow(/\.four expects at least 3 fields/);
    expect(() => parseNetlist(".four 1k P(out)")).toThrow(
      /\.four probe must be V\(node\) or I\(source\)/,
    );
  });

  it("parses .disto and .pz analysis cards", () => {
    const parsed = parseNetlist(`
.disto dec 5 1k 1meg V(out) I(Vin)
.pz V(out) Vin pole
`);
    const distoCard = {
      kind: "disto",
      mode: "dec",
      points: 5,
      startHz: 1000,
      stopHz: 1.0e6,
      probes: [
        { kind: "voltage", target: "out" },
        { kind: "current", target: "Vin" },
      ],
    };
    const pzCard = {
      kind: "pz",
      outputNode: "out",
      inputSource: "Vin",
      poleZeroKind: "pole",
    };

    expect(parsed.analyses).toEqual([distoCard, pzCard]);
    expect(parsed.distortionCards()).toEqual([distoCard]);
    expect(parsed.poleZeroCards()).toEqual([pzCard]);
  });

  it("rejects .disto and .pz cards with invalid shapes", () => {
    expect(() => parseNetlist(".disto dec 5 1k 1meg")).toThrow(
      /\.disto expects at least 6 fields/,
    );
    expect(() => parseNetlist(".disto dec 5 1k 1meg P(out)")).toThrow(
      /\.disto probe must be V\(node\) or I\(source\)/,
    );
    expect(() => parseNetlist(".pz out Vin")).toThrow(/\.pz output must be a voltage probe/);
    expect(() => parseNetlist(".pz V(out) Vin residue")).toThrow(/\.pz kind must be/);
  });

  it("parses transient methods from .tran cards", () => {
    const parsed = parseNetlist(".tran 1n 20n method=gear2");

    expect(parsed.tranCards()).toEqual([
      { kind: "tran", timeStep: 1.0e-9, stopTime: 20.0e-9, method: "gear2" },
    ]);
    expect(parsed.transientMethod(parsed.tranCards()[0])).toBe("gear2");
  });

  it("falls back to .options method and lets .tran take precedence", () => {
    const parsed = parseNetlist(`
.options method=trap
.tran 1n 20n method=euler
`);

    expect(parsed.optionsCards()[0].values.get("method")).toBe("trap");
    expect(parsed.transientMethod()).toBe("trap");
    expect(parsed.transientMethod(parsed.tranCards()[0])).toBe("euler");
  });

  it("rejects unsupported transient method values", () => {
    expect(() => parseNetlist(".tran 1n 20n method=bogus")).toThrow(
      /must be euler, trap, or gear2/,
    );
    expect(() => parseNetlist(".options method=bogus")).toThrow(
      /must be euler, trap, or gear2/,
    );
  });

  it("rejects .options cards with empty values", () => {
    expect(() => parseNetlist(".options gmin=")).toThrow(/\.options "gmin" requires a value/);
  });

  it("rejects unsupported capacitor element parameters", () => {
    expect(() => parseNetlist("C1 in 0 1u FOO=1")).toThrow(
      /unsupported capacitor parameter/,
    );
  });

  it("rejects unsupported inductor element parameters", () => {
    expect(() => parseNetlist("L1 in 0 1u FOO=1")).toThrow(
      /unsupported inductor parameter/,
    );
  });

  it("parses independent-source AC specs separately from DC bias", () => {
    const parsed = parseNetlist(`
Vin in 0 DC 10 AC 2 90
Vbias bias 0 5
R1 in out 1k
R2 out 0 1k
.ac dec 10 1k 1k
`);

    const vin = parsed.circuit.elements()[0];
    expect(vin).toMatchObject({
      kind: "voltage-source",
      voltage: 10.0,
      ac: { magnitude: 2.0, phaseDegrees: 90.0 },
    });
    const bias = parsed.circuit.elements()[1];
    expect(bias).toMatchObject({ kind: "voltage-source", voltage: 5.0 });

    const points = acSweep(parsed.circuit, 1_000.0, 1_000.0, 10);
    const out = points[0].voltage("out");
    const biasVoltage = points[0].voltage("bias");
    expect(out).not.toBeUndefined();
    expect(biasVoltage).not.toBeUndefined();
    expect(out!.real).toBeCloseTo(0.0, 9);
    expect(out!.imag).toBeCloseTo(1.0, 9);
    expect(biasVoltage!.real).toBeCloseTo(0.0, 9);
    expect(biasVoltage!.imag).toBeCloseTo(0.0, 9);
  });

  it("parses .tf transfer-function analysis cards", () => {
    const parsed = parseNetlist(`
Vin in 0 DC 1
R1 in out 1k
R2 out 0 1k
.tf V(out) Vin
`);

    expect(parsed.analyses).toEqual([
      { kind: "tf", outputNode: "out", inputSource: "Vin" },
    ]);
    expect(parsed.tfCards()).toEqual([
      { kind: "tf", outputNode: "out", inputSource: "Vin" },
    ]);
    const [card] = parsed.tfCards();
    const result = tf(parsed.circuit, card.outputNode, card.inputSource);
    expect(result.transferRatio).toBeCloseTo(0.5, 9);
  });

  it("rejects .tf cards without a voltage output probe", () => {
    expect(() =>
      parseNetlist(`
Vin in 0 DC 1
R1 in out 1k
.tf out Vin
`),
    ).toThrow(/\.tf output must be a voltage probe/);
  });

  it("parses .sens DC sensitivity analysis cards", () => {
    const parsed = parseNetlist(`
Vin in 0 DC 1
Rtop in out 1k
Rbot out 0 1k
.sens V(out)
`);

    expect(parsed.analyses).toEqual([{ kind: "sens", outputNode: "out" }]);
    expect(parsed.sensCards()).toEqual([{ kind: "sens", outputNode: "out" }]);
    const [card] = parsed.sensCards();
    const result = sensDc(parsed.circuit, card.outputNode);
    expect(result.nominalVoltage).toBeCloseTo(0.5, 9);
    expect(result.entry("Vin", "voltage")).not.toBeUndefined();
  });

  it("rejects .sens cards without a voltage output probe", () => {
    expect(() =>
      parseNetlist(`
Vin in 0 DC 1
R1 in out 1k
.sens out
`),
    ).toThrow(/\.sens output must be a voltage probe/);
  });

  it("parses .mc Monte Carlo DC analysis cards", () => {
    const parsed = parseNetlist(`
Vin in 0 DC 1
Rtop in out 1k
Rbot out 0 1k
.mc V(out) 6 0 uniform 7
`);

    expect(parsed.analyses).toEqual([
      {
        kind: "mc",
        outputNode: "out",
        nTrials: 6,
        tolerance: 0.0,
        distribution: "uniform",
        seed: 7,
      },
    ]);
    expect(parsed.mcCards()).toEqual(parsed.analyses);
    const [card] = parsed.mcCards();
    const result = mcDc(parsed.circuit, card.outputNode, card.nTrials, {
      tolerance: card.tolerance,
      distribution: card.distribution,
      seed: card.seed,
    });
    expect(result.nTrials).toBe(6);
    expect(result.mean).toBeCloseTo(0.5, 9);
    expect(result.stdDev).toBeCloseTo(0.0, 12);
  });

  it("rejects .mc cards without a voltage output probe", () => {
    expect(() =>
      parseNetlist(`
Vin in 0 DC 1
R1 in out 1k
.mc out 10
`),
    ).toThrow(/\.mc output must be a voltage probe/);
  });

  it("parses .noise AC noise analysis cards", () => {
    const parsed = parseNetlist(`
.temp 75
Vin in 0 DC 1
Rtop in out 1k
Rbot out 0 1k
.noise V(out) Vin 1k temp=300
`);

    expect(parsed.analyses).toEqual([
      {
        kind: "temp",
        temperaturesCelsius: [75.0],
      },
      {
        kind: "noise",
        outputNode: "out",
        inputSource: "Vin",
        frequenciesHz: [1000.0],
        temperature: 300.0,
        temperatureIsExplicit: true,
      },
    ]);
    expect(parsed.noiseCards()).toEqual([parsed.analyses[1]]);
    const [card] = parsed.noiseCards();
    expect(parsed.noiseTemperatureKelvin(card)).toBe(300.0);
    const result = noiseAc(
      parsed.circuit,
      card.outputNode,
      card.inputSource,
      card.frequenciesHz,
      parsed.noiseTemperatureKelvin(card),
    );
    expect(result.outputNode).toBe("out");
    expect(result.inputSource).toBe("Vin");
    expect(result.points).toHaveLength(1);
    expect(result.points[0].outputPsd).toBeGreaterThan(0.0);
  });

  it("uses .temp for noise analysis when .noise omits temp", () => {
    const parsed = parseNetlist(`
.temp 50
Vin in 0 DC 1
Rtop in out 1k
Rbot out 0 1k
.noise V(out) Vin 1k
`);
    const [card] = parsed.noiseCards();

    expect(card.temperatureIsExplicit).toBeUndefined();
    expect(parsed.noiseTemperatureKelvin(card)).toBeCloseTo(323.15, 12);
  });

  it("rejects .noise cards without a voltage output probe", () => {
    expect(() =>
      parseNetlist(`
Vin in 0 DC 1
R1 in out 1k
.noise out Vin 1k
`),
    ).toThrow(/\.noise output must be a voltage probe/);
  });

  it("parses VCVS elements into operating-point circuits", () => {
    const parsed = parseNetlist(`
Vctrl in 0 DC 1.5
Eamp out 0 in 0 4
Rload out 0 1k
.op
`);

    const elements = parsed.circuit.elements();
    expect(elements[1]).toMatchObject({
      kind: "vcvs",
      controlPositive: "in",
      gain: 4.0,
    });

    const result = dcOp(parsed.circuit);
    expect(result.voltage("out")).toBeCloseTo(6.0, 9);
  });

  it("parses CCCS elements into operating-point circuits", () => {
    const parsed = parseNetlist(`
Vin in 0 DC 1
Rin in sense 1k
Vsense sense 0 DC 0
Fcopy out 0 Vsense 2
Rload out 0 500
.op
`);

    const elements = parsed.circuit.elements();
    expect(elements[3]).toMatchObject({
      kind: "cccs",
      controlSource: "Vsense",
      gain: 2.0,
    });

    const result = dcOp(parsed.circuit);
    expect(result.voltage("out")).toBeCloseTo(-1.0, 9);
  });

  it("parses CCVS elements into operating-point circuits", () => {
    const parsed = parseNetlist(`
Vin in 0 DC 1
Rin in sense 1k
Vsense sense 0 DC 0
Hamp out 0 Vsense 1k
Rload out 0 500
.op
`);

    const elements = parsed.circuit.elements();
    expect(elements[3]).toMatchObject({
      kind: "ccvs",
      controlSource: "Vsense",
      transresistanceOhms: 1000.0,
    });

    const result = dcOp(parsed.circuit);
    expect(result.voltage("out")).toBeCloseTo(1.0, 9);
  });

  it("parses diode models into operating-point circuits", () => {
    const parsed = parseNetlist(`
.model fast D(IS=1e-12 VT=25m N=2 BV=5 IBV=1u CJO=2p TT=4n RS=3)
V1 in 0 DC 0.7
D1 in out fast
Rload out 0 1k
.op
`);

    expect(parsed.models.get("fast")).toEqual({
      name: "fast",
      kind: "D",
      params: new Map([
        ["IS", 1.0e-12],
        ["VT", 25.0e-3],
        ["N", 2.0],
        ["BV", 5.0],
        ["IBV", 1.0e-6],
        ["CJO", 2.0e-12],
        ["TT", 4.0e-9],
        ["RS", 3.0],
      ]),
    });
    expect(parsed.circuit.elements()[1]).toMatchObject({
      kind: "diode",
      name: "D1",
      anode: "in",
      cathode: "out",
      saturationCurrent: 1.0e-12,
      thermalVoltage: 25.0e-3,
      emissionCoefficient: 2.0,
      breakdownVoltage: 5.0,
      breakdownCurrent: 1.0e-6,
      junctionCapacitance: 2.0e-12,
      transitTime: 4.0e-9,
      seriesResistance: 3.0,
    });

    const result = dcOp(parsed.circuit);
    expect(result.voltage("out")).toBeGreaterThan(0.0);
    expect(result.voltage("out")).toBeLessThan(0.7);
  });

  it("parses the DIODE model type alias", () => {
    const parsed = parseNetlist(".model clamp DIODE(IS=2p)\nD1 in out clamp");

    expect(parsed.models.get("clamp")?.kind).toBe("D");
    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "diode",
      saturationCurrent: 2.0e-12,
    });
  });

  it("parses the diode JS saturation-current alias", () => {
    const parsed = parseNetlist(`
.model clamp D(JS=2p)
D1 in out clamp
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "diode",
      saturationCurrent: 2.0e-12,
    });
  });

  it.each([
    ["IS", "0"],
    ["IS", "-1p"],
    ["IS", "1e999"],
    ["JS", "0"],
    ["JS", "-1p"],
    ["JS", "1e999"],
  ])("rejects invalid diode %s saturation current %s", (parameter, value) => {
    expect(() => parseNetlist(`.model clamp D(${parameter}=${value})`)).toThrow(
      "diode IS must be finite and positive",
    );
  });

  it("parses the diode CJ junction-capacitance alias", () => {
    const parsed = parseNetlist(`
.model clamp D(CJ=3p)
D1 in out clamp
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "diode",
      junctionCapacitance: 3.0e-12,
    });
  });

  it.each([
    ["CJO", "-1p"],
    ["CJO", "1e999"],
    ["CJ", "-1p"],
    ["CJ", "1e999"],
    ["CJ0", "-1p"],
    ["CJ0", "1e999"],
  ])("rejects invalid diode %s junction capacitance %s", (parameter, value) => {
    expect(() => parseNetlist(`.model clamp D(${parameter}=${value})`)).toThrow(
      "diode CJO must be finite and non-negative",
    );
  });

  it.each([
    ["0", 0.0],
    ["4n", 4.0e-9],
  ])("accepts valid diode TT transit time %s", (value, expected) => {
    const parsed = parseNetlist(`.model clamp D(TT=${value})\nD1 in out clamp`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "diode",
      transitTime: expected,
    });
  });

  it.each(["-1n", "1e999"])("rejects invalid diode TT transit time %s", (value) => {
    expect(() => parseNetlist(`.model clamp D(TT=${value})`)).toThrow(
      "diode TT must be finite and non-negative",
    );
  });

  it("parses the diode V_T thermal-voltage alias", () => {
    const parsed = parseNetlist(`
.model clamp D(V_T=27m)
D1 in out clamp
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "diode",
      thermalVoltage: 27.0e-3,
    });
  });

  it.each([
    ["VT", "0"],
    ["VT", "-1m"],
    ["VT", "1e999"],
    ["V_T", "0"],
    ["V_T", "-1m"],
    ["V_T", "1e999"],
  ])("rejects invalid diode %s thermal voltage %s", (parameter, value) => {
    expect(() => parseNetlist(`.model clamp D(${parameter}=${value})`)).toThrow(
      "diode VT must be finite and positive",
    );
  });

  it.each([
    ["0.5", 0.5],
    ["2", 2.0],
  ])("accepts valid diode N emission coefficient %s", (value, expected) => {
    const parsed = parseNetlist(`.model clamp D(N=${value})\nD1 in out clamp`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "diode",
      emissionCoefficient: expected,
    });
  });

  it.each(["0", "-1", "1e999"])(
    "rejects invalid diode N emission coefficient %s",
    (value) => {
      expect(() => parseNetlist(`.model clamp D(N=${value})`)).toThrow(
        "diode N must be finite and positive",
      );
    },
  );

  it.each([
    ["1", 1.0],
    ["5.5", 5.5],
  ])("accepts valid diode BV breakdown voltage %s", (value, expected) => {
    const parsed = parseNetlist(`.model clamp D(BV=${value})\nD1 in out clamp`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "diode",
      breakdownVoltage: expected,
    });
  });

  it.each(["0", "-1", "1e999"])("rejects invalid diode BV %s", (value) => {
    expect(() => parseNetlist(`.model clamp D(BV=${value})`)).toThrow(
      "diode BV must be finite and positive",
    );
  });

  it.each([
    ["1u", 1.0e-6],
    ["2m", 2.0e-3],
  ])("accepts valid diode IBV breakdown current %s", (value, expected) => {
    const parsed = parseNetlist(`.model clamp D(IBV=${value})\nD1 in out clamp`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "diode",
      breakdownCurrent: expected,
    });
  });

  it.each(["0", "-1u", "1e999"])("rejects invalid diode IBV %s", (value) => {
    expect(() => parseNetlist(`.model clamp D(IBV=${value})`)).toThrow(
      "diode IBV must be finite and positive",
    );
  });

  it.each([
    ["0", 0.0],
    ["2.5", 2.5],
  ])("accepts valid diode RS series resistance %s", (value, expected) => {
    const parsed = parseNetlist(`.model clamp D(RS=${value})\nD1 in out clamp`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "diode",
      seriesResistance: expected,
    });
  });

  it.each(["-1", "1e999"])("rejects invalid diode RS %s", (value) => {
    expect(() => parseNetlist(`.model clamp D(RS=${value})`)).toThrow(
      "diode RS must be finite and non-negative",
    );
  });

  it.each(["VJ", "PB"])("lowers diode %s junction potential alias", (parameter) => {
    const parsed = parseNetlist(`.model clamp D(${parameter}=0.8)\nD1 in out clamp`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "diode",
      junctionPotential: 0.8,
    });
  });

  it("prefers canonical diode VJ over PB", () => {
    const parsed = parseNetlist(`.model clamp D(PB=0.8 VJ=0.7)\nD1 in out clamp`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "diode",
      junctionPotential: 0.7,
    });
  });

  it.each([
    ["VJ", "0"],
    ["VJ", "-0.1"],
    ["VJ", "1e999"],
    ["PB", "0"],
    ["PB", "-0.1"],
    ["PB", "1e999"],
  ])("rejects invalid diode %s junction potential %s", (parameter, value) => {
    expect(() => parseNetlist(`.model clamp D(${parameter}=${value})`)).toThrow(
      "diode VJ must be finite and positive",
    );
  });

  it.each(["M", "MJ"])("lowers diode %s grading coefficient alias", (parameter) => {
    const parsed = parseNetlist(`.model clamp D(${parameter}=0.4)\nD1 in out clamp`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "diode",
      gradingCoefficient: 0.4,
    });
  });

  it("prefers canonical diode M over MJ", () => {
    const parsed = parseNetlist(`.model clamp D(MJ=0.4 M=0.3)\nD1 in out clamp`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "diode",
      gradingCoefficient: 0.3,
    });
  });

  it.each([
    ["M", "-0.1"],
    ["M", "1e999"],
    ["MJ", "-0.1"],
    ["MJ", "1e999"],
  ])("rejects invalid diode %s grading coefficient %s", (parameter, value) => {
    expect(() => parseNetlist(`.model clamp D(${parameter}=${value})`)).toThrow(
      "diode M must be finite and non-negative",
    );
  });

  it.each([
    ["0", 0.0],
    ["0.6", 0.6],
  ])("lowers valid diode FC depletion coefficient %s", (value, expected) => {
    const parsed = parseNetlist(`.model clamp D(FC=${value})\nD1 in out clamp`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "diode",
      forwardBiasDepletionCoefficient: expected,
    });
  });

  it.each(["-0.1", "1", "1e999"])(
    "rejects invalid diode FC depletion coefficient %s",
    (value) => {
      expect(() => parseNetlist(`.model clamp D(FC=${value})`)).toThrow(
        "diode FC must be finite and in [0, 1)",
      );
    },
  );

  it.each([
    ["-1", -1.0],
    ["4", 4.0],
  ])("lowers finite diode XTI temperature exponent %s", (value, expected) => {
    const parsed = parseNetlist(`.model clamp D(XTI=${value})\nD1 in out clamp`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "diode",
      saturationCurrentTemperatureExponent: expected,
    });
  });

  it("rejects non-finite diode XTI temperature exponent", () => {
    expect(() => parseNetlist(`.model clamp D(XTI=1e999)`)).toThrow(
      "diode XTI must be finite",
    );
  });

  it.each([
    ["0.5", 0.5],
    ["1.2", 1.2],
  ])("lowers positive diode EG energy gap %s", (value, expected) => {
    const parsed = parseNetlist(`.model clamp D(EG=${value})\nD1 in out clamp`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "diode",
      energyGapElectronVolts: expected,
    });
  });

  it.each(["0", "-0.1", "1e999"])("rejects invalid diode EG %s", (value) => {
    expect(() => parseNetlist(`.model clamp D(EG=${value})`)).toThrow(
      "diode EG must be finite and positive",
    );
  });

  it.each([
    ["0", 0.0],
    ["2e-18", 2.0e-18],
  ])("lowers valid diode KF flicker-noise coefficient %s", (value, expected) => {
    const parsed = parseNetlist(`.model clamp D(KF=${value})\nD1 in out clamp`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "diode",
      flickerNoiseCoefficient: expected,
    });
  });

  it.each(["-1e-18", "1e999"])("rejects invalid diode KF %s", (value) => {
    expect(() => parseNetlist(`.model clamp D(KF=${value})`)).toThrow(
      "diode KF must be finite and non-negative",
    );
  });

  it.each([
    ["0", 0.0],
    ["1.5", 1.5],
  ])("lowers valid diode AF flicker-noise exponent %s", (value, expected) => {
    const parsed = parseNetlist(`.model clamp D(AF=${value})\nD1 in out clamp`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "diode",
      flickerNoiseExponent: expected,
    });
  });

  it.each(["-0.1", "1e999"])("rejects invalid diode AF %s", (value) => {
    expect(() => parseNetlist(`.model clamp D(AF=${value})`)).toThrow(
      "diode AF must be finite and non-negative",
    );
  });

  it("parses BJT models into operating-point circuits", () => {
    const parsed = parseNetlist(`
.model fast NPN(IS=1e-14 BF=120 VT=25m CJE=2p CJC=3p TF=4n TR=5n)
Vcc vcc 0 DC 5
Vb base 0 DC 0.7
Rc vcc col 100
Q1 col base 0 fast
.op
`);

    expect(parsed.models.get("fast")).toEqual({
      name: "fast",
      kind: "NPN",
      params: new Map([
        ["IS", 1.0e-14],
        ["BF", 120.0],
        ["VT", 25.0e-3],
        ["CJE", 2.0e-12],
        ["CJC", 3.0e-12],
        ["TF", 4.0e-9],
        ["TR", 5.0e-9],
      ]),
    });
    expect(parsed.circuit.elements()[3]).toMatchObject({
      kind: "bjt",
      name: "Q1",
      collector: "col",
      base: "base",
      emitter: "0",
      polarity: "NPN",
      saturationCurrent: 1.0e-14,
      forwardBeta: 120.0,
      thermalVoltage: 25.0e-3,
      baseEmitterCapacitance: 2.0e-12,
      baseCollectorCapacitance: 3.0e-12,
      forwardTransitTime: 4.0e-9,
      reverseTransitTime: 5.0e-9,
    });

    const result = dcOp(parsed.circuit);
    expect(result.voltage("col")).toBeGreaterThan(0.0);
    expect(result.voltage("col")).toBeLessThan(5.0);
  });

  it.each(["0", "-1p", "1e999"])(
    "rejects invalid BJT saturation current %s",
    (value) => {
      expect(() => parseNetlist(`.model fast NPN(IS=${value})`)).toThrow(
        "BJT IS must be finite and positive",
      );
    },
  );

  it("parses PNP BJT model aliases", () => {
    const parsed = parseNetlist(`
.model slow PNP(IS=2e-14 BETA_F=80 VT=26m)
Q2 out base emit slow
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      name: "Q2",
      polarity: "PNP",
      saturationCurrent: 2.0e-14,
      forwardBeta: 80.0,
    });
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("bjt");
    if (element.kind !== "bjt") {
      throw new Error("unexpected element kind");
    }
    expect(element.thermalVoltage).toBeCloseTo(26.0e-3, 12);
  });

  it("parses the BJT BETA forward-beta alias with canonical precedence", () => {
    const parsed = parseNetlist(`
.model fast NPN(BF=120 BETA=90 BETA_F=80 HFE=70)
Q1 col base emit fast
.model slow PNP(BETA=75)
Q2 col base emit slow
.model legacy NPN(HFE=65)
Q3 col base emit legacy
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      forwardBeta: 120.0,
    });
    expect(parsed.circuit.elements()[1]).toMatchObject({
      kind: "bjt",
      forwardBeta: 75.0,
    });
    expect(parsed.circuit.elements()[2]).toMatchObject({
      kind: "bjt",
      forwardBeta: 65.0,
    });
  });

  it.each([
    ["BF", "0"],
    ["BF", "-1"],
    ["BF", "1e999"],
    ["BETA", "0"],
    ["BETA", "-1"],
    ["BETA", "1e999"],
    ["BETA_F", "0"],
    ["BETA_F", "-1"],
    ["BETA_F", "1e999"],
    ["HFE", "0"],
    ["HFE", "-1"],
    ["HFE", "1e999"],
  ])("rejects invalid BJT %s forward beta %s", (parameter, value) => {
    expect(() => parseNetlist(`.model fast NPN(${parameter}=${value})`)).toThrow(
      "BJT BF must be finite and positive",
    );
  });

  it("parses the BJT V_T thermal-voltage alias with canonical precedence", () => {
    const parsed = parseNetlist(`
.model fast NPN(VT=25m V_T=27m)
Q1 col base emit fast
.model slow PNP(V_T=28m)
Q2 col base emit slow
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      thermalVoltage: 25.0e-3,
    });
    expect(parsed.circuit.elements()[1]).toMatchObject({
      kind: "bjt",
      thermalVoltage: 28.0e-3,
    });
  });

  it.each([
    ["VT", "0"],
    ["VT", "-1m"],
    ["VT", "1e999"],
    ["V_T", "0"],
    ["V_T", "-1m"],
    ["V_T", "1e999"],
  ])("rejects invalid BJT %s thermal voltage %s", (parameter, value) => {
    expect(() => parseNetlist(`.model fast NPN(${parameter}=${value})`)).toThrow(
      "BJT VT must be finite and positive",
    );
  });

  it("parses the BJT CJE0 capacitance alias with canonical precedence", () => {
    const parsed = parseNetlist(`
.model fast NPN(CJE=2p CJE0=3p CBE=4p)
Q1 col base emit fast
.model slow PNP(CJE0=5p)
Q2 col base emit slow
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      baseEmitterCapacitance: 2.0e-12,
    });
    expect(parsed.circuit.elements()[1]).toMatchObject({
      kind: "bjt",
      baseEmitterCapacitance: 5.0e-12,
    });
  });

  it.each([
    ["CJE", "-1p"],
    ["CJE", "1e999"],
    ["CJE0", "-1p"],
    ["CJE0", "1e999"],
    ["CBE", "-1p"],
    ["CBE", "1e999"],
  ])("rejects invalid BJT %s base-emitter capacitance %s", (parameter, value) => {
    expect(() => parseNetlist(`.model fast NPN(${parameter}=${value})`)).toThrow(
      "BJT CJE must be finite and non-negative",
    );
  });

  it("parses the BJT CJC0 capacitance alias with canonical precedence", () => {
    const parsed = parseNetlist(`
.model fast NPN(CJC=2p CJC0=3p CBC=4p)
Q1 col base emit fast
.model slow PNP(CJC0=5p)
Q2 col base emit slow
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      baseCollectorCapacitance: 2.0e-12,
    });
    expect(parsed.circuit.elements()[1]).toMatchObject({
      kind: "bjt",
      baseCollectorCapacitance: 5.0e-12,
    });
  });

  it.each([
    ["CJC", "-1p"],
    ["CJC", "1e999"],
    ["CJC0", "-1p"],
    ["CJC0", "1e999"],
    ["CBC", "-1p"],
    ["CBC", "1e999"],
  ])("rejects invalid BJT %s base-collector capacitance %s", (parameter, value) => {
    expect(() => parseNetlist(`.model fast NPN(${parameter}=${value})`)).toThrow(
      "BJT CJC must be finite and non-negative",
    );
  });

  it.each([
    ["TF", "-1n"],
    ["TF", "1e999"],
    ["TR", "-1n"],
    ["TR", "1e999"],
  ])("rejects invalid BJT %s transit time %s", (parameter, value) => {
    expect(() => parseNetlist(`.model fast NPN(${parameter}=${value})`)).toThrow(
      `BJT ${parameter} must be finite and non-negative`,
    );
  });

  it("preserves valid BJT transit times", () => {
    const parsed = parseNetlist(`
.model fast NPN(TF=4n TR=5n)
Q1 col base emit fast
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      forwardTransitTime: 4.0e-9,
      reverseTransitTime: 5.0e-9,
    });
  });

  it.each([
    ["-1", -1.0],
    ["0", 0.0],
    ["4.5", 4.5],
  ])("parses BJT temperature exponent %s", (value, expected) => {
    const parsed = parseNetlist(`
.model fast NPN(XTI=${value})
Q1 col base emit fast
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      saturationCurrentTemperatureExponent: expected,
    });
  });

  it("rejects a non-finite BJT temperature exponent", () => {
    expect(() => parseNetlist(".model fast NPN(XTI=1e999)")).toThrow(
      "BJT XTI must be finite",
    );
  });

  it.each([
    ["0.5", 0.5],
    ["1.2", 1.2],
  ])("parses BJT energy gap %s", (value, expected) => {
    const parsed = parseNetlist(`
.model fast NPN(EG=${value})
Q1 col base emit fast
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      energyGapElectronVolts: expected,
    });
  });

  it.each(["0", "-0.1", "1e999"])("rejects invalid BJT energy gap %s", (value) => {
    expect(() => parseNetlist(`.model fast NPN(EG=${value})`)).toThrow(
      "BJT EG must be finite and positive",
    );
  });

  it.each([
    ["VAF", "0", 0.0],
    ["VA", "0", 0.0],
    ["VAF", "80", 80.0],
    ["VA", "80", 80.0],
  ])("parses BJT forward Early voltage %s=%s", (alias, value, expected) => {
    const parsed = parseNetlist(`.model fast NPN(${alias}=${value})\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      forwardEarlyVoltage: expected,
    });
  });

  it("gives BJT VAF precedence over VA", () => {
    const parsed = parseNetlist(`.model fast NPN(VA=20 VAF=80)\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      forwardEarlyVoltage: 80.0,
    });
  });

  it.each([
    ["VAF", "-0.1"],
    ["VA", "-0.1"],
    ["VAF", "1e999"],
    ["VA", "1e999"],
  ])("rejects invalid BJT forward Early voltage %s=%s", (alias, value) => {
    expect(() => parseNetlist(`.model fast NPN(${alias}=${value})`)).toThrow(
      "BJT VAF must be finite and non-negative",
    );
  });

  it.each([
    ["VAR", "0", 0.0],
    ["VB", "0", 0.0],
    ["VAR", "120", 120.0],
    ["VB", "120", 120.0],
  ])("parses BJT reverse Early voltage %s=%s", (alias, value, expected) => {
    const parsed = parseNetlist(`.model fast NPN(${alias}=${value})\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      reverseEarlyVoltage: expected,
    });
  });

  it("gives BJT VAR precedence over VB", () => {
    const parsed = parseNetlist(`.model fast NPN(VB=40 VAR=120)\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      reverseEarlyVoltage: 120.0,
    });
  });

  it.each([
    ["VAR", "-0.1"],
    ["VB", "-0.1"],
    ["VAR", "1e999"],
    ["VB", "1e999"],
  ])("rejects invalid BJT reverse Early voltage %s=%s", (alias, value) => {
    expect(() => parseNetlist(`.model fast NPN(${alias}=${value})`)).toThrow(
      "BJT VAR must be finite and non-negative",
    );
  });

  it.each([
    ["0.5", 0.5],
    ["2", 2.0],
  ])("parses BJT forward emission coefficient NF=%s", (value, expected) => {
    const parsed = parseNetlist(`.model fast NPN(NF=${value})\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      forwardEmissionCoefficient: expected,
    });
  });

  it.each(["0", "-0.1", "1e999"])(
    "rejects invalid BJT forward emission coefficient NF=%s",
    (value) => {
      expect(() => parseNetlist(`.model fast NPN(NF=${value})`)).toThrow(
        "BJT NF must be finite and positive",
      );
    },
  );

  it.each([
    ["0.5", 0.5],
    ["2", 2.0],
  ])("parses BJT reverse emission coefficient NR=%s", (value, expected) => {
    const parsed = parseNetlist(`.model fast NPN(NR=${value})\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      reverseEmissionCoefficient: expected,
    });
  });

  it.each(["0", "-0.1", "1e999"])(
    "rejects invalid BJT reverse emission coefficient NR=%s",
    (value) => {
      expect(() => parseNetlist(`.model fast NPN(NR=${value})`)).toThrow(
        "BJT NR must be finite and positive",
      );
    },
  );

  it.each([
    ["VJE", "0.5", 0.5],
    ["PE", "0.5", 0.5],
    ["VJE", "0.8", 0.8],
    ["PE", "0.8", 0.8],
  ])("parses BJT base-emitter junction potential %s=%s", (alias, value, expected) => {
    const parsed = parseNetlist(`.model fast NPN(${alias}=${value})\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      baseEmitterJunctionPotential: expected,
    });
  });

  it("gives BJT VJE precedence over PE", () => {
    const parsed = parseNetlist(`.model fast NPN(PE=0.5 VJE=0.8)\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      baseEmitterJunctionPotential: 0.8,
    });
  });

  it.each([
    ["VJE", "0"],
    ["PE", "0"],
    ["VJE", "-0.1"],
    ["PE", "-0.1"],
    ["VJE", "1e999"],
    ["PE", "1e999"],
  ])("rejects invalid BJT base-emitter junction potential %s=%s", (alias, value) => {
    expect(() => parseNetlist(`.model fast NPN(${alias}=${value})`)).toThrow(
      "BJT VJE must be finite and positive",
    );
  });

  it.each([
    ["MJE", "0", 0.0],
    ["ME", "0", 0.0],
    ["MJE", "0.4", 0.4],
    ["ME", "0.4", 0.4],
  ])("parses BJT base-emitter grading coefficient %s=%s", (alias, value, expected) => {
    const parsed = parseNetlist(`.model fast NPN(${alias}=${value})\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      baseEmitterGradingCoefficient: expected,
    });
  });

  it("gives BJT MJE precedence over ME", () => {
    const parsed = parseNetlist(`.model fast NPN(ME=0.2 MJE=0.4)\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      baseEmitterGradingCoefficient: 0.4,
    });
  });

  it.each([
    ["MJE", "-0.1"],
    ["ME", "-0.1"],
    ["MJE", "1"],
    ["ME", "1"],
    ["MJE", "1e999"],
    ["ME", "1e999"],
  ])("rejects invalid BJT base-emitter grading coefficient %s=%s", (alias, value) => {
    expect(() => parseNetlist(`.model fast NPN(${alias}=${value})`)).toThrow(
      "BJT MJE must be finite and in [0, 1)",
    );
  });

  it.each([
    ["VJC", "0.5", 0.5],
    ["PC", "0.5", 0.5],
    ["VJC", "0.8", 0.8],
    ["PC", "0.8", 0.8],
  ])("parses BJT base-collector junction potential %s=%s", (alias, value, expected) => {
    const parsed = parseNetlist(`.model fast NPN(${alias}=${value})\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      baseCollectorJunctionPotential: expected,
    });
  });

  it("gives BJT VJC precedence over PC", () => {
    const parsed = parseNetlist(`.model fast NPN(PC=0.5 VJC=0.8)\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      baseCollectorJunctionPotential: 0.8,
    });
  });

  it.each([
    ["VJC", "0"],
    ["PC", "0"],
    ["VJC", "-0.1"],
    ["PC", "-0.1"],
    ["VJC", "1e999"],
    ["PC", "1e999"],
  ])("rejects invalid BJT base-collector junction potential %s=%s", (alias, value) => {
    expect(() => parseNetlist(`.model fast NPN(${alias}=${value})`)).toThrow(
      "BJT VJC must be finite and positive",
    );
  });

  it.each([
    ["MJC", "0", 0.0],
    ["MC", "0", 0.0],
    ["MJC", "0.4", 0.4],
    ["MC", "0.4", 0.4],
  ])("parses BJT base-collector grading coefficient %s=%s", (alias, value, expected) => {
    const parsed = parseNetlist(`.model fast NPN(${alias}=${value})\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      baseCollectorGradingCoefficient: expected,
    });
  });

  it("gives BJT MJC precedence over MC", () => {
    const parsed = parseNetlist(`.model fast NPN(MC=0.2 MJC=0.4)\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      baseCollectorGradingCoefficient: 0.4,
    });
  });

  it.each([
    ["MJC", "-0.1"],
    ["MC", "-0.1"],
    ["MJC", "1"],
    ["MC", "1"],
    ["MJC", "1e999"],
    ["MC", "1e999"],
  ])("rejects invalid BJT base-collector grading coefficient %s=%s", (alias, value) => {
    expect(() => parseNetlist(`.model fast NPN(${alias}=${value})`)).toThrow(
      "BJT MJC must be finite and in [0, 1)",
    );
  });

  it.each([
    ["0", 0.0],
    ["0.5", 0.5],
  ])("parses BJT forward-bias depletion coefficient FC=%s", (value, expected) => {
    const parsed = parseNetlist(`.model fast NPN(FC=${value})\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      forwardBiasDepletionCoefficient: expected,
    });
  });

  it.each(["-0.1", "1", "1e999"])(
    "rejects invalid BJT forward-bias depletion coefficient FC=%s",
    (value) => {
      expect(() => parseNetlist(`.model fast NPN(FC=${value})`)).toThrow(
        "BJT FC must be finite and in [0, 1)",
      );
    },
  );

  it.each([
    ["IKF", "0", 0.0],
    ["IK", "0", 0.0],
    ["IKF", "2m", 2.0e-3],
    ["IK", "2m", 2.0e-3],
  ])("parses BJT forward beta roll-off current %s=%s", (alias, value, expected) => {
    const parsed = parseNetlist(`.model fast NPN(${alias}=${value})\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      forwardBetaRolloffCurrent: expected,
    });
  });

  it("gives BJT IKF precedence over IK", () => {
    const parsed = parseNetlist(`.model fast NPN(IK=1m IKF=2m)\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      forwardBetaRolloffCurrent: 2.0e-3,
    });
  });

  it.each([
    ["IKF", "-1m"],
    ["IK", "-1m"],
    ["IKF", "1e999"],
    ["IK", "1e999"],
  ])("rejects invalid BJT forward beta roll-off current %s=%s", (alias, value) => {
    expect(() => parseNetlist(`.model fast NPN(${alias}=${value})`)).toThrow(
      "BJT IKF must be finite and non-negative",
    );
  });

  it.each([
    ["ISE", "3p", 3.0e-12],
    ["C2", "2", 2.0e-14],
  ])("parses BJT base-emitter leakage parameter %s=%s", (parameter, value, expected) => {
    const parsed = parseNetlist(`.model fast NPN(${parameter}=${value})\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      baseEmitterLeakageSaturationCurrent: expected,
    });
  });

  it("gives BJT ISE precedence over C2", () => {
    const parsed = parseNetlist(`.model fast NPN(IS=2p C2=-1 ISE=4p)\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      baseEmitterLeakageSaturationCurrent: 4.0e-12,
    });
  });

  it.each([
    ["ISE", "-1p"],
    ["ISE", "1e999"],
    ["C2", "-1"],
    ["C2", "1e999"],
  ])("rejects invalid BJT base-emitter leakage parameter %s=%s", (parameter, value) => {
    expect(() => parseNetlist(`.model fast NPN(${parameter}=${value})`)).toThrow(
      `BJT ${parameter} must be finite and non-negative`,
    );
  });

  it("rejects a non-finite BJT C2-derived leakage current", () => {
    expect(() => parseNetlist(`.model fast NPN(IS=1e308 C2=2)`)).toThrow(
      "BJT ISE must be finite and non-negative",
    );
  });

  it.each([
    ["0.5", 0.5],
    ["2", 2.0],
  ])("parses BJT base-emitter leakage emission coefficient NE=%s", (value, expected) => {
    const parsed = parseNetlist(`.model fast NPN(NE=${value})\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      baseEmitterLeakageEmissionCoefficient: expected,
    });
  });

  it.each(["0", "-0.1", "1e999"])(
    "rejects invalid BJT base-emitter leakage emission coefficient NE=%s",
    (value) => {
      expect(() => parseNetlist(`.model fast NPN(NE=${value})`)).toThrow(
        "BJT NE must be finite and positive",
      );
    },
  );

  it.each([
    ["ISC", "3p", 3.0e-12],
    ["C4", "2", 2.0e-14],
  ])("parses BJT base-collector leakage parameter %s=%s", (parameter, value, expected) => {
    const parsed = parseNetlist(`.model fast NPN(${parameter}=${value})\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      baseCollectorLeakageSaturationCurrent: expected,
    });
  });

  it("gives BJT ISC precedence over C4", () => {
    const parsed = parseNetlist(`.model fast NPN(IS=2p C4=-1 ISC=4p)\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      baseCollectorLeakageSaturationCurrent: 4.0e-12,
    });
  });

  it.each([
    ["ISC", "-1p"],
    ["ISC", "1e999"],
    ["C4", "-1"],
    ["C4", "1e999"],
  ])("rejects invalid BJT base-collector leakage parameter %s=%s", (parameter, value) => {
    expect(() => parseNetlist(`.model fast NPN(${parameter}=${value})`)).toThrow(
      `BJT ${parameter} must be finite and non-negative`,
    );
  });

  it("rejects a non-finite BJT C4-derived leakage current", () => {
    expect(() => parseNetlist(`.model fast NPN(IS=1e308 C4=2)`)).toThrow(
      "BJT ISC must be finite and non-negative",
    );
  });

  it.each([
    ["0.5", 0.5],
    ["2", 2.0],
  ])("parses BJT base-collector leakage emission coefficient NC=%s", (value, expected) => {
    const parsed = parseNetlist(`.model fast NPN(NC=${value})\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      baseCollectorLeakageEmissionCoefficient: expected,
    });
  });

  it.each(["0", "-0.1", "1e999"])(
    "rejects invalid BJT base-collector leakage emission coefficient NC=%s",
    (value) => {
      expect(() => parseNetlist(`.model fast NPN(NC=${value})`)).toThrow(
        "BJT NC must be finite and positive",
      );
    },
  );

  it.each([
    ["-1", -1.0],
    ["2", 2.0],
  ])("parses BJT forward-beta temperature exponent XTB=%s", (value, expected) => {
    const parsed = parseNetlist(`.model fast NPN(XTB=${value})\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      forwardBetaTemperatureExponent: expected,
    });
  });

  it("rejects a non-finite BJT forward-beta temperature exponent", () => {
    expect(() => parseNetlist(`.model fast NPN(XTB=1e999)`)).toThrow(
      "BJT XTB must be finite",
    );
  });

  it.each(["BR", "BETA_R"])("parses BJT reverse beta %s", (alias) => {
    const parsed = parseNetlist(`.model fast NPN(${alias}=25)\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      reverseBeta: 25.0,
    });
  });

  it("gives BJT BR precedence over BETA_R", () => {
    const parsed = parseNetlist(`.model fast NPN(BETA_R=-1 BR=30)\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      reverseBeta: 30.0,
    });
  });

  it.each([
    ["BR", "0"],
    ["BR", "-1"],
    ["BR", "1e999"],
    ["BETA_R", "0"],
    ["BETA_R", "-1"],
    ["BETA_R", "1e999"],
  ])("rejects invalid BJT reverse beta %s=%s", (alias, value) => {
    expect(() => parseNetlist(`.model fast NPN(${alias}=${value})`)).toThrow(
      "BJT BR must be finite and positive",
    );
  });

  it.each([
    ["0", 0.0],
    ["2m", 2.0e-3],
  ])("parses BJT reverse-beta roll-off current IKR=%s", (value, expected) => {
    const parsed = parseNetlist(`.model fast NPN(IKR=${value})\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      reverseBetaRolloffCurrent: expected,
    });
  });

  it.each(["-1m", "1e999"])("rejects invalid BJT IKR=%s", (value) => {
    expect(() => parseNetlist(`.model fast NPN(IKR=${value})`)).toThrow(
      "BJT IKR must be finite and non-negative",
    );
  });

  it.each([
    ["TNOM", "50"],
    ["T_NOM", "75"],
  ])("parses BJT nominal temperature %s=%s", (alias, value) => {
    const parsed = parseNetlist(`.model fast NPN(${alias}=${value})\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      nominalTemperatureKelvin: Number(value) + 273.15,
    });
  });

  it("gives BJT TNOM precedence over T_NOM", () => {
    const parsed = parseNetlist(`.model fast NPN(TNOM=25 T_NOM=50)\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      nominalTemperatureKelvin: 298.15,
    });
  });

  it.each([
    ["TNOM", "0"],
    ["TNOM", "-1"],
    ["TNOM", "1e999"],
    ["T_NOM", "0"],
    ["T_NOM", "-1"],
    ["T_NOM", "1e999"],
  ])("rejects invalid BJT nominal temperature %s=%s", (alias, value) => {
    expect(() => parseNetlist(`.model fast NPN(${alias}=${value})`)).toThrow(
      "BJT TNOM must be finite and positive",
    );
  });

  it.each([
    ["0", 0.0],
    ["2e-18", 2.0e-18],
  ])("parses BJT flicker-noise coefficient KF=%s", (value, expected) => {
    const parsed = parseNetlist(`.model fast NPN(KF=${value})\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      flickerNoiseCoefficient: expected,
    });
  });

  it.each(["-1e-18", "1e999"])("rejects invalid BJT KF=%s", (value) => {
    expect(() => parseNetlist(`.model fast NPN(KF=${value})`)).toThrow(
      "BJT KF must be finite and non-negative",
    );
  });

  it.each([
    ["0", 0.0],
    ["1.5", 1.5],
  ])("parses BJT flicker-noise exponent AF=%s", (value, expected) => {
    const parsed = parseNetlist(`.model fast NPN(AF=${value})\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      flickerNoiseExponent: expected,
    });
  });

  it.each(["-0.1", "1e999"])("rejects invalid BJT AF=%s", (value) => {
    expect(() => parseNetlist(`.model fast NPN(AF=${value})`)).toThrow(
      "BJT AF must be finite and non-negative",
    );
  });

  it.each([
    ["0", 0.0],
    ["45", 45.0],
  ])("parses BJT forward excess phase PTF=%s", (value, expected) => {
    const parsed = parseNetlist(`.model fast NPN(PTF=${value})\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      forwardExcessPhaseDegrees: expected,
    });
  });

  it.each(["-0.1", "1e999"])("rejects invalid BJT PTF=%s", (value) => {
    expect(() => parseNetlist(`.model fast NPN(PTF=${value})`)).toThrow(
      "BJT PTF must be finite and non-negative",
    );
  });

  it.each([
    ["0", 0.0],
    ["2.5", 2.5],
  ])("parses BJT forward transit-time bias coefficient XTF=%s", (value, expected) => {
    const parsed = parseNetlist(`.model fast NPN(XTF=${value})\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      forwardTransitTimeBiasCoefficient: expected,
    });
  });

  it.each(["-0.1", "1e999"])("rejects invalid BJT XTF=%s", (value) => {
    expect(() => parseNetlist(`.model fast NPN(XTF=${value})`)).toThrow(
      "BJT XTF must be finite and non-negative",
    );
  });

  it.each([
    ["0", 0.0],
    ["2m", 2.0e-3],
  ])("parses BJT forward transit-time current ITF=%s", (value, expected) => {
    const parsed = parseNetlist(`.model fast NPN(ITF=${value})\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      forwardTransitTimeCurrent: expected,
    });
  });

  it.each(["-1m", "1e999"])("rejects invalid BJT ITF=%s", (value) => {
    expect(() => parseNetlist(`.model fast NPN(ITF=${value})`)).toThrow(
      "BJT ITF must be finite and non-negative",
    );
  });

  it.each([
    ["0", 0.0],
    ["600m", 0.6],
  ])("parses BJT forward transit-time voltage VTF=%s", (value, expected) => {
    const parsed = parseNetlist(`.model fast NPN(VTF=${value})\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      forwardTransitTimeVoltage: expected,
    });
  });

  it.each(["-1m", "1e999"])("rejects invalid BJT VTF=%s", (value) => {
    expect(() => parseNetlist(`.model fast NPN(VTF=${value})`)).toThrow(
      "BJT VTF must be finite and non-negative",
    );
  });

  it.each([
    ["0", 0.0],
    ["12.5", 12.5],
  ])("parses BJT emitter resistance RE=%s", (value, expected) => {
    const parsed = parseNetlist(`.model fast NPN(RE=${value})\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      emitterResistance: expected,
    });
  });

  it.each(["-1", "1e999"])("rejects invalid BJT RE=%s", (value) => {
    expect(() => parseNetlist(`.model fast NPN(RE=${value})`)).toThrow(
      "BJT RE must be finite and non-negative",
    );
  });

  it.each([
    ["0", 0.0],
    ["13.5", 13.5],
  ])("parses BJT collector resistance RC=%s", (value, expected) => {
    const parsed = parseNetlist(`.model fast NPN(RC=${value})\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      collectorResistance: expected,
    });
  });

  it.each(["-1", "1e999"])("rejects invalid BJT RC=%s", (value) => {
    expect(() => parseNetlist(`.model fast NPN(RC=${value})`)).toThrow(
      "BJT RC must be finite and non-negative",
    );
  });

  it.each([
    ["0", 0.0],
    ["14.5", 14.5],
  ])("parses BJT base resistance RB=%s", (value, expected) => {
    const parsed = parseNetlist(`.model fast NPN(RB=${value})\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      baseResistance: expected,
    });
  });

  it.each(["-1", "1e999"])("rejects invalid BJT RB=%s", (value) => {
    expect(() => parseNetlist(`.model fast NPN(RB=${value})`)).toThrow(
      "BJT RB must be finite and non-negative",
    );
  });

  it.each([
    ["0", 0.0],
    ["2.5", 2.5],
  ])("parses BJT minimum base resistance RBM=%s", (value, expected) => {
    const parsed = parseNetlist(`.model fast NPN(RBM=${value})\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      minimumBaseResistance: expected,
    });
  });

  it("preserves the fallback when BJT RBM is omitted", () => {
    const parsed = parseNetlist(".model fast NPN(RB=14)\nQ1 col base emit fast");

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      minimumBaseResistance: undefined,
    });
  });

  it.each(["-1", "1e999"])("rejects invalid BJT RBM=%s", (value) => {
    expect(() => parseNetlist(`.model fast NPN(RBM=${value})`)).toThrow(
      "BJT RBM must be finite and non-negative",
    );
  });

  it.each([
    ["0", 0.0],
    ["5u", 5.0e-6],
  ])("parses BJT base-resistance half-current IRB=%s", (value, expected) => {
    const parsed = parseNetlist(`.model fast NPN(IRB=${value})\nQ1 col base emit fast`);

    const transistor = parsed.circuit.elements()[0];
    expect(transistor).toMatchObject({ kind: "bjt" });
    expect(transistor.baseResistanceHalfCurrent).toBeCloseTo(expected);
  });

  it.each(["-1u", "1e999"])("rejects invalid BJT IRB=%s", (value) => {
    expect(() => parseNetlist(`.model fast NPN(IRB=${value})`)).toThrow(
      "BJT IRB must be finite and non-negative",
    );
  });

  it.each([
    ["0", 0.0],
    ["0.4", 0.4],
    ["1", 1.0],
  ])("parses BJT base-collector capacitance fraction XCJC=%s", (value, expected) => {
    const parsed = parseNetlist(`.model fast NPN(XCJC=${value})\nQ1 col base emit fast`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      baseCollectorCapacitanceFraction: expected,
    });
  });

  it("defaults omitted BJT XCJC to one", () => {
    const parsed = parseNetlist(".model fast NPN\nQ1 col base emit fast");

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      baseCollectorCapacitanceFraction: 1.0,
    });
  });

  it.each(["-0.1", "1.1", "1e999"])("rejects invalid BJT XCJC=%s", (value) => {
    expect(() => parseNetlist(`.model fast NPN(XCJC=${value})`)).toThrow(
      "BJT XCJC must be finite and between zero and one",
    );
  });

  it("parses JFET models into operating-point circuits", () => {
    const parsed = parseNetlist(`
.model fast NJF(BETA=2m VTO=-3 LAMBDA=0.02)
J1 drain gate source fast
`);

    expect(parsed.models.get("fast")).toEqual({
      name: "fast",
      kind: "NJF",
      params: new Map([
        ["BETA", 2.0e-3],
        ["VTO", -3.0],
        ["LAMBDA", 0.02],
      ]),
    });
    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "jfet",
      name: "J1",
      drain: "drain",
      gate: "gate",
      source: "source",
      polarity: "NJF",
      beta: 2.0e-3,
      thresholdVoltage: -3.0,
      channelLengthModulation: 0.02,
    });
  });

  it("parses the NJFET model type alias", () => {
    const parsed = parseNetlist(".model fast NJFET(BETA=2m)\nJ1 drain gate source fast");

    expect(parsed.models.get("fast")?.kind).toBe("NJF");
    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "jfet",
      polarity: "NJF",
      beta: 2.0e-3,
    });
  });

  it("parses the NJ model type alias", () => {
    const parsed = parseNetlist(".model fast NJ(BETA=2m)\nJ1 drain gate source fast");

    expect(parsed.models.get("fast")?.kind).toBe("NJF");
    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "jfet",
      polarity: "NJF",
      beta: 2.0e-3,
    });
  });

  it("parses the JFET BET alias with canonical precedence", () => {
    const parsed = parseNetlist(
      ".model canonical NJF(BETA=2m BET=1m B=500u)\n" +
        ".model aliased NJF(BET=900u)\n" +
        "J1 drain gate source canonical\n" +
        "J2 drain gate source aliased",
    );

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "jfet",
      beta: 2.0e-3,
    });
    expect(parsed.circuit.elements()[1]).toMatchObject({
      kind: "jfet",
      beta: 900.0e-6,
    });
  });

  it.each([
    ["BETA", "0"],
    ["BETA", "-1m"],
    ["BETA", "1e999"],
    ["BET", "0"],
    ["BET", "-1m"],
    ["BET", "1e999"],
  ])("rejects invalid JFET transconductance %s=%s", (parameter, value) => {
    expect(() => parseNetlist(`.model fast NJF(${parameter}=${value})`)).toThrow(
      "JFET BETA must be finite and positive",
    );
  });

  it("parses JFET threshold aliases with canonical precedence", () => {
    const parsed = parseNetlist(
      ".model canonical NJF(VTO=-3 VT0=-2 VTH=-1)\n" +
        ".model vtzero NJF(VT0=-2)\n" +
        ".model threshold NJF(VTH=-1)\n" +
        "J1 drain gate source canonical\n" +
        "J2 drain gate source vtzero\n" +
        "J3 drain gate source threshold",
    );

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "jfet",
      thresholdVoltage: -3.0,
    });
    expect(parsed.circuit.elements()[1]).toMatchObject({
      kind: "jfet",
      thresholdVoltage: -2.0,
    });
    expect(parsed.circuit.elements()[2]).toMatchObject({
      kind: "jfet",
      thresholdVoltage: -1.0,
    });
  });

  it.each(["VTO", "VT0", "VTH"])(
    "rejects non-finite JFET threshold voltage %s",
    (parameter) => {
      expect(() => parseNetlist(`.model fast NJF(${parameter}=1e999)`)).toThrow(
        "JFET VTO must be finite",
      );
    },
  );

  it("parses the JFET LAM alias with canonical precedence", () => {
    const parsed = parseNetlist(
      ".model canonical NJF(LAMBDA=0.02 LAM=0.03)\n" +
        ".model aliased NJF(LAM=0.04)\n" +
        "J1 drain gate source canonical\n" +
        "J2 drain gate source aliased",
    );

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "jfet",
      channelLengthModulation: 0.02,
    });
    expect(parsed.circuit.elements()[1]).toMatchObject({
      kind: "jfet",
      channelLengthModulation: 0.04,
    });
  });

  it.each(["LAMBDA", "LAM"])(
    "rejects non-finite JFET channel-length modulation %s",
    (parameter) => {
      expect(() => parseNetlist(`.model fast NJF(${parameter}=1e999)`)).toThrow(
        "JFET LAMBDA must be finite",
      );
    },
  );

  it("parses the PJFET model type alias", () => {
    const parsed = parseNetlist(".model fast PJFET(BETA=2m)\nJ1 drain gate source fast");

    expect(parsed.models.get("fast")?.kind).toBe("PJF");
    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "jfet",
      polarity: "PJF",
      beta: 2.0e-3,
    });
  });

  it("parses the PJ model type alias", () => {
    const parsed = parseNetlist(".model fast PJ(BETA=2m)\nJ1 drain gate source fast");

    expect(parsed.models.get("fast")?.kind).toBe("PJF");
    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "jfet",
      polarity: "PJF",
      beta: 2.0e-3,
    });
  });

  it.each(["CGS", "CGS0"])(
    "parses the JFET %s gate-source capacitance alias",
    (parameter) => {
      const parsed = parseNetlist(`
.model fast NJF(${parameter}=3p)
J1 drain gate source fast
`);

      expect(parsed.circuit.elements()[0]).toMatchObject({
        kind: "jfet",
        gateSourceCapacitance: 3.0e-12,
      });
    },
  );

  it.each(["-1p", "1e999"])(
    "rejects invalid JFET gate-source capacitance %s",
    (value) => {
      expect(() => parseNetlist(`.model fast NJF(CGS=${value})`)).toThrow(
        "JFET CGS must be finite and non-negative",
      );
    },
  );

  it.each(["CGD", "CGD0"])(
    "parses the JFET %s gate-drain capacitance alias",
    (parameter) => {
      const parsed = parseNetlist(`
.model fast NJF(${parameter}=4p)
J1 drain gate source fast
`);

      expect(parsed.circuit.elements()[0]).toMatchObject({
        kind: "jfet",
        gateDrainCapacitance: 4.0e-12,
      });
    },
  );

  it.each(["-1p", "1e999"])(
    "rejects invalid JFET gate-drain capacitance %s",
    (value) => {
      expect(() => parseNetlist(`.model fast NJF(CGD=${value})`)).toThrow(
        "JFET CGD must be finite and non-negative",
      );
    },
  );

  it("parses the JFET flicker-noise coefficient", () => {
    const parsed = parseNetlist(`
.model fast NJF(KF=2e-18)
J1 drain gate source fast
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "jfet",
      flickerNoiseCoefficient: 2.0e-18,
    });
  });

  it.each(["-1e-18", "1e999"])(
    "rejects invalid JFET flicker-noise coefficient %s",
    (value) => {
      expect(() => parseNetlist(`.model fast NJF(KF=${value})`)).toThrow(
        "JFET KF must be finite and non-negative",
      );
    },
  );

  it("parses the JFET flicker-noise exponent", () => {
    const parsed = parseNetlist(`
.model fast NJF(AF=1.4)
J1 drain gate source fast
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "jfet",
      flickerNoiseExponent: 1.4,
    });
  });

  it.each(["-0.1", "1e999"])(
    "rejects invalid JFET flicker-noise exponent %s",
    (value) => {
      expect(() => parseNetlist(`.model fast NJF(AF=${value})`)).toThrow(
        "JFET AF must be finite and non-negative",
      );
    },
  );

  it.each(["PB", "VJ"])("parses the JFET %s junction-potential alias", (parameter) => {
    const parsed = parseNetlist(`
.model fast NJF(${parameter}=0.8)
J1 drain gate source fast
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "jfet",
      junctionPotential: 0.8,
    });
  });

  it("prefers canonical JFET PB over the VJ alias", () => {
    const parsed = parseNetlist(`
.model fast NJF(PB=0.9 VJ=0.8)
J1 drain gate source fast
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "jfet",
      junctionPotential: 0.9,
    });
  });

  it.each(["PB", "VJ"])("rejects invalid JFET %s junction potential", (parameter) => {
    for (const value of ["0", "-0.1", "1e999"]) {
      expect(() => parseNetlist(`.model fast NJF(${parameter}=${value})`)).toThrow(
        "JFET PB must be finite and positive",
      );
    }
  });

  it.each([
    ["0", 0.0],
    ["0.6", 0.6],
  ])("parses JFET forward-bias depletion coefficient %s", (value, expected) => {
    const parsed = parseNetlist(`
.model fast NJF(FC=${value})
J1 drain gate source fast
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "jfet",
      forwardBiasDepletionCoefficient: expected,
    });
  });

  it.each(["-0.1", "1", "1e999"])(
    "rejects invalid JFET forward-bias depletion coefficient %s",
    (value) => {
      expect(() => parseNetlist(`.model fast NJF(FC=${value})`)).toThrow(
        "JFET FC must be finite and in [0, 1)",
      );
    },
  );

  it("parses the JFET gate saturation current", () => {
    const parsed = parseNetlist(`
.model fast NJF(IS=2p)
J1 drain gate source fast
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "jfet",
      gateSaturationCurrent: 2.0e-12,
    });
  });

  it.each(["0", "-1p", "1e999"])("rejects invalid JFET gate saturation current %s", (value) => {
    expect(() => parseNetlist(`.model fast NJF(IS=${value})`)).toThrow(
      "JFET IS must be finite and positive",
    );
  });

  it.each([
    ["0", 0.0],
    ["2.5", 2.5],
  ])("parses JFET temperature exponent %s", (value, expected) => {
    const parsed = parseNetlist(`
.model fast NJF(XTI=${value})
J1 drain gate source fast
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "jfet",
      gateSaturationCurrentTemperatureExponent: expected,
    });
  });

  it("rejects a non-finite JFET temperature exponent", () => {
    expect(() => parseNetlist(".model fast NJF(XTI=1e999)")).toThrow(
      "JFET XTI must be finite",
    );
  });

  it("parses the JFET energy gap", () => {
    const parsed = parseNetlist(`
.model fast NJF(EG=1.05)
J1 drain gate source fast
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "jfet",
      bandgapVoltage: 1.05,
    });
  });

  it.each(["0", "-0.1", "1e999"])("rejects invalid JFET energy gap %s", (value) => {
    expect(() => parseNetlist(`.model fast NJF(EG=${value})`)).toThrow(
      "JFET EG must be finite and positive",
    );
  });

  it.each([
    ["1", 1.0],
    ["3", 3.0],
  ])("parses JFET noise equation level %s", (value, expected) => {
    const parsed = parseNetlist(`
.model fast NJF(NLEV=${value})
J1 drain gate source fast
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "jfet",
      noiseEquationLevel: expected,
    });
  });

  it.each(["0", "1.5", "1e999"])("rejects invalid JFET noise equation level %s", (value) => {
    expect(() => parseNetlist(`.model fast NJF(NLEV=${value})`)).toThrow(
      "JFET NLEV must be a finite integer greater than or equal to 1",
    );
  });

  it.each([
    ["0", 0.0],
    ["1.5", 1.5],
  ])("parses JFET channel noise coefficient %s", (value, expected) => {
    const parsed = parseNetlist(`
.model fast NJF(GDSNOI=${value})
J1 drain gate source fast
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "jfet",
      channelNoiseCoefficient: expected,
    });
  });

  it.each(["-0.1", "1e999"])(
    "rejects invalid JFET channel noise coefficient %s",
    (value) => {
      expect(() => parseNetlist(`.model fast NJF(GDSNOI=${value})`)).toThrow(
        "JFET GDSNOI must be finite and non-negative",
      );
    },
  );

  it.each([
    ["0", 0.0],
    ["12.5", 12.5],
  ])("parses JFET drain resistance %s", (value, expected) => {
    const parsed = parseNetlist(`
.model fast NJF(RD=${value})
J1 drain gate source fast
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "jfet",
      drainResistance: expected,
    });
  });

  it.each(["-0.1", "1e999"])(
    "rejects invalid JFET drain resistance %s",
    (value) => {
      expect(() => parseNetlist(`.model fast NJF(RD=${value})`)).toThrow(
        "JFET RD must be finite and non-negative",
      );
    },
  );

  it.each([
    ["0", 0.0],
    ["9.75", 9.75],
  ])("parses JFET source resistance %s", (value, expected) => {
    const parsed = parseNetlist(`
.model fast NJF(RS=${value})
J1 drain gate source fast
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "jfet",
      sourceResistance: expected,
    });
  });

  it.each(["-0.1", "1e999"])(
    "rejects invalid JFET source resistance %s",
    (value) => {
      expect(() => parseNetlist(`.model fast NJF(RS=${value})`)).toThrow(
        "JFET RS must be finite and non-negative",
      );
    },
  );

  it.each([
    ["-0.01", -0.01],
    ["0", 0.0],
    ["0.02", 0.02],
  ])(
    "parses JFET threshold-voltage temperature coefficient %s",
    (value, expected) => {
      const parsed = parseNetlist(`
.model fast NJF(TCV=${value})
J1 drain gate source fast
`);

      expect(parsed.circuit.elements()[0]).toMatchObject({
        kind: "jfet",
        thresholdVoltageTemperatureCoefficient: expected,
      });
    },
  );

  it("rejects a non-finite JFET threshold-voltage temperature coefficient", () => {
    expect(() => parseNetlist(".model fast NJF(TCV=1e999)")).toThrow(
      "JFET TCV must be finite",
    );
  });

  it.each([
    ["-0.004", -0.004],
    ["0", 0.0],
    ["0.006", 0.006],
  ])(
    "parses JFET alternative threshold-voltage temperature coefficient %s",
    (value, expected) => {
      const parsed = parseNetlist(`
.model fast NJF(VTOTC=${value})
J1 drain gate source fast
`);

      expect(parsed.circuit.elements()[0]).toMatchObject({
        kind: "jfet",
        alternativeThresholdVoltageTemperatureCoefficient: expected,
      });
    },
  );

  it("rejects a non-finite JFET alternative threshold-voltage temperature coefficient", () => {
    expect(() => parseNetlist(".model fast NJF(VTOTC=1e999)")).toThrow(
      "JFET VTOTC must be finite",
    );
  });

  it.each([
    ["TNOM", "50"],
    ["T_NOM", "75"],
  ])("parses JFET nominal temperature alias %s", (alias, value) => {
    const parsed = parseNetlist(`
.model fast NJF(${alias}=${value})
J1 drain gate source fast
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "jfet",
      nominalTemperatureKelvin: Number(value) + 273.15,
    });
  });

  it("gives JFET TNOM precedence over T_NOM", () => {
    const parsed = parseNetlist(`
.model fast NJF(TNOM=25 T_NOM=50)
J1 drain gate source fast
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "jfet",
      nominalTemperatureKelvin: 298.15,
    });
  });

  it.each([
    ["TNOM", "0"],
    ["T_NOM", "0"],
    ["TNOM", "-1"],
    ["T_NOM", "-1"],
    ["TNOM", "1e999"],
    ["T_NOM", "1e999"],
  ])(
    "rejects invalid JFET nominal temperature %s=%s",
    (alias, value) => {
      expect(() => parseNetlist(`.model fast NJF(${alias}=${value})`)).toThrow(
        "JFET TNOM must be finite and positive",
      );
    },
  );

  it.each([
    ["-2.5", -2.5],
    ["0", 0.0],
    ["1.75", 1.75],
  ])("parses JFET mobility temperature exponent %s", (value, expected) => {
    const parsed = parseNetlist(`
.model fast NJF(BEX=${value})
J1 drain gate source fast
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "jfet",
      mobilityTemperatureExponent: expected,
    });
  });

  it("rejects a non-finite JFET mobility temperature exponent", () => {
    expect(() => parseNetlist(".model fast NJF(BEX=1e999)")).toThrow(
      "JFET BEX must be finite",
    );
  });

  it.each([
    ["-0.5", -0.5],
    ["0", 0.0],
    ["1.25", 1.25],
  ])(
    "parses JFET alternative mobility temperature coefficient %s",
    (value, expected) => {
      const parsed = parseNetlist(`
.model fast NJF(BETATCE=${value})
J1 drain gate source fast
`);

      expect(parsed.circuit.elements()[0]).toMatchObject({
        kind: "jfet",
        mobilityTemperatureCoefficient: expected,
      });
    },
  );

  it("preserves an omitted JFET alternative mobility temperature coefficient", () => {
    const parsed = parseNetlist(`
.model fast NJF(BEX=1.5)
J1 drain gate source fast
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "jfet",
      mobilityTemperatureCoefficient: undefined,
    });
  });

  it("rejects a non-finite JFET alternative mobility temperature coefficient", () => {
    expect(() => parseNetlist(".model fast NJF(BETATCE=1e999)")).toThrow(
      "JFET BETATCE must be finite",
    );
  });

  it("parses PJF model beta aliases", () => {
    const parsed = parseNetlist(`
.model pslow PJF(B=750u)
Jp drain gate source pslow
`);

    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("jfet");
    if (element.kind !== "jfet") {
      throw new Error("unexpected element kind");
    }
    expect(element.polarity).toBe("PJF");
    expect(element.beta).toBeCloseTo(750.0e-6, 12);
    expect(element.thresholdVoltage).toBe(2.0);
  });

  it("parses MOSFET models into operating-point circuits", () => {
    const parsed = parseNetlist(`
.model nfast NMOS(VT0=0.45 KP=200u LAMBDA=0.02 CGSO=3p CGDO=4p CGBO=5p CBS=6p CBD=7p)
Vdd vdd 0 DC 1.8
Vgate gate 0 DC 1.8
Rload vdd out 1k
M1 out gate 0 0 nfast W=2u L=180n
.op
`);

    const model = parsed.models.get("nfast");
    expect(model?.name).toBe("nfast");
    expect(model?.kind).toBe("NMOS");
    expect(model?.params.get("VT0")).toBe(0.45);
    expect(model?.params.get("KP")).toBeCloseTo(200.0e-6, 12);
    expect(model?.params.get("CGSO")).toBeCloseTo(3.0e-12, 18);
    expect(parsed.circuit.elements()[3]).toMatchObject({
      kind: "mosfet",
      name: "M1",
      drain: "out",
      gate: "gate",
      source: "0",
      body: "0",
      type: "NMOS",
      params: {
        VT0: 0.45,
        LAMBDA: 0.02,
      },
    });
    const element = parsed.circuit.elements()[3];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") {
      throw new Error("unexpected element kind");
    }
    expect(element.params.KP).toBeCloseTo(200.0e-6, 12);
    expect(element.params.CGSO).toBeCloseTo(3.0e-12, 18);
    expect(element.params.CGDO).toBeCloseTo(4.0e-12, 18);
    expect(element.params.CGBO).toBeCloseTo(5.0e-12, 18);
    expect(element.params.CBS).toBeCloseTo(6.0e-12, 18);
    expect(element.params.CBD).toBeCloseTo(7.0e-12, 18);
    expect(element.params.W).toBeCloseTo(2.0e-6, 12);
    expect(element.params.L).toBeCloseTo(180.0e-9, 12);

    const result = dcOp(parsed.circuit);
    expect(result.voltage("out")).toBeGreaterThanOrEqual(0.0);
    expect(result.voltage("out")).toBeLessThan(1.8);
  });

  it("parses the NCH model type alias", () => {
    const parsed = parseNetlist(".model nfast NCH(VTO=0.45 KP=200u)\nM1 d g s b nfast");

    expect(parsed.models.get("nfast")?.kind).toBe("NMOS");
    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "mosfet",
      type: "NMOS",
    });
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") {
      throw new Error("unexpected element kind");
    }
    expect(element.params.KP).toBeCloseTo(200.0e-6, 12);
  });

  it("parses the PCH model type alias", () => {
    const parsed = parseNetlist(".model pfast PCH(VTO=0.4 KP=120u)\nM1 d g s b pfast");

    expect(parsed.models.get("pfast")?.kind).toBe("PMOS");
    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "mosfet",
      type: "PMOS",
    });
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") {
      throw new Error("unexpected element kind");
    }
    expect(element.params.KP).toBeCloseTo(120.0e-6, 12);
  });

  it("normalizes model type alias separators", () => {
    const parsed = parseNetlist(
      ".model jfast n-jfet(BETA=2m)\n" +
        ".model pfast p_ch(VTO=0.4 KP=120u)\n" +
        "J1 drain gate source jfast\n" +
        "M1 d g s b pfast",
    );

    expect(parsed.models.get("jfast")?.kind).toBe("NJF");
    expect(parsed.models.get("pfast")?.kind).toBe("PMOS");
    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "jfet",
      polarity: "NJF",
    });
    expect(parsed.circuit.elements()[1]).toMatchObject({
      kind: "mosfet",
      type: "PMOS",
    });
  });

  it("normalizes model parameter alias hyphens", () => {
    const parsed = parseNetlist(
      ".model qfast NPN(BETA-F=125)\n" +
        ".model mfast NMOS(T-NOM=325 KP=120u)\n" +
        "Q1 collector base emitter qfast\n" +
        "M1 d g s b mfast",
    );

    expect(parsed.models.get("qfast")?.params.get("BETA_F")).toBe(125.0);
    expect(parsed.models.get("mfast")?.params.get("T_NOM")).toBe(325.0);
    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      forwardBeta: 125.0,
    });
    expect(parsed.circuit.elements()[1]).toMatchObject({
      kind: "mosfet",
      params: { T_NOM: 325.0 },
    });
  });

  it("parses PMOS MOSFET aliases", () => {
    const parsed = parseNetlist(`
.model pfast PMOS(VTO=0.4 KP=120u NSUB=1.2)
Mp out gate vdd vdd pfast W=3u L=250n
`);

    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") {
      throw new Error("unexpected element kind");
    }
    expect(element.type).toBe("PMOS");
    expect(element.params.VT0).toBe(0.4);
    expect(element.params.N_SUB).toBe(1.2);
    expect(element.params.KP).toBeCloseTo(120.0e-6, 12);
    expect(element.params.W).toBeCloseTo(3.0e-6, 12);
    expect(element.params.L).toBeCloseTo(250.0e-9, 12);
  });

  it.each(["-1", "1e999"])(
    "rejects invalid MOSFET instance NRD=%s",
    (drainSquares) => {
      expect(() =>
        parseNetlist(`.model nfast NMOS\nM1 d g s b nfast NRD=${drainSquares}\n`),
      ).toThrow("MOSFET NRD must be finite and non-negative");
    },
  );

  it.each([
    ["0", 0.0],
    ["2.5", 2.5],
  ])("lowers valid MOSFET instance NRD=%s", (drainSquares, expected) => {
    const parsed = parseNetlist(`.model nfast NMOS\nM1 d g s b nfast NRD=${drainSquares}\n`);
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") throw new Error("unexpected element kind");
    expect(element.params.NRD).toBe(expected);
  });

  it.each(["-1", "1e999"])(
    "rejects invalid MOSFET instance NRS=%s",
    (sourceSquares) => {
      expect(() =>
        parseNetlist(`.model nfast NMOS\nM1 d g s b nfast NRS=${sourceSquares}\n`),
      ).toThrow("MOSFET NRS must be finite and non-negative");
    },
  );

  it.each([
    ["0", 0.0],
    ["3.5", 3.5],
  ])("lowers valid MOSFET instance NRS=%s", (sourceSquares, expected) => {
    const parsed = parseNetlist(`.model nfast NMOS\nM1 d g s b nfast NRS=${sourceSquares}\n`);
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") throw new Error("unexpected element kind");
    expect(element.params.NRS).toBe(expected);
  });

  it.each(["-1n", "1e999"])("rejects invalid MOSFET instance AD=%s", (drainArea) => {
    expect(() =>
      parseNetlist(`.model nfast NMOS\nM1 d g s b nfast AD=${drainArea}\n`),
    ).toThrow("MOSFET AD must be finite and non-negative");
  });

  it.each([
    ["0", 0.0],
    ["3n", 3.0e-9],
  ])("lowers valid MOSFET instance AD=%s", (drainArea, expected) => {
    const parsed = parseNetlist(`.model nfast NMOS\nM1 d g s b nfast AD=${drainArea}\n`);
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") throw new Error("unexpected element kind");
    expect(element.params.AD).toBeCloseTo(expected, 15);
  });

  it.each(["-1n", "1e999"])("rejects invalid MOSFET instance AS=%s", (sourceArea) => {
    expect(() =>
      parseNetlist(`.model nfast NMOS\nM1 d g s b nfast AS=${sourceArea}\n`),
    ).toThrow("MOSFET AS must be finite and non-negative");
  });

  it.each([
    ["0", 0.0],
    ["4n", 4.0e-9],
  ])("lowers valid MOSFET instance AS=%s", (sourceArea, expected) => {
    const parsed = parseNetlist(`.model nfast NMOS\nM1 d g s b nfast AS=${sourceArea}\n`);
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") throw new Error("unexpected element kind");
    expect(element.params.AS).toBeCloseTo(expected, 15);
  });

  it.each(["-1u", "1e999"])(
    "rejects invalid MOSFET instance PD=%s",
    (drainPerimeter) => {
      expect(() =>
        parseNetlist(`.model nfast NMOS\nM1 d g s b nfast PD=${drainPerimeter}\n`),
      ).toThrow("MOSFET PD must be finite and non-negative");
    },
  );

  it.each([
    ["0", 0.0],
    ["6u", 6.0e-6],
  ])("lowers valid MOSFET instance PD=%s", (drainPerimeter, expected) => {
    const parsed = parseNetlist(`.model nfast NMOS\nM1 d g s b nfast PD=${drainPerimeter}\n`);
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") throw new Error("unexpected element kind");
    expect(element.params.PD).toBeCloseTo(expected, 15);
  });

  it.each(["-1u", "1e999"])(
    "rejects invalid MOSFET instance PS=%s",
    (sourcePerimeter) => {
      expect(() =>
        parseNetlist(`.model nfast NMOS\nM1 d g s b nfast PS=${sourcePerimeter}\n`),
      ).toThrow("MOSFET PS must be finite and non-negative");
    },
  );

  it.each([
    ["0", 0.0],
    ["7u", 7.0e-6],
  ])("lowers valid MOSFET instance PS=%s", (sourcePerimeter, expected) => {
    const parsed = parseNetlist(`.model nfast NMOS\nM1 d g s b nfast PS=${sourcePerimeter}\n`);
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") throw new Error("unexpected element kind");
    expect(element.params.PS).toBeCloseTo(expected, 15);
  });

  it("parses PWL and SIN source waveforms", () => {
    const parsed = parseNetlist(`
V1 in 0 PWL(0 0, 1n 1.8, 2n 0)
I1 in 0 SIN(0 2m 1k 10u 5)
`);

    const [voltage, current] = parsed.circuit.elements();
    expect(voltage.kind).toBe("voltage-source");
    expect(current.kind).toBe("current-source");
    if (voltage.kind !== "voltage-source" || current.kind !== "current-source") {
      throw new Error("unexpected element kind");
    }
    expect(voltage.waveform?.valueAt(0.5e-9)).toBeCloseTo(0.9, 12);
    expect(current.waveform?.valueAt(1.0e-6)).toBeCloseTo(0.0, 12);
  });

  it("expands subcircuit instances into engine elements", () => {
    const parsed = parseNetlist(`
.subckt divider top mid bot
Rtop top mid 1k
Rbot mid bot 1k
.ends divider
V1 vin 0 DC 10
Xdiv vin mid 0 divider
.op
`);

    const elements = parsed.circuit.elements();
    expect(elements.map((element) => element.name)).toEqual([
      "V1",
      "Xdiv.Rtop",
      "Xdiv.Rbot",
    ]);
    expect(elements[1]).toMatchObject({
      kind: "resistor",
      n1: "vin",
      n2: "mid",
    });

    const result = dcOp(parsed.circuit);
    expect(result.voltage("mid")).toBeCloseTo(5.0, 9);
  });

  it("expands subcircuit VCVS nodes into engine elements", () => {
    const parsed = parseNetlist(`
.subckt gain inp outp
Ebuf outp 0 inp 0 2
.ends gain
V1 in 0 DC 1.25
Xgain in out gain
Rload out 0 1k
.op
`);

    const elements = parsed.circuit.elements();
    expect(elements.map((element) => element.name)).toEqual([
      "V1",
      "Xgain.Ebuf",
      "Rload",
    ]);
    expect(elements[1]).toMatchObject({
      kind: "vcvs",
      positive: "out",
      controlPositive: "in",
    });

    const result = dcOp(parsed.circuit);
    expect(result.voltage("out")).toBeCloseTo(2.5, 9);
  });

  it("expands subcircuit CCCS control sources into engine elements", () => {
    const parsed = parseNetlist(`
.subckt mirror inp outp
Rin inp sense 1k
Vsense sense 0 DC 0
Fcopy outp 0 Vsense 2
.ends mirror
Vin in 0 DC 1
Xmirror in out mirror
Rload out 0 500
.op
`);

    const elements = parsed.circuit.elements();
    expect(elements.map((element) => element.name)).toEqual([
      "Vin",
      "Xmirror.Rin",
      "Xmirror.Vsense",
      "Xmirror.Fcopy",
      "Rload",
    ]);
    expect(elements[3]).toMatchObject({
      kind: "cccs",
      positive: "out",
      controlSource: "Xmirror.Vsense",
    });

    const result = dcOp(parsed.circuit);
    expect(result.voltage("out")).toBeCloseTo(-1.0, 9);
  });

  it("expands subcircuit CCVS control sources into engine elements", () => {
    const parsed = parseNetlist(`
.subckt transimpedance inp outp
Rin inp sense 1k
Vsense sense 0 DC 0
Hamp outp 0 Vsense 1k
.ends transimpedance
Vin in 0 DC 1
Xamp in out transimpedance
Rload out 0 500
.op
`);

    const elements = parsed.circuit.elements();
    expect(elements.map((element) => element.name)).toEqual([
      "Vin",
      "Xamp.Rin",
      "Xamp.Vsense",
      "Xamp.Hamp",
      "Rload",
    ]);
    expect(elements[3]).toMatchObject({
      kind: "ccvs",
      positive: "out",
      controlSource: "Xamp.Vsense",
      transresistanceOhms: 1000.0,
    });

    const result = dcOp(parsed.circuit);
    expect(result.voltage("out")).toBeCloseTo(1.0, 9);
  });

  it("expands subcircuit diode nodes into engine elements", () => {
    const parsed = parseNetlist(`
.model clamp D(IS=1e-12 VT=25m N=2 BV=5 IBV=1u CJ0=3p TT=5n)
.subckt limiter inp outp
Dlim inp outp clamp
.ends limiter
Xlim in out limiter
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "diode",
      name: "Xlim.Dlim",
      anode: "in",
      cathode: "out",
      emissionCoefficient: 2.0,
      breakdownVoltage: 5.0,
      breakdownCurrent: 1.0e-6,
      junctionCapacitance: 3.0e-12,
      transitTime: 5.0e-9,
    });
  });

  it("expands subcircuit BJT nodes into engine elements", () => {
    const parsed = parseNetlist(`
.model fast NPN(IS=1e-14 BF=120)
.subckt stage c b e
Qamp c b e fast
.ends stage
Xstage out in 0 stage
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "bjt",
      name: "Xstage.Qamp",
      collector: "out",
      base: "in",
      emitter: "0",
      polarity: "NPN",
    });
  });

  it("expands subcircuit JFET nodes into engine elements", () => {
    const parsed = parseNetlist(`
.model nchan NJF(BETA=1m)
.subckt source_follower d g s
Jbuf d g inner nchan
Rtail inner s 100
.ends source_follower
Xbuf out in 0 source_follower
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "jfet",
      name: "Xbuf.Jbuf",
      drain: "out",
      gate: "in",
      source: "Xbuf.inner",
      beta: 1.0e-3,
    });
    expect(parsed.circuit.elements()[1]).toMatchObject({
      kind: "resistor",
      n1: "Xbuf.inner",
      n2: "0",
    });
  });

  it("expands subcircuit mutual-inductor references into engine elements", () => {
    const parsed = parseNetlist(`
.subckt transformer p1 p2 s1 s2
Lpri p1 p2 10m
Lsec s1 s2 40m
Kcore Lpri Lsec 0.9
.ends transformer
Xtx in 0 out 0 transformer
`);

    expect(parsed.circuit.elements()[2]).toMatchObject({
      kind: "mutual-inductor",
      name: "Xtx.Kcore",
      primary: "Xtx.Lpri",
      secondary: "Xtx.Lsec",
      coupling: 0.9,
    });
  });

  it("expands subcircuit transmission-line nodes into engine elements", () => {
    const parsed = parseNetlist(`
.subckt delay in out
T1 in 0 out 0 Z0=75 TD=2n
.ends delay
Xdelay a b delay
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "transmission-line",
      name: "Xdelay.T1",
      n1: "a",
      n2: "0",
      n3: "b",
      n4: "0",
      characteristicImpedanceOhms: 75.0,
      delaySeconds: 2.0e-9,
    });
  });

  it("expands subcircuit MOSFET nodes into engine elements", () => {
    const parsed = parseNetlist(`
.model nfast NMOS(W=1u L=130n)
.subckt pulldown in out vss
Mpull out in inner vss nfast
Rtail inner vss 10
.ends pulldown
Xpd gate drain 0 pulldown
`);

    expect(parsed.circuit.elements()[0]).toMatchObject({
      kind: "mosfet",
      name: "Xpd.Mpull",
      drain: "drain",
      gate: "gate",
      source: "Xpd.inner",
      body: "0",
    });
    expect(parsed.circuit.elements()[1]).toMatchObject({
      kind: "resistor",
      n1: "Xpd.inner",
      n2: "0",
    });
  });

  it("scopes subcircuit internal nodes by instance", () => {
    const parsed = parseNetlist(`
.subckt load in out
R1 in inner 1k
C1 inner out 1u
.ends load
Xleft a b load
Xright c d load
`);

    const elements = parsed.circuit.elements();
    expect(elements[0]).toMatchObject({
      kind: "resistor",
      n2: "Xleft.inner",
    });
    expect(elements[2]).toMatchObject({
      kind: "resistor",
      n2: "Xright.inner",
    });
  });

  it("parses engineering suffixes", () => {
    expect(parseValue("1k")).toBe(1.0e3);
    expect(parseValue("2.2meg")).toBe(2.2e6);
    expect(parseValue("3u")).toBe(3.0e-6);
    expect(parseValue("4n")).toBe(4.0e-9);
  });

  it("rejects unsupported elements with line numbers", () => {
    expect(() => parseNetlist("\nZ1 c b e model\n")).toThrow(NetlistParseError);
    expect(() => parseNetlist("\nZ1 c b e model\n")).toThrow("line 2: unsupported element");
  });

  it("rejects unknown diode models", () => {
    expect(() => parseNetlist("D1 a 0 missing\n")).toThrow(
      "line 1: unknown model \"missing\" for diode \"D1\"",
    );
  });

  it("rejects non-diode models for diode elements", () => {
    expect(() => parseNetlist(".model amp NPN(IS=1e-12)\nD1 a 0 amp\n")).toThrow(
      'line 2: model "amp" has kind "NPN", expected "D"',
    );
  });

  it("rejects unknown BJT models", () => {
    expect(() => parseNetlist("Q1 c b e missing\n")).toThrow(
      'line 1: unknown model "missing" for BJT "Q1"',
    );
  });

  it("rejects non-BJT models for BJT elements", () => {
    expect(() => parseNetlist(".model clamp D(IS=1e-12)\nQ1 c b e clamp\n")).toThrow(
      'line 2: model "clamp" has kind "D", expected "NPN" or "PNP"',
    );
  });

  it("rejects unknown MOSFET models", () => {
    expect(() => parseNetlist("M1 d g s b missing\n")).toThrow(
      'line 1: unknown model "missing" for MOSFET "M1"',
    );
  });

  it("rejects non-MOSFET models for MOSFET elements", () => {
    expect(() => parseNetlist(".model clamp D(IS=1e-12)\nM1 d g s b clamp\n")).toThrow(
      'line 2: model "clamp" has kind "D", expected "NMOS" or "PMOS"',
    );
  });

  it("rejects MOSFET parameters without assignments", () => {
    expect(() => parseNetlist(".model nfast NMOS\nM1 d g s b nfast W\n")).toThrow(
      'line 2: invalid MOSFET parameter syntax "W"',
    );
  });

  it.each(["0", "2", "1.000000000002", "1e999"])(
    "rejects unsupported MOSFET model LEVEL=%s",
    (level) => {
      expect(() => parseNetlist(`.model nfast NMOS(LEVEL=${level})\nM1 d g s b nfast\n`)).toThrow(
        "only MOS LEVEL=1 model cards are supported",
      );
    },
  );

  it.each(["LEVEL=1", "LEVEL=1.0000000000005", ""])(
    "preserves supported MOSFET model parameters %s",
    (parameters) => {
      const parsed = parseNetlist(`.model nfast NMOS(${parameters})\nM1 d g s b nfast\n`);
      expect(parsed.circuit.elements()[0].kind).toBe("mosfet");
    },
  );

  it.each(["0", "-1n", "1e999"])(
    "rejects invalid MOSFET model TOX=%s",
    (oxideThickness) => {
      expect(() =>
        parseNetlist(`.model nfast NMOS(TOX=${oxideThickness})\nM1 d g s b nfast\n`),
      ).toThrow("MOSFET TOX must be finite and positive");
    },
  );

  it("lowers MOSFET model oxide thickness", () => {
    const parsed = parseNetlist(".model nfast NMOS(TOX=7n)\nM1 d g s b nfast\n");
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") {
      throw new Error("unexpected element kind");
    }
    expect(element.params.TOX).toBeCloseTo(7.0e-9, 15);
  });

  it.each(["-1", "1e999"])("rejects invalid MOSFET model U0=%s", (surfaceMobility) => {
    expect(() =>
      parseNetlist(`.model nfast NMOS(U0=${surfaceMobility})\nM1 d g s b nfast\n`),
    ).toThrow("MOSFET U0 must be finite and non-negative");
  });

  it.each(["U0", "UO"])(
    "lowers MOSFET model surface-mobility alias %s and derives KP",
    (alias) => {
      const parsed = parseNetlist(`.model nfast NMOS(${alias}=450 TOX=12n)\nM1 d g s b nfast\n`);
      const element = parsed.circuit.elements()[0];
      expect(element.kind).toBe("mosfet");
      if (element.kind !== "mosfet") {
        throw new Error("unexpected element kind");
      }
      expect(element.params.U0).toBe(450.0);
      expect(element.params.KP).toBeCloseTo(1.294924875e-4, 15);
    },
  );

  it("preserves explicit MOSFET model KP over mobility derivation", () => {
    const parsed = parseNetlist(".model nfast NMOS(U0=450 TOX=12n KP=123u)\nM1 d g s b nfast\n");
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") {
      throw new Error("unexpected element kind");
    }
    expect(element.params.KP).toBeCloseTo(123.0e-6, 15);
  });

  it.each(["0", "-1u", "1e999"])(
    "rejects invalid explicit MOSFET model KP=%s",
    (transconductance) => {
      expect(() =>
        parseNetlist(`.model nfast NMOS(KP=${transconductance})\nM1 d g s b nfast\n`),
      ).toThrow("MOSFET KP must be finite and positive");
    },
  );

  it("preserves positive explicit MOSFET model transconductance", () => {
    const parsed = parseNetlist(".model nfast NMOS(KP=175u)\nM1 d g s b nfast\n");
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") {
      throw new Error("unexpected element kind");
    }
    expect(element.params.KP).toBeCloseTo(175.0e-6, 15);
  });

  it.each(["VT0", "VTO", "VTH"])(
    "rejects non-finite MOSFET model threshold alias %s",
    (alias) => {
      expect(() =>
        parseNetlist(`.model nfast NMOS(${alias}=1e999)\nM1 d g s b nfast\n`),
      ).toThrow("MOSFET VT0 must be finite");
    },
  );

  it.each(["VT0", "VTO", "VTH"])(
    "lowers finite MOSFET model threshold alias %s",
    (alias) => {
      const parsed = parseNetlist(`.model nfast NMOS(${alias}=-0.38)\nM1 d g s b nfast\n`);
      const element = parsed.circuit.elements()[0];
      expect(element.kind).toBe("mosfet");
      if (element.kind !== "mosfet") {
        throw new Error("unexpected element kind");
      }
      expect(element.params.VT0).toBe(-0.38);
    },
  );

  it.each(["LAMBDA", "LAM"])(
    "rejects non-finite MOSFET model channel-modulation alias %s",
    (alias) => {
      expect(() =>
        parseNetlist(`.model nfast NMOS(${alias}=1e999)\nM1 d g s b nfast\n`),
      ).toThrow("MOSFET LAMBDA must be finite");
    },
  );

  it.each(["LAMBDA", "LAM"])(
    "lowers finite MOSFET model channel-modulation alias %s",
    (alias) => {
      const parsed = parseNetlist(`.model nfast NMOS(${alias}=-0.02)\nM1 d g s b nfast\n`);
      const element = parsed.circuit.elements()[0];
      expect(element.kind).toBe("mosfet");
      if (element.kind !== "mosfet") {
        throw new Error("unexpected element kind");
      }
      expect(element.params.LAMBDA).toBe(-0.02);
    },
  );

  it.each(["-0.01", "1e999"])("rejects invalid MOSFET model GAMMA=%s", (bodyEffect) => {
    expect(() =>
      parseNetlist(`.model nfast NMOS(GAMMA=${bodyEffect})\nM1 d g s b nfast\n`),
    ).toThrow("MOSFET GAMMA must be finite and non-negative");
  });

  it.each(["0", "0.45"])("lowers valid MOSFET model GAMMA=%s", (bodyEffect) => {
    const parsed = parseNetlist(
      `.model nfast NMOS(GAMMA=${bodyEffect})\nM1 d g s b nfast\n`,
    );
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") {
      throw new Error("unexpected element kind");
    }
    expect(element.params.GAMMA).toBe(Number(bodyEffect));
  });

  it.each(["0", "-0.01", "1e999"])(
    "rejects invalid MOSFET model PHI=%s",
    (surfacePotential) => {
      expect(() =>
        parseNetlist(`.model nfast NMOS(PHI=${surfacePotential})\nM1 d g s b nfast\n`),
      ).toThrow("MOSFET PHI must be finite and positive");
    },
  );

  it("lowers positive MOSFET model surface potential", () => {
    const parsed = parseNetlist(".model nfast NMOS(PHI=0.65)\nM1 d g s b nfast\n");
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") {
      throw new Error("unexpected element kind");
    }
    expect(element.params.PHI).toBe(0.65);
  });

  it.each(["0", "-1u", "1e999"])("rejects invalid MOSFET model W=%s", (width) => {
    expect(() => parseNetlist(`.model nfast NMOS(W=${width})\nM1 d g s b nfast\n`)).toThrow(
      "MOSFET W must be finite and positive",
    );
  });

  it("lowers positive MOSFET model width", () => {
    const parsed = parseNetlist(".model nfast NMOS(W=4u)\nM1 d g s b nfast\n");
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") {
      throw new Error("unexpected element kind");
    }
    expect(element.params.W).toBeCloseTo(4.0e-6, 15);
  });

  it.each(["0", "-1u", "1e999"])("rejects invalid MOSFET model L=%s", (length) => {
    expect(() => parseNetlist(`.model nfast NMOS(L=${length})\nM1 d g s b nfast\n`)).toThrow(
      "MOSFET L must be finite and positive",
    );
  });

  it("lowers positive MOSFET model length", () => {
    const parsed = parseNetlist(".model nfast NMOS(L=2u)\nM1 d g s b nfast\n");
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") throw new Error("unexpected element kind");
    expect(element.params.L).toBeCloseTo(2.0e-6, 15);
  });

  it.each(["LD=-1n", "LD=1e999", "L=100n LD=50n"])(
    "rejects invalid MOSFET model lateral diffusion %s",
    (parameters) => {
      expect(() =>
        parseNetlist(`.model nfast NMOS(${parameters})\nM1 d g s b nfast\n`),
      ).toThrow("MOSFET LD must be finite and non-negative with L - 2*LD > 0");
    },
  );

  it.each([
    ["0", 0.0],
    ["10n", 10.0e-9],
  ])("lowers valid MOSFET model LD=%s", (lateralDiffusion, expected) => {
    const parsed = parseNetlist(
      `.model nfast NMOS(L=180n LD=${lateralDiffusion})\nM1 d g s b nfast\n`,
    );
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") throw new Error("unexpected element kind");
    expect(element.params.LD).toBeCloseTo(expected, 15);
  });

  it.each(["0", "-1p", "1e999"])(
    "rejects invalid MOSFET model IS=%s",
    (saturationCurrent) => {
      expect(() =>
        parseNetlist(`.model nfast NMOS(IS=${saturationCurrent})\nM1 d g s b nfast\n`),
      ).toThrow("MOSFET IS must be finite and positive");
    },
  );

  it("lowers positive MOSFET model saturation current", () => {
    const parsed = parseNetlist(".model nfast NMOS(IS=2f)\nM1 d g s b nfast\n");
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") throw new Error("unexpected element kind");
    expect(element.params.IS).toBeCloseTo(2.0e-15, 20);
  });

  it.each(["TNOM", "T_NOM"])("validates MOSFET model nominal-temperature alias %s", (alias) => {
    for (const temperature of ["0", "-1", "1e999"]) {
      expect(() =>
        parseNetlist(`.model nfast NMOS(${alias}=${temperature})\nM1 d g s b nfast\n`),
      ).toThrow("MOSFET TNOM must be finite and positive");
    }
    const parsed = parseNetlist(`.model nfast NMOS(${alias}=325)\nM1 d g s b nfast\n`);
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") throw new Error("unexpected element kind");
    expect(element.params.T_NOM).toBe(325);
  });

  it("gives MOSFET T_NOM precedence over TNOM", () => {
    const parsed = parseNetlist(".model nfast NMOS(T_NOM=325 TNOM=350)\nM1 d g s b nfast\n");
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") throw new Error("unexpected element kind");
    expect(element.params.T_NOM).toBe(325);
  });

  it.each(["-1", "1e999"])("rejects invalid MOSFET model RD=%s", (drainResistance) => {
    expect(() =>
      parseNetlist(`.model nfast NMOS(RD=${drainResistance})\nM1 d g s b nfast\n`),
    ).toThrow("MOSFET RD must be finite and non-negative");
  });

  it.each(["0", "12.5"])("lowers valid MOSFET model RD=%s", (drainResistance) => {
    const parsed = parseNetlist(
      `.model nfast NMOS(RD=${drainResistance})\nM1 d g s b nfast\n`,
    );
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") throw new Error("unexpected element kind");
    expect(element.params.RD).toBe(Number(drainResistance));
  });

  it.each(["-1", "1e999"])("rejects invalid MOSFET model RS=%s", (sourceResistance) => {
    expect(() =>
      parseNetlist(`.model nfast NMOS(RS=${sourceResistance})\nM1 d g s b nfast\n`),
    ).toThrow("MOSFET RS must be finite and non-negative");
  });

  it.each(["0", "9.75"])("lowers valid MOSFET model RS=%s", (sourceResistance) => {
    const parsed = parseNetlist(
      `.model nfast NMOS(RS=${sourceResistance})\nM1 d g s b nfast\n`,
    );
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") throw new Error("unexpected element kind");
    expect(element.params.RS).toBe(Number(sourceResistance));
  });

  it.each(["-1", "1e999"])("rejects invalid MOSFET model RSH=%s", (sheetResistance) => {
    expect(() =>
      parseNetlist(`.model nfast NMOS(RSH=${sheetResistance})\nM1 d g s b nfast\n`),
    ).toThrow("MOSFET RSH must be finite and non-negative");
  });

  it.each(["0", "42.5"])("lowers valid MOSFET model RSH=%s", (sheetResistance) => {
    const parsed = parseNetlist(
      `.model nfast NMOS(RSH=${sheetResistance})\nM1 d g s b nfast\n`,
    );
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") throw new Error("unexpected element kind");
    expect(element.params.RSH).toBe(Number(sheetResistance));
  });

  it.each(["-1p", "1e999"])(
    "rejects invalid MOSFET model CJ=%s",
    (junctionCapacitance) => {
      expect(() =>
        parseNetlist(`.model nfast NMOS(CJ=${junctionCapacitance})\nM1 d g s b nfast\n`),
      ).toThrow("MOSFET CJ must be finite and non-negative");
    },
  );

  it.each([
    ["0", 0.0],
    ["2p", 2.0e-12],
  ])("lowers valid MOSFET model CJ=%s", (junctionCapacitance, expected) => {
    const parsed = parseNetlist(
      `.model nfast NMOS(CJ=${junctionCapacitance})\nM1 d g s b nfast\n`,
    );
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") throw new Error("unexpected element kind");
    expect(element.params.CJ).toBeCloseTo(expected, 15);
  });

  it.each(["-1p", "1e999"])(
    "rejects invalid MOSFET model CJSW=%s",
    (sidewallCapacitance) => {
      expect(() =>
        parseNetlist(`.model nfast NMOS(CJSW=${sidewallCapacitance})\nM1 d g s b nfast\n`),
      ).toThrow("MOSFET CJSW must be finite and non-negative");
    },
  );

  it.each([
    ["0", 0.0],
    ["3p", 3.0e-12],
  ])("lowers valid MOSFET model CJSW=%s", (sidewallCapacitance, expected) => {
    const parsed = parseNetlist(
      `.model nfast NMOS(CJSW=${sidewallCapacitance})\nM1 d g s b nfast\n`,
    );
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") throw new Error("unexpected element kind");
    expect(element.params.CJSW).toBeCloseTo(expected, 15);
  });

  it.each(["-1p", "1e999"])("rejects invalid MOSFET model CGSO=%s", (overlap) => {
    expect(() =>
      parseNetlist(`.model nfast NMOS(CGSO=${overlap})\nM1 d g s b nfast\n`),
    ).toThrow("MOSFET CGSO must be finite and non-negative");
  });

  it.each([
    ["0", 0.0],
    ["3p", 3.0e-12],
  ])("lowers valid MOSFET model CGSO=%s", (overlap, expected) => {
    const parsed = parseNetlist(
      `.model nfast NMOS(CGSO=${overlap})\nM1 d g s b nfast\n`,
    );
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") throw new Error("unexpected element kind");
    expect(element.params.CGSO).toBeCloseTo(expected, 15);
  });

  it.each(["-1p", "1e999"])("rejects invalid MOSFET model CGDO=%s", (overlap) => {
    expect(() =>
      parseNetlist(`.model nfast NMOS(CGDO=${overlap})\nM1 d g s b nfast\n`),
    ).toThrow("MOSFET CGDO must be finite and non-negative");
  });

  it.each([
    ["0", 0.0],
    ["4p", 4.0e-12],
  ])("lowers valid MOSFET model CGDO=%s", (overlap, expected) => {
    const parsed = parseNetlist(
      `.model nfast NMOS(CGDO=${overlap})\nM1 d g s b nfast\n`,
    );
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") throw new Error("unexpected element kind");
    expect(element.params.CGDO).toBeCloseTo(expected, 15);
  });

  it.each(["-1p", "1e999"])("rejects invalid MOSFET model CGBO=%s", (overlap) => {
    expect(() =>
      parseNetlist(`.model nfast NMOS(CGBO=${overlap})\nM1 d g s b nfast\n`),
    ).toThrow("MOSFET CGBO must be finite and non-negative");
  });

  it.each([
    ["0", 0.0],
    ["5p", 5.0e-12],
  ])("lowers valid MOSFET model CGBO=%s", (overlap, expected) => {
    const parsed = parseNetlist(
      `.model nfast NMOS(CGBO=${overlap})\nM1 d g s b nfast\n`,
    );
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") throw new Error("unexpected element kind");
    expect(element.params.CGBO).toBeCloseTo(expected, 15);
  });

  it.each(["-1p", "1e999"])("rejects invalid MOSFET model JS=%s", (junctionCurrent) => {
    expect(() =>
      parseNetlist(`.model nfast NMOS(JS=${junctionCurrent})\nM1 d g s b nfast\n`),
    ).toThrow("MOSFET JS must be finite and non-negative");
  });

  it.each([
    ["0", 0.0],
    ["4p", 4.0e-12],
  ])("lowers valid MOSFET model JS=%s", (junctionCurrent, expected) => {
    const parsed = parseNetlist(
      `.model nfast NMOS(JS=${junctionCurrent})\nM1 d g s b nfast\n`,
    );
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") throw new Error("unexpected element kind");
    expect(element.params.JS).toBeCloseTo(expected, 15);
  });

  it.each(["0", "-0.1", "1e999"])(
    "rejects invalid MOSFET model PB=%s",
    (bulkPotential) => {
      expect(() =>
        parseNetlist(`.model nfast NMOS(PB=${bulkPotential})\nM1 d g s b nfast\n`),
      ).toThrow("MOSFET PB must be finite and positive");
    },
  );

  it("lowers positive MOSFET model PB", () => {
    const parsed = parseNetlist(".model nfast NMOS(PB=0.72)\nM1 d g s b nfast\n");
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") throw new Error("unexpected element kind");
    expect(element.params.PB).toBeCloseTo(0.72, 15);
  });

  it.each(["-0.1", "1e999"])(
    "rejects invalid MOSFET model MJ=%s",
    (gradingCoefficient) => {
      expect(() =>
        parseNetlist(`.model nfast NMOS(MJ=${gradingCoefficient})\nM1 d g s b nfast\n`),
      ).toThrow("MOSFET MJ must be finite and non-negative");
    },
  );

  it.each(["0", "0.45"])("lowers valid MOSFET model MJ=%s", (gradingCoefficient) => {
    const parsed = parseNetlist(
      `.model nfast NMOS(MJ=${gradingCoefficient})\nM1 d g s b nfast\n`,
    );
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") throw new Error("unexpected element kind");
    expect(element.params.MJ).toBe(Number(gradingCoefficient));
  });

  it.each(["-0.1", "1e999"])(
    "rejects invalid MOSFET model MJSW=%s",
    (gradingCoefficient) => {
      expect(() =>
        parseNetlist(`.model nfast NMOS(MJSW=${gradingCoefficient})\nM1 d g s b nfast\n`),
      ).toThrow("MOSFET MJSW must be finite and non-negative");
    },
  );

  it.each(["0", "0.33"])("lowers valid MOSFET model MJSW=%s", (gradingCoefficient) => {
    const parsed = parseNetlist(
      `.model nfast NMOS(MJSW=${gradingCoefficient})\nM1 d g s b nfast\n`,
    );
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") throw new Error("unexpected element kind");
    expect(element.params.MJSW).toBe(Number(gradingCoefficient));
  });

  it.each(["-0.1", "1", "1e999"])(
    "rejects invalid MOSFET model FC=%s",
    (coefficient) => {
      expect(() =>
        parseNetlist(`.model nfast NMOS(FC=${coefficient})\nM1 d g s b nfast\n`),
      ).toThrow("MOSFET FC must be finite and in [0, 1)");
    },
  );

  it.each(["0", "0.5"])("lowers valid MOSFET model FC=%s", (coefficient) => {
    const parsed = parseNetlist(`.model nfast NMOS(FC=${coefficient})\nM1 d g s b nfast\n`);
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") throw new Error("unexpected element kind");
    expect(element.params.FC).toBe(Number(coefficient));
  });

  it.each(["-1e-18", "1e999"])(
    "rejects invalid MOSFET model KF=%s",
    (coefficient) => {
      expect(() =>
        parseNetlist(`.model nfast NMOS(KF=${coefficient})\nM1 d g s b nfast\n`),
      ).toThrow("MOSFET KF must be finite and non-negative");
    },
  );

  it.each(["0", "2e-18"])("lowers valid MOSFET model KF=%s", (coefficient) => {
    const parsed = parseNetlist(`.model nfast NMOS(KF=${coefficient})\nM1 d g s b nfast\n`);
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") throw new Error("unexpected element kind");
    expect(element.params.KF).toBe(Number(coefficient));
  });

  it.each(["-0.1", "1e999"])("rejects invalid MOSFET model AF=%s", (exponent) => {
    expect(() =>
      parseNetlist(`.model nfast NMOS(AF=${exponent})\nM1 d g s b nfast\n`),
    ).toThrow("MOSFET AF must be finite and non-negative");
  });

  it.each(["0", "1.5"])("lowers valid MOSFET model AF=%s", (exponent) => {
    const parsed = parseNetlist(`.model nfast NMOS(AF=${exponent})\nM1 d g s b nfast\n`);
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") throw new Error("unexpected element kind");
    expect(element.params.AF).toBe(Number(exponent));
  });

  it.each([
    ["CJS", "CBS"],
    ["CJD", "CBD"],
  ] as const)("validates and lowers MOSFET %s as %s", (alias, canonical) => {
    expect(() =>
      parseNetlist(`.model nfast NMOS(${alias}=-1p)\nM1 d g s b nfast\n`),
    ).toThrow(`MOSFET ${canonical} must be finite and non-negative`);
    const parsed = parseNetlist(`.model nfast NMOS(${alias}=2p)\nM1 d g s b nfast\n`);
    const element = parsed.circuit.elements()[0];
    expect(element.kind).toBe("mosfet");
    if (element.kind !== "mosfet") throw new Error("unexpected element kind");
    expect(element.params[canonical]).toBeCloseTo(2.0e-12, 18);
  });

  it.each([
    ["NSS=-1", "MOSFET NSS must be finite and non-negative"],
    ["NSS=1e999", "MOSFET NSS must be finite and non-negative"],
    ["TPG=0.5", "MOSFET TPG must be -1, 0, or 1"],
  ])("rejects invalid MOSFET process parameter %s", (parameter, message) => {
    expect(() =>
      parseNetlist(`.model nfast NMOS(${parameter})\nM1 d g s b nfast\n`),
    ).toThrow(message);
  });

  it("derives MOSFET electrostatic defaults with explicit precedence", () => {
    const derived = parseNetlist(
      ".model nfast NMOS(NSUB=4e15 TOX=100n NSS=1e10 TPG=-1)\nM1 d g s b nfast\n",
    ).circuit.elements()[0];
    const explicit = parseNetlist(
      ".model nfast NMOS(NSUB=4e15 TOX=100n NSS=1e10 TPG=-1 " +
        "VT0=0.61 GAMMA=0.42 PHI=0.73)\nM1 d g s b nfast\n",
    ).circuit.elements()[0];
    expect(derived.kind).toBe("mosfet");
    expect(explicit.kind).toBe("mosfet");
    if (derived.kind !== "mosfet" || explicit.kind !== "mosfet") {
      throw new Error("unexpected element kind");
    }
    expect(derived.params.GAMMA).toBeGreaterThan(0.0);
    expect(derived.params.PHI).toBeGreaterThan(0.0);
    expect(derived.params.VT0).not.toBeCloseTo(0.7, 12);
    expect(explicit.params.VT0).toBeCloseTo(0.61, 15);
    expect(explicit.params.GAMMA).toBeCloseTo(0.42, 15);
    expect(explicit.params.PHI).toBeCloseTo(0.73, 15);
  });

  it("rejects unbalanced waveform parentheses", () => {
    expect(() => parseNetlist("V1 in 0 PULSE(0 1\n")).toThrow("unclosed parenthesis");
  });

  it("rejects unknown subcircuit instances", () => {
    expect(() => parseNetlist("X1 a b missing\n")).toThrow(
      "line 1: unknown subcircuit \"missing\"",
    );
  });
});
