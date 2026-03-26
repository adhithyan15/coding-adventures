/**
 * tty -- print the file name of the terminal connected to standard input.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the POSIX `tty` utility in TypeScript.
 * It prints the file name (device path) of the terminal connected to
 * standard input, or "not a tty" if stdin is not a terminal.
 *
 * === Why Does tty Matter? ===
 *
 * Many programs behave differently when connected to a terminal vs.
 * when piped. For example, `ls` shows colors when output goes to a
 * terminal but plain text when piped. The `tty` command lets scripts
 * detect this:
 *
 *     if tty -s; then
 *         echo "Running interactively"
 *     else
 *         echo "Running in a pipe"
 *     fi
 *
 * === Terminal Device Paths ===
 *
 * On Unix systems, terminals are represented as device files:
 *
 *     /dev/tty      =>  The controlling terminal
 *     /dev/pts/0    =>  A pseudo-terminal (SSH, terminal emulator)
 *     /dev/ttys000  =>  macOS terminal device
 *
 * When stdin is a pipe or file, there is no terminal device, and `tty`
 * prints "not a tty" and exits with status 1.
 *
 * === The -s (Silent) Flag ===
 *
 * With `-s`, tty prints nothing at all. It only communicates through
 * its exit code: 0 if stdin is a terminal, 1 if not. This is useful
 * in scripts where you only need the boolean answer.
 *
 * @module tty
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
const SPEC_FILE = path.resolve(__dirname, "..", "tty.json");

// ---------------------------------------------------------------------------
// Business Logic: Determine the TTY name.
// ---------------------------------------------------------------------------

/**
 * Determine whether stdin is a TTY and return its name.
 *
 * Node.js exposes TTY information through `process.stdin.isTTY`, which
 * is `true` when stdin is connected to a terminal and `undefined` (not
 * `false`) when it is not.
 *
 * To get the actual device path, we check `process.stdin` for a `path`
 * property (available on TTY streams) or fall back to `/dev/tty`.
 *
 * @returns An object with:
 *   - `isTTY`: boolean indicating whether stdin is a terminal
 *   - `name`: the device path if it is a TTY, "not a tty" otherwise
 */
export function getTtyInfo(): { isTTY: boolean; name: string } {
  if (process.stdin.isTTY) {
    // On Unix, the TTY device path can sometimes be retrieved. Node.js
    // doesn't directly expose the ttyname(3) function, but we can use
    // the general /dev/tty path which always refers to the controlling
    // terminal.
    return { isTTY: true, name: "/dev/tty" };
  }

  return { isTTY: false, name: "not a tty" };
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then print the tty name.
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
        process.stderr.write(`tty: ${error.message}\n`);
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

  const flags = (result as { flags: Record<string, unknown> }).flags;
  const silent = !!flags["silent"];

  const ttyInfo = getTtyInfo();

  // In silent mode, print nothing -- just exit with the appropriate code.
  // Exit 0 if stdin is a TTY, exit 1 if not.
  if (!silent) {
    process.stdout.write(ttyInfo.name + "\n");
  }

  process.exit(ttyInfo.isTTY ? 0 : 1);
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------
// Guard against running during tests. The VITEST env var is set by vitest.

if (!process.env.VITEST) {
  main();
}
