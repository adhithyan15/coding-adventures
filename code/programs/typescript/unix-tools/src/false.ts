/**
 * false -- do nothing, unsuccessfully.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the POSIX `false` utility in TypeScript.
 * It does absolutely nothing and exits with status code 1 (failure).
 *
 * `false` is the counterpart to `true`. Where `true` always succeeds,
 * `false` always fails. It's used in shell scripting for:
 *
 * - Breaking loops:    `while false; do ...; done` (never executes)
 * - Testing:           `if false; then ...; fi` (never true)
 * - Conditional chains: `command && false` (force failure)
 *
 * === The Mirror Image of true ===
 *
 * This program is structurally identical to `true.ts`. The only
 * difference is the exit code: 1 instead of 0. Both programs support
 * `--help` and `--version` via CLI Builder's builtin flags, and both
 * ignore all other arguments.
 *
 * This symmetry is a nice property: `true` and `false` are the boolean
 * constants of shell scripting, and their implementations reflect that
 * simplicity.
 *
 * @module false
 */

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
const SPEC_FILE = path.resolve(__dirname, "..", "false.json");

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then exit unsuccessfully.
// ---------------------------------------------------------------------------

/**
 * Entry point.
 *
 * Identical structure to `true.ts`, but exits with code 1 instead of 0.
 * The --help and --version flags still exit 0 (they succeeded at their
 * task of showing help/version info), matching GNU coreutils behavior.
 */
function main(): void {
  // --- Step 1: Parse arguments ---------------------------------------------

  let result;

  try {
    const parser = new Parser(SPEC_FILE, process.argv);
    result = parser.parse();
  } catch (err: unknown) {
    // For `false`, even parse errors result in exit 1.
    // GNU coreutils `false` always fails, regardless of arguments.
    process.exit(1);
  }

  // --- Step 2: Dispatch on result type -------------------------------------

  if ("text" in result) {
    // HelpResult -- user asked for --help.
    // Note: --help exits 0 even for `false`. The help request succeeded.
    process.stdout.write(result.text + "\n");
    process.exit(0);
  }

  if ("version" in result && !("flags" in result)) {
    // VersionResult -- user asked for --version.
    // Same reasoning: the version request succeeded, so exit 0.
    process.stdout.write(result.version + "\n");
    process.exit(0);
  }

  // --- Step 3: Business logic ----------------------------------------------
  // The entire business logic of `false`: exit unsuccessfully.
  // The mirror image of `true` -- always returns failure.

  process.exit(1);
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

main();
