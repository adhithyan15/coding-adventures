/**
 * basename -- strip directory and suffix from filenames.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `basename` utility in TypeScript.
 * It prints NAME with any leading directory components removed. If a
 * SUFFIX is specified, it is also removed from the end.
 *
 * === How basename Works ===
 *
 * basename extracts the filename from a path:
 *
 *     basename /usr/bin/sort    =>   sort
 *     basename include/stdio.h .h    =>   stdio
 *     basename -s .h include/stdio.h  =>   stdio
 *
 * === The Algorithm ===
 *
 * 1. Remove all trailing slashes from the path.
 * 2. If the entire path was slashes, return "/".
 * 3. Remove the directory prefix (everything up to the last slash).
 * 4. If a suffix is specified and the name ends with it (and the name
 *    is not equal to the suffix), remove the suffix.
 *
 * This matches the POSIX specification for basename.
 *
 * === Multiple Mode (-a) ===
 *
 * By default, basename processes only one NAME argument (with an optional
 * second argument as the suffix). With `-a` or `-s SUFFIX`, it processes
 * all arguments as names, applying the same suffix removal to each.
 *
 * @module basename
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
const SPEC_FILE = path.resolve(__dirname, "..", "basename.json");

// ---------------------------------------------------------------------------
// Business Logic: Strip directory and suffix.
// ---------------------------------------------------------------------------

/**
 * Compute the basename of a path, optionally stripping a suffix.
 *
 * We implement this manually rather than using Node's `path.basename`
 * because the POSIX/GNU behavior has specific rules about trailing
 * slashes and the suffix that differ subtly from Node's implementation.
 *
 * === Step-by-Step ===
 *
 * 1. Strip trailing slashes: "/usr/bin/" -> "/usr/bin"
 * 2. Handle all-slashes case: "///" -> "/"
 * 3. Take everything after the last slash: "/usr/bin" -> "bin"
 * 4. Strip suffix if it matches and doesn't consume entire name.
 */
function computeBasename(pathname: string, suffix?: string): string {
  // Step 1: Remove trailing slashes.
  let stripped = pathname.replace(/\/+$/, "");

  // Step 2: If the path was entirely slashes, the basename is "/".
  if (stripped === "") {
    return "/";
  }

  // Step 3: Remove the directory prefix.
  const lastSlash = stripped.lastIndexOf("/");
  let name = lastSlash === -1 ? stripped : stripped.slice(lastSlash + 1);

  // Step 4: Remove the suffix if specified.
  // The suffix must not be the entire name (e.g., basename ".h" ".h" => ".h").
  if (suffix && suffix.length > 0 && name.endsWith(suffix) && name !== suffix) {
    name = name.slice(0, -suffix.length);
  }

  return name;
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then strip directory and suffix.
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
        process.stderr.write(`basename: ${error.message}\n`);
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

  const multiple = !!flags["multiple"];
  const suffixFlag = flags["suffix"] as string | undefined;
  const zero = !!flags["zero"];
  const terminator = zero ? "\0" : "\n";

  // Get the name arguments.
  let names = args["name"] as string[] | string;
  if (typeof names === "string") {
    names = [names];
  }

  // --- Step 4: Determine suffix and process names --------------------------
  //
  // GNU basename has two calling conventions:
  //
  //   basename NAME [SUFFIX]     -- single name, optional suffix
  //   basename -a [-s SUFFIX] NAME...  -- multiple names
  //
  // When neither -a nor -s is given and exactly two arguments are provided,
  // the second argument is treated as the suffix (traditional POSIX mode).

  let suffix: string | undefined = suffixFlag;
  let namesToProcess: string[];

  if (multiple || suffixFlag !== undefined) {
    // Multiple mode: all arguments are names.
    namesToProcess = names;
  } else if (names.length === 2) {
    // Traditional mode: second argument is the suffix.
    namesToProcess = [names[0]];
    suffix = names[1];
  } else {
    // Single name, no suffix.
    namesToProcess = [names[0]];
  }

  // --- Step 5: Output results ----------------------------------------------

  for (const name of namesToProcess) {
    process.stdout.write(computeBasename(name, suffix) + terminator);
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

main();
