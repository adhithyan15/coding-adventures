/** Paired microbenchmarks: collect reproducible raw pairs, then analyze them. */
import { median, medianInterval } from "@coding-adventures/rank-statistics";

const MIN_I64 = -(1n << 63n);
const MAX_I64 = (1n << 63n) - 1n;
const MAX_PAIRS = 10_000;

export type Int64 = number | bigint;
export type PairOrder = "baseline-first" | "candidate-first";
export interface Pair {
  readonly baseline_ns: Int64;
  readonly candidate_ns: Int64;
  readonly order: PairOrder;
}
export interface Analysis {
  readonly ratio: number;
  readonly interval: readonly [number, number] | null;
  readonly verdict: "faster" | "slower" | "equivalent" | "inconclusive";
}
export interface Config {
  readonly pairs: number;
  readonly confidence_percent: number;
  readonly minimum_factor: number;
  readonly seed: Int64;
  readonly target_ns: Int64;
  readonly max_batch: Int64;
}
export interface Report {
  readonly config: Config;
  readonly batch_size: bigint;
  readonly pairs: readonly Pair[];
  readonly analysis: Analysis;
}
export type Batch = (operations: bigint) => Int64;
export type Clock = () => bigint;

interface NativeJson {
  parse(text: string, reviver?: (this: unknown, key: string, value: unknown, context?: { source?: string }) => unknown): unknown;
  stringify(value: unknown, replacer?: (this: unknown, key: string, value: unknown) => unknown): string | undefined;
  rawJSON?: (text: string) => unknown;
}
const nativeJson = JSON as unknown as NativeJson;

/** Reject duplicate decoded object names while leaving parsing to Node's JSON parser. */
function assertUniqueObjectMembers(text: string): void {
  const scopes: Array<Set<string> | null> = [];
  let index = 0;

  while (index < text.length) {
    const character = text[index];
    if (character === '"') {
      let ending = index + 1;
      while (ending < text.length) {
        if (text[ending] === "\\") {
          ending += 2;
          continue;
        }
        if (text[ending] === '"') break;
        ending += 1;
      }
      if (ending >= text.length) return; // Node's parser reports malformed strings.

      let after = ending + 1;
      while (after < text.length && /[\t\n\r ]/.test(text[after])) after += 1;
      const current = scopes[scopes.length - 1];
      if (current !== undefined && current !== null && text[after] === ":") {
        const key = nativeJson.parse(text.slice(index, ending + 1));
        if (typeof key !== "string") throw new SyntaxError("invalid JSON object member name");
        if (current.has(key)) throw new SyntaxError("duplicate JSON object member");
        current.add(key);
      }
      index = ending + 1;
      continue;
    }

    if (character === "{") scopes.push(new Set<string>());
    else if (character === "[") scopes.push(null);
    else if (character === "}" || character === "]") scopes.pop();
    index += 1;
  }
}

/** Parse JSON without rounding integer tokens outside binary64's exact range. */
export function parseLosslessJson(text: string): unknown {
  return nativeJson.parse(text, function (_key, value, context) {
    if (typeof value !== "number") return value;
    if (context?.source === undefined) throw new Error("Node.js 22.15 or newer is required for lossless integer JSON");
    if (!/^-?(0|[1-9][0-9]*)$/.test(context.source)) return value;
    const exact = BigInt(context.source);
    if (exact > BigInt(Number.MAX_SAFE_INTEGER) || exact < BigInt(Number.MIN_SAFE_INTEGER)) return exact;
    return value;
  });
}

/** Serialize BigInt fields as exact JSON number tokens (Node.js 22+). */
export function stringifyLosslessJson(value: unknown): string {
  if (typeof nativeJson.rawJSON !== "function") throw new Error("Node.js 22 or newer is required for lossless integer JSON");
  const text = nativeJson.stringify(value, (_key, item) => typeof item === "bigint" ? nativeJson.rawJSON!(item.toString()) : item);
  if (text === undefined) throw new TypeError("value cannot be serialized as JSON");
  return text;
}

