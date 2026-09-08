import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { describe, expect, it } from "vitest";

import {
  CLI_ERROR_CODE,
  CLI_INSPECTION_SCHEMA_VERSION,
  CLI_RESULT_SCHEMA_VERSION,
  inspectNetlistJson,
  runNetlistJson,
} from "../src/index.js";

describe("Berkeley SPICE CLI contract", () => {
  it("returns the shared JSON envelope for the core analysis plan", () => {
    const corpus = JSON.parse(readFileSync(
      new URL("../../../../grammars/spice/berkeley-v1-cli-corpus.json", import.meta.url),
      "utf8",
    )) as {
      schemaVersion: number;
      suite: string;
      cases: readonly {
        deck: string;
        expected: { title: string; analysisKinds: readonly string[]; recordCounts: readonly number[] };
      }[];
    };
    const testCase = corpus.cases[0]!;
    const payload = JSON.parse(runNetlistJson(testCase.deck)) as {
      schemaVersion: number;
      title: string | null;
      analyses: readonly { kind: string; records: readonly Record<string, string>[] }[];
    };

    expect(corpus).toMatchObject({ schemaVersion: 1, suite: "berkeley-v1-cli" });
    expect(payload.schemaVersion).toBe(CLI_RESULT_SCHEMA_VERSION);
    expect(payload.title).toBe(testCase.expected.title);
    expect(payload.analyses.map((analysis) => analysis.kind)).toEqual(testCase.expected.analysisKinds);
    expect(payload.analyses.map((analysis) => analysis.records.length)).toEqual(testCase.expected.recordCounts);
  });

  it("runs the bundled CLI against standard input", () => {
    const corpus = JSON.parse(readFileSync(
      new URL("../../../../grammars/spice/berkeley-v1-cli-corpus.json", import.meta.url),
      "utf8",
    )) as { cases: readonly { deck: string; expected: { title: string } }[] };
    const testCase = corpus.cases[0]!;
    const result = spawnSync(
      process.execPath,
      [fileURLToPath(new URL("../dist/cli.js", import.meta.url)), "run", "--json", "-"],
      { encoding: "utf8", input: testCase.deck },
    );

    expect(result.status, result.stderr).toBe(0);
    expect(JSON.parse(result.stdout).title).toBe(testCase.expected.title);
  });

  it("inspects the complete runnable plan through the API and CLI", () => {
    const corpus = JSON.parse(readFileSync(
      new URL("../../../../grammars/spice/berkeley-v1-cli-corpus.json", import.meta.url),
      "utf8",
    )) as {
      cases: readonly {
        deck: string;
        expected: { analysisIndexes: readonly number[]; analysisKinds: readonly string[] };
      }[];
    };
    const testCase = corpus.cases[0]!;
    const payload = JSON.parse(inspectNetlistJson(testCase.deck)) as {
      schemaVersion: number;
      analyses: readonly { index: number; kind: string }[];
    };
    const result = spawnSync(
      process.execPath,
      [fileURLToPath(new URL("../dist/cli.js", import.meta.url)), "inspect", "--json", "-"],
      { encoding: "utf8", input: testCase.deck },
    );

    expect(payload.schemaVersion).toBe(CLI_INSPECTION_SCHEMA_VERSION);
    expect(payload.analyses.map((analysis) => analysis.index)).toEqual(testCase.expected.analysisIndexes);
    expect(payload.analyses.map((analysis) => analysis.kind)).toEqual(testCase.expected.analysisKinds);
    expect(result.status, result.stderr).toBe(0);
    expect(JSON.parse(result.stdout)).toEqual(payload);
  });

  it("reports the shared stable failure code", () => {
    const corpus = JSON.parse(readFileSync(
      new URL("../../../../grammars/spice/berkeley-v1-cli-corpus.json", import.meta.url),
      "utf8",
    )) as {
      failureCases: readonly { deck: string; expected: { exitStatus: number; diagnosticCode: string } }[];
    };
    const testCase = corpus.failureCases[0]!;
    const result = spawnSync(
      process.execPath,
      [fileURLToPath(new URL("../dist/cli.js", import.meta.url)), "run", "--json", "-"],
      { encoding: "utf8", input: testCase.deck },
    );

    expect(result.status).toBe(testCase.expected.exitStatus);
    expect(result.stderr).toMatch(new RegExp(`^${testCase.expected.diagnosticCode}: `));
    expect(result.stderr).toMatch(new RegExp(`^${CLI_ERROR_CODE}: `));
  });

  it("freezes the shared Berkeley v1 release gate", () => {
    const readCorpus = (name: string): Record<string, unknown> => JSON.parse(readFileSync(
      new URL(`../../../../grammars/spice/${name}`, import.meta.url),
      "utf8",
    )) as Record<string, unknown>;
    const release = readCorpus("berkeley-v1-release-manifest.json");
    const core = readCorpus("berkeley-v1-op-corpus.json");
    const syntax = readCorpus("berkeley-v1-syntax-corpus.json");
    const cli = readCorpus("berkeley-v1-cli-corpus.json");
    const gate = release.corpusGate as {
      minimumCoreCaseCount: number;
      requiredAnalysisKinds: readonly string[];
      requiredDeviceKinds: readonly string[];
      minimumSyntaxCaseCount: number;
      requiredSyntaxClassifications: readonly string[];
      minimumCliSuccessCaseCount: number;
      minimumCliFailureCaseCount: number;
    };
    const coreCases = core.cases as readonly { analysis: string; kind: string }[];
    const syntaxCases = syntax.cases as readonly { classification: string }[];
    const cliCases = cli.cases as readonly unknown[];
    const cliFailureCases = cli.failureCases as readonly unknown[];
    const cliContract = release.cli as {
      result: { schemaVersion: number };
      failureDiagnostic: { code: string };
    };

    expect(release).toMatchObject({ schemaVersion: 1, suite: "berkeley-v1-release" });
    expect(coreCases.length).toBeGreaterThanOrEqual(gate.minimumCoreCaseCount);
    expect(new Set(coreCases.map((testCase) => testCase.analysis)))
      .toEqual(expect.objectContaining(new Set(gate.requiredAnalysisKinds)));
    expect(new Set(coreCases.map((testCase) => testCase.kind)))
      .toEqual(expect.objectContaining(new Set(gate.requiredDeviceKinds)));
    expect(syntaxCases.length).toBeGreaterThanOrEqual(gate.minimumSyntaxCaseCount);
    expect(new Set(syntaxCases.map((testCase) => testCase.classification)))
      .toEqual(expect.objectContaining(new Set(gate.requiredSyntaxClassifications)));
    expect(cliCases.length).toBeGreaterThanOrEqual(gate.minimumCliSuccessCaseCount);
    expect(cliFailureCases.length).toBeGreaterThanOrEqual(gate.minimumCliFailureCaseCount);
    expect(cliContract.result.schemaVersion).toBe(CLI_RESULT_SCHEMA_VERSION);
    expect(cliContract.failureDiagnostic.code).toBe(CLI_ERROR_CODE);
  });
});
