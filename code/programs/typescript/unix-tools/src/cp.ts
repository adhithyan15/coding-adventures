/**
 * cp -- copy files and directories.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `cp` utility in TypeScript.
 * It copies one or more source files or directories to a destination.
 *
 * === How cp Works ===
 *
 * The basic operation is straightforward: read bytes from a source
 * file and write them to a destination file.
 *
 *     cp src.txt dst.txt        =>  copy src.txt to dst.txt
 *     cp a.txt b.txt dir/       =>  copy both files into dir/
 *     cp -r srcdir/ dstdir/     =>  copy entire directory tree
 *
 * === Overwrite Semantics ===
 *
 * By default, cp silently overwrites existing files. Three flags
 * modify this behavior, and they are mutually exclusive:
 *
 *     -f (--force):       Remove destination if it can't be opened, then retry.
 *     -i (--interactive): Prompt the user before overwriting (we skip in lib).
 *     -n (--no-clobber):  Never overwrite an existing file.
 *
 *     Flag        Existing file?    Action
 *     ----        --------------    ------
 *     (none)      yes               Overwrite silently
 *     -f          yes               Remove, then write
 *     -i          yes               Prompt user
 *     -n          yes               Skip (do nothing)
 *
 * === Recursive Copying ===
 *
 * Without -R (--recursive), cp refuses to copy directories. With -R,
 * it descends into directories and recreates the entire tree at the
 * destination. Node.js 16.7+ provides `fs.cpSync` which handles this
 * natively.
 *
 * === Verbose Mode ===
 *
 * With -v (--verbose), cp prints each file copied:
 *
 *     'src.txt' -> 'dst.txt'
 *
 * @module cp
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
const SPEC_FILE = path.resolve(__dirname, "..", "cp.json");

// ---------------------------------------------------------------------------
// Types: Options that control cp's behavior.
// ---------------------------------------------------------------------------

/**
 * Configuration options for copy operations.
 *
 * These correspond to the common cp flags. The `force`, `noClobber`,
 * and `interactive` flags are mutually exclusive in practice -- when
 * more than one is given, the last one wins (GNU behavior), but our
 * CLI spec enforces mutual exclusivity at parse time.
 */
export interface CopyOptions {
  /** Remove destination and retry if it can't be opened (-f). */
  force: boolean;
  /** Never overwrite an existing file (-n). */
  noClobber: boolean;
  /** Copy directories recursively (-R). */
  recursive: boolean;
  /** Print each file as it is copied (-v). */
  verbose: boolean;
}

// ---------------------------------------------------------------------------
// Business Logic: Copy a single file.
// ---------------------------------------------------------------------------

/**
 * Copy a single file from `src` to `dst`.
 *
 * This function handles:
 * - Regular file-to-file copy.
 * - No-clobber mode: skip if destination exists.
 * - Force mode: remove existing destination before copying.
 * - Recursive mode: copy entire directory trees.
 * - Verbose mode: return a description of what was done.
 *
 * The function returns an array of verbose messages (empty if verbose
 * is false). This lets the caller decide how to display them, and
 * makes the function easy to test.
 *
 * @param src     The source path (file or directory).
 * @param dst     The destination path.
 * @param opts    Copy options controlling behavior.
 * @returns       Array of verbose messages (empty if not verbose).
 * @throws        If the source doesn't exist or copy fails.
 *
 * @example
 * ```ts
 * copyFile("/tmp/a.txt", "/tmp/b.txt", { force: false, noClobber: false, recursive: false, verbose: true });
 * // => ["'/tmp/a.txt' -> '/tmp/b.txt'"]
 * ```
 */