function asI64(value: unknown, name: string, minimum = MIN_I64, maximum = MAX_I64): bigint {
  let integer: bigint;
  if (typeof value === "bigint") integer = value;
  else if (typeof value === "number" && Number.isInteger(value)) {
    if (!Number.isSafeInteger(value)) throw new RangeError(`${name} number is not exact; use bigint`);
    integer = BigInt(value);
  } else throw new TypeError(`${name} must be a bigint or integer number`);
  if (integer < minimum || integer > maximum) throw new RangeError(`${name} must fit signed 64-bit integer range`);
  return integer;
}

function factor(value: unknown): asserts value is number {
  if (typeof value !== "number" || !Number.isFinite(value) || value <= 1) {
    throw new RangeError("minimum_factor must be finite and greater than 1");
  }
}

export function defaults(): Config {
  return { pairs: 40, confidence_percent: 95, minimum_factor: 1.05, seed: 0n, target_ns: 100_000n, max_batch: 65_536n };
}

export function validateConfig(config: Config): void {
  if (config === null || typeof config !== "object") throw new TypeError("config must be an object");
  if (!Number.isInteger(config.pairs) || config.pairs < 1 || config.pairs > MAX_PAIRS) throw new RangeError("pairs must be from 1 through 10000");
  if (!Number.isInteger(config.confidence_percent) || config.confidence_percent < 1 || config.confidence_percent > 99) throw new RangeError("confidence_percent must be from 1 through 99");
  factor(config.minimum_factor);
  asI64(config.seed, "seed");
  asI64(config.target_ns, "target_ns", 1n);
  asI64(config.max_batch, "max_batch", 1n);
}

/** Analyze paired candidate/baseline latencies using VER07b thresholds. */
export function analyze(pairs: readonly Pair[], confidencePercent = 95, minimumFactor = 1.05): Analysis {
  if (!Array.isArray(pairs) || pairs.length === 0) throw new RangeError("pairs must not be empty");
  if (!Number.isInteger(confidencePercent) || confidencePercent < 1 || confidencePercent > 99) throw new RangeError("confidence_percent must be from 1 through 99");
  factor(minimumFactor);
  if (pairs.length > MAX_PAIRS) throw new RangeError(`pairs must contain at most ${MAX_PAIRS} observations`);
  const ratios = pairs.map((pair, index) => {
    if (pair === null || typeof pair !== "object" || (pair.order !== "baseline-first" && pair.order !== "candidate-first")) {
      throw new TypeError(`pair ${index} has invalid order`);
    }
    const baseline = asI64(pair.baseline_ns, "baseline_ns", 1n);
    const candidate = asI64(pair.candidate_ns, "candidate_ns", 1n);
    return Number(candidate) / Number(baseline);
  });
  const ratio = median(ratios);
  const bounds = medianInterval(ratios, confidencePercent);
  if (bounds === null) return { ratio, interval: null, verdict: "inconclusive" };
  const fastBoundary = 1 / minimumFactor;
  let verdict: Analysis["verdict"] = "inconclusive";
  if (bounds.high < fastBoundary) verdict = "faster";
  else if (bounds.low > minimumFactor) verdict = "slower";
  else if (bounds.low >= fastBoundary && bounds.high <= minimumFactor) verdict = "equivalent";
  return { ratio, interval: [bounds.low, bounds.high], verdict };
}

class SplitMix64 {
  private state: bigint;
  constructor(seed: Int64) { this.state = BigInt.asUintN(64, asI64(seed, "seed")); }
  next(): bigint {
    this.state = BigInt.asUintN(64, this.state + 0x9e3779b97f4a7c15n);
    let z = this.state;
    z = BigInt.asUintN(64, (z ^ (z >> 30n)) * 0xbf58476d1ce4e5b9n);
    z = BigInt.asUintN(64, (z ^ (z >> 27n)) * 0x94d049bb133111ebn);
    return BigInt.asUintN(64, z ^ (z >> 31n));
  }
  bounded(n: number): number {
    const bound = BigInt(n);
    const threshold = BigInt.asUintN(64, -bound) % bound;
    for (;;) { const x = this.next(); if (x >= threshold) return Number(x % bound); }
  }
}

