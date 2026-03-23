/**
 * chmod -- change file mode bits.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the POSIX `chmod` utility in TypeScript.
 * It changes the permission bits of files and directories.
 *
 * === How File Permissions Work ===
 *
 * Unix permissions are organized as three sets of three bits:
 *
 *     Owner    Group    Others
 *     r w x    r w x    r w x
 *     4 2 1    4 2 1    4 2 1
 *
 * Each set controls read (r=4), write (w=2), and execute (x=1)
 * permissions. The three values are combined into an octal number:
 *
 *     755 = rwxr-xr-x = owner can do everything, others can read+execute
 *     644 = rw-r--r-- = owner can read+write, others can only read
 *     700 = rwx------ = only the owner can access the file
 *
 * === Symbolic Mode Notation ===
 *
 * chmod supports two ways to specify permissions:
 *
 * **Octal mode** -- a 3 or 4 digit octal number:
 *
 *     chmod 755 myfile     =>  rwxr-xr-x
 *     chmod 0644 myfile    =>  rw-r--r--
 *
 * **Symbolic mode** -- human-readable permission changes:
 *
 *     chmod u+x myfile     =>  add execute for owner
 *     chmod go-w myfile    =>  remove write for group and others
 *     chmod a=r myfile     =>  set read-only for all
 *     chmod u+rwx,go+rx    =>  multiple changes separated by commas
 *
 * The symbolic format is: [ugoa][+-=][rwxXst]
 *
 *     Who:   u=owner, g=group, o=others, a=all
 *     Op:    +=add, -=remove, ==set exactly
 *     What:  r=read, w=write, x=execute, X=execute if directory or already executable,
 *            s=setuid/setgid, t=sticky bit
 *
 * @module chmod
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

const __filename_chmod = fileURLToPath(import.meta.url);
const __dirname_chmod = path.dirname(__filename_chmod);
const SPEC_FILE = path.resolve(__dirname_chmod, "..", "chmod.json");

// ---------------------------------------------------------------------------
// Types: Options that control chmod's behavior.
// ---------------------------------------------------------------------------

/**
 * Configuration options for chmod operations.
 *
 *     Flag   Option      Meaning
 *     ----   ------      -------
 *     -R     recursive   Change files and directories recursively
 *     -v     verbose     Print a diagnostic for every file processed
 *     -c     changes     Like verbose but only report actual changes
 *     -f     silent      Suppress most error messages
 */
export interface ChmodOptions {
  /** Change files and directories recursively (-R). */
  recursive: boolean;
  /** Print a diagnostic for every file processed (-v). */
  verbose: boolean;
  /** Like verbose, but only report when a change is made (-c). */
  changes: boolean;
  /** Suppress most error messages (-f). */
  silent: boolean;
}

// ---------------------------------------------------------------------------
// Types: Parsed symbolic mode expression.
// ---------------------------------------------------------------------------

/**
 * A single symbolic mode clause like "u+rx" or "go-w".
 *
 * A symbolic mode string like "u+rx,go-w" is parsed into an array
 * of these clauses, each applied in sequence to the current mode.
 */
export interface SymbolicClause {
  /** Which permission sets to modify: 'u', 'g', 'o', or 'a'. */
  who: string[];
  /** The operation: '+' (add), '-' (remove), or '=' (set). */
  operator: "+" | "-" | "=";
  /** Which permission bits: 'r', 'w', 'x', 'X', 's', 't'. */
  permissions: string[];
}

// ---------------------------------------------------------------------------
// Business Logic: Parse octal mode strings.
// ---------------------------------------------------------------------------

/**
 * Parse an octal mode string into a numeric file mode.
 *
 * Octal modes can be 1-4 digits:
 *
 *     "755"  =>  0o755  (rwxr-xr-x)
 *     "0644" =>  0o644  (rw-r--r--)
 *     "7"    =>  0o7    (------rwx)
 *
 * @param modeStr  The octal mode string.
 * @returns        The numeric mode, or null if not a valid octal string.
 *
 * @example
 * ```ts
 * parseOctalMode("755");   // => 0o755
 * parseOctalMode("u+x");  // => null (not octal)
 * ```
 */
export function parseOctalMode(modeStr: string): number | null {
  // An octal mode is all digits 0-7, with length 1-4.
  if (!/^[0-7]{1,4}$/.test(modeStr)) {
    return null;
  }

  return parseInt(modeStr, 8);
}

// ---------------------------------------------------------------------------
// Business Logic: Parse symbolic mode strings.
// ---------------------------------------------------------------------------

/**
 * Parse a symbolic mode string into an array of clauses.
 *
 * The symbolic mode format is a comma-separated list of clauses:
 *
 *     "u+rx,go-w,o=r"
 *
 * Each clause has the format: [ugoa]*[+-=][rwxXst]*
 *
 * If no "who" is specified, 'a' (all) is implied:
 *
 *     "+x"   =>  same as "a+x"
 *     "=rw"  =>  same as "a=rw"
 *
 * @param modeStr  The symbolic mode string.
 * @returns        Array of parsed clauses.
 * @throws         If the mode string is invalid.
 *
 * @example
 * ```ts
 * parseSymbolicMode("u+rx");
 * // => [{ who: ["u"], operator: "+", permissions: ["r", "x"] }]
 *
 * parseSymbolicMode("u+rx,go-w");
 * // => [
 * //   { who: ["u"], operator: "+", permissions: ["r", "x"] },
 * //   { who: ["g", "o"], operator: "-", permissions: ["w"] },
 * // ]
 * ```
 */
