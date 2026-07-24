import { describe, expect, it } from "vitest";
import {
  Circuit,
  SinWaveform,
  SpiceError,
  analyzeCustomModelSource,
  bSourceCurrent,
  bSourceVoltage,
  bjt,
  bjtAtTemperature,
  bjtFromModelCard,
  circuitAtTemperature,
  cccs,
  ccvs,
  customLinearConductanceModel,
  currentSource,
  dcCorners,
  dcInitialVectorFromConditions,
  dcOp,
  dcOpWithInitialConditions,
  dcSweep,
  dcSweepCorners,
  dcTemperatureSweep,
  dcTemperatureSweepCorners,
  deviceModelAuditFixtures,
  deviceModelBehaviorAuditFixtures,
  deviceModelReferenceDeckAuditAnalysisSummary,
  deviceModelReferenceDeckAuditAnalysisSummaryRecords,
  deviceModelReferenceDeckAuditFixtures,
  deviceModelReferenceDeckAuditGate,
  deviceModelReferenceDeckAuditGateCoverageDigest,
  deviceModelReferenceDeckAuditGateCoverageDigestRecords,
  deviceModelReferenceDeckAuditGateIssueRecords,
  deviceModelReferenceDeckAuditGateIssueSummary,
  deviceModelReferenceDeckAuditGateIssueSummaryRecords,
  deviceModelReferenceDeckAuditMatrix,
  deviceModelReferenceDeckAuditMatrixRecords,
  deviceModelReferenceDeckAuditRecords,
  deviceModelReferenceDeckAuditSummary,
  deviceModelReferenceDeckAuditSummaryRecords,
  deviceModelTemperatureAuditFixtures,
  diode,
  diodeAtTemperature,
  diodeFromModelCard,
  formatCornerDcSweepTable,
  formatCornerDcTable,
  formatCornerTemperatureDcTable,
  formatDeckDcSweepTable,
  formatDcSweepTable,
  formatDeviceModelReferenceDeckAuditAnalysisSummaryCsv,
  formatDeviceModelReferenceDeckAuditAnalysisSummaryJson,
  formatDeviceModelReferenceDeckAuditAnalysisSummaryTable,
  formatDeviceModelReferenceDeckAuditCsv,
  formatDeviceModelReferenceDeckAuditGateCoverageDigestCsv,
  formatDeviceModelReferenceDeckAuditGateCoverageDigestJson,
  formatDeviceModelReferenceDeckAuditGateCoverageDigestTable,
  formatDeviceModelReferenceDeckAuditGateIssueCsv,
  formatDeviceModelReferenceDeckAuditGateIssueJson,
  formatDeviceModelReferenceDeckAuditGateIssueSummaryCsv,
  formatDeviceModelReferenceDeckAuditGateIssueSummaryJson,
  formatDeviceModelReferenceDeckAuditGateIssueSummaryTable,
  formatDeviceModelReferenceDeckAuditGateIssueTable,
  formatDeviceModelReferenceDeckAuditGateReport,
  formatDeviceModelReferenceDeckAuditJson,
  formatDeviceModelReferenceDeckAuditMatrixCsv,
  formatDeviceModelReferenceDeckAuditMatrixJson,
  formatDeviceModelReferenceDeckAuditMatrixTable,
  formatDeviceModelReferenceDeckAuditSummaryCsv,
  formatDeviceModelReferenceDeckAuditSummaryJson,
  formatDeviceModelReferenceDeckAuditSummaryTable,
  formatDeviceModelReferenceDeckAuditTable,
  formatModelCardSupportedParameterCoverageCsv,
  formatModelCardSupportedParameterCoverageGateIssueCsv,
  formatModelCardSupportedParameterCoverageGateIssueJson,
  formatModelCardSupportedParameterCoverageGateIssueTable,
  formatModelCardSupportedParameterCoverageGateReport,
  formatModelCardSupportedParameterCoverageJson,
  formatModelCardSupportedParameterCoverageSummaryCsv,
  formatModelCardSupportedParameterCoverageSummaryJson,
  formatModelCardSupportedParameterCoverageSummaryTable,
  formatModelCardSupportedParameterCoverageTable,
  formatMeasurementTable,
  formatTemperatureDcTable,
  inductor,
  jfet,
  jfetFromModelCard,
  measureDcSweepDeck,
  measureDcSweepProbe,
  modelCardSupportedParameterCoverage,
  modelCardSupportedParameterCoverageGate,
  modelCardSupportedParameterCoverageGateIssueRecords,
  modelCardSupportedParameterCoverageRecords,
  modelCardSupportedParameterCoverageSummary,
  modelCardSupportedParameterCoverageSummaryRecords,
  mosfet,
  mosfetFromModelCard,
  normalizeModelCard,
  normalizeModelCardType,
  resistor,
  resolveDeckInitialConditions,
  subcircuitDefinition,
  vccs,
  vcvs,
  voltageSource,
  voltageSourceWithWaveform,
  xInstance,
} from "../src/index.js";

function expectClose(actual: number | undefined, expected: number): void {
  expect(actual).not.toBeUndefined();
  expect(actual!).toBeCloseTo(expected, 9);
}

