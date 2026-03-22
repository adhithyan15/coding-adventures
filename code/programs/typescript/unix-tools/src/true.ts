/**
 * true -- do nothing, successfully.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the POSIX `true` utility in TypeScript.
 * It does absolutely nothing and exits with status code 0 (success).
 *
 * That may sound useless, but `true` is a fundamental building block in
 * shell scripting. It's used in:
 *
 * - Infinite loops:      `while true; do ...; done`
 * - Conditional chains:  `command || true` (suppress failure)
 * - Default commands:    placeholder in if/else branches
 *
 * === Why Does true Accept --help and --version? ===
 *
 * POSIX `true` ignores all arguments and always exits 0. GNU coreutils
 * extends this by supporting `--help` and `--version`. We follow the GNU
 * convention, which means CLI Builder handles those two flags for us.
 * Any other arguments are silently ignored (we just don't look at them).
 *
 * === The Simplest Possible CLI Builder Program ===
 *
 * This is the minimal CLI Builder program: no flags, no arguments, no
 * commands. The JSON spec defines only `--help` and `--version` via
 * `builtin_flags`. The business logic is a single `process.exit(0)`.
 *
 * @module true
 */

import * as path from "node:path";
import { fileURLToPath } from "node:url";

// ---------------------------------------------------------------------------
// Import CLI Builder.
// ---------------------------------------------------------------------------
// We import from the package's source directly, following the pattern
// established by other TypeScript programs in this monorepo. The
// `file:` dependency in package.json resolves to the local cli-builder
// package.

import { Parser } from "@coding-adventures/cli-builder";

// ---------------------------------------------------------------------------
// Locate the JSON spec file.
// ---------------------------------------------------------------------------
// In ESM, there is no `__dirname` global. Instead, we derive the current
// file's directory from `import.meta.url`, which gives us a `file://` URL.
// `fileURLToPath` converts it to a filesystem path, and `path.dirname`
// extracts the directory.
//
// The spec file lives one level up from `src/`, so `..` from this file's
// directory gets us to the project root where `true.json` lives.

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const SPEC_FILE = path.resolve(__dirname, "..", "true.json");

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then exit successfully.
// ---------------------------------------------------------------------------

/**
 * Entry point.
 *
 * The flow is:
 * 1. Create a Parser with the spec file and process.argv.
 * 2. Call parse() to get a result.
 * 3. Discriminate the result type using duck typing:
 *    - `"text" in result` => HelpResult (user passed --help)
 *    - `"version" in result` and no `"flags"` => VersionResult (--version)
 *    - Otherwise => ParseResult -- just exit 0
 * 4. For ParseResult, do nothing and exit 0. That's the whole point.
 *
 * CLI Builder throws `ParseErrors` on invalid input, but for `true` we
 * still catch and report them, matching GNU coreutils behavior where
 * `true --help` works but invalid long flags are ignored.
 */
function main(): void {
  // --- Step 1: Parse arguments ---------------------------------------------
  // Hand the spec file and process.argv to CLI Builder.

  let result;

  try {
    const parser = new Parser(SPEC_FILE, process.argv);
    result = parser.parse();
  } catch (err: unknown) {
    // For `true`, even parse errors result in exit 0.
    // GNU coreutils `true` ignores everything. But we still handle
    // --help and --version if parsing succeeds.
    process.exit(0);
  }

  // --- Step 2: Dispatch on result type -------------------------------------

  if ("text" in result) {
    // HelpResult -- user asked for --help.
    process.stdout.write(result.text + "\n");
    process.exit(0);
  }

  if ("version" in result && !("flags" in result)) {
    // VersionResult -- user asked for --version.
    process.stdout.write(result.version + "\n");
    process.exit(0);
  }

  // --- Step 3: Business logic ----------------------------------------------
  // The entire business logic of `true`: exit successfully.
  // No flags to check, no output to produce. Just success.

  process.exit(0);
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

main();