function makeSchedule(count: number, seed: Int64): PairOrder[] {
  const schedule: PairOrder[] = [
    ...Array.from({ length: Math.ceil(count / 2) }, () => "baseline-first" as const),
    ...Array.from({ length: Math.floor(count / 2) }, () => "candidate-first" as const),
  ];
  const random = new SplitMix64(seed);
  for (let i = schedule.length - 1; i > 0; i -= 1) {
    const j = random.bounded(i + 1);
    [schedule[i], schedule[j]] = [schedule[j], schedule[i]];
  }
  return schedule;
}

/** Verify, calibrate and measure paired batches; failures return no partial report. */
export function collect(
  baseline: Batch,
  candidate: Batch,
  expectedChecksum: Int64,
  config: Config = defaults(),
  clock: Clock = monotonicClock,
): Report {
  validateConfig(config);
  if (typeof baseline !== "function" || typeof candidate !== "function" || typeof clock !== "function") throw new TypeError("baseline, candidate and clock are required functions");
  const normalizedConfig: Config = {
    ...config,
    seed: asI64(config.seed, "seed"),
    target_ns: asI64(config.target_ns, "target_ns", 1n),
    max_batch: asI64(config.max_batch, "max_batch", 1n),
  };
  const checksum = asI64(expectedChecksum, "expected_checksum");
  const verify = (fn: Batch): void => {
    const actual = asI64(fn(1n), "checksum");
    if (actual !== checksum) throw new Error("checksum mismatch");
  };
  verify(baseline);
  verify(candidate);

  let last: bigint | undefined;
  const tick = (): bigint => {
    const value = clock();
    const timestamp = asI64(value, "clock", 0n);
    if (last !== undefined && timestamp < last) throw new RangeError("clock must not regress");
    last = timestamp;
    return timestamp;
  };
  const timed = (fn: Batch, size: bigint): bigint => {
    const expected = checksum * size;
    if (expected < MIN_I64 || expected > MAX_I64) throw new RangeError("batch checksum arithmetic must fit signed 64-bit integer range");
    const start = tick();
    const actual = asI64(fn(size), "checksum");
    const end = tick();
    if (actual !== expected) throw new Error("checksum mismatch");
    return end - start;
  };

  const maximumBatch = normalizedConfig.max_batch as bigint;
  const target = normalizedConfig.target_ns as bigint;
  let batchSize = 1n;
  for (;;) {
    const left = timed(baseline, batchSize);
    const right = timed(candidate, batchSize);
    if (left >= target && right >= target) break;
    if (batchSize >= maximumBatch) throw new Error("calibration limit exhausted");
    batchSize = batchSize * 2n > maximumBatch ? maximumBatch : batchSize * 2n;
  }

  const observations: Pair[] = [];
  for (const order of makeSchedule(normalizedConfig.pairs, normalizedConfig.seed)) {
    let baselineNs: bigint;
    let candidateNs: bigint;
    if (order === "baseline-first") {
      baselineNs = timed(baseline, batchSize);
      candidateNs = timed(candidate, batchSize);
    } else {
      candidateNs = timed(candidate, batchSize);
      baselineNs = timed(baseline, batchSize);
    }
    if (baselineNs <= 0n || candidateNs <= 0n) throw new RangeError("measured durations must be positive");
    observations.push({ baseline_ns: baselineNs, candidate_ns: candidateNs, order });
  }
  return { config: normalizedConfig, batch_size: batchSize, pairs: observations, analysis: analyze(observations, normalizedConfig.confidence_percent, normalizedConfig.minimum_factor) };
}

export function monotonicClock(): bigint {
  return process.hrtime.bigint();
}

function validateUnicodeScalars(value: string, field: string): void {
  for (let index = 0; index < value.length; index += 1) {
    const unit = value.charCodeAt(index);
    if (unit >= 0xd800 && unit <= 0xdbff) {
      const next = value.charCodeAt(index + 1);
      if (!(next >= 0xdc00 && next <= 0xdfff)) throw new RangeError(`${field} contains an unpaired surrogate`);
      index += 1;
    } else if (unit >= 0xdc00 && unit <= 0xdfff) {
      throw new RangeError(`${field} contains an unpaired surrogate`);
    }
  }
}

function validateNonEmptyMetadataString(value: unknown, field: string): asserts value is string {
  if (typeof value !== "string") throw new TypeError(`${field} metadata must be a string`);
  if (value.length === 0 || value.includes("\0")) {
    throw new RangeError(`${field} metadata must be non-empty and contain no NUL`);
  }
  validateUnicodeScalars(value, field);
}

