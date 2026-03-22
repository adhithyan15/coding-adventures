/**
 * logname -- print the user's login name.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the POSIX `logname` utility in TypeScript.
 * It prints the name of the user who originally logged in to this session.
 *
 * === How logname Differs from whoami ===
 *
 * The key distinction is about identity persistence across privilege changes:
 *
 *     Command    Question                     Source
 *     -------    --------                     ------
 *     whoami     "Who am I now?"              Effective user ID (euid)
 *     logname    "Who logged in originally?"  Login records (utmp/LOGNAME)
 *
 * Consider this scenario:
 *
 *     $ ssh alice@server        # Alice logs in
 *     $ logname                 # => "alice"
 *     $ whoami                  # => "alice"
 *     $ sudo -u bob bash
 *     $ logname                 # => "alice" (still the login user)
 *     $ whoami                  # => "bob"   (now running as bob)
 *
 * === Implementation ===
 *
 * On POSIX systems, the login name comes from `getlogin()` or the utmp
 * database. In Node.js, the closest equivalent is `process.env.LOGNAME`,
 * which is set by the login process. We fall back to `process.env.USER`
 * if LOGNAME is not available.
 *
 * @module logname
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
const SPEC_FILE = path.resolve(__dirname, "..", "logname.json");

// ---------------------------------------------------------------------------
// Business Logic: Get the login name.
// ---------------------------------------------------------------------------

/**
 * Return the login name of the current user.
 *
 * We check `LOGNAME` first because it specifically represents the login
 * name, as opposed to the current effective user. The `USER` variable
 * is a fallback -- on most systems it equals LOGNAME, but after `su` or
 * `sudo`, USER may change while LOGNAME stays the same.
 *
 * @returns The login name, or null if it cannot be determined.
 */
export function getLoginName(): string | null {
  return process.env.LOGNAME ?? process.env.USER ?? null;
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then print the login name.
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
        process.stderr.write(`logname: ${error.message}\n`);
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
  // Get and print the login name.

  const loginName = getLoginName();

  if (loginName === null) {
    process.stderr.write("logname: no login name\n");
    process.exit(1);
  }

  process.stdout.write(loginName + "\n");
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------
// Guard against running during tests. The VITEST env var is set by vitest.

if (!process.env.VITEST) {
  main();
}
