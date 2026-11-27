import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { analyze, collect, defaults, parseLosslessJson, replayJson, replayReport, stringifyLosslessJson, type Config } from "../src/index.js";
import { expectedSum, indexedBatch, iteratorBatch, makeArray, parseArgs, WORKLOAD_ID } from "../src/array-driver.js";

const packagedFixture = readFileSync(new URL("../testdata/microbenchmark-pairs-v1.json", import.meta.url), "utf8");
const sharedFixture = readFileSync(new URL("../../../../specs/fixtures/microbenchmark-pairs-v1.json", import.meta.url), "utf8");
expect(packagedFixture).toBe(sharedFixture);
const fixture = JSON.parse(packagedFixture) as {
  confidence_percent: number;
  minimum_factor: number;
  cases: Array<{ baseline_ns: number; candidate_ns: number[]; ratio: number; interval: [number, number] | null; verdict: string }>;
};

describe("paired microbenchmarks", () => {
  it("replays every shared analysis case", () => {
    for (const example of fixture.cases) {
      const pairs = example.candidate_ns.map((candidate_ns, index) => ({ baseline_ns: example.baseline_ns, candidate_ns, order: index % 2 ? "candidate-first" as const : "baseline-first" as const }));
      expect(analyze(pairs, fixture.confidence_percent, fixture.minimum_factor)).toEqual({ ratio: example.ratio, interval: example.interval, verdict: example.verdict });
    }
  });

  it("bounds public analysis before inspecting an oversized pair list", () => {
    const pair = { baseline_ns: 1, candidate_ns: 1, order: "baseline-first" as const };
    expect(analyze(Array(10_000).fill(pair)).ratio).toBe(1);
    const malformed = { baseline_ns: 0, candidate_ns: 0, order: "invalid" as const };
    expect(() => analyze(Array(10_001).fill(malformed))).toThrow(/at most 10000/);
    expect(() => analyze(Array(10_001).fill(malformed), 0)).toThrow(/confidence_percent/);
  });

  it("collects a seeded, balanced schedule and replays losslessly", () => {
    let tick = 0n;
    const config: Config = { ...defaults(), pairs: 7, target_ns: 1n, max_batch: 8n, seed: 71n };
    const report = collect((n) => { tick += 4n * n; return 10n * n; }, (n) => { tick += 3n * n; return 10n * n; }, 10n, config, () => tick);
    expect(report.batch_size).toBe(1n);
    expect(report.pairs.filter((pair) => pair.order === "baseline-first")).toHaveLength(4);
    expect(report.pairs.filter((pair) => pair.order === "candidate-first")).toHaveLength(3);
    expect(report.pairs.every((pair) => pair.baseline_ns === 4n && pair.candidate_ns === 3n)).toBe(true);
    expect(replayJson(stringifyLosslessJson(report))).toEqual(report);
    const reordered = { ...(parseLosslessJson(stringifyLosslessJson(report)) as object), analysis: { verdict: report.analysis.verdict, interval: report.analysis.interval, ratio: report.analysis.ratio } };
    expect(replayReport(reordered)).toEqual(report);
  });

  it("doubles a common calibration batch and rejects exhausted zero-time calibration", () => {
    let tick = 0n;
    const config: Config = { ...defaults(), pairs: 1, target_ns: 5n, max_batch: 4n };
    const report = collect((n) => { tick += 3n * n; return n; }, (n) => { tick += 4n * n; return n; }, 1n, config, () => tick);
    expect(report.batch_size).toBe(2n);
    expect(() => collect((n) => n, (n) => n, 1, { ...config, max_batch: 1n }, () => 0n)).toThrow(/calibration limit exhausted/);
  });

  it("allows batch counts above the default cap", () => {
    let tick = 0n;
    const config: Config = { ...defaults(), pairs: 1, target_ns: 65_537n, max_batch: 9223372036854775807n };
    const batch = (n: bigint) => { tick += n; return n; };
    const report = collect(batch, batch, 1n, config, () => tick);
    expect(report.batch_size).toBe(131_072n);
  });

  it("rejects invalid config, inputs, clock, workload, checksum, and tampered reports", () => {
    expect(() => collect(() => 1, () => 1, 1, { ...defaults(), pairs: 0 }, () => 0n)).toThrow(RangeError);
    expect(() => analyze([{ baseline_ns: 0, candidate_ns: 1, order: "baseline-first" }])).toThrow(RangeError);
    expect(() => analyze([{ baseline_ns: 1, candidate_ns: 1, order: "other" as "baseline-first" }])).toThrow();
    let tick = 2n;
    expect(() => collect((n) => n, (n) => n, 1, { ...defaults(), pairs: 1, target_ns: 1 }, () => { tick -= 1n; return tick; })).toThrow(/regress/);
    expect(() => collect(() => { throw new Error("workload"); }, (n) => n, 1, defaults(), () => 0n)).toThrow("workload");
    expect(() => collect((n) => n, (n) => n + 1n, 1, defaults(), () => 0n)).toThrow(/checksum/);
    expect(() => analyze([{ baseline_ns: Number.MAX_SAFE_INTEGER + 1, candidate_ns: 1, order: "baseline-first" }])).toThrow(RangeError);
    expect(() => replayJson('{"config":{}}')).toThrow();
    const report = collect((n) => n, (n) => n, 1, { ...defaults(), pairs: 1, target_ns: 1n }, (() => { let t = 0n; return () => (t += 1n); })());
    expect(() => replayReport({ ...report, analysis: { ...report.analysis, verdict: "faster" } })).toThrow(/does not match/);
    const balanced = collect((n) => n, (n) => n, 1, { ...defaults(), pairs: 3, target_ns: 1n }, (() => { let t = 0n; return () => (t += 1n); })());
    const unbalanced = balanced.pairs.map((pair) => ({ ...pair, order: "candidate-first" as const }));
    expect(() => replayReport({ ...balanced, pairs: unbalanced })).toThrow(/not balanced/);
  });

  it("defines and implements the same indexed array workload", () => {
    const values = makeArray();
    expect(values).toHaveLength(4096);
    expect(values.slice(0, 4)).toEqual([3, 20, 37, 54]);
    expect(indexedBatch(values, 3n)).toBe(expectedSum(values) * 3n);
    expect(iteratorBatch(values, 3n)).toBe(expectedSum(values) * 3n);
    expect(WORKLOAD_ID).toBe("indexed-array-sum-4096");
    expect(parseArgs(["--pairs", "3", "--confidence", "90", "--factor", "1.1", "--seed", "17", "--target-ns", "10", "--max-batch", "4"])).toEqual({ pairs: 3, confidence_percent: 90, minimum_factor: 1.1, seed: 17n, target_ns: 10n, max_batch: 4n });
    expect(() => parseArgs(["--mystery", "2"])).toThrow(/invalid argument/);
    for (const inherited of ["constructor", "__proto__", "toString"]) {
      expect(() => parseArgs([inherited, "2"])).toThrow(/invalid argument/);
    }
    expect(() => parseArgs(["--pairs"])).toThrow(/invalid argument/);
    for (const inherited of ["constructor", "toString", "__proto__"]) {
      expect(() => parseArgs([inherited, "1"])).toThrow(/invalid argument/);
    }
    for (const raw of ["not-a-number", "Infinity"]) {
      expect(() => parseArgs(["--pairs", raw])).toThrow(/invalid numeric argument/);
    }
    for (const raw of ["1.5", "not-an-integer"]) {
      expect(() => parseArgs(["--seed", raw])).toThrow(/invalid integer argument/);
    }
  });

  it("runs the native Node CLI and replays its JSON without a timing threshold", () => {
    const cliPath = fileURLToPath(new URL("../dist/array-driver.js", import.meta.url));
    const child = spawnSync(process.execPath, [cliPath, "--pairs", "6", "--target-ns", "1", "--max-batch", "8", "--revision", "test-revision"], { encoding: "utf8" });
    expect(child.status).toBe(0);
    const raw = parseLosslessJson(child.stdout) as Record<string, unknown>;
    const replayed = replayReport(raw);
    expect(raw.workload).toBe(WORKLOAD_ID);
    expect(raw.revision).toBe("test-revision");
    expect(raw.runtime).toBe(process.version);
    expect(raw.arch).toBe(process.arch);
    expect("expected_checksum_per_operation" in raw).toBe(false);
    expect(replayed.pairs).toHaveLength(6);
    expect(replayed.pairs.every((pair) => BigInt(pair.baseline_ns) > 0n && BigInt(pair.candidate_ns) > 0n)).toBe(true);
  });

  it("falls through empty ambient revision labels and rejects an empty explicit label", () => {
    const cliPath = fileURLToPath(new URL("../dist/array-driver.js", import.meta.url));
    const args = [cliPath, "--pairs", "1", "--target-ns", "1", "--max-batch", "1"];
    for (const [environment, expected] of [
      [{ GIT_REVISION: "", GITHUB_SHA: "fallback" }, "fallback"],
      [{ GIT_REVISION: "", GITHUB_SHA: "" }, "unknown"],
      [{ GIT_REVISION: "primary", GITHUB_SHA: "secondary" }, "primary"],
    ] as const) {
      const child = spawnSync(process.execPath, args, {
        encoding: "utf8",
        env: { ...process.env, ...environment },
      });
      expect(child.status, child.stderr).toBe(0);
      expect((parseLosslessJson(child.stdout) as Record<string, unknown>).revision).toBe(expected);
      expect(replayJson(child.stdout).pairs).toHaveLength(1);
    }

    const invalid = spawnSync(process.execPath, [...args, "--revision", ""], {
      encoding: "utf8",
      env: { ...process.env, GIT_REVISION: "", GITHUB_SHA: "" },
    });
    expect(invalid.status).not.toBe(0);
    expect(invalid.stdout).toBe("");
  });

  it("validates complete metadata before projecting it from core replay", () => {
    let tick = 0n;
    const report = collect((n) => n, (n) => n, 1n,
      { ...defaults(), pairs: 1, target_ns: 1n }, () => (tick += 1n));
    const core = parseLosslessJson(stringifyLosslessJson(report)) as Record<string, unknown>;
    const complete = {
      ...core,
      workload: "dependent-link-order",
      invocation: [] as string[],
      revision: "r",
      runtime: "Node.js",
      os: "test",
      arch: "test",
    };
    const replayBoth = (value: Record<string, unknown>) => {
      expect(replayReport(value)).toEqual(report);
      expect(replayJson(stringifyLosslessJson(value))).toEqual(report);
    };
    replayBoth(complete);
    replayBoth({ ...complete, invocation: [""] });
    for (const field of ["workload", "invocation", "revision", "runtime", "os", "arch"]) {
      const partial = { ...complete };
      delete (partial as Record<string, unknown>)[field];
      expect(() => replayReport(partial)).toThrow();
      expect(() => replayJson(stringifyLosslessJson(partial))).toThrow();
    }
    const profileOnly = { ...core, profile: "release" } as Record<string, unknown>;
    expect(() => replayReport(profileOnly)).toThrow();
    expect(() => replayJson(stringifyLosslessJson(profileOnly))).toThrow();

    for (const field of ["revision", "runtime", "os", "arch", "profile"]) {
      for (const invalid of [null, "", "bad\0value"]) {
        const malformed = { ...complete, profile: "release", [field]: invalid };
        expect(() => replayReport(malformed)).toThrow();
        expect(() => replayJson(stringifyLosslessJson(malformed))).toThrow();
      }
      replayBoth({ ...complete, profile: "release", [field]: " " });
    }
    for (const invalid of [null, "", " \t\n\r\f\v", "\0", "x\0y", "\ud800", "\udfff"]) {
      const malformed = { ...complete, workload: invalid };
      expect(() => replayReport(malformed)).toThrow();
      expect(() => replayJson(stringifyLosslessJson(malformed))).toThrow();
    }
    for (const invalid of [["bad\0arg"], [1], null]) {
      const malformed = { ...complete, invocation: invalid };
      expect(() => replayReport(malformed)).toThrow();
      expect(() => replayJson(stringifyLosslessJson(malformed))).toThrow();
    }
    for (const accepted of ["\u2003", "\ud83d\ude00", "\ufffd", "\\uD800"]) {
      replayBoth({ ...complete, workload: accepted });
    }
  });

  it("rejects duplicate decoded JSON members while allowing separate object scopes", () => {
    let tick = 0n;
    const report = collect((n) => n, (n) => n, 1n,
      { ...defaults(), pairs: 1, target_ns: 1n }, () => (tick += 1n));
    const valid = stringifyLosslessJson({
      ...report, workload: "generic-workload", invocation: [], revision: "shared",
      runtime: "shared", os: "test", arch: "test",
    });
    expect(replayJson(valid)).toEqual(report);
    // The same member name appears in the root and config objects, not one scope.

    const invalid = [
      valid.replace('"pairs":1', '"pairs":1,"pairs":1'),
      valid.replace('"baseline_ns":1', '"baseline_ns":1,"baseline_ns":1'),
      valid.replace('"verdict":"inconclusive"', '"verdict":"inconclusive","verdict":"inconclusive"'),
      valid.replace('"revision":"shared"', '"revision":"shared","revision":"other"'),
      valid.replace('"pairs":1', '"pairs":1,"pa\\u0069rs":1'),
    ];
    for (const text of invalid) expect(() => replayJson(text)).toThrow(/duplicate JSON object member/);
    expect(() => replayJson(`${valid.slice(0, -1)},}`)).toThrow();
    for (const incomplete of [valid.slice(0, -1), `${valid} trailing`, `${valid} {}`]) {
      expect(() => replayJson(incomplete)).toThrow();
    }

    const unknown = { ...report, extra: true };
    expect(() => replayReport(unknown)).toThrow(/unknown report member/);
    expect(() => replayJson(stringifyLosslessJson(unknown))).toThrow(/unknown report member/);
  });

  it("replays a maximum-size balanced raw report with repeated row member names", () => {
    const pairs = Array.from({ length: 10_000 }, (_, index) => ({
      baseline_ns: 1n,
      candidate_ns: 1n,
      order: index % 2 === 0 ? "baseline-first" as const : "candidate-first" as const,
    }));
    const config = { ...defaults(), pairs: 10_000 };
    const report = {
      config,
      batch_size: 1n,
      pairs,
      analysis: analyze(pairs, config.confidence_percent, config.minimum_factor),
    };
    const replayed = replayJson(stringifyLosslessJson(report));
    expect(replayed.pairs).toHaveLength(10_000);
    expect(replayed.analysis).toEqual(report.analysis);
  });

  it("preserves signed-64-bit JSON integer tokens above 2^53", () => {
    const parsed = parseLosslessJson(`{"seed":-9223372036854775808,"duration":9007199254740993}`) as { seed: bigint; duration: bigint };
    expect(parsed.seed).toBe(-9223372036854775808n);
    expect(parsed.duration).toBe(9007199254740993n);
    expect(stringifyLosslessJson(parsed)).toBe(`{"seed":-9223372036854775808,"duration":9007199254740993}`);
  });

  it("accepts full signed-64-bit seeds, checksums, clock ticks and durations", () => {
    const config: Config = { ...defaults(), pairs: 6, seed: -9223372036854775808n, target_ns: 1n, max_batch: 9223372036854775807n };
    const maximum = 9223372036854775807n;
    const report = collect(() => maximum, () => maximum, maximum, config, (() => { let now = maximum - 1_000_000n; return () => (now += 10n); })());
    expect(report.pairs).toHaveLength(6);
    expect(BigInt(report.pairs[0].baseline_ns)).toBe(10n);
    expect(replayJson(stringifyLosslessJson(report))).toEqual(report);
    expect(analyze([{ baseline_ns: maximum, candidate_ns: maximum, order: "baseline-first" }]).ratio).toBe(1);
    const largeDuration = 9_007_199_254_740_993n;
    const replayedLarge = replayJson(stringifyLosslessJson({
      config: { ...defaults(), pairs: 1 }, batch_size: 1n,
      pairs: [{ baseline_ns: largeDuration, candidate_ns: largeDuration, order: "baseline-first" }],
      analysis: analyze([{ baseline_ns: largeDuration, candidate_ns: largeDuration, order: "baseline-first" }]),
    }));
    expect(replayedLarge.pairs[0].baseline_ns).toBe(largeDuration);
    expect(() => analyze([{ baseline_ns: 9223372036854775808n, candidate_ns: 1n, order: "baseline-first" }])).toThrow(RangeError);
    expect(() => collect((n) => n, (n) => n, 1n, { ...config, pairs: 1, target_ns: 1n, max_batch: 1n }, () => maximum)).toThrow(/calibration limit exhausted/);
    const minimum = -9223372036854775808n;
    const negative = collect(() => minimum, () => minimum, minimum, { ...config, pairs: 1 }, (() => { let t = 0n; return () => (t += 1n); })());
    expect(replayJson(stringifyLosslessJson(negative)).pairs[0].baseline_ns).toBe(1n);
  });
});
