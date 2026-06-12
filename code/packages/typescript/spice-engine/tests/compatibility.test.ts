import { describe, expect, it } from "vitest";
import {
  type CompatibilityDeck,
  compatibilityCorpus,
  formatCompatibilityCorpusTable,
  formatReleaseReadinessReport,
  releaseReadinessGates,
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
});
