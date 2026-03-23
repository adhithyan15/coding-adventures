/**
 * dirname -- strip last component from file name.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `dirname` utility in TypeScript.
 * It outputs each NAME with its last non-slash component and trailing
 * slashes removed. If NAME contains no slashes, it outputs "." (the
 * current directory).
 *
 * === How dirname Works ===
 *
 * dirname extracts the directory portion of a path:
 *
 *     dirname /usr/bin/sort    =>   /usr/bin
 *     dirname stdio.h          =>   .
 *     dirname /usr/             =>   /
 *     dirname /                 =>   /
 *
 * === The Algorithm (POSIX Specification) ===
 *
 * The POSIX algorithm for dirname is:
 *
 * 1. If the string is "//", skip to step 5. (Some systems treat "//"
 *    as special, but we treat it as "/").
 * 2. Remove trailing slashes.
 * 3. If there are no slashes remaining, return ".".
 * 4. Remove everything after the last slash.
 * 5. Remove trailing slashes (again).
 * 6. If the string is empty, return "/".
 *
 * @module dirname
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
const __dirname_resolved = path.dirname(fileURLToPath(import.meta.url));
const SPEC_FILE = path.resolve(__dirname_resolved, "..", "dirname.json");

// ---------------------------------------------------------------------------
// Business Logic: Extract the directory portion of a path.
// ---------------------------------------------------------------------------

/**
 * Compute the dirname of a path.
 *
 * We implement this according to the POSIX specification rather than
 * using Node's `path.dirname`, because the POSIX behavior has specific
 * edge cases that we want to match exactly.
 *
 * === Examples and Edge Cases ===
 *
 *     Input          Output     Reason
 *     -----          ------     ------
 *     /usr/bin       /usr       Normal case
 *     /usr/          /          Trailing slash removed, then dirname
 *     usr            .          No slashes => current directory
 *     /              /          Root directory
 *     .              .          Current directory
 *     ..             .          Parent directory (dirname is cwd)
 *     (empty)        .          Empty string => current directory
 */
function computeDirname(pathname: string): string {
  // Handle empty string.
  if (pathname === "") {
    return ".";
  }

  // Step 1: Remove trailing slashes (but don't remove all of them if
  // the path is entirely slashes).
  let stripped = pathname.replace(/\/+$/, "");

  // If removing trailing slashes left us empty, the path was all slashes.
  // Return "/".
  if (stripped === "") {
    return "/";
  }

  // Step 2: Find the last slash.
  const lastSlash = stripped.lastIndexOf("/");

  // Step 3: If no slash found, the entire path is a filename in the
  // current directory, so return ".".
  if (lastSlash === -1) {
    return ".";
  }

  // Step 4: Remove everything after the last slash.
  let dir = stripped.slice(0, lastSlash);

  // Step 5: Remove trailing slashes from the result.
  dir = dir.replace(/\/+$/, "");

  // Step 6: If we've removed everything, we were at the root.
  if (dir === "") {
    return "/";
  }

  return dir;
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then strip last component.
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
        process.stderr.write(`dirname: ${error.message}\n`);
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

  const zero = !!flags["zero"];
  const terminator = zero ? "\0" : "\n";

  let names = args["names"] as string[] | string;
  if (typeof names === "string") {
    names = [names];
  }

  // --- Step 4: Output results ----------------------------------------------

  for (const name of names) {
    process.stdout.write(computeDirname(name) + terminator);
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

main();
