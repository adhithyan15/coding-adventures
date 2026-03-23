/**
 * printenv -- print environment variables.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `printenv` utility in TypeScript.
 * It prints the values of the specified environment variables. If no
 * variables are specified, it prints all environment variables as
 * NAME=VALUE pairs.
 *
 * === How printenv Works ===
 *
 *     printenv              =>   all variables, one per line (NAME=VALUE)
 *     printenv HOME         =>   just the value of $HOME
 *     printenv HOME PATH    =>   values of $HOME and $PATH, one per line
 *
 * === Exit Status ===
 *
 * printenv exits with status 0 if all specified variables are found, or
 * with status 1 if any are not found. When printing all variables (no
 * arguments), it always exits 0.
 *
 * === printenv vs env ===
 *
 * Both `printenv` and `env` can display environment variables, but they
 * differ in purpose:
 *
 * - `printenv` is for *reading* variables. It can query specific ones.
 * - `env` is for *modifying* the environment before running a command.
 *
 * When called with no arguments, both produce similar output. But
 * `printenv HOME` prints just the value, while `env` has no such option.
 *
 * === NUL Termination (-0) ===
 *
 * With `-0`, output lines are terminated with NUL instead of newline.
 * This is useful when piping to `xargs -0` to handle values containing
 * newlines (though environment variable values rarely contain newlines).
 *
 * @module printenv
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
const SPEC_FILE = path.resolve(__dirname, "..", "printenv.json");

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then print environment variables.
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
        process.stderr.write(`printenv: ${error.message}\n`);
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

  const nullTerminated = !!flags["null"];
  const terminator = nullTerminated ? "\0" : "\n";

  // Get the list of variable names to print.
  let variables = args["variables"] as string[] | string | undefined;
  if (typeof variables === "string") {
    variables = [variables];
  }

  // --- Step 4: Print environment variables ---------------------------------

  if (!variables || variables.length === 0) {
    // No specific variables requested: print all environment variables
    // as NAME=VALUE pairs, sorted alphabetically.
    const env = process.env;
    const keys = Object.keys(env).sort();

    for (const key of keys) {
      const value = env[key];
      if (value !== undefined) {
        process.stdout.write(`${key}=${value}${terminator}`);
      }
    }
  } else {
    // Specific variables requested: print just their values.
    // Exit with 1 if any variable is not found.
    let allFound = true;

    for (const varName of variables) {
      const value = process.env[varName];
      if (value !== undefined) {
        process.stdout.write(value + terminator);
      } else {
        // Variable not found. Don't print anything for it, but
        // set the exit code to 1.
        allFound = false;
      }
    }

    if (!allFound) {
      process.exitCode = 1;
    }
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

main();
