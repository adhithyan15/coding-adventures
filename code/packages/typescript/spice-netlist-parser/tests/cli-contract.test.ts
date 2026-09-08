import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { describe, expect, it } from "vitest";

import { CLI_RESULT_SCHEMA_VERSION, runNetlistJson } from "../src/index.js";

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
});
