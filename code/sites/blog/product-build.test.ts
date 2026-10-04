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
    const reports = await mkdtemp(join(tmpdir(), "forme-blog-product-"));
    try {
      await runForme("clean");
      const clean = await build(resolve(reports, "clean.json"));
      const warm = await build(resolve(reports, "warm.json"));

      expect(clean.outcome).toBe("success");
      expect(clean.reproducible).toBe(true);
      expect(clean.stages.map(stage => [stage.instanceId, stage.outcome])).toEqual([
        ["source", "success"],
        ["parse", "success"],
        ["attach-interactivity", "success"],
        ["resolve-assets", "success"],
        ["route", "success"],
        ["collect-posts", "success"],
        ["render-pages", "success"],
        ["render-terminal", "success"],
        ["package-terminal", "success"],
        ["load-assets", "success"],
        ["render-surface", "success"],
        ["emit-articles", "success"],
        ["emit-surface", "success"],
      ]);
      const warmOutcomes = warm.stages.map(stage => [stage.instanceId, stage.outcome]);
      expect(warmOutcomes.filter(([instanceId]) =>
        instanceId !== "parse" && instanceId !== "attach-interactivity")).toEqual([
        ["source", "skipped"],
        ["resolve-assets", "success"],
        ["route", "skipped"],
        ["collect-posts", "skipped"],
        ["render-pages", "skipped"],
        ["render-terminal", "skipped"],
        ["package-terminal", "skipped"],
        ["load-assets", "success"],
        ["render-surface", "skipped"],
        ["emit-articles", "skipped"],
        ["emit-surface", "skipped"],
      ]);
      expect([
        [["parse", "skipped"], ["attach-interactivity", "success"]],
        [["parse", "success"], ["attach-interactivity", "skipped"]],
      ]).toContainEqual(warmOutcomes.slice(1, 3));
      expect(warm.stages.every(stage => stage.inputChanged === false)).toBe(true);
      expect(warm.stages.filter(stage =>
        stage.instanceId === "parse" || stage.instanceId === "attach-interactivity")
        .every(stage => stage.cacheMisses === 0)).toBe(true);
      expect(warm.stages.filter(stage => stage.outcome === "skipped")
        .every(stage => stage.cacheHits === 1 && stage.cacheMisses === 0)).toBe(true);
      expect(warm.buildId).toBe(clean.buildId);
      expect(Object.keys(clean.outputs)).toEqual(["articles", "surface", "terminal"]);
      expect(JSON.stringify(warm.outputs)).toBe(JSON.stringify(clean.outputs));
      await expectInteractivityOutput(clean);
      expectTerminalOutput(clean);
      await expectFilesMatchReport(warm, ["articles", "surface"]);
    } finally {
      await rm(reports, { recursive: true, force: true });
    }
  }, 30_000);
});

async function build(reportPath: string): Promise<BuildReport> {
  await runForme("build", "--reproducible", "--report", reportPath);
  return JSON.parse(await readFile(reportPath, "utf8")) as BuildReport;
}

async function expectInteractivityOutput(report: BuildReport): Promise<void> {
  const articles = report.outputs.articles;
  expect(articles).toBeDefined();
  const hello = articles?.manifest.routes.find(
    route => route.pattern === "/blog/2026-05-15-hello-forme.html",
  );
  expect(hello?.islands).toEqual(["pipeline-step-explorer"]);
  expect(articles?.manifest.routes.filter(route => route.pattern !== hello?.pattern)
    .every(route => route.islands.length === 0)).toBe(true);
  expect(articles?.manifest.assets.filter(asset => asset.mime === "text/javascript"))
    .toHaveLength(1);
  const helloHtml = await readFile(
    resolve(here, "dist/blog/2026-05-15-hello-forme.html"),
    "utf8",
  );
  expect([...helloHtml.matchAll(/<li><strong><code>([^<]+)<\/code><\/strong>/g)]
    .map(match => match[1])).toEqual([
      "forme-source-fs",
      "forme-parse-markdown",
      "blog-attach-interactivity",
      "forme-resolve-asset-refs-fs",
      "forme-router",
      "forme-collect-chronological",
      "forme-render-static",
      "forme-render-terminal",
      "forme-render-terminal/package",
      "forme-load-assets-fs",
      "blog-surface",
      "forme-emit-site-fs",
      "forme-emit-fs",
    ]);
}

function expectTerminalOutput(report: BuildReport): void {
  const terminal = report.outputs.terminal;
  expect(terminal).toBeDefined();
  expect(terminal?.manifest.routes).toHaveLength(3);
  expect(terminal?.files.map(file => file.path)).toEqual([
    "terminal/blog/2026-05-08-capability-typed-stages.html.ansi",
    "terminal/blog/2026-05-08-capability-typed-stages.html.degradations.json",
    "terminal/blog/2026-05-12-why-forme.html.ansi",
    "terminal/blog/2026-05-12-why-forme.html.degradations.json",
    "terminal/blog/2026-05-15-hello-forme.html.ansi",
    "terminal/blog/2026-05-15-hello-forme.html.degradations.json",
  ]);
  expect(terminal?.files.every(file => file.sha256 !== null)).toBe(true);
}

async function runForme(...args: string[]): Promise<void> {
  const result = await execFileAsync(process.execPath, [forme, ...args], { cwd: here });
  expect(result.stderr).toBe("");
}

async function expectFilesMatchReport(report: BuildReport, names: readonly string[]): Promise<void> {
  for (const name of names) {
    const output = report.outputs[name]!;
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
    readonly manifest: {
      readonly buildTime: string;
      readonly routes: readonly { readonly pattern: string; readonly islands: readonly string[] }[];
      readonly assets: readonly { readonly mime: string }[];
    };
    readonly files: readonly { readonly path: string; readonly sha256: string | null }[];
  }>>;
}
