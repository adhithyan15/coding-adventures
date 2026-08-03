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
.model fast D(IS=1e-12 VT=25m N=2 BV=5 IBV=1u CJO=2p TT=4n)
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
    });

    const result = dcOp(parsed.circuit);
    expect(result.voltage("out")).toBeGreaterThan(0.0);
    expect(result.voltage("out")).toBeLessThan(0.7);
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

  it("rejects unbalanced waveform parentheses", () => {
    expect(() => parseNetlist("V1 in 0 PULSE(0 1\n")).toThrow("unclosed parenthesis");
  });

  it("rejects unknown subcircuit instances", () => {
    expect(() => parseNetlist("X1 a b missing\n")).toThrow(
      "line 1: unknown subcircuit \"missing\"",
    );
  });
});
