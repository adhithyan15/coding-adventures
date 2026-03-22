/**
 * ln -- make links between files.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `ln` utility in TypeScript. It
 * creates links between files -- either hard links (default) or symbolic
 * links (with -s).
 *
 * === Hard Links vs Symbolic Links ===
 *
 * A **hard link** is another directory entry pointing to the same inode
 * (the same data on disk). Both names are equal -- deleting one doesn't
 * affect the other. Hard links cannot span filesystems and cannot link
 * to directories.
 *
 * A **symbolic link** (symlink) is a special file that contains a path
 * to another file. It's like a shortcut. The symlink can point anywhere,
 * even to files on other filesystems or to files that don't exist yet.
 *
 *     ln target link           =>   hard link: link -> same inode as target
 *     ln -s target link        =>   symlink: link -> "target" (a path string)
 *
 * === Usage Patterns ===
 *
 * Two-argument form:
 *     ln TARGET LINK_NAME      =>   create LINK_NAME pointing to TARGET
 *
 * One-argument form:
 *     ln TARGET                =>   create link in current dir with same name
 *
 * Multiple targets with directory:
 *     ln TARGET... DIRECTORY   =>   create links in DIRECTORY for each TARGET
 *
 * @module ln
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
const SPEC_FILE = path.resolve(__dirname, "..", "ln.json");

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then create links.
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
        process.stderr.write(`ln: ${error.message}\n`);
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

  const symbolic = !!flags["symbolic"];
  const force = !!flags["force"];
  const verbose = !!flags["verbose"];
  const noDereference = !!flags["no_dereference"];

  // Normalize targets list.
  let targets = args["targets"] as string[] | string;
  if (typeof targets === "string") {
    targets = [targets];
  }

  // --- Step 4: Determine link creation mode --------------------------------
  //
  // ln has three usage patterns:
  //   1. ln TARGET                    => link basename(TARGET) in current dir
  //   2. ln TARGET LINK_NAME          => create LINK_NAME -> TARGET
  //   3. ln TARGET... DIRECTORY       => create links in DIRECTORY
  //
  // We detect the mode by checking if the last argument is a directory.

  if (targets.length === 1) {
    // One-argument form: create a link with the same basename in cwd.
    const target = targets[0];
    const linkName = path.basename(target);
    createLink(target, linkName, symbolic, force, verbose, noDereference);
  } else if (targets.length === 2) {
    // Check if the last argument is an existing directory.
    const last = targets[targets.length - 1];
    let isDir = false;
    try {
      isDir = fs.statSync(last).isDirectory();
    } catch {
      // Not a directory or doesn't exist.
    }

    if (isDir) {
      // Create link inside the directory.
      const target = targets[0];
      const linkName = path.join(last, path.basename(target));
      createLink(target, linkName, symbolic, force, verbose, noDereference);
    } else {
      // Two-argument form: target and link name.
      createLink(targets[0], targets[1], symbolic, force, verbose, noDereference);
    }
  } else {
    // Multiple targets: last must be a directory.
    const dir = targets[targets.length - 1];
    let isDir = false;
    try {
      isDir = fs.statSync(dir).isDirectory();
    } catch {
      // Not a directory.
    }

    if (!isDir) {
      process.stderr.write(`ln: target '${dir}' is not a directory\n`);
      process.exit(1);
    }

    for (let i = 0; i < targets.length - 1; i++) {
      const target = targets[i];
      const linkName = path.join(dir, path.basename(target));
      createLink(target, linkName, symbolic, force, verbose, noDereference);
    }
  }
}

// ---------------------------------------------------------------------------
// Business Logic: Create a single link.
// ---------------------------------------------------------------------------

/**
 * Create a single link (hard or symbolic).
 *
 * If `force` is true, we remove the destination first if it exists.
 * If `symbolic` is true, we create a symbolic link; otherwise a hard link.
 *
 * The `noDereference` flag affects how we treat existing symlinks at the
 * destination: if the destination is a symlink to a directory, normally
 * ln would create the link inside that directory. With -n, it treats the
 * symlink as a regular file and replaces it.
 */
function createLink(
  target: string,
  linkName: string,
  symbolic: boolean,
  force: boolean,
  verbose: boolean,
  noDereference: boolean
): void {
  // Handle the case where linkName is a directory (or symlink to directory).
  let finalLinkName = linkName;
  try {
    const stat = noDereference
      ? fs.lstatSync(linkName)
      : fs.statSync(linkName);
    if (stat.isDirectory()) {
      finalLinkName = path.join(linkName, path.basename(target));
    }
  } catch {
    // Doesn't exist -- that's fine.
  }

  // Remove existing file if force is set.
  if (force) {
    try {
      fs.unlinkSync(finalLinkName);
    } catch {
      // Ignore errors -- file might not exist.
    }
  }

  try {
    if (symbolic) {
      fs.symlinkSync(target, finalLinkName);
    } else {
      fs.linkSync(target, finalLinkName);
    }

    if (verbose) {
      const arrow = symbolic ? " -> " : " => ";
      process.stdout.write(`'${finalLinkName}'${arrow}'${target}'\n`);
    }
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : String(err);
    process.stderr.write(`ln: ${message}\n`);
    process.exitCode = 1;
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

main();
