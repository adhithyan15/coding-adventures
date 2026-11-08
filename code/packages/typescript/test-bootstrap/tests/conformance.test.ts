import { describe, it } from "node:test";
import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";
import { parseFile } from "../dist/tfv.js";
import { buildSelfCorpusSuite } from "../dist/corpus.js";
import { expect } from "../dist/matchers.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FIXTURE_PATH = path.resolve(
  __dirname,
  "../../../../specs/fixtures/test-bootstrap-v1/cases.tfv"
);

describe("TF01 Conformance self-corpus", () => {
  it("executes all 85 cases in test-bootstrap-v1/cases.tfv", async () => {
    const content = fs.readFileSync(FIXTURE_PATH, "utf-8");
    const records = parseFile(content);

    expect(records.length).toBe(85);

    const suite = buildSelfCorpusSuite(records);
    const summary = await suite.run({ seed: 0n, silent: true });

    expect(summary.failed).toBe(0);
    expect(summary.passed).toBe(records.length);
    expect(summary.exitCode).toBe(0);
  });
});
