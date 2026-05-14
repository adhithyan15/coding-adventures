import { dcOp } from "@coding-adventures/spice-engine";
import { describe, expect, it } from "vitest";
import {
  NetlistParseError,
  parseNetlist,
  parseValue,
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

  it("parses reactive elements, VCCS, source waveforms, and analysis cards", () => {
    const parsed = parseNetlist(`
Vstep in 0 PULSE(0 1 0 1n 1n 10n 20n)
I1 out 0 1m
Rload in out 2.2k
Cload out 0 10p
L1 out 0 1u
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
    expect(parsed.analyses).toEqual([
      { kind: "tran", timeStep: 1.0e-9, stopTime: 20.0e-9 },
      { kind: "dc", sourceName: "Vstep", start: 0.0, stop: 1.0, step: 0.5 },
      { kind: "ac", mode: "dec", points: 10, startHz: 1.0e3, stopHz: 1.0e6 },
    ]);
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
    expect(() => parseNetlist("\nD1 a 0 diode\n")).toThrow(NetlistParseError);
    expect(() => parseNetlist("\nD1 a 0 diode\n")).toThrow("line 2: unsupported element");
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
