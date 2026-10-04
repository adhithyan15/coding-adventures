/**
 * FM-B069 release-scale evidence.
 *
 * Stopwatch-only tests are a property of whichever CI host happens to run
 * them.  This benchmark therefore records wall time for humans while gating
 * deterministic work: exactly 1,000 HTML and terminal routes on both runs,
 * followed by 999 parse-cache hits and one miss after one source edit.  That
 * catches accidental full rebuilds without inventing a machine-speed SLA.
 */
import { execFile } from "node:child_process";
import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { performance } from "node:perf_hooks";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);
const here = dirname(fileURLToPath(import.meta.url));
const forme = resolve(here, "../../packages/typescript/forme-cli/bin/forme.mjs");
export const BENCHMARK_ROOT = ".forme/release-benchmark";
const benchmarkRoot = resolve(here, BENCHMARK_ROOT);
const dataRoot = resolve(benchmarkRoot, "data");
const summaryPath = resolve(here, "dist/.forme-release-benchmark.json");

export const BENCHMARK_PAGE_COUNT = 1_000;
const EDITED_PAGE = Math.floor(BENCHMARK_PAGE_COUNT / 2);

export interface BenchmarkStage {
  readonly instanceId: string;
  readonly outcome: string;
  readonly itemsProduced: number;
  readonly cacheHits: number;
  readonly cacheMisses: number;
}

export interface BenchmarkBuild {
  readonly outcome: string;
  readonly buildId: string;
  readonly stages: readonly BenchmarkStage[];
  readonly outputs: Readonly<Record<string, {
    readonly manifest: { readonly routes: readonly unknown[] };
  }>>;
}

export interface BenchmarkTimings {
  readonly cleanElapsedMs: number;
  readonly incrementalElapsedMs: number;
}

export function benchmarkSource(index: number, edited: boolean): string {
  if (!Number.isSafeInteger(index) || index < 0 || index >= BENCHMARK_PAGE_COUNT) {
    throw new Error("release benchmark requires a safe page index inside the corpus");
  }
  const serial = String(index).padStart(4, "0");
  return `---\n` +
    `title: Forme release benchmark ${serial}\n` +
    `date: 2026-01-01\n` +
    `slug: release-benchmark-${serial}\n` +
    `excerpt: Deterministic generated release-scale input ${serial}.\n` +
    `---\n\n` +
    `# Release benchmark page ${serial}\n\n` +
    `This bounded page proves the live Forme product path at release scale.\n` +
    (edited ? `\nThe single incremental edit belongs to page ${serial}.\n` : "");
}

export function evaluateBenchmark(
  clean: BenchmarkBuild,
  incremental: BenchmarkBuild,
  timings: BenchmarkTimings,
): Readonly<Record<string, unknown>> {
  requireSuccessfulBuild("clean", clean);
  requireSuccessfulBuild("incremental", incremental);
  if (clean.buildId === incremental.buildId) {
    throw new Error("release benchmark incremental edit did not change the build identity");
  }

  const cleanSource = requireStage(clean, "source");
  const cleanParse = requireStage(clean, "parse");
  const incrementalSource = requireStage(incremental, "source");
  const incrementalParse = requireStage(incremental, "parse");
  for (const [label, source] of [["clean", cleanSource], ["incremental", incrementalSource]] as const) {
    if (source.outcome !== "success" || source.itemsProduced !== BENCHMARK_PAGE_COUNT) {
      throw new Error(`${label} source produced ${source.itemsProduced}; expected ${BENCHMARK_PAGE_COUNT}`);
    }
  }
  if (cleanParse.cacheHits !== 0 || cleanParse.cacheMisses !== BENCHMARK_PAGE_COUNT) {
    throw new Error(
      `clean parse work was ${cleanParse.cacheHits} hit(s)/${cleanParse.cacheMisses} miss(es); ` +
      `expected 0/${BENCHMARK_PAGE_COUNT}`,
    );
  }
  if (incrementalParse.cacheHits !== BENCHMARK_PAGE_COUNT - 1 || incrementalParse.cacheMisses !== 1) {
    throw new Error(
      `incremental parse work was ${incrementalParse.cacheHits} hit(s)/` +
      `${incrementalParse.cacheMisses} miss(es); expected ${BENCHMARK_PAGE_COUNT - 1}/1`,
    );
  }

  for (const [label, build] of [["clean", clean], ["incremental", incremental]] as const) {
    requireRouteCount(label, build, "articles");
    requireRouteCount(label, build, "terminal");
  }

  return Object.freeze({
    schemaVersion: 1,
    pageCount: BENCHMARK_PAGE_COUNT,
    editedPage: EDITED_PAGE,
    clean: Object.freeze({
      elapsedMs: boundedElapsed(timings.cleanElapsedMs),
      buildId: clean.buildId,
      parsedPages: cleanParse.cacheMisses,
    }),
    incremental: Object.freeze({
      elapsedMs: boundedElapsed(timings.incrementalElapsedMs),
      buildId: incremental.buildId,
      reusedParsedPages: incrementalParse.cacheHits,
      parsedPages: incrementalParse.cacheMisses,
    }),
  });
}