function validateWorkloadMetadata(raw: Record<string, unknown>): void {
  const group = ["workload", "invocation", "revision", "runtime", "os", "arch"];
  const present = group.filter((key) => Object.prototype.hasOwnProperty.call(raw, key));
  const hasProfile = Object.prototype.hasOwnProperty.call(raw, "profile");
  if (present.length === 0) {
    if (hasProfile) throw new TypeError("profile metadata requires the complete metadata group");
    return;
  }
  if (present.length !== group.length) {
    throw new TypeError("report metadata group must contain all six fields");
  }

  const workload = raw.workload;
  validateNonEmptyMetadataString(workload, "workload");
  if (/^[ \t\n\r\v\f]*$/.test(workload)) {
    throw new RangeError("workload must not be blank ASCII whitespace");
  }

  for (const field of ["revision", "runtime", "os", "arch"] as const) {
    validateNonEmptyMetadataString(raw[field], field);
  }
  if (!Array.isArray(raw.invocation) || raw.invocation.some((item) => typeof item !== "string")) {
    throw new TypeError("invocation metadata must be an array of strings");
  }
  for (const item of raw.invocation) {
    if (item.includes("\0")) throw new RangeError("invocation metadata must not contain NUL");
    validateUnicodeScalars(item, "invocation element");
  }
  if (hasProfile) validateNonEmptyMetadataString(raw.profile, "profile");
}

function validateRootMembers(raw: Record<string, unknown>): void {
  const allowed = new Set([
    "config", "batch_size", "pairs", "analysis", "workload", "invocation",
    "revision", "runtime", "os", "arch", "profile",
  ]);
  for (const key of Object.keys(raw)) {
    if (!allowed.has(key)) throw new TypeError(`unknown report member: ${key}`);
  }
}

/** Recompute analysis from the report's raw observations and reject tampering. */
export function replayReport(value: unknown): Report {
  if (value === null || typeof value !== "object") throw new TypeError("report must be an object");
  const raw = value as Record<string, unknown>;
  validateRootMembers(raw);
  validateWorkloadMetadata(raw);
  const config = raw.config as Config;
  validateConfig(config);
  const batchSize = asI64(raw.batch_size, "batch_size", 1n);
  const maximumBatch = asI64(config.max_batch, "max_batch", 1n);
  if (batchSize > maximumBatch) throw new RangeError("batch_size exceeds max_batch");
  if (!Array.isArray(raw.pairs) || raw.pairs.length !== config.pairs) throw new RangeError("report pair count does not match config");
  const pairs = raw.pairs as Pair[];
  const baselineFirstCount = pairs.filter((pair) => pair !== null && typeof pair === "object" && pair.order === "baseline-first").length;
  if (baselineFirstCount !== Math.ceil(config.pairs / 2)) throw new Error("report pair order is not balanced");
  const analysis = analyze(pairs, config.confidence_percent, config.minimum_factor);
  if (raw.analysis === null || typeof raw.analysis !== "object") throw new TypeError("analysis must be an object");
  const claimed = raw.analysis as Record<string, unknown>;
  const interval = claimed.interval;
  const intervalMatches = analysis.interval === null
    ? interval === null
    : Array.isArray(interval) && interval.length === 2 && interval[0] === analysis.interval[0] && interval[1] === analysis.interval[1];
  if (claimed.ratio !== analysis.ratio || claimed.verdict !== analysis.verdict || !intervalMatches) throw new Error("reported analysis does not match raw pairs");
  return { config: { ...config, seed: asI64(config.seed, "seed"), target_ns: targetValue(config.target_ns), max_batch: maximumBatch }, batch_size: batchSize, pairs: pairs.map((pair) => ({ baseline_ns: asI64(pair.baseline_ns, "baseline_ns", 1n), candidate_ns: asI64(pair.candidate_ns, "candidate_ns", 1n), order: pair.order })), analysis };
}

function targetValue(value: Int64): bigint { return asI64(value, "target_ns", 1n); }

export function replayJson(text: string): Report {
  assertUniqueObjectMembers(text);
  return replayReport(parseLosslessJson(text));
}
