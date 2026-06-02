/**
 * scripts/benchmark.ts — performance characterization for ml-framework-core
 * ============================================================================
 *
 * Runs the same 2-layer MLP from tests/end-to-end-training.test.ts at
 * increasing batch sizes and prints a markdown table of forward +
 * backward timings.
 *
 * Two intended uses:
 *
 *   1. **Find the TS-vs-Rust crossover** — at small batch sizes the
 *      pure-TS fallback wins because the JSON+hex+FFI dispatch
 *      overhead dominates.  As batches grow, Rust's f32 SIMD pulls
 *      ahead.  The table makes the crossover visible at a glance.
 *
 *   2. **Spot regressions** — re-run after any change to ops.ts /
 *      autograd.ts and compare with prior runs.  A 2-3× slowdown
 *      anywhere should be investigated.
 *
 * Usage:
 *
 *   cd code/packages/typescript/ml-framework-core
 *   npm install
 *   npm run benchmark    # or: npx tsx scripts/benchmark.ts
 *
 * No CLI arguments.  No file output.  Stdout-only.  Mirrors the Ruby
 * benchmark.rb structure 1-for-1.
 */

import { Tensor } from "../src/index.js";
import { performance } from "node:perf_hooks";

// Configurable batch sizes — picked to bracket the 10_000-cell dispatch
// threshold (the matmul intermediate is batch * 2 = 2*batch cells, so
// batch >= 5000 triggers Rust dispatch when matrix-rust-napi is built).
const BATCH_SIZES = [100, 1000, 5000, 10_000, 50_000];
const WARMUP_RUNS = 2;
const TIMED_RUNS = 5;

/**
 * Lazy-detect whether the Rust dispatch path is available.  Tries to
 * resolve `@coding-adventures/matrix-rust-napi`; if the .node addon
 * isn't built we surface that as a header note rather than crashing.
 */
function rustAvailable(): boolean {
  try {
    // Dynamic require via createRequire so this ESM script can probe a
    // CJS addon.  We don't actually call the addon — just confirm it
    // resolves and loads without throwing.
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const { createRequire } = require("node:module") as typeof import("node:module");
    const requireFn = createRequire(import.meta.url);
    requireFn("@coding-adventures/matrix-rust-napi");
    return true;
  } catch {
    return false;
  }
}

/**
 * A single forward + backward pass at the given batch size.
 * Returns [forwardMs, backwardMs].
 */
function timeStep(batchSize: number): [number, number] {
  // Build inputs: x is (batch, 1), target is (batch, 1).  Values don't
  // matter — we're measuring time, not loss.
  const xData = Array.from({ length: batchSize }, (_, i) => [i / batchSize]);
  const yData = Array.from({ length: batchSize }, (_, i) => [2 * (i / batchSize) + 3]);
  const x = new Tensor(xData);
  const target = new Tensor(yData);

  // 2-hidden-unit MLP — minimal layer count to focus on per-cell cost
  // rather than per-op orchestration overhead.
  const w1 = new Tensor([[0.5, -0.3]]);
  w1.requiresGrad = true;
  const w2 = new Tensor([[0.4], [0.7]]);
  w2.requiresGrad = true;

  // --- forward ---
  const fwdStart = performance.now();
  const pred = x.matmul(w1).relu().matmul(w2);
  const diff = pred.sub(target);
  const loss = diff.mul(diff).mean();
  const fwdEnd = performance.now();

  // --- backward ---
  const bwdStart = performance.now();
  loss.backward();
  const bwdEnd = performance.now();

  return [fwdEnd - fwdStart, bwdEnd - bwdStart];
}

/**
 * Pick the dispatch label for a given batch size.  The (batch × hidden)
 * intermediate of `x.matmul(w1).relu` is batch * 2, so 5000 batch →
 * 10000-cell intermediate → Rust kicks in.  Crude but useful enough as
 * a column label.
 */
