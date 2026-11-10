import { describe, it } from "node:test";
import * as path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { expect } from "../dist/matchers.js";
import { testBootstrapCliSpec } from "../dist/cli_spec.js";
import { USAGE_EXIT } from "../dist/harness.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const CLI_PATH = path.resolve(__dirname, "../bin/cli.mjs");

function runCli(...args: string[]) {
  const result = spawnSync(process.execPath, [CLI_PATH, ...args], {
    encoding: "utf-8",
  });
  return {
    status: result.status,
    stdout: result.stdout ?? "",
    stderr: result.stderr ?? "",
  };
}

describe("CLI Builder integration for test-bootstrap (TF01b)", () => {
  it("exports a valid declarative CliSpec", () => {
    expect(testBootstrapCliSpec.name).toBe("test-bootstrap");
    expect(testBootstrapCliSpec.specVersion).toBe("1.0");
    expect(testBootstrapCliSpec.version).toBe("0.2.0");
    expect(testBootstrapCliSpec.parsingMode).toBe("gnu");
    expect(testBootstrapCliSpec.builtinFlags.help).toBe(true);
    expect(testBootstrapCliSpec.builtinFlags.version).toBe(true);

    const flagIds = testBootstrapCliSpec.flags.map((f) => f.id);
    expect(flagIds).toContain("seed");
    expect(flagIds).toContain("sabotage");
    expect(flagIds).toContain("reporter");
    expect(flagIds).toContain("filter");
    expect(flagIds).toContain("bail");
    expect(flagIds).toContain("timeout");
    expect(flagIds).toContain("self-corpus");
  });

  it("--help prints usage information and exits with code 0", () => {
    const res = runCli("--help");
    expect(res.status).toBe(0);
    expect(res.stdout).toContain("USAGE");
    expect(res.stdout).toContain("test-bootstrap [OPTIONS] [FILES...]");
    expect(res.stdout).toContain("--seed");
    expect(res.stdout).toContain("--sabotage");
    expect(res.stdout).toContain("--reporter");
    expect(res.stdout).toContain("-h, --help");
  });

  it("--version prints package version and exits with code 0", () => {
    const res = runCli("--version");
    expect(res.status).toBe(0);
    expect(res.stdout.trim()).toBe("0.2.0");
  });

  it("unknown flag exits with USAGE_EXIT (code 2)", () => {
    const res = runCli("--unrecognized-option");
    expect(res.status).toBe(USAGE_EXIT);
    expect(res.stderr).toContain("usage error");
  });

  it("invalid enum value exits with USAGE_EXIT (code 2)", () => {
    const res = runCli("--sabotage", "invalid-mode");
    expect(res.status).toBe(USAGE_EXIT);
    expect(res.stderr).toContain("usage error");
  });

  it("nonexistent test file exits with code 1", () => {
    const res = runCli("nonexistent-test-file.test.ts");
    expect(res.status).toBe(1);
    expect(res.stderr).toContain("test file not found");
  });

  it("executing valid test files passes with exit code 0", () => {
    const matchersTest = path.resolve(__dirname, "matchers.test.ts");
    const res = runCli(matchersTest);
    expect(res.status).toBe(0);
    expect(res.stdout).toContain("Level 1 matchers");
    expect(res.stdout).toContain("pass");
  });

  it("sabotage always-pass mode exits with code 1 and emits not ok TAP line", () => {
    const res = runCli("--sabotage", "always-pass");
    expect(res.status).toBe(1);
    expect(res.stdout).toContain("not ok");
  });

  it("default invocation runs self-corpus and exits with code 0", () => {
    const res = runCli();
    expect(res.status).toBe(0);
    expect(res.stdout).toContain("TAP version 14");
    expect(res.stdout).toContain("1..85");
  });
});
