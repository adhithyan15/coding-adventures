import { describe, expect, it } from "vitest";
import { mkdir, mkdtemp, readFile, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import {
  BENCHMARK_PAGE_COUNT,
  atomicWriteBenchmarkSummary,
  benchmarkSource,
  evaluateBenchmark,
  type BenchmarkBuild,
} from "./release-benchmark.js";

describe("Forme release benchmark", () => {
  it("generates exactly identifiable deterministic pages", () => {
    expect(BENCHMARK_PAGE_COUNT).toBe(1_000);
    expect(benchmarkSource(0, false)).toContain("title: Forme release benchmark 0000");
    expect(benchmarkSource(999, false)).toContain("# Release benchmark page 0999");
    expect(benchmarkSource(500, true)).not.toBe(benchmarkSource(500, false));
    expect(() => benchmarkSource(-1, false)).toThrow(/safe page index/);
    expect(() => benchmarkSource(BENCHMARK_PAGE_COUNT, false)).toThrow(/safe page index/);
  });

  it("accepts one-page incremental reuse without imposing a wall-clock threshold", () => {
    const clean = build({
      buildId: "clean",
      sourceOutcome: "success",
      sourceProduced: 1_000,
      parseHits: 0,
      parseMisses: 1_000,
      articleRoutes: 1_000,
      terminalRoutes: 1_000,
    });
    const incremental = build({
      buildId: "incremental",
      sourceOutcome: "success",
      sourceProduced: 1_000,
      parseHits: 999,
      parseMisses: 1,
      articleRoutes: 1_000,
      terminalRoutes: 1_000,
    });

    expect(evaluateBenchmark(clean, incremental, {
      cleanElapsedMs: 1,
      incrementalElapsedMs: 99_999,
    })).toMatchObject({
      schemaVersion: 1,
      pageCount: 1_000,
      clean: { elapsedMs: 1, parsedPages: 1_000 },
      incremental: { elapsedMs: 99_999, reusedParsedPages: 999, parsedPages: 1 },
    });
  });

  it.each([
    ["failed clean build", build({ outcome: "failed" })],
    ["incomplete corpus", build({ sourceProduced: 999 })],
    ["missing HTML route", build({ articleRoutes: 999 })],
    ["missing terminal route", build({ terminalRoutes: 999 })],
    ["unchanged incremental build", build({ buildId: "clean" })],
    ["broad incremental parse", build({ parseHits: 998, parseMisses: 2 })],
  ])("rejects %s", (_name, malformed) => {
    const clean = build({ buildId: "clean", parseHits: 0, parseMisses: 1_000 });
    expect(() => evaluateBenchmark(clean, malformed, {
      cleanElapsedMs: 1,
      incrementalElapsedMs: 1,
    })).toThrow();
  });

  it("atomically replaces a hostile report symlink without following it", async () => {
    const root = await mkdtemp(resolve(tmpdir(), "forme-release-report-test-"));
    try {
      const victim = resolve(root, "victim.json");
      const output = resolve(root, "dist/report.json");
      await writeFile(victim, "untouched\n", "utf8");
      await symlink(victim, output).catch(async error => {
        if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
        await mkdir(resolve(root, "dist"));
        await symlink(victim, output);
      });

      await atomicWriteBenchmarkSummary(output, root, "safe\n");

      expect(await readFile(victim, "utf8")).toBe("untouched\n");
      expect(await readFile(output, "utf8")).toBe("safe\n");
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  });
});

function build(overrides: {
  readonly outcome?: string;
  readonly buildId?: string;
  readonly sourceOutcome?: string;
  readonly sourceProduced?: number;
  readonly parseHits?: number;
  readonly parseMisses?: number;
  readonly articleRoutes?: number;
  readonly terminalRoutes?: number;
} = {}): BenchmarkBuild {
  const route = { pattern: "/blog/release-benchmark-0000.html" };
  return {
    outcome: overrides.outcome ?? "success",
    buildId: overrides.buildId ?? "incremental",
    stages: [
      {
        instanceId: "source",
        outcome: overrides.sourceOutcome ?? "success",
        itemsProduced: overrides.sourceProduced ?? 1_000,
        cacheHits: 0,
        cacheMisses: 0,
      },
      {
        instanceId: "parse",
        outcome: "success",
        itemsProduced: 1_000,
        cacheHits: overrides.parseHits ?? 999,
        cacheMisses: overrides.parseMisses ?? 1,
      },
    ],
    outputs: {
      articles: {
        manifest: {
          routes: Array.from({ length: overrides.articleRoutes ?? 1_000 }, () => route),
        },
      },
      terminal: {
        manifest: {
          routes: Array.from({ length: overrides.terminalRoutes ?? 1_000 }, () => route),
        },
      },
    },
  };
}