export function parseSymbolicMode(modeStr: string): SymbolicClause[] {
  const clauses: SymbolicClause[] = [];

  // Split on commas to get individual clauses.
  const parts = modeStr.split(",");

  for (const part of parts) {
    // Parse each clause with a regex.
    // Group 1: who characters (optional)
    // Group 2: operator (required)
    // Group 3: permission characters (optional)
    const match = part.match(/^([ugoa]*)([+\-=])([rwxXst]*)$/);

    if (!match) {
      throw new Error(`chmod: invalid mode: '${modeStr}'`);
    }

    const whoStr = match[1] || "a";
    const operator = match[2] as "+" | "-" | "=";
    const permStr = match[3];

    // Expand 'a' into ['u', 'g', 'o'].
    let who: string[];
    if (whoStr.includes("a") || whoStr === "") {
      who = ["u", "g", "o"];
    } else {
      who = [...new Set(whoStr.split(""))];
    }

    const permissions = permStr ? permStr.split("") : [];

    clauses.push({ who, operator, permissions });
  }

  return clauses;
}

// ---------------------------------------------------------------------------
// Business Logic: Apply symbolic mode to a file mode.
// ---------------------------------------------------------------------------

/**
 * Apply a single symbolic clause to a numeric file mode.
 *
 * This is where the bit manipulation happens. Each clause specifies
 * WHO gets WHAT permissions through WHICH operation.
 *
 * The permission bits are laid out in an octal mode like this:
 *
 *     Bit layout (12 bits total):
 *
 *     [setuid][setgid][sticky]  [owner r][owner w][owner x]  [group r][group w][group x]  [other r][other w][other x]
 *       2048    1024     512       256      128       64          32       16        8          4        2        1
 *       0o4000  0o2000  0o1000   0o400    0o200    0o100       0o040    0o020    0o010      0o004    0o002    0o001
 *
 * @param currentMode  The current numeric file mode.
 * @param clause       The symbolic clause to apply.
 * @param isDirectory  Whether the target is a directory (affects 'X').
 * @returns            The new numeric file mode.
 */
export function applySymbolicClause(
  currentMode: number,
  clause: SymbolicClause,
  isDirectory: boolean
): number {
  let mode = currentMode;

  // --- Step 1: Calculate the permission bits to modify -----------------
  // For each "who" target, calculate which bits to set/clear.

  for (const target of clause.who) {
    let bits = 0;

    // Map the permission characters to bit positions for this target.
    for (const perm of clause.permissions) {
      switch (perm) {
        case "r":
          if (target === "u") bits |= 0o400;
          else if (target === "g") bits |= 0o040;
          else if (target === "o") bits |= 0o004;
          break;

        case "w":
          if (target === "u") bits |= 0o200;
          else if (target === "g") bits |= 0o020;
          else if (target === "o") bits |= 0o002;
          break;

        case "x":
          if (target === "u") bits |= 0o100;
          else if (target === "g") bits |= 0o010;
          else if (target === "o") bits |= 0o001;
          break;

        case "X":
          // 'X' sets execute only if the file is a directory or
          // already has execute permission for some user.
          if (isDirectory || (currentMode & 0o111)) {
            if (target === "u") bits |= 0o100;
            else if (target === "g") bits |= 0o010;
            else if (target === "o") bits |= 0o001;
          }
          break;

        case "s":
          // setuid (for owner) or setgid (for group).
          if (target === "u") bits |= 0o4000;
          else if (target === "g") bits |= 0o2000;
          break;

        case "t":
          // Sticky bit -- only meaningful for "other" or "all".
          bits |= 0o1000;
          break;
      }
    }

    // --- Step 2: Apply the operation -----------------------------------

    switch (clause.operator) {
      case "+":
        // Add the specified bits.
        mode |= bits;
        break;

      case "-":
        // Remove the specified bits.
        mode &= ~bits;
        break;

      case "=":
        // Set exactly: clear the target's bits, then set new ones.
        // First, clear all permission bits for this target.
        if (target === "u") mode &= ~0o4700;
        else if (target === "g") mode &= ~0o2070;
        else if (target === "o") mode &= ~0o1007;
        // Then set the new bits.
        mode |= bits;
        break;
    }
  }

  return mode;
}

// ---------------------------------------------------------------------------
// Business Logic: Apply a complete mode specification.
// ---------------------------------------------------------------------------

