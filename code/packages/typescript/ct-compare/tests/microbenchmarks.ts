/**
 * Microbenchmarks for the ct-compare lane (VER31 F6, VER07 shape).
 *
 * Run: `node --experimental-strip-types tests/microbenchmarks.ts`. With
 * `WARRANT_BENCH_OUT=<path>` it also writes a `warrant-bench-evidence-v1`
 * manifest. Timings go to stdout only: wall-clock numbers differ on every run,
 * and the manifest must be reproducible byte for byte (the release gate
 * re-runs this script and requires the committed file to be unchanged). The
 * manifest records *which* benchmarks ran and whether each one's results were
 * correct, plus the digest of the source it measured.
 *
 * The interesting comparison is "equal" against "differ in the first byte" at
 * each size: an early-exit comparison would be much faster on the second.
 * JavaScript JITs promise nothing about timing (CT01), so the ratio is printed
 * for a human to read; it is not a pass/fail criterion.
 */
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { ctEq, ctEqU64, ctSelectBytes } from "../src/index.ts";

const PACKAGE = resolve(dirname(fileURLToPath(import.meta.url)), "..");

interface Entry {
  id: string;
  kind: "bench";
  verdict: "pass" | "fail";
}

function time(label: string, iterations: number, body: () => boolean): boolean {
  let ok = true;
  const start = process.hrtime.bigint();
  for (let i = 0; i < iterations; i += 1) ok = body() && ok;
  const ns = Number(process.hrtime.bigint() - start) / iterations;
  console.log(`${label.padEnd(34)} ${ns.toFixed(1).padStart(10)} ns/op`);
  return ok;
}

const tests: Entry[] = [];
for (const size of [16, 256, 4096]) {
  const a = new Uint8Array(size).fill(7);
  const same = a.slice();
  const early = a.slice();
  early[0] ^= 1;
  const okEqual = time(`ctEq ${size} bytes, equal`, 20_000, () => ctEq(a, same));
  const okEarly = time(`ctEq ${size} bytes, first byte differs`, 20_000, () => !ctEq(a, early));
  tests.push({ id: `bench/ctEq/${size}_bytes`, kind: "bench", verdict: okEqual && okEarly ? "pass" : "fail" });
  const okSelect = time(`ctSelectBytes ${size} bytes`, 5_000, () => ctSelectBytes(a, early, false)[0] === early[0]);
  tests.push({ id: `bench/ctSelectBytes/${size}_bytes`, kind: "bench", verdict: okSelect ? "pass" : "fail" });
}
const okU64 = time("ctEqU64", 200_000, () => ctEqU64(0xdead_beefn, 0xdead_beefn) && !ctEqU64(1n, 2n));
tests.push({ id: "bench/ctEqU64/pairs", kind: "bench", verdict: okU64 ? "pass" : "fail" });

const manifest = {
  schema: "warrant-bench-evidence-v1",
  module: "ct-compare",
  lane: "typescript",
  source_sha256: createHash("sha256").update(readFileSync(join(PACKAGE, "src", "index.ts"))).digest("hex"),
  hw_instruction_counters: "not-live",
  tests,
};
const out = process.env.WARRANT_BENCH_OUT;
if (out) writeFileSync(resolve(PACKAGE, out), `${JSON.stringify(manifest, null, 2)}\n`);
if (tests.some((t) => t.verdict !== "pass")) {
  console.error("a benchmark produced a wrong result");
  process.exit(1);
}