export function copyFile(
  src: string,
  dst: string,
  opts: CopyOptions
): string[] {
  const messages: string[] = [];

  // --- Step 1: Validate source exists ------------------------------------

  if (!fs.existsSync(src)) {
    throw new Error(`cp: cannot stat '${src}': No such file or directory`);
  }

  // --- Step 2: Determine the final destination path ----------------------
  // If dst is an existing directory, copy INTO it (like `cp a.txt dir/`).

  let finalDst = dst;
  if (fs.existsSync(dst) && fs.statSync(dst).isDirectory()) {
    finalDst = path.join(dst, path.basename(src));
  }

  // --- Step 3: Check if source is a directory ----------------------------
  // Directories require -R (--recursive).

  const srcStat = fs.statSync(src);

  if (srcStat.isDirectory()) {
    if (!opts.recursive) {
      throw new Error(
        `cp: -r not specified; omitting directory '${src}'`
      );
    }

    // No-clobber check: if the destination already exists as a file,
    // skip in no-clobber mode.
    if (opts.noClobber && fs.existsSync(finalDst)) {
      return messages;
    }

    // Force mode: remove existing destination before copying.
    if (opts.force && fs.existsSync(finalDst)) {
      fs.rmSync(finalDst, { recursive: true, force: true });
    }

    // Use fs.cpSync for recursive directory copy (Node 16.7+).
    fs.cpSync(src, finalDst, { recursive: true });

    if (opts.verbose) {
      messages.push(`'${src}' -> '${finalDst}'`);
    }

    return messages;
  }

  // --- Step 4: Handle regular file copy ----------------------------------

  // No-clobber: skip if destination already exists.
  if (opts.noClobber && fs.existsSync(finalDst)) {
    return messages;
  }

  // Force: remove existing destination before copying.
  if (opts.force && fs.existsSync(finalDst)) {
    fs.unlinkSync(finalDst);
  }

  // Copy the file. `fs.copyFileSync` is the simplest and fastest
  // approach for single files -- it uses OS-level copy-on-write
  // where available.
  fs.copyFileSync(src, finalDst);

  if (opts.verbose) {
    messages.push(`'${src}' -> '${finalDst}'`);
  }

  return messages;
}

// ---------------------------------------------------------------------------
// Business Logic: Copy multiple files to a directory.
// ---------------------------------------------------------------------------

/**
 * Copy multiple source files into a target directory.
 *
 * When cp is invoked as `cp a.txt b.txt dir/`, the last argument is
 * the directory and all preceding arguments are sources. This function
 * handles that multi-source case.
 *
 * @param sources   Array of source paths.
 * @param targetDir The target directory.
 * @param opts      Copy options.
 * @returns         Array of verbose messages.
 */
export function copyMultiple(
  sources: string[],
  targetDir: string,
  opts: CopyOptions
): string[] {
  // The target must exist and be a directory.
  if (!fs.existsSync(targetDir)) {
    throw new Error(
      `cp: target '${targetDir}' is not a directory`
    );
  }

  if (!fs.statSync(targetDir).isDirectory()) {
    throw new Error(
      `cp: target '${targetDir}' is not a directory`
    );
  }

  const allMessages: string[] = [];

  for (const src of sources) {
    const msgs = copyFile(src, targetDir, opts);
    allMessages.push(...msgs);
  }

  return allMessages;
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then copy files.
// ---------------------------------------------------------------------------

function main(): void {
  // --- Step 1: Parse arguments -------------------------------------------

  let result;

  try {
    const parser = new Parser(SPEC_FILE, process.argv);
    result = parser.parse();
  } catch (err: unknown) {
    if (err && typeof err === "object" && "errors" in err) {
      const errors = (err as { errors: Array<{ message: string }> }).errors;
      for (const error of errors) {
        process.stderr.write(`cp: ${error.message}\n`);
      }
      process.exit(1);
    }
    throw err;
  }

  // --- Step 2: Dispatch on result type -----------------------------------

  if ("text" in result) {
    process.stdout.write(result.text + "\n");
    process.exit(0);
  }

  if ("version" in result && !("flags" in result)) {
    process.stdout.write(result.version + "\n");
    process.exit(0);
  }

  // --- Step 3: Extract flags and arguments -------------------------------

  const flags = result.flags || {};
  const args: string[] = result.args?.sources || [];

  const opts: CopyOptions = {
    force: flags.force || false,
    noClobber: flags.no_clobber || false,
    recursive: flags.recursive || false,
    verbose: flags.verbose || false,
  };

  // --- Step 4: Copy files ------------------------------------------------
  // The last argument is the destination, all others are sources.

  if (args.length < 2) {
    process.stderr.write("cp: missing destination file operand\n");
    process.exit(1);
  }

  const sources = args.slice(0, -1);
  const destination = args[args.length - 1];

  try {
    let messages: string[];

    if (sources.length === 1) {
      messages = copyFile(sources[0], destination, opts);
    } else {
      messages = copyMultiple(sources, destination, opts);
    }

    for (const msg of messages) {
      process.stdout.write(msg + "\n");
    }
  } catch (err: unknown) {
    if (err instanceof Error) {
      process.stderr.write(err.message + "\n");
    }
    process.exit(1);
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

if (!process.env.VITEST) {
  main();
}
