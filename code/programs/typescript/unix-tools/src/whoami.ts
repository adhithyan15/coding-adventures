/**
 * whoami -- print effective user name.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the POSIX `whoami` utility in TypeScript.
 * It prints the user name associated with the current effective user ID
 * to standard output.
 *
 * === How whoami Differs from logname ===
 *
 * These two commands look similar but answer different questions:
 *
 *     whoami   =>   "Who am I running as RIGHT NOW?"
 *     logname  =>   "Who originally logged in?"
 *
 * For example, if user "alice" logs in and then runs `sudo -u bob whoami`,
 * the output is "bob" (the effective user). But `logname` would still
 * say "alice" (the login user).
 *
 * === Implementation ===
 *
 * We use Node.js's `os.userInfo().username`, which calls the POSIX
 * `getpwuid(geteuid())` function under the hood. This returns the
 * effective user name -- the same one that the C `whoami` would return.
 *
 * As a fallback, we check `process.env.USER`, which is set by most
 * shells but is less reliable (it can be overridden).
 *
 * @module whoami
 */

import * as os from "node:os";
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
const SPEC_FILE = path.resolve(__dirname, "..", "whoami.json");

// ---------------------------------------------------------------------------
// Business Logic: Get the effective username.
// ---------------------------------------------------------------------------

/**
 * Return the effective username.
 *
 * We try `os.userInfo().username` first because it queries the OS
 * directly (via getpwuid/geteuid on Unix). If that fails (e.g., on
 * some containerized environments where /etc/passwd is missing), we
 * fall back to the USER environment variable.
 *
 * @returns The effective username, or null if it cannot be determined.
 */
export function getEffectiveUsername(): string | null {
  try {
    return os.userInfo().username;
  } catch {
    // os.userInfo() can throw if the user's entry is missing from
    // /etc/passwd (common in minimal Docker containers).
    return process.env.USER ?? null;
  }
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then print the username.
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
        process.stderr.write(`whoami: ${error.message}\n`);
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

  // --- Step 3: Business logic ----------------------------------------------
  // Get and print the effective username.

  const username = getEffectiveUsername();

  if (username === null) {
    process.stderr.write("whoami: cannot find name for user ID\n");
    process.exit(1);
  }

  process.stdout.write(username + "\n");
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------
// Guard against running during tests. The VITEST env var is set by vitest.

if (!process.env.VITEST) {
  main();
}
