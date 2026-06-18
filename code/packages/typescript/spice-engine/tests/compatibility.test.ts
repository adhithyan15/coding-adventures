import { describe, expect, it } from "vitest";
import {
  analyzeDeckControls,
  type CompatibilityDeck,
  compatibilityCorpus,
  formatCompatibilityCorpusTable,
  formatReleaseReadinessReport,
  releaseReadinessGates,
  resolveDeckAnalyses,
  resolveDeckFourier,
  resolveDeckFunctions,
  resolveDeckInitialConditions,
  resolveDeckMeasurements,
  resolveDeckOutputs,
  resolveDeckParameters,
  resolveDeckSources,
  selectDeckAnalysisPlan,
  selectDeckOutputProbes,
} from "../src/index.js";

describe("compatibility corpus", () => {
  it("ships a release-readiness corpus with stable deck ids", () => {
    const corpus = compatibilityCorpus();

    expect(corpus.map((deck) => deck.id)).toStrictEqual([
      "dc-op-resistive-divider",
      "dc-sweep-resistive-divider",
      "ac-rc-lowpass",
      "tran-rc-step",
      "tf-resistive-divider",
    ]);
    expect(new Set(corpus.map((deck) => deck.analysis))).toEqual(
      new Set(["op", "dc", "ac", "tran", "tf"]),
    );
    expect(corpus.every((deck) => deck.netlist.toLowerCase().includes(".end"))).toBe(true);
    expect(corpus.every((deck) => deck.knownIncompatibilities.length > 0)).toBe(true);

    const report = releaseReadinessGates(corpus);

    expect(report.passed).toBe(true);
    expect(report.deckCount).toBe(5);
    expect(report.issues).toStrictEqual([]);
    expect(formatReleaseReadinessReport(report).split("\n")[1]).toBe(
      "true\t5\top,dc,ac,tran,tf\t0",
    );
  });

  it("formats a stable corpus table", () => {
    const table = formatCompatibilityCorpusTable();

    expect(table.split("\n")[0]).toBe(
      "id\tanalysis\toracle\tgolden_values\tknown_incompatibilities",
    );
    expect(table).toContain("dc-op-resistive-divider\top\tclosed-form@divider-v1");
    expect(table).toContain("V(out)=5.000000e+00V");
  });

  it("reports malformed release-readiness decks", () => {
    const malformed: CompatibilityDeck = {
      id: "",
      title: "Missing metadata",
      analysis: "noise",
      netlist: "V1 in 0 DC 1",
      oracle: { reference: "", version: "", source: "" },
      goldenValues: [
        {
          name: "V(out)",
          value: Number.POSITIVE_INFINITY,
          unit: "V",
          absoluteTolerance: -1.0,
          relativeTolerance: 0.0,
        },
      ],
      knownIncompatibilities: [],
    };

    const report = releaseReadinessGates([malformed]);
    const fields = new Set(report.issues.map((issue) => issue.field));

    expect(report.passed).toBe(false);
    expect(fields).toEqual(
      new Set([
        "id",
        "analysis",
        "netlist",
        "oracle.reference",
        "oracle.version",
        "oracle.source",
        "goldenValues[0].value",
        "goldenValues[0].tolerance",
        "knownIncompatibilities",
        "analysisCoverage",
      ]),
    );
  });

  it("treats .end as the executable deck boundary", () => {
    const summary = analyzeDeckControls(`
* ignored title
V1 in 0 DC 1
.op
.end
.include after-end.lib
.dc V1 0 1 1
`);

    expect(summary.terminated).toBe(true);
    expect(summary.endLineNumber).toBe(5);
    expect(summary.activeLines).toStrictEqual(["V1 in 0 DC 1", ".op"]);
    expect(summary.diagnostics).toStrictEqual([]);
  });

  it("reports unsupported deck-control directives before .end", () => {
    const summary = analyzeDeckControls(`
.include models.inc
.LIB vendor.lib TT
.control
op
save V(in)
probe V(out)
print op V(in)
measure tran vmax MAX V(out)
meas dc imax MAX I(V1)
fourier 1k V(out)
four 2k V(in)
reset
set noaskquit
set filetype=ascii
set wr_vecnames
set wr_singlescale
set appendwrite
set filetype=binary
write out.raw V(out)
wrdata out.dat V(out)
wrdata empty.dat
display all
listing physical
show all
showmod Q1
status
version
help tran
echo running selected deck
rusage all
where
source nested-control.cir
.source dotted-control.cir
shell echo nope
.shell echo dotted
cd models
.cd /tmp
if $&run_monte
.while $&again
foreach dev M1 M2
.repeat 2
run
quit
.endc
.end
`);

    expect(summary.terminated).toBe(true);
    expect(summary.activeLines).toStrictEqual([
      ".include models.inc",
      ".LIB vendor.lib TT",
      ".op",
      ".save V(in)",
      ".probe V(out)",
      ".print op V(in)",
      ".measure tran vmax MAX V(out)",
      ".meas dc imax MAX I(V1)",
      ".four 1k V(out)",
      ".four 2k V(in)",
    ]);
    expect(summary.diagnostics.map(({ directive, lineNumber, severity }) => [
      directive,
      lineNumber,
      severity,
    ])).toStrictEqual([
      [".include", 2, "error"],
      [".lib", 3, "error"],
      [".control", 4, "error"],
      [".control", 19, "error"],
      [".control", 22, "error"],
      [".control", 33, "error"],
      [".control", 34, "error"],
      [".control", 35, "error"],
      [".control", 36, "error"],
      [".control", 37, "error"],
      [".control", 38, "error"],
      [".control", 39, "error"],
      [".control", 40, "error"],
      [".control", 41, "error"],
      [".control", 42, "error"],
    ]);
    expect(summary.diagnostics.map((diagnostic) => diagnostic.code)).toStrictEqual([
      "SPICE_DECK_UNSUPPORTED_DIRECTIVE",
      "SPICE_DECK_UNSUPPORTED_DIRECTIVE",
      "SPICE_DECK_UNSUPPORTED_DIRECTIVE",
      "SPICE_DECK_CONTROL_COMMAND",
      "SPICE_DECK_CONTROL_COMMAND",
      "SPICE_DECK_CONTROL_SCRIPT_COMMAND",
      "SPICE_DECK_CONTROL_SCRIPT_COMMAND",
      "SPICE_DECK_CONTROL_SCRIPT_COMMAND",
      "SPICE_DECK_CONTROL_SCRIPT_COMMAND",
      "SPICE_DECK_CONTROL_WORKDIR_COMMAND",
      "SPICE_DECK_CONTROL_WORKDIR_COMMAND",
      "SPICE_DECK_CONTROL_FLOW_COMMAND",
      "SPICE_DECK_CONTROL_FLOW_COMMAND",
      "SPICE_DECK_CONTROL_FLOW_COMMAND",
      "SPICE_DECK_CONTROL_FLOW_COMMAND",
    ]);
    const measurementSummary = resolveDeckMeasurements(`${summary.activeLines.join("\n")}\n.end`);
    expect(measurementSummary.measurements.map((card) => [
      card.directive,
      card.analysis,
      card.name,
      card.mode,
      card.probe,
    ])).toStrictEqual([
      [".measure", "tran", "vmax", "max", "V(out)"],
      [".meas", "dc", "imax", "max", "I(V1)"],
    ]);
    const fourierSummary = resolveDeckFourier(`${summary.activeLines.join("\n")}\n.end`);
    expect(fourierSummary.fourier.map((card) => [
      card.directive,
      card.fundamentalFrequencyHz,
      card.probes,
    ])).toStrictEqual([
      [".four", 1000, ["V(out)"]],
      [".four", 2000, ["V(in)"]],
    ]);
  });

  it("expands include files and selected library sections", () => {
    const summary = resolveDeckSources(`
V1 in 0 DC 1
.include models.inc
.lib vendor.lib TT
.op
.end
Rafter out 0 1
`, {
      "models.inc": `
* model include
.model D1 D
Rshim in mid 10
`,
      "vendor.lib": `
.lib FF
Rfast out 0 1
.endl FF
.lib TT
Rtyp mid out 20
Ctyp out 0 1u
.endl TT
`,
    });

    expect(summary.terminated).toBe(true);
    expect(summary.endLineNumber).toBe(6);
    expect(summary.activeLines).toStrictEqual([
      "V1 in 0 DC 1",
      ".model D1 D",
      "Rshim in mid 10",
      "Rtyp mid out 20",
      "Ctyp out 0 1u",
      ".op",
    ]);
    expect(summary.includedPaths).toStrictEqual(["models.inc"]);
    expect(summary.librarySections).toStrictEqual(["vendor.lib:TT"]);
    expect(summary.diagnostics).toStrictEqual([]);
  });

  it("reports missing include and library sources plus include cycles", () => {
    const summary = resolveDeckSources(`
.include missing.inc
.include a.inc
.lib vendor.lib SS
.control
op
save V(a)
probe V(b)
print op V(a)
measure tran vmax MAX V(a)
meas dc imax MAX I(V1)
fourier 1k V(a)
four 2k V(b)
.reset
.set noaskquit
.set filetype=ascii
.set wr_vecnames
.set wr_singlescale
.set appendwrite
.write out.raw V(a)
.wrdata out.dat V(a)
.display all
.listing deck
.show all
.showmod Q1
.status
.version
.help tran
.echo running selected deck
.rusage all
.where
.source nested-control.cir
source plain-control.cir
.shell echo nope
shell echo plain
.cd nested
cd /tmp
.if $&run_monte
while $&again
.foreach dev M1 M2
repeat 2
run
.quit
.endc
.end
`, {
      "a.inc": ".include b.inc\nR1 a b 1\n",
      "b.inc": ".include a.inc\nR2 b 0 2\n",
      "vendor.lib": ".lib TT\nRtyp out 0 20\n.endl TT\n",
    });

    expect(summary.terminated).toBe(true);
    expect(summary.activeLines).toStrictEqual([
      "R2 b 0 2",
      "R1 a b 1",
      ".op",
      ".save V(a)",
      ".probe V(b)",
      ".print op V(a)",
      ".measure tran vmax MAX V(a)",
      ".meas dc imax MAX I(V1)",
      ".four 1k V(a)",
      ".four 2k V(b)",
    ]);
    expect(summary.diagnostics.map((diagnostic) => diagnostic.code)).toStrictEqual([
      "SPICE_DECK_INCLUDE_NOT_FOUND",
      "SPICE_DECK_INCLUDE_CYCLE",
      "SPICE_DECK_LIB_SECTION_NOT_FOUND",
      "SPICE_DECK_UNSUPPORTED_DIRECTIVE",
      "SPICE_DECK_CONTROL_SCRIPT_COMMAND",
      "SPICE_DECK_CONTROL_SCRIPT_COMMAND",
      "SPICE_DECK_CONTROL_SCRIPT_COMMAND",
      "SPICE_DECK_CONTROL_SCRIPT_COMMAND",
      "SPICE_DECK_CONTROL_WORKDIR_COMMAND",
      "SPICE_DECK_CONTROL_WORKDIR_COMMAND",
      "SPICE_DECK_CONTROL_FLOW_COMMAND",
      "SPICE_DECK_CONTROL_FLOW_COMMAND",
      "SPICE_DECK_CONTROL_FLOW_COMMAND",
      "SPICE_DECK_CONTROL_FLOW_COMMAND",
    ]);
    expect(summary.diagnostics.slice(3).map(({ directive, lineNumber }) => [
      directive,
      lineNumber,
    ])).toStrictEqual([
      [".control", 5],
      [".control", 32],
      [".control", 33],
      [".control", 34],
      [".control", 35],
      [".control", 36],
      [".control", 37],
      [".control", 38],
      [".control", 39],
      [".control", 40],
      [".control", 41],
    ]);
    const measurementSummary = resolveDeckMeasurements(`${summary.activeLines.join("\n")}\n.end`);
    expect(measurementSummary.measurements.map((card) => [
      card.directive,
      card.analysis,
      card.name,
      card.mode,
      card.probe,
    ])).toStrictEqual([
      [".measure", "tran", "vmax", "max", "V(a)"],
      [".meas", "dc", "imax", "max", "I(V1)"],
    ]);
    const fourierSummary = resolveDeckFourier(`${summary.activeLines.join("\n")}\n.end`);
    expect(fourierSummary.fourier.map((card) => [
      card.directive,
      card.fundamentalFrequencyHz,
      card.probes,
    ])).toStrictEqual([
      [".four", 1000, ["V(a)"]],
      [".four", 2000, ["V(b)"]],
    ]);
    expect(summary.diagnostics.slice(0, 3).map(({ source, lineNumber, target }) => [
      source,
      lineNumber,
      target,
    ])).toStrictEqual([
      ["<deck>", 2, "missing.inc"],
      ["b.inc", 1, "a.inc"],
      ["<deck>", 4, "vendor.lib:SS"],
    ]);
  });

  it("rewrites braced and quoted parameter expressions", () => {
    const summary = resolveDeckParameters(`
.param RLOAD=2k SCALE=3 TOTAL=RLOAD*SCALE
V1 in 0 DC {scale+1}
R1 in out {total}
C1 out 0 '2u*scale'
.op
.end
Rafter out 0 {total}
`);

    expect(summary.terminated).toBe(true);
    expect(summary.endLineNumber).toBe(7);
    expect(summary.parameters.map((parameter) => [parameter.name, parameter.value])).toStrictEqual([
      ["RLOAD", 2000],
      ["SCALE", 3],
      ["TOTAL", 6000],
    ]);
    expect(summary.activeLines).toStrictEqual([
      "V1 in 0 DC 4",
      "R1 in out 6000",
      "C1 out 0 0.000006",
      ".op",
    ]);
    expect(summary.diagnostics).toStrictEqual([]);
  });

  it("evaluates scalar .func calls in parameter expressions", () => {
    const summary = resolveDeckParameters(`
.func gain(x) {x*2}
.param BASE=2 SCALE=3 SHIFT=1 TOTAL=blend(base,scale,shift)
.func blend(a,b,c) 'gain(a)+b+c'
R1 in out {gain(total)}
B1 out 0 V='blend(1,2,3)'
.op
.end
`);

    expect(summary.terminated).toBe(true);
    expect(summary.endLineNumber).toBe(8);
    expect(summary.activeLines).toStrictEqual([
      "R1 in out 16",
      "B1 out 0 V=7",
      ".op",
    ]);
    expect(summary.parameters.map((parameter) => [parameter.name, parameter.value])).toStrictEqual([
      ["BASE", 2],
      ["SCALE", 3],
      ["SHIFT", 1],
      ["TOTAL", 8],
    ]);
    expect(summary.diagnostics).toStrictEqual([]);
  });

  it("reports bad scalar .func calls in parameter expressions", () => {
    const summary = resolveDeckParameters(`
.func one(x) {x+1}
.func loop(x) {loop(x)}
.param GOOD=one(1) BAD=unknown(1) ARITY=one(1,2) RECUR=loop(1)
R1 in out {bad}
R2 out 0 {good}
.end
`);

    expect(summary.activeLines).toStrictEqual([
      "R1 in out {bad}",
      "R2 out 0 2",
    ]);
    expect(summary.parameters.map((parameter) => [parameter.name, parameter.value])).toStrictEqual([
      ["GOOD", 2],
    ]);
    expect(summary.diagnostics.map((diagnostic) => diagnostic.code)).toStrictEqual([
      "SPICE_DECK_PARAM_EXPRESSION",
      "SPICE_DECK_PARAM_EXPRESSION",
      "SPICE_DECK_PARAM_EXPRESSION",
      "SPICE_DECK_PARAM_UNRESOLVED",
    ]);
    expect(summary.diagnostics.slice(0, 3).map((diagnostic) => diagnostic.parameter)).toStrictEqual([
      "BAD",
      "ARITY",
      "RECUR",
    ]);
    expect(summary.diagnostics.map((diagnostic) => diagnostic.expression)).toStrictEqual([
      "unknown(1)",
      "one(1,2)",
      "loop(1)",
      "bad",
    ]);
  });

  it("extracts .ic and .nodeset node-voltage hints", () => {
    const summary = resolveDeckInitialConditions(`
V1 in 0 DC 1
.ic V(out)=1.2 V(mid)='2.5'
.nodeset V(bias)={700m}
.op
.end
.ic V(after)=9
`);

    expect(summary.terminated).toBe(true);
    expect(summary.endLineNumber).toBe(6);
    expect(summary.activeLines).toStrictEqual(["V1 in 0 DC 1", ".op"]);
    expect(summary.initialConditions.map(({ directive, node, value, lineNumber }) => [
      directive,
      node,
      value,
      lineNumber,
    ])).toStrictEqual([
      [".ic", "out", 1.2, 3],
      [".ic", "mid", 2.5, 3],
    ]);
    expect(summary.nodesets).toHaveLength(1);
    expect(summary.nodesets[0]).toMatchObject({
      directive: ".nodeset",
      node: "bias",
      lineNumber: 4,
    });
    expect(summary.nodesets[0].value).toBeCloseTo(0.7);
    expect(summary.diagnostics).toStrictEqual([]);
  });

  it("reports malformed .ic and .nodeset assignments", () => {
    const summary = resolveDeckInitialConditions(`
.ic out=1 V()=2 V(ok)=bad V(good)=1k
.nodeset
.nodeset I(L1)=2
.end
`);

    expect(summary.terminated).toBe(true);
    expect(summary.endLineNumber).toBe(5);
    expect(summary.activeLines).toStrictEqual([]);
    expect(summary.initialConditions.map(({ directive, node, value, lineNumber }) => [
      directive,
      node,
      value,
      lineNumber,
    ])).toStrictEqual([[".ic", "good", 1000, 2]]);
    expect(summary.nodesets).toStrictEqual([]);
    expect(summary.diagnostics.map((diagnostic) => diagnostic.code)).toStrictEqual([
      "SPICE_DECK_CONDITION_TARGET",
      "SPICE_DECK_CONDITION_TARGET",
      "SPICE_DECK_CONDITION_EXPRESSION",
      "SPICE_DECK_CONDITION_ARGUMENT",
      "SPICE_DECK_CONDITION_TARGET",
    ]);
    expect(summary.diagnostics.map((diagnostic) => diagnostic.directive)).toStrictEqual([
      ".ic",
      ".ic",
      ".ic",
      ".nodeset",
      ".nodeset",
    ]);
  });

  it("extracts .func definitions", () => {
    const summary = resolveDeckFunctions(`
R1 in out {gain(vin)}
.func gain(x) {x*2}
.func blend(a,b,weight) 'a*(1-weight)+b*weight'
.op
.end
.func after(x) {x}
`);

    expect(summary.terminated).toBe(true);
    expect(summary.endLineNumber).toBe(6);
    expect(summary.activeLines).toStrictEqual(["R1 in out {gain(vin)}", ".op"]);
    expect(summary.functions.map(({ name, arguments: args, expression, lineNumber }) => [
      name,
      args,
      expression,
      lineNumber,
    ])).toStrictEqual([
      ["gain", ["x"], "x*2", 3],
      ["blend", ["a", "b", "weight"], "a*(1-weight)+b*weight", 4],
    ]);
    expect(summary.diagnostics).toStrictEqual([]);
  });

  it("reports malformed .func definitions", () => {
    const summary = resolveDeckFunctions(`
.func
.func 1bad(x) {x}
.func noexpr(x)
.func badarg(1x,x) {x}
.func dup(x,x) {x}
.end
`);

    expect(summary.terminated).toBe(true);
    expect(summary.endLineNumber).toBe(7);
    expect(summary.activeLines).toStrictEqual([]);
    expect(summary.functions).toStrictEqual([]);
    expect(summary.diagnostics.map((diagnostic) => diagnostic.code)).toStrictEqual([
      "SPICE_DECK_FUNC_ARGUMENT",
      "SPICE_DECK_FUNC_SIGNATURE",
      "SPICE_DECK_FUNC_EXPRESSION",
      "SPICE_DECK_FUNC_ARGUMENT",
      "SPICE_DECK_FUNC_ARGUMENT",
    ]);
    expect(summary.diagnostics.map((diagnostic) => diagnostic.functionName)).toStrictEqual([
      undefined,
      "1bad",
      "noexpr",
      "badarg",
      "dup",
    ]);
  });

  it("extracts transient .measure cards", () => {
    const summary = resolveDeckMeasurements(`
V1 in 0 PULSE(0 1 0 1n 1n 1m 2m)
.measure tran swing pp V(out) FROM=1m TO={3m}
.meas transient settled final 'V(out)'
.measure tran sample find V(out) AT={1.5m}
.measure tran crossing when V(out)=0.5 FROM=1m TO=3m RISE=1
.measure tran prop_delay TRIG V(in) VAL=0.5 RISE=1 TARG V(out) VAL=0.5 FALL=1 FROM=0 TO=4m
.measure dc dcmax max V(out) FROM=1 TO=3
.measure ac acmax max V(out) FROM=1k TO=10k
.tran 1m 4m
.end
.measure tran after max V(out)
`);

    expect(summary.terminated).toBe(true);
    expect(summary.endLineNumber).toBe(11);
    expect(summary.activeLines).toStrictEqual([
      "V1 in 0 PULSE(0 1 0 1n 1n 1m 2m)",
      ".tran 1m 4m",
    ]);
    expect(summary.measurements.map(({
      directive,
      analysis,
      name,
      mode,
      probe,
      lineNumber,
      fromValue,
      toValue,
      atValue,
      targetValue,
      crossingKind,
      crossingCount,
      triggerProbe,
      triggerValue,
      triggerCrossingKind,
      triggerCrossingCount,
    }) => [
      directive,
      analysis,
      name,
      mode,
      probe,
      lineNumber,
      fromValue,
      toValue,
      atValue,
      targetValue,
      crossingKind,
      crossingCount,
      triggerProbe,
      triggerValue,
      triggerCrossingKind,
      triggerCrossingCount,
    ])).toStrictEqual([
      [".measure", "tran", "swing", "pp", "V(out)", 3, 0.001, 0.003, undefined, undefined, undefined, undefined, undefined, undefined, undefined, undefined],
      [".meas", "transient", "settled", "last", "V(out)", 4, undefined, undefined, undefined, undefined, undefined, undefined, undefined, undefined, undefined, undefined],
      [".measure", "tran", "sample", "find", "V(out)", 5, undefined, undefined, 0.0015, undefined, undefined, undefined, undefined, undefined, undefined, undefined],
      [".measure", "tran", "crossing", "when", "V(out)", 6, 0.001, 0.003, undefined, 0.5, "rise", 1, undefined, undefined, undefined, undefined],
      [".measure", "tran", "prop_delay", "delay", "V(out)", 7, 0, 0.004, undefined, 0.5, "fall", 1, "V(in)", 0.5, "rise", 1],
      [".measure", "dc", "dcmax", "max", "V(out)", 8, 1, 3, undefined, undefined, undefined, undefined, undefined, undefined, undefined, undefined],
      [".measure", "ac", "acmax", "max", "V(out)", 9, 1000, 10000, undefined, undefined, undefined, undefined, undefined, undefined, undefined, undefined],
    ]);
    expect(summary.diagnostics).toStrictEqual([]);
  });

  it("reports unsupported .measure subsets", () => {
    const summary = resolveDeckMeasurements(`
.measure tf gain max V(out)
.measure tran badmode deriv V(out)
.measure tran badname max V(out) FROM=2m TO=1m
.measure tran badopt max V(out) AT=1m
.measure tran badexpr max V(out) FROM={1+}
.end
`);

    expect(summary.terminated).toBe(true);
    expect(summary.endLineNumber).toBe(7);
    expect(summary.measurements).toStrictEqual([]);
    expect(summary.diagnostics.map((diagnostic) => diagnostic.code)).toStrictEqual([
      "SPICE_DECK_MEASURE_ANALYSIS",
      "SPICE_DECK_MEASURE_MODE",
      "SPICE_DECK_MEASURE_WINDOW",
      "SPICE_DECK_MEASURE_ARGUMENT",
      "SPICE_DECK_MEASURE_EXPRESSION",
    ]);
  });

  it("extracts transient .four cards", () => {
    const summary = resolveDeckFourier(`
V1 in 0 SIN(0 1 1k)
.tran 1u 2m
.four {1k} V(in) V(out) HARMONICS=5 FROM=1m
.four 2k "I(V1)"
.end
.four 3k V(ignored)
`);

    expect(summary.activeLines).toStrictEqual(["V1 in 0 SIN(0 1 1k)", ".tran 1u 2m"]);
    expect(summary.terminated).toBe(true);
    expect(summary.endLineNumber).toBe(6);
    expect(summary.diagnostics).toStrictEqual([]);
    expect(summary.fourier).toHaveLength(2);
    expect(summary.fourier[0].fundamentalFrequencyHz).toBeCloseTo(1000.0, 12);
    expect(summary.fourier[0].probes).toStrictEqual(["V(in)", "V(out)"]);
    expect(summary.fourier[0].harmonics).toBe(5);
    expect(summary.fourier[0].fromValue).toBeCloseTo(1.0e-3, 12);
    expect(summary.fourier[1].probes).toStrictEqual(["I(V1)"]);
    expect(summary.fourier[1].harmonics).toBeUndefined();
  });

  it("reports unsupported .four subsets", () => {
    const summary = resolveDeckFourier(`
.four 0 V(out)
.four 1k
.four 1k V(out) HARMONICS=1.5
.four 1k V(out) TO=2m
.four 1k ""
.end
`);

    expect(summary.fourier).toStrictEqual([]);
    expect(summary.diagnostics.map((diagnostic) => diagnostic.code).sort()).toStrictEqual([
      "SPICE_DECK_FOURIER_ARGUMENT",
      "SPICE_DECK_FOURIER_ARGUMENT",
      "SPICE_DECK_FOURIER_ARGUMENT",
      "SPICE_DECK_FOURIER_FREQUENCY",
      "SPICE_DECK_FOURIER_PROBE",
    ]);
  });

  it("extracts .save, .probe, .print, and .plot output cards", () => {
    const summary = resolveDeckOutputs(`
V1 in 0 DC 1
.save V(out) i(V1)
.probe tran V(clk)
.probe AC V(out)
.print dc V(load) I(V2)
.plot ac I(V3)
.end
.save V(ignored)
`);

    expect(summary.activeLines).toStrictEqual(["V1 in 0 DC 1"]);
    expect(summary.terminated).toBe(true);
    expect(summary.endLineNumber).toBe(8);
    expect(summary.diagnostics).toStrictEqual([]);
    expect(
      summary.selections.map((selection) => [
        selection.directive,
        selection.analysis,
        selection.probes,
      ]),
    ).toStrictEqual([
      [".save", undefined, ["V(out)", "I(V1)"]],
      [".probe", "tran", ["V(clk)"]],
      [".probe", "ac", ["V(out)"]],
      [".print", "dc", ["V(load)", "I(V2)"]],
      [".plot", "ac", ["I(V3)"]],
    ]);

    expect(
      selectDeckOutputProbes(
        `
.save V(out) I(V1)
.probe tran V(out) V(clk)
.print tran I(V2)
.plot tran V(extra)
.probe ac V(freq)
.end
`,
        "transient",
      ),
    ).toStrictEqual(["V(out)", "I(V1)", "V(clk)", "I(V2)", "V(extra)"]);
  });

  it("reports invalid .save, .probe, .print, and .plot output cards", () => {
    const summary = resolveDeckOutputs(`
.save
.probe tran
.print tran
.print foo V(out)
.plot tran
.plot foo V(out)
.save P(out)
.probe dc V(out) bad-token
.print dc bad-token
.plot dc bad-token
.end
`);

    expect(summary.diagnostics.map((diagnostic) => diagnostic.code).sort()).toStrictEqual([
      "SPICE_DECK_OUTPUT_ANALYSIS",
      "SPICE_DECK_OUTPUT_ANALYSIS",
      "SPICE_DECK_OUTPUT_ARGUMENT",
      "SPICE_DECK_OUTPUT_ARGUMENT",
      "SPICE_DECK_OUTPUT_ARGUMENT",
      "SPICE_DECK_OUTPUT_ARGUMENT",
      "SPICE_DECK_OUTPUT_PROBE",
      "SPICE_DECK_OUTPUT_PROBE",
      "SPICE_DECK_OUTPUT_PROBE",
      "SPICE_DECK_OUTPUT_PROBE",
    ]);
    expect(summary.selections[0].probes).toStrictEqual(["V(out)"]);
    expect(() => selectDeckOutputProbes("\n.save\n.end\n", "dc")).toThrow(/line 2/);
  });

  it("extracts supported deck analysis cards", () => {
    const summary = resolveDeckAnalyses(`
V1 in 0 DC 0
R1 in out 1k
.op
.dc V1 0 5 1
.ac dec 10 1k 1Meg
.tran 1u 2m 0 10u uic
.end
.tran 1u 1m
`);

    expect(summary.activeLines).toStrictEqual(["V1 in 0 DC 0", "R1 in out 1k"]);
    expect(summary.terminated).toBe(true);
    expect(summary.endLineNumber).toBe(8);
    expect(summary.diagnostics).toStrictEqual([]);
    expect(summary.analyses.map((analysis) => analysis.analysis)).toStrictEqual([
      "op",
      "dc",
      "ac",
      "tran",
    ]);

    const dc = summary.analyses[1];
    expect(dc.directive).toBe(".dc");
    expect(dc.sourceName).toBe("V1");
    expect(dc.startValue).toBeCloseTo(0.0);
    expect(dc.stopValue).toBeCloseTo(5.0);
    expect(dc.stepValue).toBeCloseTo(1.0);

    const ac = summary.analyses[2];
    expect(ac.directive).toBe(".ac");
    expect(ac.sweepKind).toBe("dec");
    expect(ac.pointCount).toBe(10);
    expect(ac.startFrequencyHz).toBeCloseTo(1.0e3);
    expect(ac.stopFrequencyHz).toBeCloseTo(1.0e6);

    const tran = summary.analyses[3];
    expect(tran.directive).toBe(".tran");
    expect(tran.stepTime).toBeCloseTo(1.0e-6);
    expect(tran.stopTime).toBeCloseTo(2.0e-3);
    expect(tran.startTime).toBeCloseTo(0.0);
    expect(tran.maxStep).toBeCloseTo(1.0e-5);
    expect(tran.useInitialConditions).toBe(true);
  });

  it("reports invalid deck analysis cards", () => {
    const summary = resolveDeckAnalyses(`
.op extra
.dc V1 0 1 0
.dc V1 1 0 1
.ac decade 10 1 10
.ac lin 0 1 10
.tran 0 1m
.tran 1u 2m 0 1u extra
.end
`);

    expect(summary.analyses).toStrictEqual([]);
    expect(summary.diagnostics.map((diagnostic) => diagnostic.code).sort()).toStrictEqual([
      "SPICE_DECK_ANALYSIS_ARGUMENT",
      "SPICE_DECK_ANALYSIS_ARGUMENT",
      "SPICE_DECK_ANALYSIS_INTERVAL",
      "SPICE_DECK_ANALYSIS_MODE",
      "SPICE_DECK_ANALYSIS_SWEEP",
      "SPICE_DECK_ANALYSIS_SWEEP",
      "SPICE_DECK_ANALYSIS_SWEEP",
    ]);
  });

  it("defaults and selects deck analysis plans", () => {
    const implicit = selectDeckAnalysisPlan(`
V1 in 0 DC 1
R1 in 0 1k
.end
`);
    expect(implicit.directive).toBe(".op");
    expect(implicit.analysis).toBe("op");
    expect(implicit.lineNumber).toBe(0);

    const selected = selectDeckAnalysisPlan(
      `
V1 in 0 DC 0
.dc V1 0 5 1
.tran 1u 2m
.end
`,
      "transient",
    );
    expect(selected.directive).toBe(".tran");
    expect(selected.analysis).toBe("tran");
    expect(selected.lineNumber).toBe(4);
    expect(selected.stopTime).toBeCloseTo(2.0e-3);
  });

  it("reports ambiguous or invalid deck analysis plan selection", () => {
    expect(() =>
      selectDeckAnalysisPlan(`
.dc V1 0 5 1
.tran 1u 2m
.end
`),
    ).toThrow(/multiple analysis cards/);

    expect(() =>
      selectDeckAnalysisPlan(
        `
.tran 1u 2m
.tran 2u 4m
.end
`,
        ".tran",
      ),
    ).toThrow(/multiple \.tran analysis cards/);

    expect(() => selectDeckAnalysisPlan(".op\n.end\n", "noise")).toThrow(
      /unsupported analysis/,
    );

    expect(() =>
      selectDeckAnalysisPlan(`
.dc V1 0 1 0
.end
`),
    ).toThrow(/line 2: \.dc step value must be non-zero/);
  });
});