async function main(): Promise<void> {
  await rm(benchmarkRoot, { recursive: true, force: true });
  try {
    await writeCorpus();
    await runForme("clean", "--config", "release-benchmark.config.ts");

    const cleanResult = await measuredBuild("clean.json");
    await writeFile(pagePath(EDITED_PAGE), benchmarkSource(EDITED_PAGE, true), "utf8");
    const incrementalResult = await measuredBuild("incremental.json");
    const summary = evaluateBenchmark(cleanResult.build, incrementalResult.build, {
      cleanElapsedMs: cleanResult.elapsedMs,
      incrementalElapsedMs: incrementalResult.elapsedMs,
    });

    await mkdir(dirname(summaryPath), { recursive: true });
    const text = `${JSON.stringify(summary, null, 2)}\n`;
    await writeFile(summaryPath, text, "utf8");
    process.stdout.write(text);
  } finally {
    await rm(benchmarkRoot, { recursive: true, force: true });
  }
}

async function writeCorpus(): Promise<void> {
  await mkdir(dataRoot, { recursive: true });
  // Small batches avoid turning the benchmark generator itself into an open
  // file-descriptor stress test on Windows.
  for (let start = 0; start < BENCHMARK_PAGE_COUNT; start += 32) {
    const end = Math.min(start + 32, BENCHMARK_PAGE_COUNT);
    await Promise.all(Array.from({ length: end - start }, (_, offset) => {
      const index = start + offset;
      return writeFile(pagePath(index), benchmarkSource(index, false), "utf8");
    }));
  }
}

async function measuredBuild(reportName: string): Promise<{
  readonly build: BenchmarkBuild;
  readonly elapsedMs: number;
}> {
  const reportPath = resolve(benchmarkRoot, reportName);
  const start = performance.now();
  await runForme("build", "--config", "release-benchmark.config.ts", "--report", reportPath);
  const elapsedMs = performance.now() - start;
  return {
    build: JSON.parse(await readFile(reportPath, "utf8")) as BenchmarkBuild,
    elapsedMs,
  };
}

async function runForme(...args: string[]): Promise<void> {
  const result = await execFileAsync(process.execPath, [forme, ...args], {
    cwd: here,
    maxBuffer: 16 * 1024 * 1024,
  });
  if (result.stderr !== "") throw new Error(`forme benchmark stderr: ${result.stderr.slice(0, 4_096)}`);
}

function pagePath(index: number): string {
  return resolve(dataRoot, `release-benchmark-${String(index).padStart(4, "0")}.md`);
}

function requireSuccessfulBuild(label: string, build: BenchmarkBuild): void {
  if (build.outcome !== "success") throw new Error(`${label} release benchmark outcome was ${build.outcome}`);
}

function requireStage(build: BenchmarkBuild, instanceId: string): BenchmarkStage {
  const stage = build.stages.find(candidate => candidate.instanceId === instanceId);
  if (stage === undefined) throw new Error(`release benchmark report is missing stage ${instanceId}`);
  return stage;
}

function requireRouteCount(label: string, build: BenchmarkBuild, output: string): void {
  const routes = build.outputs[output]?.manifest.routes;
  if (!Array.isArray(routes) || routes.length !== BENCHMARK_PAGE_COUNT) {
    throw new Error(
      `${label} ${output} route count was ${Array.isArray(routes) ? routes.length : "missing"}; ` +
      `expected ${BENCHMARK_PAGE_COUNT}`,
    );
  }
}

function boundedElapsed(value: number): number {
  if (!Number.isFinite(value) || value < 0) throw new Error("release benchmark elapsed time is invalid");
  return Math.round(value * 100) / 100;
}

const invokedPath = process.argv[1] === undefined ? "" : resolve(process.argv[1]);
if (invokedPath === fileURLToPath(import.meta.url)) {
  main().catch(error => {
    process.stderr.write(`${error instanceof Error ? error.stack ?? error.message : String(error)}\n`);
    process.exitCode = 1;
  });
}
