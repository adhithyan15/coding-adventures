/**
 * nproc -- print the number of processing units available.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `nproc` utility in TypeScript.
 * It prints the number of processing units (CPU cores) available to the
 * current process.
 *
 * === Why nproc Matters ===
 *
 * Many build tools and parallel processing utilities need to know how
 * many cores are available. `nproc` provides this information in a
 * portable way:
 *
 *     make -j$(nproc)          # Use all available cores for building
 *     parallel -j$(nproc)      # Run that many jobs in parallel
 *
 * === --all vs Default ===
 *
 * By default, nproc reports the number of CPUs *available* to the
 * current process. With `--all`, it reports the number of CPUs
 * *installed* in the system. These can differ when:
 *
 * - The process is running in a cgroup with CPU limits (containers)
 * - CPU affinity has been set with `taskset` or `numactl`
 * - Some CPUs are offline
 *
 * In Node.js, `os.cpus().length` returns the number of logical CPUs
 * available, which we use for both modes (Node.js doesn't distinguish
 * between available and installed).
 *
 * === --ignore N ===
 *
 * The `--ignore N` flag subtracts N from the count. This is useful
 * when you want to leave some cores free:
 *
 *     make -j$(nproc --ignore=1)    # Leave one core for the OS
 *
 * The result is clamped to a minimum of 1 -- you always get at least
 * one processing unit.
 *
 * @module nproc
 */

import * as os from "node:os";
import * as path from "node:path";
import { fileURLToPath } from "node:url";

// ---------------------------------------------------------------------------
// Import CLI Builder.
// ---------------------------------------------------------------------------

import { Parser } from "@coding-adventures/cli-builder";

// ---------------------------------------------------------------------------
// Locate the JSON spec file.
// ---------------------------------------------------------------------------

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const SPEC_FILE = path.resolve(__dirname, "..", "nproc.json");

// ---------------------------------------------------------------------------
// Business Logic: Count processing units.
// ---------------------------------------------------------------------------

/**
 * Return the number of available processing units.
 *
 * Node.js's `os.cpus()` returns an array of objects describing each
 * logical CPU/core. The length of this array is the CPU count.
 *
 * @param ignore Number of CPUs to subtract from the count.
 * @returns The number of available CPUs minus `ignore`, but at least 1.
 *
 * === Why clamp to 1? ===
 *
 * GNU nproc guarantees at least 1 in the output. If you run
 * `nproc --ignore=9999` on a 4-core machine, you get 1, not -9995.
 * This prevents tools from trying to run with 0 or negative parallelism.
 */
export function getProcessorCount(ignore: number = 0): number {
  const total = os.cpus().length;
  return Math.max(1, total - ignore);
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then print the CPU count.
// ---------------------------------------------------------------------------

function main(): void {
  // --- Step 1: Parse arguments ---------------------------------------------

  let result;

  try {
    const parser = new Parser(SPEC_FILE, process.argv);
    result = parser.parse();
  } catch (err: unknown) {
    if (err && typeof err === "object" && "errors" in err) {
      const errors = (err as { errors: Array<{ message: string }> }).errors;
      for (const error of errors) {
        process.stderr.write(`nproc: ${error.message}\n`);
      }
      process.exit(1);
    }
    throw err;
  }

  // --- Step 2: Dispatch on result type -------------------------------------

  if ("text" in result) {
    process.stdout.write(result.text + "\n");
    process.exit(0);
  }

  if ("version" in result && !("flags" in result)) {
    process.stdout.write(result.version + "\n");
    process.exit(0);
  }

  // --- Step 3: Business logic ----------------------------------------------

  const flags = (result as { flags: Record<string, unknown> }).flags;
  const ignore = (flags["ignore"] as number) ?? 0;

  // Note: --all doesn't change behavior in Node.js since os.cpus()
  // always returns all logical CPUs. We accept the flag for GNU
  // compatibility but the result is the same.

  const count = getProcessorCount(ignore);
  process.stdout.write(count + "\n");
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------
// Guard against running during tests. The VITEST env var is set by vitest.

if (!process.env.VITEST) {
  main();
}
