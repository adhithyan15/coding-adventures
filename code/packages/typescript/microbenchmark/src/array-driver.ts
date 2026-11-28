#!/usr/bin/env node
/** The shared indexed traversal workload: indexed baseline vs native iterator. */
import { collect, defaults, replayReport, stringifyLosslessJson, type Config } from "./index.js";

declare const process: {
  argv: string[];
  env: Record<string, string | undefined>;
  version: string;
  platform: string;
  arch: string;
  exitCode?: number;
};

export const WORKLOAD_ID = "indexed-array-sum-4096";

export function makeArray(): number[] {
  return Array.from({ length: 4096 }, (_, i) => (17 * i + 3) % 251);
}

export function expectedSum(values: readonly number[]): bigint {
  let total = 0n;
  for (const value of values) total += BigInt(value);
  return total;
}

export function indexedBatch(values: readonly number[], operations: bigint): bigint {
  let total = 0n;
  for (let op = 0n; op < operations; op += 1n) {
    for (let i = 0; i < values.length; i += 1) total += BigInt(values[i]);
  }
  return total;
}

export function iteratorBatch(values: readonly number[], operations: bigint): bigint {
  let total = 0n;
  for (let op = 0n; op < operations; op += 1n) for (const value of values) total += BigInt(value);
  return total;
}

export function parseArgs(args: readonly string[]): Config {
  const config: { -readonly [K in keyof Config]: Config[K] } = { ...defaults() };
  const flags: Record<string, keyof Config> = {
    "--pairs": "pairs", "--confidence": "confidence_percent", "--factor": "minimum_factor",
    "--seed": "seed", "--target-ns": "target_ns", "--max-batch": "max_batch",
  };
  for (let i = 0; i < args.length; i += 1) {
    if (args[i] === "--revision") {
      if (i + 1 >= args.length) throw new Error("invalid argument: --revision");
      i += 1;
      continue;
    }
    const flag = args[i];
    const key = Object.hasOwn(flags, flag) ? flags[flag] : undefined;
    if (key === undefined || i + 1 >= args.length) throw new Error(`invalid argument: ${args[i]}`);
    const raw = args[++i];
    if (key === "pairs" || key === "confidence_percent" || key === "minimum_factor") {
      const value = Number(raw);
      if (!Number.isFinite(value)) throw new Error(`invalid numeric argument for ${args[i - 1]}`);
      if (key === "pairs") config.pairs = value;
      else if (key === "confidence_percent") config.confidence_percent = value;
      else config.minimum_factor = value;
    } else {
      let value: bigint;
      try { value = BigInt(raw); } catch { throw new Error(`invalid integer argument for ${args[i - 1]}`); }
      if (key === "seed") config.seed = value;
      else if (key === "target_ns") config.target_ns = value;
      else config.max_batch = value;
    }
  }
  return config;
}

function revisionFrom(args: readonly string[]): string {
  const index = args.indexOf("--revision");
  if (index >= 0) {
    if (index + 1 >= args.length || args[index + 1].startsWith("--")) throw new Error("invalid argument: --revision");
    return args[index + 1];
  }
  const primary = process.env.GIT_REVISION;
  if (primary !== undefined && primary.length > 0) return primary;
  const fallback = process.env.GITHUB_SHA;
  return fallback !== undefined && fallback.length > 0 ? fallback : "unknown";
}

export function runArrayBenchmark(config: Config, invocation: readonly string[], revision = "unknown") {
  const values = makeArray();
  const perOperation = expectedSum(values);
  const report = collect(
    (n) => indexedBatch(values, n),
    (n) => iteratorBatch(values, n),
    perOperation,
    config,
  );
  const result = {
    workload: WORKLOAD_ID,
    invocation: [...invocation],
    revision,
    runtime: process.version,
    os: process.platform,
    arch: process.arch,
    ...report,
  };
  replayReport(result);
  return result;
}

if (process.argv[1]?.endsWith("array-driver.js") || process.argv[1]?.endsWith("array-driver.ts")) {
  try {
    console.log(stringifyLosslessJson(runArrayBenchmark(parseArgs(process.argv.slice(2)), process.argv.slice(1), revisionFrom(process.argv.slice(2)))));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