describe("dcOp", () => {
  it("normalizes model-card type aliases", () => {
    expect(normalizeModelCardType("diode")).toBe("D");
    expect(normalizeModelCardType("n-jfet")).toBe("NJF");
    expect(normalizeModelCardType("pch")).toBe("PMOS");
  });

  it("exports stable model-card supported parameter coverage", () => {
    const coverage = modelCardSupportedParameterCoverage();
    expect(coverage).toHaveLength(149);
    expect(coverage[0]).toStrictEqual({
      kind: "D",
      canonicalParameter: "IS",
      acceptedNames: ["IS", "JS"],
      aliasCount: 2,
    });
    expect(coverage.at(-1)).toStrictEqual({
      kind: "PMOS",
      canonicalParameter: "MJ",
      acceptedNames: ["MJ"],
      aliasCount: 1,
    });

    const table = formatModelCardSupportedParameterCoverageTable();
    expect(table.split("\n")[0]).toBe("kind\tcanonical_parameter\taccepted_names\talias_count");
    expect(table.split("\n")[1]).toBe("D\tIS\tIS|JS\t2");
    expect(table).toContain("NMOS\tVT0\tVT0|VTO|VTH\t3");
    expect(table.split("\n").at(-1)).toBe("PMOS\tMJ\tMJ\t1");
    const records = modelCardSupportedParameterCoverageRecords();
    expect(records).toHaveLength(149);
    expect(records[0]).toStrictEqual({
      kind: "D",
      canonical_parameter: "IS",
      accepted_names: "IS|JS",
      alias_count: "2",
    });
    expect(formatModelCardSupportedParameterCoverageCsv()).toMatch(
      /^kind,canonical_parameter,accepted_names,alias_count\nD,IS,IS\|JS,2\n/,
    );
    expect(JSON.parse(formatModelCardSupportedParameterCoverageJson())).toStrictEqual(records);
  });

  it("exports stable model-card supported parameter coverage summaries", () => {
    const summary = modelCardSupportedParameterCoverageSummary();
    expect(summary).toHaveLength(7);
    expect(summary[0]).toStrictEqual({
      kind: "D",
      canonicalParameterCount: 15,
      acceptedNameCount: 21,
      aliasedParameterCount: 5,
      maxAliasCount: 3,
      aliasedParameters: ["IS", "VT", "CJO", "VJ", "M"],
    });
    expect(summary[5]).toStrictEqual({
      kind: "NMOS",
      canonicalParameterCount: 18,
      acceptedNameCount: 25,
      aliasedParameterCount: 6,
      maxAliasCount: 3,
      aliasedParameters: ["VT0", "LAMBDA", "N_SUB", "T_NOM", "CBS", "CBD"],
    });
    expect(summary.at(-1)?.kind).toBe("PMOS");

    const table = formatModelCardSupportedParameterCoverageSummaryTable();
    expect(table.split("\n")[0]).toBe(
      "kind\tcanonical_parameter_count\taccepted_name_count\taliased_parameter_count\tmax_alias_count\taliased_parameters",
    );
    expect(table.split("\n")[1]).toBe("D\t15\t21\t5\t3\tIS|VT|CJO|VJ|M");
    expect(table.split("\n").at(-1)).toBe(
      "PMOS\t18\t25\t6\t3\tVT0|LAMBDA|N_SUB|T_NOM|CBS|CBD",
    );
    const records = modelCardSupportedParameterCoverageSummaryRecords();
    expect(records).toHaveLength(7);
    expect(records[0]).toStrictEqual({
      kind: "D",
      canonical_parameter_count: "15",
      accepted_name_count: "21",
      aliased_parameter_count: "5",
      max_alias_count: "3",
      aliased_parameters: "IS|VT|CJO|VJ|M",
    });
    expect(formatModelCardSupportedParameterCoverageSummaryCsv()).toMatch(
      /^kind,canonical_parameter_count,accepted_name_count,aliased_parameter_count,max_alias_count,aliased_parameters\nD,15,21,5,3,IS\|VT\|CJO\|VJ\|M\n/,
    );
    expect(JSON.parse(formatModelCardSupportedParameterCoverageSummaryJson())).toStrictEqual(records);
  });

  it("passes the current model-card supported parameter coverage gate", () => {
    const report = modelCardSupportedParameterCoverageGate();

    expect(report).toStrictEqual({
      passed: true,
      kindCount: 7,
      expectedKindCount: 7,
      canonicalParameterCount: 149,
      expectedCanonicalParameterCount: 149,
      acceptedNameCount: 217,
      aliasedParameterCount: 55,
      maxAliasCount: 4,
      issues: [],
    });
    expect(formatModelCardSupportedParameterCoverageGateReport(report)).toBe(
      "passed\tkind_count\texpected_kind_count\tcanonical_parameter_count\texpected_canonical_parameter_count\taccepted_name_count\taliased_parameter_count\tmax_alias_count\tissue_count\ntrue\t7\t7\t149\t149\t217\t55\t4\t0",
    );
    expect(formatModelCardSupportedParameterCoverageGateIssueTable(report)).toBe(
      "kind\tfield\tmessage",
    );
    expect(modelCardSupportedParameterCoverageGateIssueRecords(report)).toStrictEqual([]);
    expect(formatModelCardSupportedParameterCoverageGateIssueCsv(report)).toBe(
      "kind,field,message\n",
    );
    expect(JSON.parse(formatModelCardSupportedParameterCoverageGateIssueJson(report))).toStrictEqual(
      [],
    );
  });

  it("reports missing model-card supported parameter alias families", () => {
    const trimmed = modelCardSupportedParameterCoverage().filter(
      (row) => !(row.kind === "NMOS" && row.canonicalParameter === "VT0"),
    );

    const report = modelCardSupportedParameterCoverageGate(trimmed);

    expect(report.passed).toBe(false);
    expect(report.kindCount).toBe(7);
    expect(report.canonicalParameterCount).toBe(148);
    expect(report.acceptedNameCount).toBe(214);
    expect(report.aliasedParameterCount).toBe(54);
    expect(report.maxAliasCount).toBe(4);
    expect(report.issues).toHaveLength(4);
    expect(report.issues[0]).toStrictEqual({
      kind: "NMOS",
      field: "canonical_parameter_count",
      message: "expected NMOS to expose 18 canonical supported parameters, found 17",
    });
    expect(report.issues.at(-1)).toStrictEqual({
      kind: "NMOS",
      field: "max_alias_count",
      message: "expected NMOS max alias count 3, found 2",
    });
    expect(formatModelCardSupportedParameterCoverageGateReport(report)).toBe(
      "passed\tkind_count\texpected_kind_count\tcanonical_parameter_count\texpected_canonical_parameter_count\taccepted_name_count\taliased_parameter_count\tmax_alias_count\tissue_count\nfalse\t7\t7\t148\t149\t214\t54\t4\t4\nkind\tfield\tmessage\nNMOS\tcanonical_parameter_count\texpected NMOS to expose 18 canonical supported parameters, found 17\nNMOS\taccepted_name_count\texpected NMOS to expose 25 accepted model-card names, found 22\nNMOS\taliased_parameter_count\texpected NMOS to expose 6 alias-bearing parameters, found 5\nNMOS\tmax_alias_count\texpected NMOS max alias count 3, found 2",
    );
    const records = modelCardSupportedParameterCoverageGateIssueRecords(report);
    expect(records[0]).toStrictEqual({
      kind: "NMOS",
      field: "canonical_parameter_count",
      message: "expected NMOS to expose 18 canonical supported parameters, found 17",
    });
    expect(formatModelCardSupportedParameterCoverageGateIssueCsv(report)).toMatch(
      /^kind,field,message\nNMOS,canonical_parameter_count,"expected NMOS to expose 18 canonical supported parameters, found 17"\n/,
    );
    expect(JSON.parse(formatModelCardSupportedParameterCoverageGateIssueJson(report))).toStrictEqual(
      records,
    );
  });

  it("normalizes model-card aliases into device instances", () => {
    const diodeCard = normalizeModelCard("Dfast", "diode", {
      JS: 2.0e-14,
      CJ: 1.5e-12,
      TT: 4.0e-9,
      PB: 0.8,
      MJ: 0.4,
      FC: 0.35,
      XTI: 2.2,
      EG: 1.05,
      RS: 10.0,
      KF: 1.0e-12,
      AF: 1.3,
    });
    const diodeModel = diodeFromModelCard("D1", "a", "k", diodeCard);
    expect(diodeCard.parameters).toStrictEqual({
      IS: 2.0e-14,
      CJO: 1.5e-12,
      TT: 4.0e-9,
      VJ: 0.8,
      M: 0.4,
      FC: 0.35,
      XTI: 2.2,
      EG: 1.05,
      RS: 10.0,
      KF: 1.0e-12,
      AF: 1.3,
    });
    expect(diodeCard.unsupportedParameters).toStrictEqual([]);
    expectClose(diodeModel.saturationCurrent, 2.0e-14);
    expectClose(diodeModel.junctionCapacitance, 1.5e-12);
    expectClose(diodeModel.transitTime, 4.0e-9);
    expectClose(diodeModel.junctionPotential, 0.8);
    expectClose(diodeModel.gradingCoefficient, 0.4);
    expectClose(diodeModel.forwardBiasDepletionCoefficient, 0.35);
    expectClose(diodeModel.saturationCurrentTemperatureExponent, 2.2);
    expectClose(diodeModel.energyGapElectronVolts, 1.05);
    expectClose(diodeModel.seriesResistance, 10.0);
    expectClose(diodeModel.flickerNoiseCoefficient, 1.0e-12);
    expectClose(diodeModel.flickerNoiseExponent, 1.3);

    const bjtCard = normalizeModelCard("Qsmall", "npn", {
      BETA: 125.0,
      CBE: 2.0e-12,
      XTI: 2.4,
      XTB: 1.5,
      BETA_R: 0.25,
      EG: 1.05,
      VA: 80.0,
      VB: 120.0,
      IK: 2.0e-3,
      IKR: 3.0e-3,
      T_NOM: 50.0,
      KF: 1.0e-12,
      AF: 1.3,
      PTF: 30.0,
      XTF: 2.0,
      ITF: 4.0e-3,
      VTF: 0.6,
      RE: 12.0,
      RC: 13.0,
      RB: 14.0,
      RBM: 2.0,
      IRB: 5.0e-6,
      XCJC: 0.4,
      ISE: 3.0e-13,
      NE: 1.7,
      ISC: 4.0e-13,
      NC: 1.8,
      NF: 1.2,
      NR: 1.3,
      PE: 0.8,
      ME: 0.4,
      PC: 0.7,
      MC: 0.45,
      FC: 0.4,
    });
    const bjtModel = bjtFromModelCard("Q1", "c", "b", "e", bjtCard);
    expect(bjtCard.parameters).toStrictEqual({ BF: 125.0, BR: 0.25, CJE: 2.0e-12, XTI: 2.4, XTB: 1.5, EG: 1.05, VAF: 80.0, VAR: 120.0, IKF: 2.0e-3, IKR: 3.0e-3, TNOM: 50.0, KF: 1.0e-12, AF: 1.3, PTF: 30.0, XTF: 2.0, ITF: 4.0e-3, VTF: 0.6, RE: 12.0, RC: 13.0, RB: 14.0, RBM: 2.0, IRB: 5.0e-6, XCJC: 0.4, ISE: 3.0e-13, NE: 1.7, ISC: 4.0e-13, NC: 1.8, NF: 1.2, NR: 1.3, VJE: 0.8, MJE: 0.4, VJC: 0.7, MJC: 0.45, FC: 0.4 });
    expect(bjtModel.polarity).toBe("NPN");
    expectClose(bjtModel.forwardBeta, 125.0);
    expectClose(bjtModel.reverseBeta, 0.25);
    expectClose(bjtModel.baseEmitterCapacitance, 2.0e-12);
    expectClose(bjtModel.saturationCurrentTemperatureExponent, 2.4);
    expectClose(bjtModel.forwardBetaTemperatureExponent, 1.5);
    expectClose(bjtModel.energyGapElectronVolts, 1.05);
    expectClose(bjtModel.forwardEarlyVoltage, 80.0);
    expectClose(bjtModel.reverseEarlyVoltage, 120.0);
    expectClose(bjtModel.forwardBetaRolloffCurrent, 2.0e-3);
    expectClose(bjtModel.reverseBetaRolloffCurrent, 3.0e-3);
    expectClose(bjtModel.nominalTemperatureKelvin, 323.15);
    expectClose(bjtModel.flickerNoiseCoefficient, 1.0e-12);
    expectClose(bjtModel.flickerNoiseExponent, 1.3);
    expectClose(bjtModel.forwardExcessPhaseDegrees, 30.0);
    expectClose(bjtModel.forwardTransitTimeBiasCoefficient, 2.0);
    expectClose(bjtModel.forwardTransitTimeCurrent, 4.0e-3);
    expectClose(bjtModel.forwardTransitTimeVoltage, 0.6);
    expectClose(bjtModel.emitterResistance, 12.0);
    expectClose(bjtModel.collectorResistance, 13.0);
    expectClose(bjtModel.baseResistance, 14.0);
    expectClose(bjtModel.minimumBaseResistance, 2.0);
    expectClose(bjtModel.baseResistanceHalfCurrent, 5.0e-6);
    expectClose(bjtModel.baseCollectorCapacitanceFraction, 0.4);
    expectClose(bjtModel.baseEmitterLeakageSaturationCurrent, 3.0e-13);
    expectClose(bjtModel.baseEmitterLeakageEmissionCoefficient, 1.7);
    expectClose(bjtModel.baseCollectorLeakageSaturationCurrent, 4.0e-13);
    expectClose(bjtModel.baseCollectorLeakageEmissionCoefficient, 1.8);
    expectClose(bjtModel.forwardEmissionCoefficient, 1.2);
    expectClose(bjtModel.reverseEmissionCoefficient, 1.3);
    expectClose(bjtModel.baseEmitterJunctionPotential, 0.8);
    expectClose(bjtModel.baseEmitterGradingCoefficient, 0.4);
    expectClose(bjtModel.baseCollectorJunctionPotential, 0.7);
    expectClose(bjtModel.baseCollectorGradingCoefficient, 0.45);
    expectClose(bjtModel.forwardBiasDepletionCoefficient, 0.4);

    const jfetCard = normalizeModelCard("Jn", "njfet", { BET: 9.0e-4, VT0: -1.8, LAM: 0.02, KF: 1.0e-12, AF: 1.3, VJ: 0.8 });
    const jfetModel = jfetFromModelCard("J1", "d", "g", "s", jfetCard);
    expect(jfetCard.parameters).toStrictEqual({ BETA: 9.0e-4, VTO: -1.8, LAMBDA: 0.02, KF: 1.0e-12, AF: 1.3, PB: 0.8 });
    expect(jfetModel.polarity).toBe("NJF");
    expectClose(jfetModel.beta, 9.0e-4);
    expectClose(jfetModel.thresholdVoltage, -1.8);
    expectClose(jfetModel.channelLengthModulation, 0.02);
    expectClose(jfetModel.flickerNoiseCoefficient, 1.0e-12);
    expectClose(jfetModel.flickerNoiseExponent, 1.3);
    expectClose(jfetModel.junctionPotential, 0.8);

    const mosCard = normalizeModelCard("Mn", "nmos", {
      LEVEL: 1.0,
      VTO: 0.55,
      LAM: 0.04,
      NSUB: 1.6,
      CJD: 3.0e-13,
      PB: 0.9,
      MJ: 0.45,
    });
    const mosModel = mosfetFromModelCard("M1", "d", "g", "s", "b", mosCard);
    expect(mosCard.parameters).toStrictEqual({
      LEVEL: 1.0,
      VT0: 0.55,
      LAMBDA: 0.04,
      N_SUB: 1.6,
      CBD: 3.0e-13,
      PB: 0.9,
      MJ: 0.45,
    });
    expect(mosModel.type).toBe("NMOS");
    expectClose(mosModel.params.VT0, 0.55);
    expectClose(mosModel.params.LAMBDA, 0.04);
    expectClose(mosModel.params.N_SUB, 1.6);
    expectClose(mosModel.params.CBD, 3.0e-13);
    expectClose(mosModel.params.PB, 0.9);
    expectClose(mosModel.params.MJ, 0.45);
  });

  it("derives BJT legacy leakage ratios with explicit-current precedence", () => {
    const legacyCard = normalizeModelCard("Qlegacy", "npn", {
      IS: 2.0e-14,
      C2: 15.0,
      C4: 20.0,
    });
    const legacy = bjtFromModelCard("Q1", "c", "b", "e", legacyCard);

    expect(legacyCard.parameters).toStrictEqual({
      IS: 2.0e-14,
      C2: 15.0,
      C4: 20.0,
    });
    expectClose(legacy.baseEmitterLeakageSaturationCurrent, 3.0e-13);
    expectClose(legacy.baseCollectorLeakageSaturationCurrent, 4.0e-13);

    const explicitCard = normalizeModelCard("Qexplicit", "pnp", {
      IS: 2.0e-14,
      C2: 15.0,
      ISE: 5.0e-13,
      C4: 20.0,
      ISC: 6.0e-13,
    });
    const explicit = bjtFromModelCard("Q2", "c", "b", "e", explicitCard);
    expectClose(explicit.baseEmitterLeakageSaturationCurrent, 5.0e-13);
    expectClose(explicit.baseCollectorLeakageSaturationCurrent, 6.0e-13);
  });

  it("provides cross-language device model audit fixtures", () => {
    const fixtures = deviceModelAuditFixtures();
    expect(fixtures.map((fixture) => fixture.kind)).toStrictEqual(["D", "NPN", "NJF", "NMOS"]);
    expectClose(fixtures[0]!.parameters.IS, 2.0e-14);
    expectClose(fixtures[1]!.parameters.BF, 125.0);
    expectClose(fixtures[2]!.parameters.VTO, -1.8);
    expectClose(fixtures[3]!.parameters.VT0, 0.55);
  });

  it("runs device model behavior audit fixtures as reference bias points", () => {
    const fixtures = deviceModelBehaviorAuditFixtures();
    expect(fixtures.map((fixture) => fixture.name)).toStrictEqual([
      "diode-forward-bias",
      "bjt-emitter-follower",
      "jfet-source-bias",
      "mos-level1-common-source",
    ]);

    for (const fixture of fixtures) {
      const result = dcOp(fixture.circuit);
      const value = result.voltage(fixture.probeNode);
      expect(result.converged).toBe(true);
      expect(value).not.toBeUndefined();
      expect(value!).toBeGreaterThanOrEqual(fixture.expectedMin);
      expect(value!).toBeLessThanOrEqual(fixture.expectedMax);
      expect(fixture.deckLines[0]!.startsWith("* device-model behavior fixture:")).toBe(true);
      expect(fixture.deckLines).toContain(".op");
      expect(fixture.deckLines.some((line) => line.startsWith(".model "))).toBe(true);
    }
  });

  it("runs device model temperature audit fixtures as reference sweeps", () => {
    const fixtures = deviceModelTemperatureAuditFixtures();
    expect(fixtures.map((fixture) => fixture.name)).toStrictEqual([
      "diode-forward-bias",
      "bjt-emitter-follower",
      "jfet-source-bias",
      "mos-level1-common-source",
    ]);

    for (const fixture of fixtures) {
      const result = dcTemperatureSweep(
        fixture.circuit,
        fixture.temperaturePoints.map((point) => point.temperatureKelvin),
        {},
        fixture.nominalTemperatureKelvin,
        fixture.energyGapElectronVolts,
      );
      expect(fixture.deckLines).toContain(".temp 260.15 300.15 340.15");
      expect(fixture.deckLines[0]!.startsWith("* device-model temperature fixture:")).toBe(true);
      expect(result.points).toHaveLength(fixture.temperaturePoints.length);
      for (let index = 0; index < result.points.length; index += 1) {
        const actual = result.points[index]!;
        const expected = fixture.temperaturePoints[index]!;
        const value = actual.result.voltage(fixture.probeNode);
        expect(actual.result.converged).toBe(true);
        expectClose(actual.temperatureKelvin, expected.temperatureKelvin);
        expect(value).not.toBeUndefined();
        expect(value!).toBeGreaterThanOrEqual(expected.expectedMin);
        expect(value!).toBeLessThanOrEqual(expected.expectedMax);
      }
    }

    const jfetFixture = fixtures.find((fixture) => fixture.kind === "NJF");
    expect(jfetFixture?.temperatureBehavior.startsWith("JFET temperature scaling is intentionally")).toBe(true);
  });

  it("summarizes device model reference deck audit fixture coverage", () => {
    const fixtures = deviceModelReferenceDeckAuditFixtures();
    expect(fixtures).toHaveLength(20);
    expect(fixtures[0]!.name).toBe("diode-forward-bias:op");
    expect(fixtures.at(-1)!.name).toBe("mos-level1-storage-charge:tran");

    const expectedAnalyses = ["ac", "noise", "op", "temperature", "tran"];
    expect([...new Set(fixtures.map((fixture) => fixture.kind))].sort()).toStrictEqual([
      "D",
      "NJF",
      "NMOS",
      "NPN",
    ]);
    for (const kind of ["D", "NPN", "NJF", "NMOS"]) {
      expect(
        [
          ...new Set(
            fixtures
              .filter((fixture) => fixture.kind === kind)
              .map((fixture) => fixture.analysis),
          ),
        ].sort(),
      ).toStrictEqual(expectedAnalyses);
    }

    for (const fixture of fixtures) {
      expect(fixture.reference).toBe("SPICE2/SPICE3-style local model-depth fixture");
      expect(fixture.expectedBehavior.length).toBeGreaterThan(0);
      expect(fixture.deckLines[0]!.startsWith("* device-model ")).toBe(true);
      expect(fixture.deckLines.some((line) => line.startsWith(".model "))).toBe(true);
      expect(fixture.deckLines.at(-1)).toBe(".end");
    }
  });

  it("formats a stable device model reference deck audit table", () => {
    const table = formatDeviceModelReferenceDeckAuditTable();
    const lines = table.split("\n");
    expect(lines).toHaveLength(21);
    expect(lines[0]).toBe("name\tkind\tanalysis\tmodel\treference\texpected_behavior\tdeck_lines");
    expect(lines[1]).toBe(
      "diode-forward-bias:op\tD\top\tDfast\tSPICE2/SPICE3-style local model-depth fixture\tDC probe out remains in [0.55, 0.65] V\t8",
    );
    expect(lines.at(-1)).toBe(
      "mos-level1-storage-charge:tran\tNMOS\ttran\tMn\tSPICE2/SPICE3-style local model-depth fixture\tLevel-1 MOS CGSO/CGDO/CGBO plus CBS/CBD contribute transient gate-overlap and depletion-shaped bulk-junction storage; explicit Cstore keeps the fixture comparable with other charge audits\t10",
    );
  });

  it("formats stable device model reference deck audit records", () => {
    const records = deviceModelReferenceDeckAuditRecords();
    expect(records).toHaveLength(20);
    expect(records[0]).toStrictEqual({
      name: "diode-forward-bias:op",
      kind: "D",
      analysis: "op",
      model: "Dfast",
      reference: "SPICE2/SPICE3-style local model-depth fixture",
      expected_behavior: "DC probe out remains in [0.55, 0.65] V",
      deck_lines: "8",
    });
    expect(records.at(-1)?.name).toBe("mos-level1-storage-charge:tran");
    expect(records.at(-1)?.deck_lines).toBe("10");

    const csvLines = formatDeviceModelReferenceDeckAuditCsv().split(/\r?\n/u).filter(Boolean);
    expect(csvLines[0]).toBe("name,kind,analysis,model,reference,expected_behavior,deck_lines");
    expect(csvLines[1]).toBe(
      'diode-forward-bias:op,D,op,Dfast,SPICE2/SPICE3-style local model-depth fixture,"DC probe out remains in [0.55, 0.65] V",8',
    );

    expect(JSON.parse(formatDeviceModelReferenceDeckAuditJson())).toStrictEqual(records);
  });

  it("formats stable device model reference deck audit summary records", () => {
    const summary = deviceModelReferenceDeckAuditSummary();
    expect(summary).toHaveLength(4);
    expect(summary[0]).toStrictEqual({
      kind: "D",
      fixtureCount: 5,
      analyses: ["op", "temperature", "ac", "noise", "tran"],
      missingAnalyses: [],
      deckLineCount: 42,
      references: ["SPICE2/SPICE3-style local model-depth fixture"],
    });

    expect(formatDeviceModelReferenceDeckAuditSummaryTable()).toBe(
      [
        "kind\tfixture_count\tanalyses\tmissing_analyses\tdeck_lines\treferences",
        "D\t5\top,temperature,ac,noise,tran\t\t42\tSPICE2/SPICE3-style local model-depth fixture",
        "NPN\t5\top,temperature,ac,noise,tran\t\t47\tSPICE2/SPICE3-style local model-depth fixture",
        "NJF\t5\top,temperature,ac,noise,tran\t\t52\tSPICE2/SPICE3-style local model-depth fixture",
        "NMOS\t5\top,temperature,ac,noise,tran\t\t47\tSPICE2/SPICE3-style local model-depth fixture",
      ].join("\n"),
    );

    const records = deviceModelReferenceDeckAuditSummaryRecords();
    expect(records[0]).toStrictEqual({
      kind: "D",
      fixture_count: "5",
      analyses: "op,temperature,ac,noise,tran",
      missing_analyses: "",
      deck_lines: "42",
      references: "SPICE2/SPICE3-style local model-depth fixture",
    });
    expect(formatDeviceModelReferenceDeckAuditSummaryCsv().split(/\r?\n/u)[1]).toBe(
      'D,5,"op,temperature,ac,noise,tran",,42,SPICE2/SPICE3-style local model-depth fixture',
    );
    expect(JSON.parse(formatDeviceModelReferenceDeckAuditSummaryJson())).toStrictEqual(records);
  });

  it("reports missing device model reference deck audit summary analyses", () => {
    const fixtures = deviceModelReferenceDeckAuditFixtures().filter(
      (fixture) => !(fixture.kind === "NMOS" && fixture.analysis === "tran"),
    );

    const summary = deviceModelReferenceDeckAuditSummary(fixtures);
    const nmos = summary.find((row) => row.kind === "NMOS");

    expect(nmos?.fixtureCount).toBe(4);
    expect(nmos?.analyses).toStrictEqual(["op", "temperature", "ac", "noise"]);
    expect(nmos?.missingAnalyses).toStrictEqual(["tran"]);
    expect(nmos?.deckLineCount).toBe(37);
    expect(formatDeviceModelReferenceDeckAuditSummaryTable(fixtures)).toContain(
      "NMOS\t4\top,temperature,ac,noise\ttran\t37\tSPICE2/SPICE3-style local model-depth fixture",
    );
  });

  it("exports stable device model reference deck audit analysis summaries", () => {
    const summary = deviceModelReferenceDeckAuditAnalysisSummary();
    expect(summary).toHaveLength(5);
    expect(summary[0]).toStrictEqual({
      analysis: "op",
      fixtureCount: 4,
      kinds: ["D", "NPN", "NJF", "NMOS"],
      missingKinds: [],
      deckLineCount: 36,
      references: ["SPICE2/SPICE3-style local model-depth fixture"],
    });

    expect(formatDeviceModelReferenceDeckAuditAnalysisSummaryTable()).toBe(
      [
        "analysis\tfixture_count\tkinds\tmissing_kinds\tdeck_lines\treferences",
        "op\t4\tD,NPN,NJF,NMOS\t\t36\tSPICE2/SPICE3-style local model-depth fixture",
        "temperature\t4\tD,NPN,NJF,NMOS\t\t40\tSPICE2/SPICE3-style local model-depth fixture",
        "ac\t4\tD,NPN,NJF,NMOS\t\t36\tSPICE2/SPICE3-style local model-depth fixture",
        "noise\t4\tD,NPN,NJF,NMOS\t\t36\tSPICE2/SPICE3-style local model-depth fixture",
        "tran\t4\tD,NPN,NJF,NMOS\t\t40\tSPICE2/SPICE3-style local model-depth fixture",
      ].join("\n"),
    );

    const records = deviceModelReferenceDeckAuditAnalysisSummaryRecords();
    expect(records[0]).toStrictEqual({
      analysis: "op",
      fixture_count: "4",
      kinds: "D,NPN,NJF,NMOS",
      missing_kinds: "",
      deck_lines: "36",
      references: "SPICE2/SPICE3-style local model-depth fixture",
    });
    expect(formatDeviceModelReferenceDeckAuditAnalysisSummaryCsv().split(/\r?\n/u)[1]).toBe(
      'op,4,"D,NPN,NJF,NMOS",,36,SPICE2/SPICE3-style local model-depth fixture',
    );
    expect(JSON.parse(formatDeviceModelReferenceDeckAuditAnalysisSummaryJson())).toStrictEqual(records);
  });

  it("reports missing device model reference deck audit analysis summary kinds", () => {
    const fixtures = deviceModelReferenceDeckAuditFixtures().filter(
      (fixture) => !(fixture.kind === "NMOS" && fixture.analysis === "tran"),
    );

    const summary = deviceModelReferenceDeckAuditAnalysisSummary(fixtures);
    const tran = summary.find((row) => row.analysis === "tran");

    expect(tran?.fixtureCount).toBe(3);
    expect(tran?.kinds).toStrictEqual(["D", "NPN", "NJF"]);
    expect(tran?.missingKinds).toStrictEqual(["NMOS"]);
    expect(tran?.deckLineCount).toBe(30);
    expect(formatDeviceModelReferenceDeckAuditAnalysisSummaryTable(fixtures)).toContain(
      "tran\t3\tD,NPN,NJF\tNMOS\t30\tSPICE2/SPICE3-style local model-depth fixture",
    );
  });

  it("exports stable device model reference deck audit matrix rows", () => {
    const matrix = deviceModelReferenceDeckAuditMatrix();
    expect(matrix).toHaveLength(4);
    expect(matrix[0]).toStrictEqual({
      kind: "D",
      fixtureCount: 5,
      op: "diode-forward-bias:op",
      temperature: "diode-forward-bias:temperature",
      ac: "diode-capacitance-ac:ac",
      noise: "diode-shot-noise:noise",
      tran: "diode-storage-charge:tran",
      missingAnalyses: [],
      extraAnalyses: [],
      deckLineCount: 42,
    });

    expect(formatDeviceModelReferenceDeckAuditMatrixTable()).toBe(
      [
        "kind\tfixture_count\top\ttemperature\tac\tnoise\ttran\tmissing_analyses\textra_analyses\tdeck_lines",
        "D\t5\tdiode-forward-bias:op\tdiode-forward-bias:temperature\tdiode-capacitance-ac:ac\tdiode-shot-noise:noise\tdiode-storage-charge:tran\t\t\t42",
        "NPN\t5\tbjt-emitter-follower:op\tbjt-emitter-follower:temperature\tbjt-capacitance-ac:ac\tbjt-shot-noise:noise\tbjt-storage-charge:tran\t\t\t47",
        "NJF\t5\tjfet-source-bias:op\tjfet-source-bias:temperature\tjfet-capacitance-ac:ac\tjfet-channel-noise:noise\tjfet-storage-charge:tran\t\t\t52",
        "NMOS\t5\tmos-level1-common-source:op\tmos-level1-common-source:temperature\tmos-level1-capacitance-ac:ac\tmos-level1-channel-noise:noise\tmos-level1-storage-charge:tran\t\t\t47",
      ].join("\n"),
    );

    const records = deviceModelReferenceDeckAuditMatrixRecords();
    expect(records[0]).toStrictEqual({
      kind: "D",
      fixture_count: "5",
      op: "diode-forward-bias:op",
      temperature: "diode-forward-bias:temperature",
      ac: "diode-capacitance-ac:ac",
      noise: "diode-shot-noise:noise",
      tran: "diode-storage-charge:tran",
      missing_analyses: "",
      extra_analyses: "",
      deck_lines: "42",
    });
    expect(formatDeviceModelReferenceDeckAuditMatrixCsv().split(/\r?\n/u)[1]).toBe(
      "D,5,diode-forward-bias:op,diode-forward-bias:temperature,diode-capacitance-ac:ac,diode-shot-noise:noise,diode-storage-charge:tran,,,42",
    );
    expect(JSON.parse(formatDeviceModelReferenceDeckAuditMatrixJson())).toStrictEqual(records);
  });

  it("reports missing device model reference deck audit matrix analyses", () => {
    const fixtures = deviceModelReferenceDeckAuditFixtures().filter(
      (fixture) => !(fixture.kind === "NMOS" && fixture.analysis === "tran"),
    );

    const matrix = deviceModelReferenceDeckAuditMatrix(fixtures);
    const nmos = matrix.find((row) => row.kind === "NMOS");

    expect(nmos?.fixtureCount).toBe(4);
    expect(nmos?.tran).toBe("");
    expect(nmos?.missingAnalyses).toStrictEqual(["tran"]);
    expect(nmos?.deckLineCount).toBe(37);
    expect(formatDeviceModelReferenceDeckAuditMatrixTable(fixtures)).toContain(
      "NMOS\t4\tmos-level1-common-source:op\tmos-level1-common-source:temperature\tmos-level1-capacitance-ac:ac\tmos-level1-channel-noise:noise\t\ttran\t\t37",
    );
  });

  it("formats a stable device model reference deck audit gate report", () => {
    const report = deviceModelReferenceDeckAuditGate();

    expect(report.passed).toBe(true);
    expect(report.fixtureCount).toBe(20);
    expect(report.expectedKinds).toStrictEqual(["D", "NPN", "NJF", "NMOS"]);
    expect(report.expectedAnalyses).toStrictEqual(["op", "temperature", "ac", "noise", "tran"]);
    expect(report.issues).toStrictEqual([]);
    expect(formatDeviceModelReferenceDeckAuditGateReport(report)).toBe(
      "passed\tfixture_count\texpected_kinds\texpected_analyses\tissue_count\ntrue\t20\tD,NPN,NJF,NMOS\top,temperature,ac,noise,tran\t0",
    );
    const digest = deviceModelReferenceDeckAuditGateCoverageDigest(report);
    expect(digest).toStrictEqual({
      passed: true,
      fixtureCount: 20,
      expectedPairCount: 20,
      coveredPairCount: 20,
      missingPairCount: 0,
      issueCount: 0,
      issueFields: [],
    });
    expect(formatDeviceModelReferenceDeckAuditGateCoverageDigestTable(report)).toBe(
      "passed\tfixture_count\texpected_pair_count\tcovered_pair_count\tmissing_pair_count\tissue_count\tissue_fields\ntrue\t20\t20\t20\t0\t0\t",
    );
  });

  it("reports missing reference deck audit gate coverage", () => {
    const fixtures = deviceModelReferenceDeckAuditFixtures().filter(
      (fixture) => !(fixture.kind === "NMOS" && fixture.analysis === "tran"),
    );

    const report = deviceModelReferenceDeckAuditGate(fixtures);
    const table = formatDeviceModelReferenceDeckAuditGateReport(report);

    expect(report.passed).toBe(false);
    expect(report.issues.some((issue) => issue.fixtureName === "NMOS:tran" && issue.field === "coverage")).toBe(true);
    expect(table).toContain("fixture_name\tfield\tmessage");
    expect(table).toContain("NMOS:tran\tcoverage\tmissing required NMOS tran reference-deck audit row");

    expect(formatDeviceModelReferenceDeckAuditGateIssueTable(report)).toBe(
      "fixture_name\tfield\tmessage\nNMOS:tran\tcoverage\tmissing required NMOS tran reference-deck audit row",
    );
    const records = deviceModelReferenceDeckAuditGateIssueRecords(report);
    expect(records).toStrictEqual([
      {
        fixture_name: "NMOS:tran",
        field: "coverage",
        message: "missing required NMOS tran reference-deck audit row",
      },
    ]);
    expect(formatDeviceModelReferenceDeckAuditGateIssueCsv(report)).toBe(
      "fixture_name,field,message\nNMOS:tran,coverage,missing required NMOS tran reference-deck audit row\n",
    );
    expect(JSON.parse(formatDeviceModelReferenceDeckAuditGateIssueJson(report))).toStrictEqual(records);

    const summary = deviceModelReferenceDeckAuditGateIssueSummary(report);
    expect(summary).toStrictEqual([
      {
        field: "coverage",
        issueCount: 1,
        fixtureNames: ["NMOS:tran"],
        messages: ["missing required NMOS tran reference-deck audit row"],
      },
    ]);
    expect(formatDeviceModelReferenceDeckAuditGateIssueSummaryTable(report)).toBe(
      "field\tissue_count\tfixture_names\tmessages\ncoverage\t1\tNMOS:tran\tmissing required NMOS tran reference-deck audit row",
    );
    const summaryRecords = deviceModelReferenceDeckAuditGateIssueSummaryRecords(report);
    expect(summaryRecords).toStrictEqual([
      {
        field: "coverage",
        issue_count: "1",
        fixture_names: "NMOS:tran",
        messages: "missing required NMOS tran reference-deck audit row",
      },
    ]);
    expect(formatDeviceModelReferenceDeckAuditGateIssueSummaryCsv(report)).toBe(
      "field,issue_count,fixture_names,messages\ncoverage,1,NMOS:tran,missing required NMOS tran reference-deck audit row\n",
    );
    expect(JSON.parse(formatDeviceModelReferenceDeckAuditGateIssueSummaryJson(report))).toStrictEqual(summaryRecords);
    const digest = deviceModelReferenceDeckAuditGateCoverageDigest(report);
    expect(digest).toStrictEqual({
      passed: false,
      fixtureCount: 19,
      expectedPairCount: 20,
      coveredPairCount: 19,
      missingPairCount: 1,
      issueCount: 1,
      issueFields: ["coverage"],
    });
    const digestRecords = deviceModelReferenceDeckAuditGateCoverageDigestRecords(report);
    expect(digestRecords).toStrictEqual([
      {
        passed: "false",
        fixture_count: "19",
        expected_pair_count: "20",
        covered_pair_count: "19",
        missing_pair_count: "1",
        issue_count: "1",
        issue_fields: "coverage",
      },
    ]);
    expect(formatDeviceModelReferenceDeckAuditGateCoverageDigestTable(report)).toBe(
      "passed\tfixture_count\texpected_pair_count\tcovered_pair_count\tmissing_pair_count\tissue_count\tissue_fields\nfalse\t19\t20\t19\t1\t1\tcoverage",
    );
    expect(formatDeviceModelReferenceDeckAuditGateCoverageDigestCsv(report)).toBe(
      "passed,fixture_count,expected_pair_count,covered_pair_count,missing_pair_count,issue_count,issue_fields\nfalse,19,20,19,1,1,coverage\n",
    );
    expect(JSON.parse(formatDeviceModelReferenceDeckAuditGateCoverageDigestJson(report))).toStrictEqual(digestRecords);
  });

  it("rejects non-Level-1 MOS model cards explicitly", () => {
    expect(() => normalizeModelCard("Mbad", "nmos", { LEVEL: 2.0 })).toThrowError(
      "only MOS LEVEL=1",
    );
  });

  it("stamps a custom-model evaluator hook as a DC current", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "in", "0", 1.0));
    circuit.add({
      kind: "custom-model",
      name: "XG",
      positive: "in",
      negative: "0",
      modelName: "hook",
      parameters: { g: 2.0e-3 },
      currentOffsetAmps: 0.0,
      evaluator: (context) => ({
        currentAmps: context.parameters.g * context.voltage,
        conductanceSiemens: context.parameters.g,
      }),
    });

    const result = dcOp(circuit);

    expectClose(result.voltage("in"), 1.0);
    expectClose(result.branchCurrent("I(V1)"), -2.0e-3);
  });

  it("stamps the custom-model linear conductance fast path as a DC current", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "in", "0", 1.0));
    circuit.add(customLinearConductanceModel("XG", "in", "0", 2.0e-3));

    const result = dcOp(circuit);

    expectClose(result.branchCurrent("I(V1)"), -2.0e-3);
  });

  it("accepts the custom-model source subset and rejects dynamic constructs", () => {
    const accepted = analyzeCustomModelSource(
      "module rlim(p, n); analog begin I(p,n) <+ g * V(p,n); end endmodule",
    );
    const rejected = analyzeCustomModelSource(
      "module cap(p, n); analog begin I(p,n) <+ ddt(C * V(p,n)); end endmodule",
    );

    expect(accepted.accepted).toBe(true);
    expect(accepted.moduleName).toBe("rlim");
    expect(accepted.terminals).toStrictEqual(["p", "n"]);
    expect(accepted.contribution).toStrictEqual(["p", "n"]);
    expect(rejected.accepted).toBe(false);
    expect(rejected.diagnostics.map((diagnostic) => diagnostic.code)).toContain(
      "CUSTOM_MODEL_FORBIDDEN_CONSTRUCT",
    );
  });

  it("solves a resistor divider midpoint voltage", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "vin", "0", 10.0));
    circuit.add(resistor("R1", "vin", "mid", 1_000.0));
    circuit.add(resistor("R2", "mid", "0", 1_000.0));

    const result = dcOp(circuit);

    expectClose(result.voltage("vin"), 10.0);
    expectClose(result.voltage("mid"), 5.0);
    expectClose(result.voltage("0"), 0.0);
    expect(result.converged).toBe(true);
    expect(result.convergenceAid).toBe("newton");
    expect(result.iterations).toBe(1);
  });

  it("seeds a DC operating point vector from parsed initial conditions", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "vin", "0", 10.0));
    circuit.add(resistor("R1", "vin", "mid", 1_000.0));
    circuit.add(resistor("R2", "mid", "0", 1_000.0));
    const summary = resolveDeckInitialConditions(`
.nodeset V(vin)=10 V(mid)=1
.ic V(mid)=4
.end
`);

    const vector = dcInitialVectorFromConditions(
      circuit,
      summary.initialConditions,
      summary.nodesets,
    );
    expect(vector).toStrictEqual([4.0, 10.0, 0.0]);

    const result = dcOpWithInitialConditions(circuit, summary, { convergenceAids: false });

    expect(result.converged).toBe(true);
    expectClose(result.voltage("vin"), 10.0);
    expectClose(result.voltage("mid"), 5.0);
  });

  it("solves a large resistor ladder through the sparse real solver path", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "n0", "0", 10.0));
    for (let index = 0; index < 34; index++) {
      circuit.add(resistor(`R${index}`, `n${index}`, `n${index + 1}`, 1_000.0));
    }
    circuit.add(resistor("R34", "n34", "0", 1_000.0));

    const result = dcOp(circuit);

    expect(result.converged).toBe(true);
    expectClose(result.voltage("n34"), 10.0 / 35.0);
    expect(result.diagnostics.matrixSize).toBe(36);
    expect(result.diagnostics.solver).toBe("sparse_real");
    expect(result.diagnostics.convergenceAid).toBe("newton");
    expectClose(result.diagnostics.tolerance, 1.0e-9);
    expect(Number.isFinite(result.diagnostics.maxDelta)).toBe(true);
    expect(result.diagnostics.newtonStepLimit).toBeUndefined();
    expect(result.diagnostics.limitedNewtonSteps).toBe(0);
    expectClose(result.diagnostics.minimumDampingFactor, 1.0);
    expect(result.diagnostics.solverProfile.matrixSize).toBe(36);
    expect(result.diagnostics.solverProfile.solver).toBe("sparse_real");
    expect(result.diagnostics.solverProfile.backend).toBe("native_sparse_gaussian");
    expect(result.diagnostics.solverProfile.structuralNonzeros).toBeGreaterThan(0);
    expect(result.diagnostics.solverProfile.density).toBeGreaterThan(0);
    expect(result.diagnostics.solverProfile.density).toBeLessThan(0.1);
    expect(result.diagnostics.solverProfile.fillInNonzeros).toBeLessThanOrEqual(36 * 36);
    expect(result.diagnostics.solverProfile.fallbackReason).toBeUndefined();
  });

  it("expands subcircuit instances into namespaced primitive elements", () => {
    const circuit = new Circuit();
    circuit.defineSubcircuit(
      subcircuitDefinition("atten2", ["in", "out"], [
        resistor("Rtop", "in", "out", 1_000.0),
        resistor("Rbot", "out", "0", 1_000.0),
      ]),
    );
    circuit.add(voltageSource("V1", "vin", "0", 10.0));
    circuit.add(xInstance("X1", ["vin", "vout"], "atten2"));

    const result = dcOp(circuit);

    expectClose(result.voltage("vout"), 5.0);
    expect(
      circuit
        .elements()
        .filter((element) => element.kind === "resistor")
        .map((element) => element.name),
    ).toEqual(["X1.Rtop", "X1.Rbot"]);
  });

  it("preserves the complete diode model through subcircuit expansion", () => {
    const circuit = new Circuit();
    circuit.defineSubcircuit(
      subcircuitDefinition("diode-cell", ["in"], [
        diode("Dcell", "in", "0", 2.0e-14, 0.026, 1.2, 6.0, 2.0e-6, 1.5e-12, 4.0e-9, 0.8, 0.4, 0.35, 2.2, 1.05, 10.0, 1.0e-12, 1.3),
      ]),
    );
    circuit.add(xInstance("X1", ["a"], "diode-cell"));

    const expanded = circuit.elements().find((element) => element.kind === "diode");
    expect(expanded?.kind).toBe("diode");
    if (expanded?.kind !== "diode") {
      throw new Error("expected expanded diode");
    }
    expectClose(expanded.junctionPotential, 0.8);
    expectClose(expanded.gradingCoefficient, 0.4);
    expectClose(expanded.forwardBiasDepletionCoefficient, 0.35);
    expectClose(expanded.saturationCurrentTemperatureExponent, 2.2);
    expectClose(expanded.energyGapElectronVolts, 1.05);
    expectClose(expanded.seriesResistance, 10.0);
    expectClose(expanded.flickerNoiseCoefficient, 1.0e-12);
    expectClose(expanded.flickerNoiseExponent, 1.3);
  });

  it("preserves the complete JFET model through subcircuit expansion", () => {
    const circuit = new Circuit();
    circuit.defineSubcircuit(
      subcircuitDefinition("jfet-cell", ["d", "g", "s"], [
        {
          ...jfet("Jcell", "d", "g", "s"),
          flickerNoiseCoefficient: 1.0e-12,
          flickerNoiseExponent: 1.3,
          junctionPotential: 0.8,
        },
      ]),
    );
    circuit.add(xInstance("X1", ["d1", "g1", "0"], "jfet-cell"));

    const expanded = circuit.elements().find((element) => element.kind === "jfet");
    expect(expanded?.kind).toBe("jfet");
    if (expanded?.kind !== "jfet") {
      throw new Error("expected expanded JFET");
    }
    expectClose(expanded.flickerNoiseCoefficient, 1.0e-12);
    expectClose(expanded.flickerNoiseExponent, 1.3);
    expectClose(expanded.junctionPotential, 0.8);
  });

  it("preserves the complete BJT model through subcircuit expansion", () => {
    const circuit = new Circuit();
    circuit.defineSubcircuit(
      subcircuitDefinition("bjt-cell", ["c", "b", "e"], [
        bjt("Qcell", "c", "b", "e", "NPN", 1e-14, 100, 0.02585, 0, 0, 0, 0, 2.4, 1.05, 80.0, 1.2, 1.3, 0.8, 0.4, 0.7, 0.45, 0.4, 120.0, 2.0e-3, 3.0e-13, 1.7, 4.0e-13, 1.8, 1.5, 0.25, 3.0e-3, 323.15, 1.0e-12, 1.3, 30.0, 2.0, 4.0e-3, 0.6, 12.0, 13.0, 14.0, 2.0, 5.0e-6, 0.4),
      ]),
    );
    circuit.add(xInstance("X1", ["c1", "b1", "0"], "bjt-cell"));

    const expanded = circuit.elements().find((element) => element.kind === "bjt");
    expect(expanded?.kind).toBe("bjt");
    if (expanded?.kind === "bjt") {
      expectClose(expanded.saturationCurrentTemperatureExponent, 2.4);
      expectClose(expanded.energyGapElectronVolts, 1.05);
      expectClose(expanded.forwardEarlyVoltage, 80.0);
      expectClose(expanded.reverseEarlyVoltage, 120.0);
      expectClose(expanded.forwardEmissionCoefficient, 1.2);
      expectClose(expanded.reverseEmissionCoefficient, 1.3);
      expectClose(expanded.baseEmitterJunctionPotential, 0.8);
      expectClose(expanded.baseEmitterGradingCoefficient, 0.4);
      expectClose(expanded.baseCollectorJunctionPotential, 0.7);
      expectClose(expanded.baseCollectorGradingCoefficient, 0.45);
      expectClose(expanded.forwardBiasDepletionCoefficient, 0.4);
      expectClose(expanded.forwardBetaRolloffCurrent, 2.0e-3);
      expectClose(expanded.baseEmitterLeakageSaturationCurrent, 3.0e-13);
      expectClose(expanded.baseEmitterLeakageEmissionCoefficient, 1.7);
      expectClose(expanded.baseCollectorLeakageSaturationCurrent, 4.0e-13);
      expectClose(expanded.baseCollectorLeakageEmissionCoefficient, 1.8);
      expectClose(expanded.forwardBetaTemperatureExponent, 1.5);
      expectClose(expanded.reverseBeta, 0.25);
      expectClose(expanded.reverseBetaRolloffCurrent, 3.0e-3);
      expectClose(expanded.nominalTemperatureKelvin, 323.15);
      expectClose(expanded.flickerNoiseCoefficient, 1.0e-12);
      expectClose(expanded.flickerNoiseExponent, 1.3);
      expectClose(expanded.forwardExcessPhaseDegrees, 30.0);
      expectClose(expanded.forwardTransitTimeBiasCoefficient, 2.0);
      expectClose(expanded.forwardTransitTimeCurrent, 4.0e-3);
      expectClose(expanded.forwardTransitTimeVoltage, 0.6);
      expectClose(expanded.emitterResistance, 12.0);
      expectClose(expanded.collectorResistance, 13.0);
      expectClose(expanded.baseResistance, 14.0);
      expectClose(expanded.minimumBaseResistance, 2.0);
      expectClose(expanded.baseResistanceHalfCurrent, 5.0e-6);
      expectClose(expanded.baseCollectorCapacitanceFraction, 0.4);
    }
  });

  it("uses positive-to-negative orientation for current sources", () => {
    const circuit = new Circuit();
    circuit.add(currentSource("I1", "0", "n1", 1.0e-3));
    circuit.add(resistor("R1", "n1", "0", 1_000.0));

    const result = dcOp(circuit);

    expectClose(result.voltage("n1"), 1.0);
  });

  it("stamps behavioral current sources from node-voltage expressions", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 2.0));
    circuit.add(bSourceCurrent("B1", "0", "out", "0.002 * V(in)"));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    const result = dcOp(circuit);

    expect(result.converged).toBe(true);
    expectClose(result.voltage("out"), 4.0);
  });

  it("stamps behavioral voltage sources from differential expressions", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 3.0));
    circuit.add(bSourceVoltage("B1", "out", "0", "2.0 * V(in, 0) + 1.0"));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    const result = dcOp(circuit);

    expect(result.converged).toBe(true);
    expectClose(result.voltage("out"), 7.0);
    expectClose(result.branchCurrent("B1"), -7.0e-3);
  });

  it("reports voltage source branch current", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "n1", "0", 10.0));
    circuit.add(resistor("R1", "n1", "0", 1_000.0));

    const result = dcOp(circuit);

    expectClose(result.branchCurrent("V1"), -10.0e-3);
    expectClose(result.branchCurrent("I(V1)"), -10.0e-3);
  });

  it("recognizes ground aliases as the zero-volt reference", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "n1", "gnd", 3.3));
    circuit.add(resistor("R1", "n1", "GND", 330.0));

    const result = dcOp(circuit);

    expectClose(result.voltage("n1"), 3.3);
    expectClose(result.voltage("gnd"), 0.0);
    expectClose(result.voltage("GND"), 0.0);
  });

  it("treats inductors as ideal shorts in DC", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "vin", "0", 1.0));
    circuit.add(resistor("R1", "vin", "out", 1_000.0));
    circuit.add(inductor("L1", "out", "0", 1.0));

    const result = dcOp(circuit);

    expectClose(result.voltage("out"), 0.0);
    expectClose(result.branchCurrent("L1"), 1.0e-3);
  });

  it("stamps VCCS current from control voltage", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vctrl", "ctrl", "0", 1.0));
    circuit.add(vccs("G1", "0", "out", "ctrl", "0", 1.0e-3));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    const result = dcOp(circuit);

    expectClose(result.voltage("out"), 1.0);
  });

  it("stamps VCVS output voltage from control voltage", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vctrl", "ctrl", "0", 1.5));
    circuit.add(vcvs("E1", "out", "0", "ctrl", "0", 2.0));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    const result = dcOp(circuit);

    expectClose(result.voltage("out"), 3.0);
    expectClose(result.branchCurrent("E1"), -3.0e-3);
  });

  it("stamps VCVS differential control polarity", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vp", "p", "0", 4.0));
    circuit.add(voltageSource("Vn", "n", "0", 1.0));
    circuit.add(vcvs("E1", "out", "0", "p", "n", 0.5));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    const result = dcOp(circuit);

    expectClose(result.voltage("out"), 1.5);
  });

  it("stamps CCCS current from a voltage-source branch current", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 1.0));
    circuit.add(resistor("Rsense", "in", "sense", 1_000.0));
    circuit.add(voltageSource("Vsense", "sense", "0", 0.0));
    circuit.add(cccs("F1", "0", "out", "Vsense", 2.0));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    const result = dcOp(circuit);

    expectClose(result.branchCurrent("Vsense"), 1.0e-3);
    expectClose(result.voltage("out"), 2.0);
  });

  it("stamps CCVS voltage from a voltage-source branch current", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 1.0));
    circuit.add(resistor("Rsense", "in", "sense", 1_000.0));
    circuit.add(voltageSource("Vsense", "sense", "0", 0.0));
    circuit.add(ccvs("H1", "out", "0", "Vsense", 2_000.0));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    const result = dcOp(circuit);

    expectClose(result.branchCurrent("Vsense"), 1.0e-3);
    expectClose(result.voltage("out"), 2.0);
    expectClose(result.branchCurrent("H1"), -2.0e-3);
  });

  it("solves a forward-biased diode operating point", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 0.7));
    circuit.add(diode("D1", "in", "out", 1.0e-12, 0.025));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    const result = dcOp(circuit);

    expect(result.voltage("out")).toBeGreaterThan(0.1);
    expect(result.voltage("out")).toBeLessThan(0.7);
  });

  it("uses diode emission coefficient in fixed-bias current", () => {
    const base = new Circuit();
    base.add(voltageSource("V1", "a", "0", 0.7));
    base.add(diode("D1", "a", "0", 1.0e-15, 0.02585));

    const highN = new Circuit();
    highN.add(voltageSource("V1", "a", "0", 0.7));
    highN.add(diode("D1", "a", "0", 1.0e-15, 0.02585, 2.0));

    const baseResult = dcOp(base);
    const highNResult = dcOp(highN);

    expect(highNResult.branchCurrent("V1")).toBeDefined();
    expect(baseResult.branchCurrent("V1")).toBeDefined();
    expect(Math.abs(highNResult.branchCurrent("V1")!)).toBeLessThan(
      Math.abs(baseResult.branchCurrent("V1")!) * 1.0e-3,
    );
  });

  it("limits fixed-bias diode current with series resistance", () => {
    const ideal = new Circuit();
    ideal.add(voltageSource("V1", "a", "0", 0.7));
    ideal.add(diode("D1", "a", "0"));

    const limited = new Circuit();
    limited.add(voltageSource("V1", "a", "0", 0.7));
    limited.add(
      diode(
        "D1",
        "a",
        "0",
        1.0e-15,
        0.02585,
        1.0,
        undefined,
        1.0e-3,
        0.0,
        0.0,
        1.0,
        0.5,
        0.5,
        3.0,
        1.11,
        100.0,
      ),
    );

    const idealCurrent = Math.abs(dcOp(ideal).branchCurrent("V1")!);
    const limitedCurrent = Math.abs(dcOp(limited).branchCurrent("V1")!);
    expect(limitedCurrent).toBeLessThan(idealCurrent);
    expect(limitedCurrent).toBeLessThanOrEqual(0.7 / 100.0);
  });

  it("uses diode breakdown voltage in reverse-bias current", () => {
    const leakage = new Circuit();
    leakage.add(voltageSource("V1", "0", "a", 5.0));
    leakage.add(diode("D1", "a", "0", 1.0e-15, 0.02585));

    const breakdown = new Circuit();
    breakdown.add(voltageSource("V1", "0", "a", 5.0));
    breakdown.add(diode("D1", "a", "0", 1.0e-15, 0.02585, 1.0, 5.0, 1.0e-6));

    const leakageResult = dcOp(leakage);
    const breakdownResult = dcOp(breakdown);

    expect(leakageResult.branchCurrent("V1")).toBeDefined();
    expect(breakdownResult.branchCurrent("V1")).toBeDefined();
    expect(Math.abs(breakdownResult.branchCurrent("V1")!)).toBeGreaterThan(
      Math.abs(leakageResult.branchCurrent("V1")!) * 1.0e6,
    );
    expect(Math.abs(breakdownResult.branchCurrent("V1")!)).toBeCloseTo(1.0e-6, 9);
  });

  it("uses diode temperature scaling in fixed-current forward voltage", () => {
    const nominal = new Circuit();
    nominal.add(voltageSource("V1", "vcc", "0", 5.0));
    nominal.add(resistor("Rbias", "vcc", "a", 4_300.0));
    nominal.add(diode("D1", "a", "0", 1.0e-15, 0.02585));

    const cold = circuitAtTemperature(nominal, 275.0);
    const hot = circuitAtTemperature(nominal, 350.0);

    const nominalResult = dcOp(nominal);
    const coldResult = dcOp(cold);
    const hotResult = dcOp(hot);

    expect(coldResult.voltage("a")).toBeGreaterThan(nominalResult.voltage("a")!);
    expect(hotResult.voltage("a")).toBeLessThan(nominalResult.voltage("a")!);
  });

  it("uses the diode model saturation-current temperature exponent", () => {
    const temperatureKelvin = 350.0;
    const nominalTemperatureKelvin = 300.15;
    const defaultHot = diodeAtTemperature(
      diode("D1", "a", "0", 1.0e-15, 0.02585, 1.0, undefined, 1.0e-3, 0.0, 0.0, 1.0, 0.5, 0.5, 3.0),
      temperatureKelvin,
      nominalTemperatureKelvin,
    );
    const flatHot = diodeAtTemperature(
      diode("D1", "a", "0", 1.0e-15, 0.02585, 1.0, undefined, 1.0e-3, 0.0, 0.0, 1.0, 0.5, 0.5, 0.0),
      temperatureKelvin,
      nominalTemperatureKelvin,
    );

    expect(defaultHot.saturationCurrent / flatHot.saturationCurrent).toBeCloseTo(
      (temperatureKelvin / nominalTemperatureKelvin) ** 3,
      12,
    );
  });

  it("uses the diode model energy gap for circuit temperature scaling", () => {
    const silicon = new Circuit();
    silicon.add(diode("D1", "a", "0", 1.0e-15, 0.02585, 1.0, undefined, 1.0e-3, 0.0, 0.0, 1.0, 0.5, 0.5, 3.0, 1.11));
    const lowerGap = new Circuit();
    lowerGap.add(diode("D1", "a", "0", 1.0e-15, 0.02585, 1.0, undefined, 1.0e-3, 0.0, 0.0, 1.0, 0.5, 0.5, 3.0, 0.8));

    const siliconHot = circuitAtTemperature(silicon, 350.0).elements()[0];
    const lowerGapHot = circuitAtTemperature(lowerGap, 350.0).elements()[0];

    expect(siliconHot.kind).toBe("diode");
    expect(lowerGapHot.kind).toBe("diode");
    if (siliconHot.kind !== "diode" || lowerGapHot.kind !== "diode") {
      throw new Error("expected diode temperature fixtures");
    }
    expect(siliconHot.saturationCurrent).toBeGreaterThan(lowerGapHot.saturationCurrent);
  });

  it("runs DC temperature sweeps and formats stable table output", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "vcc", "0", 5.0));
    circuit.add(resistor("Rbias", "vcc", "a", 4_300.0));
    circuit.add(diode("D1", "a", "0", 1.0e-15, 0.02585));

    const result = dcTemperatureSweep(circuit, [275.0, 300.15, 350.0]);

    expect(result.points[0].result.voltage("a")).toBeGreaterThan(
      result.points[1].result.voltage("a")!,
    );
    expect(result.points[2].result.voltage("a")).toBeLessThan(
      result.points[1].result.voltage("a")!,
    );
    expect(formatTemperatureDcTable(result, ["V(a)", "I(V1)"])).toBe(
      "Index\tTemperatureKelvin\tV(a)\tI(V1)\n" +
      "0\t2.750000e+02\t4.560039e+00\t-1.023164e-04\n" +
      "1\t3.001500e+02\t3.613836e+00\t-3.223638e-04\n" +
      "2\t3.500000e+02\t6.351989e-01\t-1.015070e-03\n",
    );
  });

  it("runs named-corner DC temperature sweeps and formats stable table output", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "vcc", "0", 5.0));
    circuit.add(resistor("Rbias", "vcc", "a", 4_300.0));
    circuit.add(diode("D1", "a", "0", 1.0e-15, 0.02585));

    const result = dcTemperatureSweepCorners(
      circuit,
      [275.0, 350.0],
      [
        { name: "nominal", overrides: [] },
        {
          name: "rbias-high",
          overrides: [{ elementName: "Rbias", parameter: "resistance", value: 8_600.0 }],
        },
      ],
    );

    expect(result.points.map((point) => point.cornerName)).toStrictEqual([
      "nominal",
      "rbias-high",
    ]);
    expect(result.points[0].points[0].result.voltage("a")).toBeGreaterThan(
      result.points[0].points[1].result.voltage("a")!,
    );
    expect(formatCornerTemperatureDcTable(result, ["V(a)", "I(V1)"])).toBe(
      "Corner\tIndex\tTemperatureKelvin\tV(a)\tI(V1)\n" +
      "nominal\t0\t2.750000e+02\t4.560039e+00\t-1.023164e-04\n" +
      "nominal\t1\t3.500000e+02\t6.351989e-01\t-1.015070e-03\n" +
      "rbias-high\t0\t2.750000e+02\t4.218594e+00\t-9.086118e-05\n" +
      "rbias-high\t1\t3.500000e+02\t6.144482e-01\t-5.099479e-04\n",
    );
  });

  it("uses BJT temperature scaling in fixed-base emitter voltage", () => {
    const nominal = new Circuit();
    nominal.add(voltageSource("Vcc", "vcc", "0", 5.0));
    nominal.add(voltageSource("Vbase", "base", "0", 0.72));
    nominal.add(bjt("Q1", "vcc", "base", "out", "NPN", 1.0e-14, 120.0, 0.02585));
    nominal.add(resistor("Rload", "out", "0", 1_000.0));

    const cold = circuitAtTemperature(nominal, 275.0);
    const hot = circuitAtTemperature(nominal, 350.0);

    const nominalResult = dcOp(nominal);
    const coldResult = dcOp(cold);
    const hotResult = dcOp(hot);

    expect(coldResult.voltage("out")).toBeLessThan(nominalResult.voltage("out")!);
    expect(hotResult.voltage("out")).toBeGreaterThan(nominalResult.voltage("out")!);
  });

  it("uses the BJT model temperature exponent", () => {
    const low = bjtAtTemperature(bjt("Qlow", "c", "b", "e", "NPN", 1e-14, 100, 0.02585, 0, 0, 0, 0, 0), 350);
    const high = bjtAtTemperature(bjt("Qhigh", "c", "b", "e", "NPN", 1e-14, 100, 0.02585, 0, 0, 0, 0, 4), 350);
    expect(high.saturationCurrent).toBeGreaterThan(low.saturationCurrent);
  });

  it("uses the BJT beta temperature exponent", () => {
    const transistor = {
      ...bjt("Q1", "c", "b", "e"),
      reverseBeta: 2.0,
      forwardBetaTemperatureExponent: 2.0,
    };
    const hot = bjtAtTemperature(transistor, 350);
    expect(hot.forwardBeta).toBeGreaterThan(transistor.forwardBeta);
    expect(hot.reverseBeta).toBeGreaterThan(transistor.reverseBeta);
  });

  it("uses the BJT model nominal temperature", () => {
    const transistor = {
      ...bjt("Q1", "c", "b", "e"),
      nominalTemperatureKelvin: 325.0,
    };
    const atModelNominal = bjtAtTemperature(transistor, 325.0);
    expectClose(atModelNominal.saturationCurrent, transistor.saturationCurrent);
    expectClose(atModelNominal.thermalVoltage, transistor.thermalVoltage);
  });

  it("rejects invalid BJT nominal temperatures", () => {
    const circuit = new Circuit();
    circuit.add({ ...bjt("Qbad", "c", "b", "0"), nominalTemperatureKelvin: 0.0 });
    expect(() => dcOp(circuit)).toThrowError("nominal temperature must be finite and positive");
  });

  it("rejects invalid diode flicker noise exponents", () => {
    const circuit = new Circuit();
    circuit.add({ ...diode("Dbad", "a", "0"), flickerNoiseExponent: -1.0 });
    expect(() => dcOp(circuit)).toThrowError(
      "flicker-noise exponent must be finite and non-negative",
    );
  });

  it("rejects invalid BJT flicker noise coefficients", () => {
    const circuit = new Circuit();
    circuit.add({ ...bjt("Qbad", "c", "b", "0"), flickerNoiseCoefficient: -1.0 });
    expect(() => dcOp(circuit)).toThrowError(
      "flicker noise coefficient must be finite and non-negative",
    );
  });

  it("rejects invalid BJT flicker noise exponents", () => {
    const circuit = new Circuit();
    circuit.add({ ...bjt("Qbad", "c", "b", "0"), flickerNoiseExponent: -1.0 });
    expect(() => dcOp(circuit)).toThrowError(
      "flicker noise exponent must be finite and non-negative",
    );
  });

  it("rejects invalid BJT forward excess phase", () => {
    const circuit = new Circuit();
    circuit.add({ ...bjt("Qbad", "c", "b", "0"), forwardExcessPhaseDegrees: -1.0 });
    expect(() => dcOp(circuit)).toThrowError(
      "forward excess phase must be finite and non-negative",
    );
  });

  it("rejects invalid BJT forward transit-time bias coefficients", () => {
    const circuit = new Circuit();
    circuit.add({
      ...bjt("Qbad", "c", "b", "0"),
      forwardTransitTimeBiasCoefficient: -1.0,
    });
    expect(() => dcOp(circuit)).toThrowError(
      "forward transit-time bias coefficient must be finite and non-negative",
    );
  });

  it("rejects invalid BJT forward transit-time currents", () => {
    const circuit = new Circuit();
    circuit.add({
      ...bjt("Qbad", "c", "b", "0"),
      forwardTransitTimeCurrent: -1.0,
    });
    expect(() => dcOp(circuit)).toThrowError(
      "forward transit-time current must be finite and non-negative",
    );
  });

  it("rejects invalid BJT forward transit-time voltages", () => {
    const circuit = new Circuit();
    circuit.add({
      ...bjt("Qbad", "c", "b", "0"),
      forwardTransitTimeVoltage: -1.0,
    });
    expect(() => dcOp(circuit)).toThrowError(
      "forward transit-time voltage must be finite and non-negative",
    );
  });

  it("rejects invalid BJT emitter resistances", () => {
    const circuit = new Circuit();
    circuit.add({
      ...bjt("Qbad", "c", "b", "0"),
      emitterResistance: -1.0,
    });
    expect(() => dcOp(circuit)).toThrowError(
      "emitter resistance must be finite and non-negative",
    );
  });

  it("rejects invalid BJT collector resistances", () => {
    const circuit = new Circuit();
    circuit.add({
      ...bjt("Qbad", "c", "b", "0"),
      collectorResistance: -1.0,
    });
    expect(() => dcOp(circuit)).toThrowError(
      "collector resistance must be finite and non-negative",
    );
  });

  it("rejects invalid BJT base resistances", () => {
    const circuit = new Circuit();
    circuit.add({
      ...bjt("Qbad", "c", "b", "0"),
      baseResistance: -1.0,
    });
    expect(() => dcOp(circuit)).toThrow(
      "base resistance must be finite and non-negative",
    );
  });

  it("rejects invalid BJT base-collector capacitance fractions", () => {
    const circuit = new Circuit();
    circuit.add({
      ...bjt("Qbad", "c", "b", "0"),
      baseCollectorCapacitanceFraction: 1.1,
    });
    expect(() => dcOp(circuit)).toThrow(
      "base-collector capacitance fraction must be between zero and one",
    );
  });

  it("rejects non-finite BJT beta temperature exponents", () => {
    const circuit = new Circuit();
    circuit.add({
      ...bjt("Qbad", "c", "b", "0"),
      forwardBetaTemperatureExponent: Number.NaN,
    });
    expect(() => dcOp(circuit)).toThrowError(
      "beta temperature exponent must be finite",
    );
  });

  it("scales BJT base-emitter leakage saturation current with temperature", () => {
    const transistor = {
      ...bjt("Q1", "c", "b", "e"),
      baseEmitterLeakageSaturationCurrent: 2.0e-13,
    };
    const hot = bjtAtTemperature(transistor, 350);
    expect(hot.baseEmitterLeakageSaturationCurrent).toBeGreaterThan(
      transistor.baseEmitterLeakageSaturationCurrent,
    );
  });

  it("scales BJT base-collector leakage saturation current with temperature", () => {
    const transistor = {
      ...bjt("Q1", "c", "b", "e"),
      baseCollectorLeakageSaturationCurrent: 2.0e-13,
    };
    const hot = bjtAtTemperature(transistor, 350);
    expect(hot.baseCollectorLeakageSaturationCurrent).toBeGreaterThan(
      transistor.baseCollectorLeakageSaturationCurrent,
    );
  });

  it("uses the BJT model energy gap", () => {
    const silicon = new Circuit();
    silicon.add(bjt("Qsilicon", "c", "b", "e", "NPN", 1e-14, 100, 0.02585, 0, 0, 0, 0, 3, 1.11));
    const lowerGap = new Circuit();
    lowerGap.add(bjt("Qlower", "c", "b", "e", "NPN", 1e-14, 100, 0.02585, 0, 0, 0, 0, 3, 0.8));
    const siliconHot = circuitAtTemperature(silicon, 350);
    const lowerGapHot = circuitAtTemperature(lowerGap, 350);
    const siliconBjt = siliconHot.elements()[0];
    const lowerGapBjt = lowerGapHot.elements()[0];
    if (siliconBjt?.kind !== "bjt" || lowerGapBjt?.kind !== "bjt") {
      throw new Error("expected temperature-adjusted BJTs");
    }
    expect(siliconBjt.saturationCurrent).toBeGreaterThan(lowerGapBjt.saturationCurrent);
  });

  it("rejects an invalid BJT energy gap", () => {
    const circuit = new Circuit();
    circuit.add(bjt("Qbad", "c", "b", "0", "NPN", 1e-14, 100, 0.02585, 0, 0, 0, 0, 3, 0));
    expect(() => dcOp(circuit)).toThrowError("energy gap must be finite and positive");
  });

  it("uses BJT forward Early voltage to modulate collector current", () => {
    const collectorVoltage = (forwardEarlyVoltage: number): number => {
      const circuit = new Circuit();
      circuit.add(voltageSource("Vcc", "vcc", "0", 5.0));
      circuit.add(voltageSource("Vbase", "base", "0", 0.65));
      circuit.add(resistor("Rload", "vcc", "out", 1_000.0));
      circuit.add(bjt("Q1", "out", "base", "0", "NPN", 1e-14, 100, 0.02585, 0, 0, 0, 0, 3, 1.11, forwardEarlyVoltage));
      return dcOp(circuit).voltage("out");
    };

    expect(collectorVoltage(20.0)).toBeLessThan(collectorVoltage(0.0));
  });

  it("rejects an invalid BJT forward Early voltage", () => {
    const circuit = new Circuit();
    circuit.add(bjt("Qbad", "c", "b", "0", "NPN", 1e-14, 100, 0.02585, 0, 0, 0, 0, 3, 1.11, -1.0));
    expect(() => dcOp(circuit)).toThrowError("forward Early voltage must be finite and non-negative");
  });

  it("uses BJT reverse Early voltage to modulate collector current", () => {
    const collectorVoltage = (reverseEarlyVoltage: number): number => {
      const circuit = new Circuit();
      circuit.add(voltageSource("Vcc", "vcc", "0", 5.0));
      circuit.add(voltageSource("Vbase", "base", "0", 0.65));
      circuit.add(resistor("Rload", "vcc", "out", 1_000.0));
      circuit.add({ ...bjt("Q1", "out", "base", "0"), reverseEarlyVoltage });
      return dcOp(circuit).voltage("out");
    };

    expect(collectorVoltage(20.0)).toBeGreaterThan(collectorVoltage(0.0));
  });

  it("rejects an invalid BJT reverse Early voltage", () => {
    const circuit = new Circuit();
    circuit.add({ ...bjt("Qbad", "c", "b", "0"), reverseEarlyVoltage: -1.0 });
    expect(() => dcOp(circuit)).toThrowError("reverse Early voltage must be finite and non-negative");
  });

  it("uses BJT forward beta roll-off to reduce high-current transport", () => {
    const collectorVoltage = (forwardBetaRolloffCurrent: number): number => {
      const circuit = new Circuit();
      circuit.add(voltageSource("Vcc", "vcc", "0", 5.0));
      circuit.add(voltageSource("Vbase", "base", "0", 0.65));
      circuit.add(resistor("Rload", "vcc", "out", 1_000.0));
      circuit.add({ ...bjt("Q1", "out", "base", "0"), forwardBetaRolloffCurrent });
      return dcOp(circuit).voltage("out");
    };

    expect(collectorVoltage(1.0e-4)).toBeGreaterThan(collectorVoltage(0.0));
  });

  it("uses BJT reverse beta to control base-collector junction current", () => {
    const baseCurrent = (reverseBeta: number): number => {
      const circuit = new Circuit();
      circuit.add(voltageSource("Vbase", "base", "0", 0.65));
      circuit.add(voltageSource("Vemitter", "emitter", "0", 0.65));
      circuit.add({ ...bjt("Q1", "0", "base", "emitter"), reverseBeta });
      return Math.abs(dcOp(circuit).branchCurrent("Vbase")!);
    };

    expect(baseCurrent(0.5)).toBeGreaterThan(baseCurrent(5.0));
  });

  it("rejects invalid BJT reverse beta", () => {
    const circuit = new Circuit();
    circuit.add({ ...bjt("Qbad", "c", "b", "0"), reverseBeta: 0.0 });
    expect(() => dcOp(circuit)).toThrowError(
      "reverse beta must be positive",
    );
  });

  it("uses BJT reverse beta roll-off to increase high-current base current", () => {
    const baseCurrent = (reverseBetaRolloffCurrent: number): number => {
      const circuit = new Circuit();
      circuit.add(voltageSource("Vbase", "base", "0", 0.65));
      circuit.add(voltageSource("Vemitter", "emitter", "0", 0.65));
      circuit.add({
        ...bjt("Q1", "0", "base", "emitter"),
        reverseBeta: 1.0,
        reverseBetaRolloffCurrent,
      });
      return Math.abs(dcOp(circuit).branchCurrent("Vbase")!);
    };

    expect(baseCurrent(1.0e-4)).toBeGreaterThan(baseCurrent(0.0));
  });

  it("rejects an invalid BJT reverse beta roll-off current", () => {
    const circuit = new Circuit();
    circuit.add({ ...bjt("Qbad", "c", "b", "0"), reverseBetaRolloffCurrent: -1.0 });
    expect(() => dcOp(circuit)).toThrowError(
      "reverse beta roll-off current must be finite and non-negative",
    );
  });

  it("rejects an invalid BJT forward beta roll-off current", () => {
    const circuit = new Circuit();
    circuit.add({ ...bjt("Qbad", "c", "b", "0"), forwardBetaRolloffCurrent: -1.0 });
    expect(() => dcOp(circuit)).toThrowError(
      "forward beta roll-off current must be finite and non-negative",
    );
  });

  it("uses BJT base-emitter leakage to increase base current", () => {
    const baseCurrent = (baseEmitterLeakageSaturationCurrent: number): number => {
      const circuit = new Circuit();
      circuit.add(voltageSource("Vbase", "base", "0", 0.65));
      circuit.add({
        ...bjt("Q1", "0", "base", "0"),
        baseEmitterLeakageSaturationCurrent,
        baseEmitterLeakageEmissionCoefficient: 1.5,
      });
      return Math.abs(dcOp(circuit).branchCurrent("Vbase")!);
    };

    expect(baseCurrent(1.0e-10)).toBeGreaterThan(baseCurrent(0.0));
  });

  it("rejects invalid BJT base-emitter leakage parameters", () => {
    const badCurrent = new Circuit();
    badCurrent.add({ ...bjt("Qbad", "c", "b", "0"), baseEmitterLeakageSaturationCurrent: -1.0 });
    expect(() => dcOp(badCurrent)).toThrowError("base-emitter leakage saturation current");

    const badCoefficient = new Circuit();
    badCoefficient.add({ ...bjt("Qbad", "c", "b", "0"), baseEmitterLeakageEmissionCoefficient: 0.0 });
    expect(() => dcOp(badCoefficient)).toThrowError("base-emitter leakage emission coefficient");
  });

  it("uses BJT base-collector leakage to increase base current", () => {
    const baseCurrent = (baseCollectorLeakageSaturationCurrent: number): number => {
      const circuit = new Circuit();
      circuit.add(voltageSource("Vbase", "base", "0", 0.65));
      circuit.add({
        ...bjt("Q1", "0", "base", "base"),
        baseCollectorLeakageSaturationCurrent,
        baseCollectorLeakageEmissionCoefficient: 1.5,
      });
      return Math.abs(dcOp(circuit).branchCurrent("Vbase")!);
    };

    expect(baseCurrent(1.0e-10)).toBeGreaterThan(baseCurrent(0.0));
  });

  it("rejects invalid BJT base-collector leakage parameters", () => {
    const badCurrent = new Circuit();
    badCurrent.add({ ...bjt("Qbad", "c", "b", "0"), baseCollectorLeakageSaturationCurrent: -1.0 });
    expect(() => dcOp(badCurrent)).toThrowError("base-collector leakage saturation current");

    const badCoefficient = new Circuit();
    badCoefficient.add({ ...bjt("Qbad", "c", "b", "0"), baseCollectorLeakageEmissionCoefficient: 0.0 });
    expect(() => dcOp(badCoefficient)).toThrowError("base-collector leakage emission coefficient");
  });

  it("uses BJT forward emission coefficient to reduce collector current", () => {
    const collectorVoltage = (forwardEmissionCoefficient: number): number => {
      const circuit = new Circuit();
      circuit.add(voltageSource("Vcc", "vcc", "0", 5.0));
      circuit.add(voltageSource("Vbase", "base", "0", 0.65));
      circuit.add(resistor("Rload", "vcc", "out", 1_000.0));
      circuit.add(bjt("Q1", "out", "base", "0", "NPN", 1e-14, 100, 0.02585, 0, 0, 0, 0, 3, 1.11, 0.0, forwardEmissionCoefficient));
      return dcOp(circuit).voltage("out");
    };

    expect(collectorVoltage(2.0)).toBeGreaterThan(collectorVoltage(1.0));
  });

  it("rejects an invalid BJT forward emission coefficient", () => {
    const circuit = new Circuit();
    circuit.add(bjt("Qbad", "c", "b", "0", "NPN", 1e-14, 100, 0.02585, 0, 0, 0, 0, 3, 1.11, 0.0, 0.0));
    expect(() => dcOp(circuit)).toThrowError("forward emission coefficient must be finite and positive");
  });

  it("rejects an invalid BJT reverse emission coefficient", () => {
    const circuit = new Circuit();
    circuit.add(bjt("Qbad", "c", "b", "0", "NPN", 1e-14, 100, 0.02585, 0, 0, 0, 0, 3, 1.11, 0.0, 1.0, 0.0));
    expect(() => dcOp(circuit)).toThrowError("reverse emission coefficient must be finite and positive");
  });

  it("rejects invalid BJT base-emitter depletion parameters", () => {
    const invalidPotential = new Circuit();
    invalidPotential.add(bjt("Qbad", "c", "b", "0", "NPN", 1e-14, 100, 0.02585, 0, 0, 0, 0, 3, 1.11, 0.0, 1.0, 1.0, 0.0));
    expect(() => dcOp(invalidPotential)).toThrowError("base-emitter junction potential must be finite and positive");

    const invalidGrading = new Circuit();
    invalidGrading.add(bjt("Qbad", "c", "b", "0", "NPN", 1e-14, 100, 0.02585, 0, 0, 0, 0, 3, 1.11, 0.0, 1.0, 1.0, 0.75, 1.0));
    expect(() => dcOp(invalidGrading)).toThrowError("base-emitter grading coefficient must be finite and in [0, 1)");
  });

  it("rejects invalid BJT base-collector depletion parameters", () => {
    const invalidPotential = new Circuit();
    invalidPotential.add(bjt("Qbad", "c", "b", "0", "NPN", 1e-14, 100, 0.02585, 0, 0, 0, 0, 3, 1.11, 0.0, 1.0, 1.0, 0.75, 0.33, 0.0));
    expect(() => dcOp(invalidPotential)).toThrowError("base-collector junction potential must be finite and positive");

    const invalidGrading = new Circuit();
    invalidGrading.add(bjt("Qbad", "c", "b", "0", "NPN", 1e-14, 100, 0.02585, 0, 0, 0, 0, 3, 1.11, 0.0, 1.0, 1.0, 0.75, 0.33, 0.75, 1.0));
    expect(() => dcOp(invalidGrading)).toThrowError("base-collector grading coefficient must be finite and in [0, 1)");
  });

  it("rejects invalid BJT forward-bias depletion coefficients", () => {
    for (const coefficient of [-0.1, 1.0, Number.NaN]) {
      const circuit = new Circuit();
      circuit.add(bjt("Qbad", "c", "b", "0", "NPN", 1e-14, 100, 0.02585, 0, 0, 0, 0, 3, 1.11, 0.0, 1.0, 1.0, 0.75, 0.33, 0.75, 0.33, coefficient));
      expect(() => dcOp(circuit)).toThrowError("forward-bias depletion coefficient must be finite and in [0, 1)");
    }
  });

  it("uses MOSFET temperature scaling in common-source drain voltage", () => {
    const nominal = new Circuit();
    nominal.add(voltageSource("Vdd", "vdd", "0", 1.8));
    nominal.add(voltageSource("Vgate", "gate", "0", 1.1));
    nominal.add(resistor("Rload", "vdd", "out", 1_000.0));
    nominal.add(mosfet("M1", "out", "gate", "0", "0", "NMOS", {
      VT0: 0.65,
      KP: 200.0e-6,
      W: 2.0e-6,
      L: 180.0e-9,
      LAMBDA: 0.02,
    }));

    const cold = circuitAtTemperature(nominal, 275.0);
    const hot = circuitAtTemperature(nominal, 350.0);

    const nominalResult = dcOp(nominal);
    const coldResult = dcOp(cold);
    const hotResult = dcOp(hot);

    expect(coldResult.voltage("out")).toBeGreaterThan(nominalResult.voltage("out")!);
    expect(hotResult.voltage("out")).toBeLessThan(nominalResult.voltage("out")!);
  });

  it("preserves subcircuits when applying temperature helpers", () => {
    const nominal = new Circuit();
    nominal.defineSubcircuit(
      subcircuitDefinition("atten2", ["in", "out"], [
        resistor("Rtop", "in", "out", 1_000.0),
        resistor("Rbot", "out", "0", 1_000.0),
      ]),
    );

    const adjusted = circuitAtTemperature(nominal, 350.0);
    adjusted.add(voltageSource("V1", "vin", "0", 10.0));
    adjusted.add(xInstance("X1", ["vin", "vout"], "atten2"));

    expectClose(dcOp(adjusted).voltage("vout"), 5.0);
  });

  it("solves an NPN BJT operating point", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vcc", "vcc", "0", 5.0));
    circuit.add(voltageSource("Vb", "base", "0", 0.7));
    circuit.add(resistor("Rc", "vcc", "collector", 100.0));
    circuit.add(bjt("Q1", "collector", "base", "0", "NPN", 1.0e-14, 120.0, 0.02585));

    const result = dcOp(circuit);

    expect(result.voltage("collector")).toBeGreaterThan(0.0);
    expect(result.voltage("collector")).toBeLessThan(5.0);
  });

  it("uses BJT emitter resistance to reduce fixed-base collector current", () => {
    const collectorVoltage = (emitterResistance: number): number => {
      const circuit = new Circuit();
      circuit.add(voltageSource("Vcc", "vcc", "0", 5.0));
      circuit.add(voltageSource("Vbase", "base", "0", 0.7));
      circuit.add(resistor("Rc", "vcc", "collector", 1_000.0));
      circuit.add({
        ...bjt("Q1", "collector", "base", "0"),
        emitterResistance,
      });
      return dcOp(circuit).voltage("collector") ?? 0.0;
    };

    expect(collectorVoltage(100.0)).toBeGreaterThan(collectorVoltage(0.0) + 0.5);
  });

  it("uses BJT collector resistance to drop intrinsic collector voltage", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vcollector", "collector", "0", 5.0));
    circuit.add(voltageSource("Vbase", "base", "0", 0.65));
    circuit.add({
      ...bjt("Q1", "collector", "base", "0"),
      collectorResistance: 100.0,
    });

    const intrinsic = dcOp(circuit).voltage("__spice_Q1_collector") ?? 0.0;
    expect(intrinsic).toBeGreaterThan(0.0);
    expect(intrinsic).toBeLessThan(5.0);
  });

  it("uses BJT base resistance to drop intrinsic base voltage", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vcollector", "collector", "0", 5.0));
    circuit.add(voltageSource("Vbase", "base", "0", 0.65));
    circuit.add({
      ...bjt("Q1", "collector", "base", "0"),
      baseResistance: 1_000.0,
    });

    const intrinsic = dcOp(circuit).voltage("__spice_Q1_base") ?? 0.0;
    expect(intrinsic).toBeGreaterThan(0.0);
    expect(intrinsic).toBeLessThan(0.65);
  });

  it("uses minimum BJT base resistance to reduce high-current base drop", () => {
    const intrinsicBase = (
      minimumBaseResistance: number | undefined,
      baseResistanceHalfCurrent: number,
    ): number => {
      const circuit = new Circuit();
      circuit.add(voltageSource("Vcollector", "collector", "0", 5.0));
      circuit.add(voltageSource("Vbase", "base", "0", 0.65));
      circuit.add({
        ...bjt("Q1", "collector", "base", "0"),
        baseResistance: 1_000.0,
        minimumBaseResistance,
        baseResistanceHalfCurrent,
      });
      return dcOp(circuit).voltage("__spice_Q1_base") ?? 0.0;
    };

    const fixed = intrinsicBase(undefined, 0.0);
    const biasDependent = intrinsicBase(10.0, 1.0e-6);
    expect(biasDependent).toBeGreaterThan(fixed);
    expect(biasDependent).toBeLessThan(0.65);
  });

  it("solves an NMOS operating point", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vdd", "vdd", "0", 1.8));
    circuit.add(voltageSource("Vgate", "gate", "0", 1.8));
    circuit.add(resistor("Rload", "vdd", "out", 1_000.0));
    circuit.add(mosfet("M1", "out", "gate", "0", "0", "NMOS", {
      VT0: 0.45,
      KP: 200.0e-6,
      W: 2.0e-6,
      L: 180.0e-9,
      LAMBDA: 0.02,
    }));

    const result = dcOp(circuit);

    expect(result.voltage("out")).toBeGreaterThanOrEqual(0.0);
    expect(result.voltage("out")).toBeLessThan(1.8);
    expect(result.converged).toBe(true);
    expect(result.iterations).toBeGreaterThan(0);
  });

  it("solves an N-channel JFET source-resistor bias point", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vdd", "vdd", "0", 10.0));
    circuit.add(voltageSource("Vg", "gate", "0", 0.0));
    circuit.add(resistor("Rd", "vdd", "drain", 2_000.0));
    circuit.add(resistor("Rs", "source", "0", 1_000.0));
    circuit.add(jfet("J1", "drain", "gate", "source", "NJF", 1.0e-3, -2.0));

    const result = dcOp(circuit);

    expect(result.converged).toBe(true);
    expect(result.voltage("source")).toBeCloseTo(1.0, 1);
    expect(result.voltage("drain")).toBeCloseTo(8.0, 0);
  });

  it("reports unconverged nonlinear operating points when aids are disabled", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vdd", "vdd", "0", 1.8));
    circuit.add(voltageSource("Vgate", "gate", "0", 1.8));
    circuit.add(resistor("Rload", "vdd", "out", 1_000.0));
    circuit.add(mosfet("M1", "out", "gate", "0", "0", "NMOS", {
      VT0: 0.45,
      KP: 200.0e-6,
      W: 2.0e-6,
      L: 180.0e-9,
      LAMBDA: 0.02,
    }));

    const result = dcOp(circuit, {
      maxIterations: 1,
      convergenceAids: false,
    });

    expect(result.converged).toBe(false);
    expect(result.convergenceAid).toBe("none");
    expect(result.iterations).toBe(1);
  });

  it("reports damped nonlinear Newton steps from the step limiter", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vs", "in", "0", 10.0));
    circuit.add(diode("D1", "in", "out", 1.0e-15, 0.02585));
    circuit.add(resistor("Rload", "out", "0", 100.0));

    const result = dcOp(circuit, {
      maxIterations: 1,
      convergenceAids: false,
      newtonStepLimit: 0.25,
    });

    expect(result.converged).toBe(false);
    expect(result.convergenceAid).toBe("none");
    expectClose(result.diagnostics.newtonStepLimit ?? 0.0, 0.25);
    expect(result.diagnostics.limitedNewtonSteps).toBe(1);
    expect(result.diagnostics.minimumDampingFactor).toBeGreaterThan(0.0);
    expect(result.diagnostics.minimumDampingFactor).toBeLessThan(1.0);
    expectClose(result.diagnostics.maxDelta, 0.25);
    expect(Math.max(...Array.from(result.nodeVoltages.values()).map(Math.abs))).toBeLessThanOrEqual(
      0.25 + 1.0e-12,
    );
  });

  it("recovers with pseudo-transient continuation after earlier aids fail", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vs", "in", "0", 10.0));
    circuit.add(diode("D1", "in", "out", 1.0e-15, 0.02585));
    circuit.add(resistor("Rload", "out", "0", 100.0));

    const result = dcOp(circuit, {
      maxIterations: 1,
      pseudoTransientMaxIterations: 500,
      pseudoTransientSteps: 40,
    });

    expect(result.converged).toBe(true);
    expect(result.convergenceAid).toBe("pseudo_transient");
    expect(result.voltage("out")).toBeGreaterThan(0.0);
    expect(result.voltage("out")).toBeLessThan(10.0);
  });

  it("rejects invalid MOSFET model parameters", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vdd", "vdd", "0", 1.8));
    circuit.add(voltageSource("Vgate", "gate", "0", 1.8));
    circuit.add(resistor("Rload", "vdd", "out", 1_000.0));
    circuit.add(mosfet("Mbad", "out", "gate", "0", "0", "NMOS", { KP: 0.0 }));

    expect(() => dcOp(circuit)).toThrowError("MOSFET KP must be positive");
  });

  it("rejects invalid BJT model parameters", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vcc", "vcc", "0", 5.0));
    circuit.add(voltageSource("Vb", "base", "0", 0.7));
    circuit.add(resistor("Rc", "vcc", "collector", 100.0));
    circuit.add(bjt("Qbad", "collector", "base", "0", "NPN", 1.0e-14, 0.0, 0.02585));

    expect(() => dcOp(circuit)).toThrowError("forward beta must be finite and positive");
  });

  it("rejects missing CCCS control sources", () => {
    const circuit = new Circuit();
    circuit.add(cccs("Fbad", "0", "out", "Vmissing", 2.0));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    expect(() => dcOp(circuit)).toThrowError("control source was not indexed");
  });

  it("rejects non-finite VCCS transconductance", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vctrl", "ctrl", "0", 1.0));
    circuit.add(vccs("Gbad", "0", "out", "ctrl", "0", Number.NaN));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    expect(() => dcOp(circuit)).toThrowError("transconductance must be finite");
  });

  it("rejects non-finite VCVS gain", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vctrl", "ctrl", "0", 1.0));
    circuit.add(vcvs("Ebad", "out", "0", "ctrl", "0", Number.NaN));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    expect(() => dcOp(circuit)).toThrowError("gain must be finite");
  });

  it("rejects non-finite CCCS gain", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vsense", "sense", "0", 0.0));
    circuit.add(cccs("Fbad", "0", "out", "Vsense", Number.NaN));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    expect(() => dcOp(circuit)).toThrowError("gain must be finite");
  });

  it("rejects non-finite CCVS transresistance", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vsense", "sense", "0", 0.0));
    circuit.add(ccvs("Hbad", "out", "0", "Vsense", Number.NaN));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    expect(() => dcOp(circuit)).toThrowError("transresistance must be finite");
  });

  it("uses static source value when a waveform is present", () => {
    const circuit = new Circuit();
    circuit.add(
      voltageSourceWithWaveform(
        "V1",
        "n1",
        "0",
        3.0,
        new SinWaveform(0.0, 10.0, 1_000.0),
      ),
    );
    circuit.add(resistor("R1", "n1", "0", 1_000.0));

    const result = dcOp(circuit);

    expectClose(result.voltage("n1"), 3.0);
  });

  it("sweeps voltage sources and collects operating points", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "vin", "0", 0.0));
    circuit.add(resistor("R1", "vin", "mid", 1_000.0));
    circuit.add(resistor("R2", "mid", "0", 1_000.0));

    const points = dcSweep(circuit, "V1", 0.0, 2.0, 1.0);

    expect(points).toHaveLength(3);
    expectClose(points[0].value, 0.0);
    expectClose(points[0].result.voltage("mid"), 0.0);
    expectClose(points[1].value, 1.0);
    expectClose(points[1].result.voltage("mid"), 0.5);
    expectClose(points[2].value, 2.0);
    expectClose(points[2].result.voltage("mid"), 1.0);
    expect(formatDcSweepTable("V1", points, ["V(mid)", "I(V1)"])).toBe(
      "Index\tSource\tValue\tV(mid)\tI(V1)\n" +
      "0\tV1\t0.000000e+00\t0.000000e+00\t0.000000e+00\n" +
      "1\tV1\t1.000000e+00\t5.000000e-01\t-5.000000e-04\n" +
      "2\tV1\t2.000000e+00\t1.000000e+00\t-1.000000e-03\n",
    );
    expect(formatDeckDcSweepTable("V1", points, ".save V(mid)\n.probe dc I(V1)\n.end\n")).toBe(
      "Index\tSource\tValue\tV(mid)\tI(V1)\n" +
      "0\tV1\t0.000000e+00\t0.000000e+00\t0.000000e+00\n" +
      "1\tV1\t1.000000e+00\t5.000000e-01\t-5.000000e-04\n" +
      "2\tV1\t2.000000e+00\t1.000000e+00\t-1.000000e-03\n",
    );
  });

  it("measures dc sweep probes and parsed .measure cards", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "vin", "0", 0.0));
    circuit.add(resistor("R1", "vin", "mid", 1_000.0));
    circuit.add(resistor("R2", "mid", "0", 1_000.0));

    const points = dcSweep(circuit, "V1", 0.0, 2.0, 1.0);
    const peak = measureDcSweepProbe(points, "midPeak", "V(mid)", "max", 1.0, 2.0);
    const average = measureDcSweepProbe(points, "midAvg", "V(mid)", "avg");

    expect(peak.value).toBeCloseTo(1.0, 9);
    expect(peak.analysis).toBe("dc");
    expect(average.value).toBeCloseTo(0.5, 9);
    expect(formatMeasurementTable([peak, average])).toBe(
      "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n" +
      "midPeak\tdc\tV(mid)\tmax\t1.000000e+00\t2.000000e+00\t1.000000e+00\n" +
      "midAvg\tdc\tV(mid)\tavg\t\t\t5.000000e-01\n",
    );

    const measurements = measureDcSweepDeck(
      points,
      `
.measure dc midSwing PP V(mid) FROM=0 TO=2
.meas dc midFinal FINAL V(mid)
.end
`,
    );

    expect(formatMeasurementTable(measurements)).toBe(
      "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n" +
      "midSwing\tdc\tV(mid)\tpp\t0.000000e+00\t2.000000e+00\t1.000000e+00\n" +
      "midFinal\tdc\tV(mid)\tlast\t\t\t1.000000e+00\n",
    );
  });

  it("sweeps current sources and collects operating points", () => {
    const circuit = new Circuit();
    circuit.add(currentSource("I1", "0", "n1", 0.0));
    circuit.add(resistor("R1", "n1", "0", 1_000.0));

    const points = dcSweep(circuit, "I1", 0.0, 2.0e-3, 1.0e-3);

    expect(points).toHaveLength(3);
    expectClose(points[0].result.voltage("n1"), 0.0);
    expectClose(points[1].result.voltage("n1"), 1.0);
    expectClose(points[2].result.voltage("n1"), 2.0);
  });

  it("rejects sweep steps that do not reach the stop value", () => {
    const circuit = new Circuit();

    expect(() => dcSweep(circuit, "V1", 0.0, 1.0, -0.1)).toThrowError(
      "sweep step direction",
    );
  });

  it("rejects invalid DC operating point options", () => {
    const circuit = new Circuit();

    expect(() => dcOp(circuit, { maxIterations: 0 })).toThrowError(
      "maxIterations must be a positive integer",
    );
    expect(() => dcOp(circuit, { tolerance: 0.0 })).toThrowError(
      "tolerance must be finite and positive",
    );
    expect(() => dcOp(circuit, { newtonStepLimit: 0.0 })).toThrowError(
      "newtonStepLimit must be finite and positive",
    );
  });

  it("rejects missing sweep sources", () => {
    const circuit = new Circuit();

    expect(() => dcSweep(circuit, "Vmissing", 0.0, 1.0, 1.0)).toThrowError(
      "sweep source must be an independent voltage or current source",
    );
  });

  it("returns a singular matrix error for a floating resistor", () => {
    const circuit = new Circuit();
    circuit.add(resistor("R1", "a", "b", 1_000.0));

    expect(() => dcOp(circuit)).toThrowError(SpiceError);
    expect(() => dcOp(circuit)).toThrowError("circuit matrix is singular");
  });

  it("rejects non-positive resistance", () => {
    const circuit = new Circuit();
    circuit.add(resistor("Rbad", "n1", "0", 0.0));

    expect(() => dcOp(circuit)).toThrowError("invalid element Rbad");
  });

  it("rejects duplicate voltage source names", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("V1", "n1", "0", 1.0));
    circuit.add(voltageSource("V1", "n2", "0", 2.0));

    expect(() => dcOp(circuit)).toThrowError("duplicate voltage source name");
  });
});

