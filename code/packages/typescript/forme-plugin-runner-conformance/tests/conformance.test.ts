import { cp, mkdtemp, readFile, rm, writeFile, mkdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import {
  CONFORMANCE_STAGE,
  RunnerConformanceError,
  runRunnerConformance,
  type RunnerCommand,
} from "../src/index.js";

const fixtureSource = fileURLToPath(new URL("./fixtures/typescript-runner.mjs", import.meta.url));
const packagesRoot = fileURLToPath(new URL("../..", import.meta.url));
let fixtureRoot = "";
let command: RunnerCommand;

beforeAll(async () => {
  fixtureRoot = await mkdtemp(join(tmpdir(), "forme-runner-conformance-"));
  const fixture = join(fixtureRoot, "plugin.mjs");
  await writeFile(fixture, await readFile(fixtureSource));
  await cp(join(packagesRoot, "forme-plugin-runner-ts", "dist"), join(fixtureRoot, "runner"), { recursive: true });
  for (const packageName of ["forme-types", "forme-errors", "forme-stage"]) {
    const target = join(fixtureRoot, "node_modules", "@coding-adventures", packageName);
    await mkdir(target, { recursive: true });
    await cp(join(packagesRoot, packageName, "dist"), join(target, "dist"), { recursive: true });
    await writeFile(join(target, "package.json"), JSON.stringify({
      name: `@coding-adventures/${packageName}`,
      type: "module",
      main: "dist/index.js",
    }));
  }
  command = {
    executable: process.execPath,
    args: [fixture, CONFORMANCE_STAGE.id, CONFORMANCE_STAGE.configSchemaHash],
    cwd: fixtureRoot,
    modeArgument: true,
    expectedRunner: "@coding-adventures/forme-plugin-runner-ts",
    expectedRunnerVersion: "1.0.0",
  };
});

afterAll(async () => { if (fixtureRoot) await rm(fixtureRoot, { recursive: true, force: true }); });

describe("language-neutral runner conformance", () => {
  it("accepts the TypeScript reference runner across the canonical corpus", async () => {
    await expect(runRunnerConformance(command)).resolves.toEqual({
      runner: command.expectedRunner,
      runnerVersion: command.expectedRunnerVersion,
      vectors: [
        "lifecycle", "wire-values", "capabilities", "typed-error", "cancellation",
        "stream-stream", "stream-single", "single-stream", "identity-mismatch",
        "malformed-response-envelope", "malformed-peer", "resource-bounds",
      ],
    });
  });

  it("rejects an executable that cannot satisfy the runner protocol", async () => {
    await expect(runRunnerConformance({
      ...command,
      args: ["--eval", "process.exit(0)"],
      modeArgument: false,
    })).rejects.toBeInstanceOf(RunnerConformanceError);
  });
});
