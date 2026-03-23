/**
 * rm -- remove files or directories.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `rm` utility in TypeScript. It
 * removes each specified file. By default, it does not remove directories.
 *
 * === How rm Works ===
 *
 * rm is one of the most dangerous Unix commands -- it permanently deletes
 * files with no recycle bin or undo:
 *
 *     rm file.txt               =>   removes file.txt
 *     rm -r directory/           =>   removes directory and everything in it
 *     rm -f nonexistent.txt      =>   no error even if file doesn't exist
 *     rm -rf /                   =>   DO NOT DO THIS (removes everything)
 *
 * === Safety Features ===
 *
 * Without `-r`, rm refuses to remove directories. This prevents accidental
 * deletion of entire directory trees.
 *
 * Without `-f`, rm reports errors for nonexistent files. The `-f` flag
 * silently ignores missing files, which is useful in scripts.
 *
 * The `-i` flag prompts before each removal, and `-I` prompts once if
 * more than three files are being removed. These are mutually exclusive
 * with `-f`.
 *
 * === The -d Flag ===
 *
 * The `-d` flag allows removing empty directories (like rmdir). This is
 * less dangerous than `-r` because it won't delete directories that
 * contain files.
 *
 * @module rm
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
const SPEC_FILE = path.resolve(__dirname, "..", "rm.json");

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then remove files.
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
        process.stderr.write(`rm: ${error.message}\n`);
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

  const force = !!flags["force"];
  const recursive = !!flags["recursive"];
  const removeEmptyDirs = !!flags["dir"];
  const verbose = !!flags["verbose"];

  // Normalize file list.
  let files = args["files"] as string[] | string;
  if (typeof files === "string") {
    files = [files];
  }

  // --- Step 4: Remove each file/directory ----------------------------------

  for (const file of files) {
    try {
      const stat = fs.lstatSync(file);

      if (stat.isDirectory()) {
        if (recursive) {
          // Remove directory and all contents recursively.
          fs.rmSync(file, { recursive: true, force: true });
          if (verbose) {
            process.stdout.write(`removed directory '${file}'\n`);
          }
        } else if (removeEmptyDirs) {
          // Only remove if empty (like rmdir).
          fs.rmdirSync(file);
          if (verbose) {
            process.stdout.write(`removed directory '${file}'\n`);
          }
        } else {
          process.stderr.write(`rm: cannot remove '${file}': Is a directory\n`);
          process.exitCode = 1;
        }
      } else {
        // Regular file, symlink, etc.
        fs.unlinkSync(file);
        if (verbose) {
          process.stdout.write(`removed '${file}'\n`);
        }
      }
    } catch (err: unknown) {
      if (force) {
        // With -f, ignore "no such file" errors.
        const code = (err as { code?: string }).code;
        if (code === "ENOENT") {
          continue;
        }
      }

      const message = err instanceof Error ? err.message : String(err);
      process.stderr.write(`rm: cannot remove '${file}': ${message}\n`);
      process.exitCode = 1;
    }
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

main();
