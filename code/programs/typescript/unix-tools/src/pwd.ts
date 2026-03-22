/**
 * pwd -- print the absolute pathname of the current working directory.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the POSIX `pwd` utility in TypeScript.
 * It prints the absolute path of the current working directory to
 * standard output.
 *
 * === How CLI Builder Powers This ===
 *
 * The entire command-line interface -- flags, help text, version output,
 * error messages -- is defined in `pwd.json`. This program never parses
 * a single argument by hand. Instead:
 *
 * 1. We hand `pwd.json` and `process.argv` to CLI Builder's `Parser`.
 * 2. The parser validates the input, enforces mutual exclusivity of
 *    `-L` and `-P`, generates help text, and returns a typed result.
 * 3. We discriminate on the result type and run the business logic.
 *
 * The result is that *this file contains only business logic*. All
 * parsing, validation, and help generation happen inside CLI Builder,
 * driven by the JSON spec.
 *
 * === Logical vs Physical Paths ===
 *
 * When you `cd` through a symbolic link, the shell updates the `$PWD`
 * environment variable to reflect the path *as you typed it* -- including
 * the symlink. This is the "logical" path.
 *
 * The "physical" path resolves all symlinks. For example, if `/home` is
 * a symlink to `/usr/home`:
 *
 *     Logical:  /home/user       (what $PWD says)
 *     Physical: /usr/home/user   (what the filesystem says)
 *
 * By default (`-L`), we print the logical path. With `-P`, we resolve
 * symlinks and print the physical path.
 *
 * === POSIX Compliance Note ===
 *
 * If `$PWD` is not set, or if it doesn't match the actual current
 * directory, even `-L` mode falls back to the physical path. This
 * matches POSIX behavior.
 *
 * @module pwd
 */

import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";

// ---------------------------------------------------------------------------
// Import CLI Builder.
// ---------------------------------------------------------------------------
// We import from the package's source directly, following the pattern
// established by other TypeScript programs in this monorepo. The
// `file:` dependency in package.json resolves to the local cli-builder
// package.

import { Parser } from "@coding-adventures/cli-builder";

// ---------------------------------------------------------------------------
// Locate the JSON spec file.
// ---------------------------------------------------------------------------
// In ESM, there is no `__dirname` global. Instead, we derive the current
// file's directory from `import.meta.url`, which gives us a `file://` URL.
// `fileURLToPath` converts it to a filesystem path, and `path.dirname`
// extracts the directory.
//
// The spec file lives one level up from `src/`, so `..` from this file's
// directory gets us to the project root where `pwd.json` lives.

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const SPEC_FILE = path.resolve(__dirname, "..", "pwd.json");

// ---------------------------------------------------------------------------
// Business Logic: Get the current working directory.
// ---------------------------------------------------------------------------

/**
 * Return the logical working directory.
 *
 * The logical path comes from the `$PWD` environment variable, which the
 * shell maintains as the user navigates -- including through symlinks.
 *
 * If `$PWD` is not set or is stale (doesn't match the real cwd), we fall
 * back to the physical path. This matches POSIX behavior: the logical
 * path is best-effort, never wrong.
 *
 * === Why validate $PWD? ===
 *
 * The `$PWD` variable can become stale in several ways:
 * - The directory was moved or deleted after the shell set `$PWD`.
 * - The process called `chdir()` without updating the environment.
 * - A parent process set `$PWD` to something bogus.
 *
 * So we resolve both `$PWD` and `.` to their real paths and compare.
 * If they match, `$PWD` is trustworthy and we return it. Otherwise,
 * we fall back to the physical path.
 */
function getLogicalPwd(): string {
  const envPwd = process.env.PWD;

  if (envPwd !== undefined) {
    // Verify that $PWD actually points to the current directory.
    // It could be stale if the directory was moved/deleted, or if
    // the process changed directories without updating $PWD.
    try {
      const envReal = fs.realpathSync(envPwd);
      const cwdReal = fs.realpathSync(".");
      if (envReal === cwdReal) {
        return envPwd;
      }
    } catch {
      // If realpathSync fails (e.g., path doesn't exist), fall through
      // to the physical path fallback below.
    }
  }

  // Fallback: resolve the physical path.
  // If $PWD was unset or stale, this is the safest option.
  return fs.realpathSync(".");
}

/**
 * Return the physical working directory with all symlinks resolved.
 *
 * This calls `fs.realpathSync(".")`, which follows every symlink in the
 * path to produce the canonical filesystem path. This is equivalent to
 * Python's `Path.cwd().resolve()` or the C function `realpath(".", ...)`.
 */
function getPhysicalPwd(): string {
  return fs.realpathSync(".");
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then print the cwd.
// ---------------------------------------------------------------------------

/**
 * Entry point.
 *
 * The flow is straightforward:
 * 1. Create a Parser with the spec file and process.argv.
 * 2. Call parse() to get a result.
 * 3. Discriminate the result type using duck typing:
 *    - `"text" in result` => HelpResult (user passed --help)
 *    - `"version" in result` and no `"flags"` => VersionResult (--version)
 *    - Otherwise => ParseResult with .flags and .arguments
 * 4. For ParseResult, check if the "physical" flag is set.
 *
 * CLI Builder throws `ParseErrors` on invalid input, which we catch
 * and format as error messages on stderr.
 */
function main(): void {
  // --- Step 1: Parse arguments ---------------------------------------------
  // Hand the spec file and process.argv to CLI Builder. The parser reads
  // the JSON spec, validates the flags, enforces mutual exclusivity, and
  // returns one of three result types.

  let result;

  try {
    const parser = new Parser(SPEC_FILE, process.argv);
    result = parser.parse();
  } catch (err: unknown) {
    // CLI Builder throws ParseErrors when validation fails.
    // Each error has a `.message` property describing what went wrong.
    if (err && typeof err === "object" && "errors" in err) {
      const errors = (err as { errors: Array<{ message: string }> }).errors;
      for (const error of errors) {
        process.stderr.write(`pwd: ${error.message}\n`);
      }
      process.exit(1);
    }
    throw err;
  }

  // --- Step 2: Dispatch on result type -------------------------------------
  // CLI Builder returns one of:
  //   - HelpResult:    has a `text` property (user passed --help)
  //   - VersionResult: has a `version` property but no `flags`
  //   - ParseResult:   has `flags`, `arguments`, and `commandPath`
  //
  // We use duck typing to discriminate, as recommended in the CLI Builder
  // documentation.

  if ("text" in result) {
    // HelpResult -- user asked for --help.
    process.stdout.write(result.text + "\n");
    process.exit(0);
  }

  if ("version" in result && !("flags" in result)) {
    // VersionResult -- user asked for --version.
    process.stdout.write(result.version + "\n");
    process.exit(0);
  }

  // --- Step 3: Business logic ----------------------------------------------
  // This is the *only* part that is specific to the pwd tool.
  // CLI Builder has already validated the flags, so we just check
  // whether the "physical" flag is set.
  //
  // The flags map uses the flag's `id` from pwd.json as keys.
  // For pwd.json, the relevant IDs are:
  //   - "logical"  => boolean (true if -L/--logical was passed)
  //   - "physical" => boolean (true if -P/--physical was passed)
  //
  // Default behavior (no flags): print the logical path.

  const flags = (result as { flags: Record<string, unknown> }).flags;

  if (flags["physical"]) {
    process.stdout.write(getPhysicalPwd() + "\n");
  } else {
    process.stdout.write(getLogicalPwd() + "\n");
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

main();
