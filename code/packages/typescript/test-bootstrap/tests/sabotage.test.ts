import { describe, it } from "node:test";
import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";
import { parseFile } from "../dist/tfv.js";
import { Suite, parseArgs, USAGE_EXIT } from "../dist/harness.js";
import { buildSelfCorpusSuite } from "../dist/corpus.js";
import { fail, pass } from "../dist/verdict.js";
import { expect } from "../dist/matchers.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FIXTURE_PATH = path.resolve(
  __dirname,
  "../../../../specs/fixtures/test-bootstrap-v1/cases.tfv"
);

describe("Sabotage mode failure detection", () => {
  it("sabotage always-pass mode causes expected fail cases to report failure", async () => {
    const content = fs.readFileSync(FIXTURE_PATH, "utf-8");
    const records = parseFile(content);

    const suite = buildSelfCorpusSuite(records);

    // Run in sabotage mode: every core check returns Pass, so expected failures will fail
    const summary = await suite.run({ seed: 0n, sabotage: true, silent: true });

    expect(summary.failed).toBeGreaterThan(0);
    expect(summary.exitCode).toBe(1);

    const notOkLines = summary.tapOutput.filter((line) => line.startsWith("not ok"));
    expect(notOkLines.length).toBe(summary.failed);
  });

  it("protocol error: unregistered failure ID bails out immediately", async () => {
    const suite = new Suite();
    suite.add("test-1", () => pass());
    // Direct construction of an unregistered failure ID
    suite.add("test-2", () => ({ status: "fail", failureId: "custom-unregistered-id" as any, message: "secret error data" }));
    suite.add("test-3", () => pass());

    const summary = await suite.run({ seed: 0n, silent: true });

    expect(summary.bailed).toBe(true);
    expect(summary.exitCode).toBe(1);
    expect(summary.passed).toBe(2);
    // Malformed verdict is not counted as passed or failed
    expect(summary.failed).toBe(0);

    const bailOutLine = summary.tapOutput.find((line) => line.startsWith("Bail out!"));
    expect(bailOutLine).toBe("Bail out! invalid failure id");

    // Secret error or custom ID is never printed in TAP output
    expect(summary.tapOutput.join("\n").includes("custom-unregistered-id")).toBe(false);
    expect(summary.tapOutput.join("\n").includes("secret error data")).toBe(false);
  });

  it("protocol error: duplicate test ID bails out before running anything", async () => {
    const suite = new Suite();
    suite.add("test-dup", () => pass());
    suite.add("test-dup", () => pass());

    const summary = await suite.run({ seed: 0n, silent: true });
    expect(summary.bailed).toBe(true);
    expect(summary.exitCode).toBe(1);
    expect(summary.passed).toBe(0);
    expect(summary.tapOutput).toContain("Bail out! duplicate test id test-dup");
  });

  it("protocol error: invalid test ID bails out before running anything", async () => {
    const suite = new Suite();
    suite.add("invalid id with spaces", () => pass());

    const summary = await suite.run({ seed: 0n, silent: true });
    expect(summary.bailed).toBe(true);
    expect(summary.exitCode).toBe(1);
    expect(summary.passed).toBe(0);
    expect(summary.tapOutput).toContain("Bail out! invalid test id invalid id with spaces");
  });

  it("escaping exception is caught and redacted as panic without leaking secrets", async () => {
    const suite = new Suite();
    suite.add("panicking-test", () => {
      throw new Error("SECRET_TOKEN_DO_NOT_LEAK_12345");
    });

    const summary = await suite.run({ seed: 0n, silent: true });
    expect(summary.failed).toBe(1);
    expect(summary.exitCode).toBe(1);

    const output = summary.tapOutput.join("\n");
    expect(output.includes("SECRET_TOKEN_DO_NOT_LEAK_12345")).toBe(false);
    expect(output.includes('failure_id: "panic"')).toBe(true);
    expect(output.includes('message: "the test panicked"')).toBe(true);
  });

  it("parseArgs validates arguments and enforces u64 and always-pass mode", () => {
    const opts = parseArgs(["--seed", "12345", "--sabotage", "always-pass", "--nocapture"]);
    expect(opts.seed).toBe(12345n);
    expect(opts.sabotage).toBe(true);

    expect(() => parseArgs(["--seed"])).toThrow();
    expect(() => parseArgs(["--seed", "not-a-number"])).toThrow();
    expect(() => parseArgs(["--seed", "18446744073709551616"])).toThrow();
    expect(() => parseArgs(["--sabotage"])).toThrow();
    expect(() => parseArgs(["--sabotage", "invalid-mode"])).toThrow();
    expect(() => parseArgs(["--unknown-flag"])).toThrow();
  });
});
