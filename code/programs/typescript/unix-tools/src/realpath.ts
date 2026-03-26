/**
 * realpath -- print the resolved absolute file name.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `realpath` utility in TypeScript.
 * For each FILE, it prints the resolved absolute pathname. All symbolic
 * links, `.` and `..` components are resolved.
 *
 * === How realpath Works ===
 *
 * realpath resolves a path to its canonical form:
 *
 *     realpath .                    =>   /home/user/projects
 *     realpath ../foo               =>   /home/user/foo
 *     realpath /usr/bin/../lib      =>   /usr/lib
 *     realpath symlink              =>   /actual/target/path
 *
 * === Canonicalization Modes ===
 *
 * By default (no flags), all path components must exist and symlinks are
 * resolved. The flags modify this behavior:
 *
 * - `-e` (canonicalize-existing): All components must exist (strict).
 * - `-m` (canonicalize-missing): No component needs to exist (lenient).
 * - `-s` (no-symlinks): Don't resolve symlinks, just normalize the path.
 *
 * === Relative Output ===
 *
 * `--relative-to=DIR` prints the result relative to DIR.
 * `--relative-base=DIR` prints relative if the path starts with DIR,
 * otherwise prints the absolute path.
 *
 * === Zero-Terminated Output (-z) ===
 *
 * With `-z`, each output line ends with NUL instead of newline. This is
 * useful for piping to `xargs -0`.
 *
 * @module realpath
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
const SPEC_FILE = path.resolve(__dirname, "..", "realpath.json");

// ---------------------------------------------------------------------------
// Business Logic: Resolve paths.
// ---------------------------------------------------------------------------

/**
 * Resolve a path to its canonical form.
 *
 * The behavior depends on the mode:
 * - Default / `-e`: Use fs.realpathSync, which requires all components to
 *   exist and resolves symlinks.
 * - `-m`: Resolve what we can, normalize the rest. No component needs to exist.
 * - `-s`: Don't resolve symlinks, just normalize with path.resolve.
 */
function resolvePath(
  filePath: string,
  canonicalizeExisting: boolean,
  canonicalizeMissing: boolean,
  noSymlinks: boolean
): string {
  if (noSymlinks) {
    // Just normalize the path without resolving symlinks.
    return path.resolve(filePath);
  }

  if (canonicalizeMissing) {
    // Resolve as much as possible. For missing components, just normalize.
    // We try realpathSync first; if it fails, fall back to path.resolve.
    try {
      return fs.realpathSync(filePath);
    } catch {
      return path.resolve(filePath);
    }
  }

  // Default and -e: all components must exist.
  // fs.realpathSync throws if any component is missing.
  return fs.realpathSync(filePath);
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then resolve paths.
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
        process.stderr.write(`realpath: ${error.message}\n`);
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

  const canonicalizeExisting = !!flags["canonicalize_existing"];
  const canonicalizeMissing = !!flags["canonicalize_missing"];
  const noSymlinks = !!flags["no_symlinks"];
  const quiet = !!flags["quiet"];
  const relativeTo = flags["relative_to"] as string | undefined;
  const relativeBase = flags["relative_base"] as string | undefined;
  const zero = !!flags["zero"];

  const delimiter = zero ? "\0" : "\n";

  // Normalize file list.
  let files = args["files"] as string[] | string;
  if (typeof files === "string") {
    files = [files];
  }

  // --- Step 4: Resolve and print each path ---------------------------------

  for (const file of files) {
    try {
      let resolved = resolvePath(
        file,
        canonicalizeExisting,
        canonicalizeMissing,
        noSymlinks
      );

      // Apply relative-to or relative-base if specified.
      if (relativeTo) {
        const resolvedRelTo = resolvePath(
          relativeTo,
          canonicalizeExisting,
          canonicalizeMissing,
          noSymlinks
        );
        resolved = path.relative(resolvedRelTo, resolved);
      } else if (relativeBase) {
        const resolvedBase = resolvePath(
          relativeBase,
          canonicalizeExisting,
          canonicalizeMissing,
          noSymlinks
        );
        if (resolved.startsWith(resolvedBase)) {
          resolved = path.relative(resolvedBase, resolved);
        }
        // Otherwise, keep the absolute path.
      }

      process.stdout.write(resolved + delimiter);
    } catch (err: unknown) {
      if (!quiet) {
        const message = err instanceof Error ? err.message : String(err);
        process.stderr.write(`realpath: ${file}: ${message}\n`);
      }
      process.exitCode = 1;
    }
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

main();