describe("dcCorners", () => {
  it("runs named corners with element parameter overrides", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 10.0));
    circuit.add(resistor("Rtop", "in", "out", 1_000.0));
    circuit.add(resistor("Rbot", "out", "0", 1_000.0));

    const result = dcCorners(circuit, [
      { name: "nominal", overrides: [] },
      {
        name: "rbot-fast",
        overrides: [{ elementName: "Rbot", parameter: "resistance", value: 500.0 }],
      },
      {
        name: "vin-high",
        overrides: [{ elementName: "Vin", parameter: "voltage", value: 12.0 }],
      },
      {
        name: "vin-inverted",
        overrides: [{ elementName: "Vin", parameter: "voltage", value: -10.0 }],
      },
    ]);

    expect(result.points.map((point) => point.cornerName)).toEqual([
      "nominal",
      "rbot-fast",
      "vin-high",
      "vin-inverted",
    ]);
    expect(result.points[0].result.voltage("out")).toBeCloseTo(5.0, 9);
    expect(result.points[1].result.voltage("out")).toBeCloseTo(10.0 / 3.0, 9);
    expect(result.points[2].result.voltage("out")).toBeCloseTo(6.0, 9);
    expect(result.points[3].result.voltage("out")).toBeCloseTo(-5.0, 9);
    expect(formatCornerDcTable(result, ["V(out)", "I(Vin)"])).toBe(
      "Corner\tIndex\tV(out)\tI(Vin)\n" +
      "nominal\t0\t5.000000e+00\t-5.000000e-03\n" +
      "rbot-fast\t1\t3.333333e+00\t-6.666667e-03\n" +
      "vin-high\t2\t6.000000e+00\t-6.000000e-03\n" +
      "vin-inverted\t3\t-5.000000e+00\t5.000000e-03\n",
    );
  });
});