function dispatchLabel(batchSize: number, rustOk: boolean): string {
  if (!rustOk) return "TS (no Rust)";
  return batchSize < 5_000 ? "TS" : "Rust";
}

function median(values: number[]): number {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)]!;
}

function runBenchmark(): void {
  const rustOk = rustAvailable();

  console.log("# ml-framework-core benchmark\n");
  console.log("Forward + backward pass on a 2-layer MLP (1 input → 2 hidden ReLU → 1 output)");
  console.log(`at increasing batch sizes.  Each timing is the median of ${TIMED_RUNS} runs`);
  console.log(`after ${WARMUP_RUNS} warmup runs.\n`);
  console.log(`- Node version:        ${process.version} (${process.platform}/${process.arch})`);
  console.log(`- matrix-rust-napi:    ${rustOk ? "available" : "NOT BUILT — TS fallback only"}\n`);
  console.log("| batch  | forward (ms) | backward (ms) | total (ms) | dispatch       |");
  console.log("|--------|--------------|---------------|------------|----------------|");

  type Row = {
    batch: number;
    fwdMs: number | null;
    bwdMs: number | null;
    totalMs: number | null;
    dispatch: string;
  };
  const results: Row[] = [];

  for (const batchSize of BATCH_SIZES) {
    // If matrix-rust-napi isn't available AND this batch would trigger
    // a Rust dispatch (matmul intermediate ≥ 10k cells), skip the row
    // rather than crashing with LoadError.  Same logic as Ruby
    // benchmark.rb.
    if (!rustOk && batchSize >= 5_000) {
      const dispatch = "Rust needed";
      console.log(
        `| ${String(batchSize).padStart(6)} | ${"(skipped)".padStart(12)} | ${"(skipped)".padStart(13)} | ${"(skipped)".padStart(10)} | ${dispatch.padEnd(14)} |`,
      );
      results.push({ batch: batchSize, fwdMs: null, bwdMs: null, totalMs: null, dispatch });
      continue;
    }

    // Warmup runs — let any lazy requires fire and the JIT settle.
    for (let i = 0; i < WARMUP_RUNS; i++) timeStep(batchSize);

    // Timed runs — collect all and take the median for stability.  Mean
    // is dominated by GC pauses in V8.
    const samples = Array.from({ length: TIMED_RUNS }, () => timeStep(batchSize));
    const fwdMs = median(samples.map(([f]) => f));
    const bwdMs = median(samples.map(([, b]) => b));
    const totalMs = fwdMs + bwdMs;
    const dispatch = dispatchLabel(batchSize, rustOk);

    console.log(
      `| ${String(batchSize).padStart(6)} | ${fwdMs.toFixed(2).padStart(12)} | ${bwdMs.toFixed(2).padStart(13)} | ${totalMs.toFixed(2).padStart(10)} | ${dispatch.padEnd(14)} |`,
    );
    results.push({ batch: batchSize, fwdMs, bwdMs, totalMs, dispatch });
  }

  console.log("\n## Notes\n");
  if (rustOk) {
    const rustRow = results.find((r) => r.dispatch === "Rust");
    if (rustRow) {
      console.log(`- Rust dispatch crossed in around batch=${rustRow.batch} cells per intermediate.`);
      console.log("  Below that, the pure-TS element-wise loop is faster than the");
      console.log("  JSON+hex+FFI envelope round-trip.");
    } else {
      console.log("- All batches stayed in the pure-TS path.");
    }
  } else {
    console.log("- All timings reflect the pure-TS fallback path.");
    console.log("  Build the matrix-rust-napi native addon (`cd ../matrix-rust-napi && npm run build`)");
    console.log("  to compare against Rust dispatch.");
  }
  console.log("\n## Reproducing\n");
  console.log("    cd code/packages/typescript/ml-framework-core");
  console.log("    npm install");
  console.log("    npm run benchmark");
}

runBenchmark();
