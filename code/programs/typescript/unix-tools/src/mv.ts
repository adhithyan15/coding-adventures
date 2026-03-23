/**
 * mv -- move (rename) files and directories.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `mv` utility in TypeScript.
 * It moves or renames files and directories. Moving is conceptually
 * "rename in place" when source and destination are on the same
 * filesystem, or "copy + delete" when they're on different filesystems.
 *
 * === How mv Works ===
 *
 *     mv old.txt new.txt        =>  rename old.txt to new.txt
 *     mv a.txt b.txt dir/       =>  move both files into dir/
 *     mv olddir/ newdir/        =>  rename directory
 *
 * === rename vs copy+delete ===
 *
 * Under the hood, `mv` tries `rename(2)` first. This is an atomic
 * operation that just updates the directory entry -- the file data
 * doesn't move at all. It's instant regardless of file size.
 *
 * But `rename(2)` fails across filesystem boundaries (EXDEV error).
 * In that case, mv falls back to:
 *   1. Copy the file to the destination.
 *   2. Delete the original.
 *
 * In Node.js, `fs.renameSync` wraps `rename(2)`. When it throws
 * EXDEV, we use `fs.copyFileSync` + `fs.unlinkSync` as the fallback.
 *
 * === Overwrite Semantics ===
 *
 *     Flag        Existing file?    Action
 *     ----        --------------    ------
 *     (none)      yes               Overwrite silently
 *     -f          yes               Force overwrite (same as default)
 *     -i          yes               Prompt user (interactive)
 *     -n          yes               Skip (do nothing)
 *
 * @module mv
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
const SPEC_FILE = path.resolve(__dirname, "..", "mv.json");

// ---------------------------------------------------------------------------
// Types: Options that control mv's behavior.
// ---------------------------------------------------------------------------

/**
 * Configuration options for move operations.
 */
export interface MoveOptions {
  /** Force overwrite without prompting (-f). */
  force: boolean;
  /** Never overwrite an existing file (-n). */
  noClobber: boolean;
  /** Print each file as it is moved (-v). */
  verbose: boolean;
  /** Only move when source is newer than destination (-u). */
  update: boolean;
}

// ---------------------------------------------------------------------------
// Business Logic: Move a single file.
// ---------------------------------------------------------------------------

/**
 * Move (rename) a file or directory from `src` to `dst`.
 *
 * This function attempts `fs.renameSync` first for an atomic rename.
 * If that fails with EXDEV (cross-device), it falls back to copy +
 * delete. For directories, the fallback uses `fs.cpSync` + `fs.rmSync`.
 *
 * @param src     The source path.
 * @param dst     The destination path.
 * @param opts    Move options.
 * @returns       Array of verbose messages (empty if not verbose).
 * @throws        If the source doesn't exist or move fails.
 *
 * @example
 * ```ts
 * moveFile("/tmp/a.txt", "/tmp/b.txt", { force: false, noClobber: false, verbose: true, update: false });
 * // => ["renamed '/tmp/a.txt' -> '/tmp/b.txt'"]
 * ```
 */
export function moveFile(
  src: string,
  dst: string,
  opts: MoveOptions
): string[] {
  const messages: string[] = [];

  // --- Step 1: Validate source exists ------------------------------------

  if (!fs.existsSync(src)) {
    throw new Error(
      `mv: cannot stat '${src}': No such file or directory`
    );
  }

  // --- Step 2: Determine the final destination path ----------------------
  // If dst is an existing directory, move INTO it.

  let finalDst = dst;
  if (fs.existsSync(dst) && fs.statSync(dst).isDirectory()) {
    finalDst = path.join(dst, path.basename(src));
  }

  // --- Step 3: No-clobber check ------------------------------------------
  // If -n is set and destination exists, do nothing.

  if (opts.noClobber && fs.existsSync(finalDst)) {
    return messages;
  }

  // --- Step 4: Update check ----------------------------------------------
  // If -u is set, only move when source is newer than destination.

  if (opts.update && fs.existsSync(finalDst)) {
    const srcMtime = fs.statSync(src).mtimeMs;
    const dstMtime = fs.statSync(finalDst).mtimeMs;
    if (srcMtime <= dstMtime) {
      return messages;
    }
  }

  // --- Step 5: Attempt the rename ----------------------------------------

  try {
    fs.renameSync(src, finalDst);
  } catch (err: unknown) {
    // Cross-device rename fails with EXDEV. Fall back to copy + delete.
    if (err && typeof err === "object" && "code" in err && (err as NodeJS.ErrnoException).code === "EXDEV") {
      const srcStat = fs.statSync(src);

      if (srcStat.isDirectory()) {
        fs.cpSync(src, finalDst, { recursive: true });
        fs.rmSync(src, { recursive: true, force: true });
      } else {
        fs.copyFileSync(src, finalDst);
        fs.unlinkSync(src);
      }
    } else {
      throw err;
    }
  }

  if (opts.verbose) {
    messages.push(`renamed '${src}' -> '${finalDst}'`);
  }

  return messages;
}

// ---------------------------------------------------------------------------
// Business Logic: Move multiple files to a directory.
// ---------------------------------------------------------------------------

/**
 * Move multiple source files into a target directory.
 *
 * @param sources    Array of source paths.
 * @param targetDir  The target directory.
 * @param opts       Move options.
 * @returns          Array of verbose messages.
 */
export function moveMultiple(
  sources: string[],
  targetDir: string,
  opts: MoveOptions
): string[] {
  if (!fs.existsSync(targetDir)) {
    throw new Error(
      `mv: target '${targetDir}' is not a directory`
    );
  }

  if (!fs.statSync(targetDir).isDirectory()) {
    throw new Error(
      `mv: target '${targetDir}' is not a directory`
    );
  }

  const allMessages: string[] = [];

  for (const src of sources) {
    const msgs = moveFile(src, targetDir, opts);
    allMessages.push(...msgs);
  }

  return allMessages;
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then move files.
// ---------------------------------------------------------------------------

function main(): void {
  let result;

  try {
    const parser = new Parser(SPEC_FILE, process.argv);
    result = parser.parse();
  } catch (err: unknown) {
    if (err && typeof err === "object" && "errors" in err) {
      const errors = (err as { errors: Array<{ message: string }> }).errors;
      for (const error of errors) {
        process.stderr.write(`mv: ${error.message}\n`);
      }
      process.exit(1);
    }
    throw err;
  }

  if ("text" in result) {
    process.stdout.write(result.text + "\n");
    process.exit(0);
  }

  if ("version" in result && !("flags" in result)) {
    process.stdout.write(result.version + "\n");
    process.exit(0);
  }

  const flags = result.flags || {};
  const args: string[] = result.args?.sources || [];

  const opts: MoveOptions = {
    force: flags.force || false,
    noClobber: flags.no_clobber || false,
    verbose: flags.verbose || false,
    update: flags.update || false,
  };

  if (args.length < 2) {
    process.stderr.write("mv: missing destination file operand\n");
    process.exit(1);
  }

  const sources = args.slice(0, -1);
  const destination = args[args.length - 1];

  try {
    let messages: string[];

    if (sources.length === 1) {
      messages = moveFile(sources[0], destination, opts);
    } else {
      messages = moveMultiple(sources, destination, opts);
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
