import { createHash } from "node:crypto";
import { execFile } from "node:child_process";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import { describe, expect, it } from "vitest";

const execFileAsync = promisify(execFile);
const here = dirname(fileURLToPath(import.meta.url));
const forme = resolve(here, "../../packages/typescript/forme-cli/bin/forme.mjs");

describe("live product scheduling", () => {
  it("keeps a reproducible clean build byte-identical while safely skipping the warm build", async () => {
    const reports = await mkdtemp(join(tmpdir(), "forme-landing-product-"));
    try {
      await runForme("clean");
      const clean = await build(resolve(reports, "clean.json"));
      const warm = await build(resolve(reports, "warm.json"));

      expect(clean.outcome).toBe("success");
      expect(clean.reproducible).toBe(true);
      expect(clean.stages.map(stage => [stage.instanceId, stage.outcome])).toEqual([
        ["source", "success"],
        ["parse", "success"],
        ["resolve-assets", "success"],
        ["route", "success"],
        ["render", "success"],
        ["load-assets", "success"],
        ["emit", "success"],
      ]);
      expect(warm.stages.map(stage => [stage.instanceId, stage.outcome])).toEqual([
        ["source", "skipped"],
        ["parse", "skipped"],
        ["resolve-assets", "success"],
        ["route", "skipped"],
        ["render", "success"],
        ["load-assets", "success"],
        ["emit", "skipped"],
      ]);
      expect(warm.stages.every(stage => stage.inputChanged === false)).toBe(true);
      expect(warm.stages.filter(stage => stage.outcome === "skipped")
        .every(stage => stage.cacheHits === 1 && stage.cacheMisses === 0)).toBe(true);
      expect(warm.buildId).toBe(clean.buildId);
      expect(Object.keys(clean.outputs)).toEqual(["site"]);
      expect(JSON.stringify(warm.outputs)).toBe(JSON.stringify(clean.outputs));
      await expectFilesMatchReport(warm);
    } finally {
      await rm(reports, { recursive: true, force: true });
    }
  });
});

async function build(reportPath: string): Promise<BuildReport> {
  await runForme("build", "--reproducible", "--report", reportPath);
  return JSON.parse(await readFile(reportPath, "utf8")) as BuildReport;
}

async function runForme(...args: string[]): Promise<void> {
  const result = await execFileAsync(process.execPath, [forme, ...args], { cwd: here });
  expect(result.stderr).toBe("");
}

async function expectFilesMatchReport(report: BuildReport): Promise<void> {
  for (const output of Object.values(report.outputs)) {
    expect(output.manifest.buildTime).toBe("1970-01-01T00:00:00.000Z");
    for (const file of output.files) {
      const bytes = await readFile(resolve(here, "dist", file.path));
      expect(createHash("sha256").update(bytes).digest("hex")).toBe(file.sha256);
    }
  }
}

interface BuildReport {
  readonly outcome: string;
  readonly buildId: string;
  readonly reproducible: boolean;
  readonly stages: readonly {
    readonly instanceId: string;
    readonly outcome: string;
    readonly inputChanged: boolean | null;
    readonly cacheHits: number;
    readonly cacheMisses: number;
  }[];
  readonly outputs: Readonly<Record<string, {
    readonly manifest: { readonly buildTime: string };
    readonly files: readonly { readonly path: string; readonly sha256: string | null }[];
  }>>;
}