/**
 * Calculate the new mode for a file given a mode specification string.
 *
 * This function handles both octal and symbolic modes. It's the
 * single entry point for mode calculation.
 *
 * @param modeStr      The mode specification (e.g., "755" or "u+rx,go-w").
 * @param currentMode  The file's current mode (needed for symbolic modes).
 * @param isDirectory  Whether the target is a directory.
 * @returns            The new numeric mode.
 * @throws             If the mode string is invalid.
 *
 * @example
 * ```ts
 * calculateMode("755", 0o644, false);      // => 0o755
 * calculateMode("u+x", 0o644, false);      // => 0o744
 * calculateMode("go-rwx", 0o755, false);   // => 0o700
 * ```
 */
export function calculateMode(
  modeStr: string,
  currentMode: number,
  isDirectory: boolean
): number {
  // Try octal first -- it's simpler.
  const octal = parseOctalMode(modeStr);
  if (octal !== null) {
    return octal;
  }

  // Parse as symbolic mode.
  const clauses = parseSymbolicMode(modeStr);

  // Apply each clause in sequence.
  let mode = currentMode;
  for (const clause of clauses) {
    mode = applySymbolicClause(mode, clause, isDirectory);
  }

  return mode;
}

// ---------------------------------------------------------------------------
// Business Logic: Apply chmod to a single file.
// ---------------------------------------------------------------------------

/**
 * Result of a chmod operation on a single file.
 */
export interface ChmodResult {
  /** The file path that was changed. */
  filePath: string;
  /** The old mode (before chmod). */
  oldMode: number;
  /** The new mode (after chmod). */
  newMode: number;
  /** Whether the mode actually changed. */
  changed: boolean;
  /** Diagnostic message (for verbose/changes output). */
  message: string;
}

/**
 * Format a file mode as an octal string for display.
 *
 * @param mode  The numeric file mode.
 * @returns     A 4-digit octal string like "0755".
 */
export function formatMode(mode: number): string {
  return "0" + (mode & 0o7777).toString(8).padStart(3, "0");
}

/**
 * Apply a mode change to a single file or directory.
 *
 * @param filePath  Path to the file.
 * @param modeStr   Mode specification (octal or symbolic).
 * @param opts      chmod options.
 * @returns         The result of the operation.
 * @throws          If the file doesn't exist or can't be modified.
 */
export function chmodFile(
  filePath: string,
  modeStr: string,
  opts: ChmodOptions
): ChmodResult {
  const stat = fs.statSync(filePath);
  const oldMode = stat.mode & 0o7777; // Extract just the permission bits.
  const isDirectory = stat.isDirectory();

  const newMode = calculateMode(modeStr, oldMode, isDirectory);
  const changed = oldMode !== newMode;

  if (changed) {
    fs.chmodSync(filePath, newMode);
  }

  const message = `mode of '${filePath}' changed from ${formatMode(oldMode)} to ${formatMode(newMode)}`;

  return { filePath, oldMode, newMode, changed, message };
}

// ---------------------------------------------------------------------------
// Business Logic: Recursive chmod.
// ---------------------------------------------------------------------------

/**
 * Apply a mode change recursively to a directory tree.
 *
 * This walks the directory tree depth-first, applying the mode
 * change to every file and directory encountered.
 *
 * @param dirPath  Path to the directory.
 * @param modeStr  Mode specification.
 * @param opts     chmod options.
 * @returns        Array of results for each file/directory processed.
 */
export function chmodRecursive(
  dirPath: string,
  modeStr: string,
  opts: ChmodOptions
): ChmodResult[] {
  const results: ChmodResult[] = [];

  // Apply to the directory itself first.
  try {
    results.push(chmodFile(dirPath, modeStr, opts));
  } catch (err) {
    if (!opts.silent) throw err;
  }

  // Then recurse into contents.
  const entries = fs.readdirSync(dirPath, { withFileTypes: true });

  for (const entry of entries) {
    const fullPath = path.join(dirPath, entry.name);

    if (entry.isDirectory()) {
      // Recurse into subdirectories.
      results.push(...chmodRecursive(fullPath, modeStr, opts));
    } else {
      try {
        results.push(chmodFile(fullPath, modeStr, opts));
      } catch (err) {
        if (!opts.silent) throw err;
      }
    }
  }

  return results;
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then change modes.
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
        process.stderr.write(`chmod: ${error.message}\n`);
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
  const args = result.args || {};

  const modeStr = args.mode as string;
  const files: string[] = Array.isArray(args.files) ? args.files : [args.files];

  const opts: ChmodOptions = {
    recursive: flags.recursive || false,
    verbose: flags.verbose || false,
    changes: flags.changes || false,
    silent: flags.silent || false,
  };

  let exitCode = 0;

  for (const filePath of files) {
    try {
      let results: ChmodResult[];

      if (opts.recursive) {
        results = chmodRecursive(filePath, modeStr, opts);
      } else {
        results = [chmodFile(filePath, modeStr, opts)];
      }

      for (const r of results) {
        if (opts.verbose) {
          process.stdout.write(r.message + "\n");
        } else if (opts.changes && r.changed) {
          process.stdout.write(r.message + "\n");
        }
      }
    } catch (err: unknown) {
      if (!opts.silent) {
        if (err instanceof Error) {
          process.stderr.write(`chmod: ${err.message}\n`);
        }
      }
      exitCode = 1;
    }
  }

  process.exit(exitCode);
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

if (!process.env.VITEST) {
  main();
}