describe("dcSweepCorners", () => {
  it("runs source sweeps at each named corner", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 0.0));
    circuit.add(resistor("Rtop", "in", "out", 1_000.0));
    circuit.add(resistor("Rbot", "out", "0", 1_000.0));

    const result = dcSweepCorners(circuit, "Vin", 0.0, 10.0, 5.0, [
      { name: "nominal", overrides: [] },
      {
        name: "rbot-fast",
        overrides: [{ elementName: "Rbot", parameter: "resistance", value: 500.0 }],
      },
    ]);

    expect(result.sourceName).toBe("Vin");
    expect(result.points.map((point) => point.cornerName)).toEqual(["nominal", "rbot-fast"]);
    expect(result.points[0].points.map((point) => point.value)).toEqual([0.0, 5.0, 10.0]);
    expect(result.points[0].points.map((point) => point.result.voltage("out"))).toEqual([
      0.0,
      2.5,
      5.0,
    ]);
    expect(result.points[1].points[0].result.voltage("out")).toBeCloseTo(0.0, 9);
    expect(result.points[1].points[1].result.voltage("out")).toBeCloseTo(5.0 / 3.0, 9);
    expect(result.points[1].points[2].result.voltage("out")).toBeCloseTo(10.0 / 3.0, 9);
    expect(formatCornerDcSweepTable(result, ["V(out)", "I(Vin)"])).toBe(
      "Corner\tIndex\tSource\tValue\tV(out)\tI(Vin)\n" +
      "nominal\t0\tVin\t0.000000e+00\t0.000000e+00\t0.000000e+00\n" +
      "nominal\t1\tVin\t5.000000e+00\t2.500000e+00\t-2.500000e-03\n" +
      "nominal\t2\tVin\t1.000000e+01\t5.000000e+00\t-5.000000e-03\n" +
      "rbot-fast\t0\tVin\t0.000000e+00\t0.000000e+00\t0.000000e+00\n" +
      "rbot-fast\t1\tVin\t5.000000e+00\t1.666667e+00\t-3.333333e-03\n" +
      "rbot-fast\t2\tVin\t1.000000e+01\t3.333333e+00\t-6.666667e-03\n",
    );
  });
});
