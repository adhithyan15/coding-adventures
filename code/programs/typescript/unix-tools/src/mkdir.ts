/**
 * mkdir -- make directories.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `mkdir` utility in TypeScript. It
 * creates one or more directories. With `-p`, it creates parent directories
 * as needed and does not error if the directory already exists.
 *
 * === How mkdir Works ===
 *
 * mkdir creates the named directories in the order given:
 *
 *     mkdir dir1 dir2          =>   creates dir1 and dir2
 *     mkdir -p a/b/c           =>   creates a, a/b, and a/b/c
 *     mkdir -m 755 mydir       =>   creates mydir with mode 755
 *
 * === The -p Flag (Parents) ===
 *
 * Without `-p`, mkdir fails if:
 * - The directory already exists.
 * - A parent directory doesn't exist.
 *
 * With `-p`, mkdir:
 * - Creates all necessary parent directories.
 * - Does not complain if the directory already exists.
 *
 * This is the most commonly used flag. Scripts almost always use `mkdir -p`
 * because they can't assume the parent directories exist.
 *
 * === Mode (-m) ===
 *
 * The `-m` flag sets the permission mode for the created directory. The
 * mode is specified as an octal string (e.g., "755"). If not specified,
 * the default is determined by the process umask.
 *
 * @module mkdir
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
const SPEC_FILE = path.resolve(__dirname, "..", "mkdir.json");

// ---------------------------------------------------------------------------
// Business Logic: Parse octal mode strings.
// ---------------------------------------------------------------------------

/**
 * Parse an octal mode string like "755" into a numeric file mode.
 *
 * Octal is the traditional way Unix permissions are expressed:
 *   - 7 = rwx (read + write + execute)
 *   - 5 = r-x (read + execute)
 *   - 0 = --- (no permissions)
 *
 * The three digits represent: owner, group, others.
 * So "755" means: owner can do everything, group and others can read/execute.
 */
function parseMode(modeStr: string): number {
  const mode = parseInt(modeStr, 8);
  if (isNaN(mode)) {
    throw new Error(`invalid mode: '${modeStr}'`);
  }
  return mode;
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then create directories.
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
        process.stderr.write(`mkdir: ${error.message}\n`);
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
  const modeStr = flags["mode"] as string | undefined;
  const mode = modeStr ? parseMode(modeStr) : undefined;

  // Normalize directory list.
  let dirs = args["directories"] as string[] | string;
  if (typeof dirs === "string") {
    dirs = [dirs];
  }

  // --- Step 4: Create each directory ---------------------------------------

  for (const dir of dirs) {
    try {
      if (parents) {
        // recursive: true creates parent dirs and doesn't error on existing.
        fs.mkdirSync(dir, { recursive: true, mode });
      } else {
        fs.mkdirSync(dir, { mode });
      }

      if (verbose) {
        process.stdout.write(`mkdir: created directory '${dir}'\n`);
      }
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      process.stderr.write(`mkdir: ${message}\n`);
      process.exitCode = 1;
    }
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

main();
