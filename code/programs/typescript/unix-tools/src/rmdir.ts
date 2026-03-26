/**
 * rmdir -- remove empty directories.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `rmdir` utility in TypeScript. It
 * removes each specified directory, but only if the directory is empty.
 *
 * === How rmdir Works ===
 *
 * rmdir is the safe counterpart to `rm -r`. It refuses to delete directories
 * that contain files, preventing accidental data loss:
 *
 *     rmdir emptydir           =>   removes emptydir (if empty)
 *     rmdir notempty           =>   error: directory not empty
 *     rmdir -p a/b/c           =>   removes c, then b, then a
 *
 * === The -p Flag (Parents) ===
 *
 * With `-p`, rmdir removes each component of the path. For example,
 * `rmdir -p a/b/c` is equivalent to:
 *
 *     rmdir a/b/c
 *     rmdir a/b
 *     rmdir a
 *
 * Each directory must be empty at the time of removal. If any removal
 * fails, the remaining parent directories are not attempted.
 *
 * === --ignore-fail-on-non-empty ===
 *
 * This long flag suppresses the error message when a directory cannot be
 * removed because it is not empty. Other errors (permission denied, etc.)
 * are still reported.
 *
 * @module rmdir
 */

import * as fs from "node:fs";
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
const SPEC_FILE = path.resolve(__dirname, "..", "rmdir.json");

// ---------------------------------------------------------------------------
// Business Logic: Remove directory and optionally its parents.
// ---------------------------------------------------------------------------

/**
 * Get the parent components of a path for -p mode.
 *
 * For "a/b/c", this returns ["a/b/c", "a/b", "a"].
 * Each component is removed in order (deepest first).
 */
function getParentChain(dir: string): string[] {
  const chain: string[] = [dir];
  let current = dir;

  while (true) {
    const parent = path.dirname(current);
    // path.dirname returns "." for relative paths with no parent,
    // or "/" for the root. Either way, stop.
    if (parent === current || parent === ".") {
      break;
    }
    chain.push(parent);
    current = parent;
  }

  return chain;
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then remove directories.
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
        process.stderr.write(`rmdir: ${error.message}\n`);
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

  // --- Step 3: Extract flags and arguments ---------------------------------

  const flags = (result as { flags: Record<string, unknown> }).flags;
  const args = (result as { arguments: Record<string, unknown> }).arguments;

  const parents = !!flags["parents"];
  const verbose = !!flags["verbose"];
  const ignoreFail = !!flags["ignore_fail"];

  // Normalize directory list.
  let dirs = args["directories"] as string[] | string;
  if (typeof dirs === "string") {
    dirs = [dirs];
  }

  // --- Step 4: Remove each directory ---------------------------------------

  for (const dir of dirs) {
    const chain = parents ? getParentChain(dir) : [dir];

    for (const d of chain) {
      try {
        fs.rmdirSync(d);

        if (verbose) {
          process.stdout.write(`rmdir: removing directory, '${d}'\n`);
        }
      } catch (err: unknown) {
        // Check if this is a "not empty" error that we should ignore.
        const code = (err as { code?: string }).code;
        if (ignoreFail && code === "ENOTEMPTY") {
          continue;
        }

        const message = err instanceof Error ? err.message : String(err);
        process.stderr.write(`rmdir: ${message}\n`);
        process.exitCode = 1;
        // If removing a parent fails, stop the chain for this path.
        break;
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

main();
