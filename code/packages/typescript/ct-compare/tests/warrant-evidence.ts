/**
 * Run the Warrant corpus against this package (VER04i, the TypeScript lane).
 *
 * This is the consumer registry's `runner_entry` for `typescript/ct-compare`.
 * It fixes the four package-specific paths and forwards the command to the
 * shared runner, `@coding-adventures/warrant-corpus`:
 *
 *     node --experimental-strip-types tests/warrant-evidence.ts run --out evidence/test-manifest.json
 *     node --experimental-strip-types tests/warrant-evidence.ts check evidence/test-manifest.json
 *     node --experimental-strip-types tests/warrant-evidence.ts sabotage
 *
 * `run --out` regenerates the committed manifest from a real execution;
 * `check` regenerates it in memory and fails if the committed file drifted;
 * `sabotage` proves a deliberately broken export makes the runner print
 * `not ok` and exit non-zero, once per negative control.
 */

import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { main } from "@coding-adventures/warrant-corpus";

const PACKAGE = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const REPO_ROOT = resolve(PACKAGE, "..", "..", "..", "..");
const CORPUS = join(REPO_ROOT, "code", "specs", "fixtures", "ct-compare-warrant-v1");

/** Forward `argv` (`<command> [options]`) with this package's paths. */
export function run(argv: readonly string[]): Promise<number> {
  const [command, ...rest] = argv;
  const common = [
    "--repo-root", REPO_ROOT,
    "--corpus", CORPUS,
    "--binding", join(PACKAGE, "evidence.bindings"),
    "--package-dir", PACKAGE,
  ]; // prettier-ignore
  return main([command ?? "", ...common, ...rest]);
}

process.exitCode = await run(process.argv.slice(2));
