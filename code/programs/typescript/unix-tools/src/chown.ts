/**
 * chown -- change file owner and group.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the POSIX `chown` utility in TypeScript.
 * It changes the ownership of files and directories.
 *
 * === How File Ownership Works ===
 *
 * Every file on a Unix system has two ownership attributes:
 *
 *     Owner (user):  The user who owns the file (identified by UID)
 *     Group:         The group that owns the file (identified by GID)
 *
 * You can see these with `ls -l`:
 *
 *     -rw-r--r--  1  alice  staff  1024  Jan  1 12:00  file.txt
 *                    ^^^^^  ^^^^^
 *                    owner  group
 *
 * === Ownership Specification Formats ===
 *
 * chown accepts the owner/group specification in several formats:
 *
 *     OWNER          Change only the owner
 *     OWNER:GROUP    Change both owner and group
 *     OWNER:         Change owner and set group to owner's login group
 *     :GROUP         Change only the group (like chgrp)
 *     OWNER.GROUP    Same as OWNER:GROUP (legacy syntax)
 *
 * OWNER and GROUP can be specified as names (e.g., "alice") or
 * numeric IDs (e.g., "501"). In our implementation, we only support
 * numeric IDs since resolving user/group names requires system
 * libraries that aren't available in pure Node.js.
 *
 * === Why chown Usually Requires Root ===
 *
 * On most Unix systems, only the superuser (root) can change file
 * ownership. Regular users can only change the group of their own
 * files to a group they belong to. Our implementation handles the
 * permission errors gracefully.
 *
 * @module chown
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

const __filename_chown = fileURLToPath(import.meta.url);
const __dirname_chown = path.dirname(__filename_chown);
const SPEC_FILE = path.resolve(__dirname_chown, "..", "chown.json");

// ---------------------------------------------------------------------------
// Types: Options that control chown's behavior.
// ---------------------------------------------------------------------------

/**
 * Configuration options for chown operations.
 *
 *     Flag   Option          Meaning
 *     ----   ------          -------
 *     -R     recursive       Change ownership recursively
 *     -v     verbose         Print a diagnostic for every file processed
 *     -c     changes         Like verbose but only report actual changes
 *     -f     silent          Suppress most error messages
 *     -h     noDereference   Affect symlinks, not their targets
 */
export interface ChownOptions {
  /** Change ownership recursively (-R). */
  recursive: boolean;
  /** Print a diagnostic for every file processed (-v). */
  verbose: boolean;
  /** Like verbose, but only report when a change is made (-c). */
  changes: boolean;
  /** Suppress most error messages (-f). */
  silent: boolean;
  /** Affect symlinks instead of their targets (-h). */
  noDereference: boolean;
}

// ---------------------------------------------------------------------------
// Types: Parsed ownership specification.
// ---------------------------------------------------------------------------

/**
 * Parsed ownership specification from the OWNER[:GROUP] argument.
 *
 * Either uid or gid (or both) may be null, indicating "don't change".
 *
 *     "1000"       => { uid: 1000, gid: null }
 *     "1000:100"   => { uid: 1000, gid: 100 }
 *     ":100"       => { uid: null, gid: 100 }
 *     "1000:"      => { uid: 1000, gid: null }  (use owner's group)
 */
export interface OwnerSpec {
  /** The new owner UID, or null to leave unchanged. */
  uid: number | null;
  /** The new group GID, or null to leave unchanged. */
  gid: number | null;
}

// ---------------------------------------------------------------------------
// Business Logic: Parse the OWNER[:GROUP] specification.
// ---------------------------------------------------------------------------

/**
 * Parse an ownership specification string into uid/gid values.
 *
 * The ownership spec can take several forms:
 *
 *     "1000"       Only change the owner to UID 1000
 *     "1000:100"   Change owner to 1000 and group to 100
 *     "1000:"      Change owner to 1000 (group left to caller)
 *     ":100"       Only change the group to GID 100
 *     "1000.100"   Legacy syntax, same as "1000:100"
 *
 * We only support numeric IDs in this implementation. Name resolution
 * would require system calls like getpwnam(3) that aren't available
 * in pure Node.js without native modules.
 *
 * @param spec  The ownership specification string.
 * @returns     The parsed uid/gid values.
 * @throws      If the specification is invalid.
 *
 * @example
 * ```ts
 * parseOwnerSpec("1000:100");  // => { uid: 1000, gid: 100 }
 * parseOwnerSpec(":100");      // => { uid: null, gid: 100 }
 * parseOwnerSpec("1000");      // => { uid: 1000, gid: null }
 * ```
 */
export function parseOwnerSpec(spec: string): OwnerSpec {
  // Determine the separator: ':' is standard, '.' is legacy.
  let separator: string | null = null;

  if (spec.includes(":")) {
    separator = ":";
  } else if (spec.includes(".")) {
    separator = ".";
  }

  if (separator) {
    // Split on the separator.
    const parts = spec.split(separator);
    const ownerPart = parts[0];
    const groupPart = parts.slice(1).join(separator);

    const uid = ownerPart ? parseId(ownerPart) : null;
    const gid = groupPart ? parseId(groupPart) : null;

    return { uid, gid };
  }

  // No separator: just an owner.
  return { uid: parseId(spec), gid: null };
}

/**
 * Parse a single ID string (numeric) into a number.
 *
 * @param idStr  The ID string to parse.
 * @returns      The numeric ID.
 * @throws       If the string is not a valid number.
 */
function parseId(idStr: string): number {
  const id = parseInt(idStr, 10);
  if (isNaN(id) || id < 0) {
    throw new Error(`chown: invalid user or group: '${idStr}'`);
  }
  return id;
}

// ---------------------------------------------------------------------------
// Business Logic: Apply ownership change to a single file.
// ---------------------------------------------------------------------------

/**
 * Result of a chown operation on a single file.
 */
export interface ChownResult {
  /** The file path that was processed. */
  filePath: string;
  /** The old owner UID. */
  oldUid: number;
  /** The old group GID. */
  oldGid: number;
  /** The new owner UID. */
  newUid: number;
  /** The new group GID. */
  newGid: number;
  /** Whether the ownership actually changed. */
  changed: boolean;
  /** Diagnostic message for verbose/changes output. */
  message: string;
  /** Whether an error occurred (e.g., permission denied). */
  error: boolean;
}

/**
 * Change the ownership of a single file.
 *
 * This function stats the file to get current ownership, calculates
 * the new ownership based on the spec (keeping unchanged values),
 * and applies the change using fs.chownSync or fs.lchownSync.
 *
 * @param filePath  Path to the file.
 * @param spec      Parsed ownership specification.
 * @param opts      chown options.
 * @returns         The result of the operation.
 */
export function chownFile(
  filePath: string,
  spec: OwnerSpec,
  opts: ChownOptions
): ChownResult {
  // --- Step 1: Get current ownership -----------------------------------

  const stat = opts.noDereference
    ? fs.lstatSync(filePath)
    : fs.statSync(filePath);

  const oldUid = stat.uid;
  const oldGid = stat.gid;

  // --- Step 2: Calculate new ownership ---------------------------------
  // If the spec doesn't specify a uid or gid, keep the current value.

  const newUid = spec.uid !== null ? spec.uid : oldUid;
  const newGid = spec.gid !== null ? spec.gid : oldGid;

  const changed = oldUid !== newUid || oldGid !== newGid;

  // --- Step 3: Apply the change ----------------------------------------

  if (changed) {
    try {
      if (opts.noDereference) {
        fs.lchownSync(filePath, newUid, newGid);
      } else {
        fs.chownSync(filePath, newUid, newGid);
      }
    } catch (err: unknown) {
      const errMsg = err instanceof Error ? err.message : String(err);
      return {
        filePath,
        oldUid,
        oldGid,
        newUid,
        newGid,
        changed: false,
        message: `chown: changing ownership of '${filePath}': ${errMsg}`,
        error: true,
      };
    }
  }

  const message = changed
    ? `changed ownership of '${filePath}' from ${oldUid}:${oldGid} to ${newUid}:${newGid}`
    : `ownership of '${filePath}' retained as ${oldUid}:${oldGid}`;

  return {
    filePath,
    oldUid,
    oldGid,
    newUid,
    newGid,
    changed,
    message,
    error: false,
  };
}

// ---------------------------------------------------------------------------
// Business Logic: Recursive chown.
// ---------------------------------------------------------------------------

/**
 * Change ownership recursively for a directory tree.
 *
 * Walks the directory depth-first, applying ownership changes to
 * every file and directory encountered.
 *
 * @param dirPath  Path to the directory.
 * @param spec     Parsed ownership specification.
 * @param opts     chown options.
 * @returns        Array of results for each file/directory processed.
 */
export function chownRecursive(
  dirPath: string,
  spec: OwnerSpec,
  opts: ChownOptions
): ChownResult[] {
  const results: ChownResult[] = [];

  // Apply to the directory itself.
  results.push(chownFile(dirPath, spec, opts));

  // Recurse into contents.
  let entries: fs.Dirent[];
  try {
    entries = fs.readdirSync(dirPath, { withFileTypes: true });
  } catch {
    return results;
  }

  for (const entry of entries) {
    const fullPath = path.join(dirPath, entry.name);

    if (entry.isDirectory()) {
      results.push(...chownRecursive(fullPath, spec, opts));
    } else {
      results.push(chownFile(fullPath, spec, opts));
    }
  }

  return results;
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then change ownership.
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
        process.stderr.write(`chown: ${error.message}\n`);
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

  const ownerGroupStr = args.owner_group as string;
  const files: string[] = Array.isArray(args.files) ? args.files : [args.files];

  const opts: ChownOptions = {
    recursive: flags.recursive || false,
    verbose: flags.verbose || false,
    changes: flags.changes || false,
    silent: flags.silent || false,
    noDereference: flags.no_dereference || false,
  };

  let spec: OwnerSpec;
  try {
    spec = parseOwnerSpec(ownerGroupStr);
  } catch (err: unknown) {
    if (err instanceof Error) {
      process.stderr.write(err.message + "\n");
    }
    process.exit(1);
    return;
  }

  let exitCode = 0;

  for (const filePath of files) {
    try {
      let results: ChownResult[];

      if (opts.recursive) {
        results = chownRecursive(filePath, spec, opts);
      } else {
        results = [chownFile(filePath, spec, opts)];
      }

      for (const r of results) {
        if (r.error) {
          if (!opts.silent) {
            process.stderr.write(r.message + "\n");
          }
          exitCode = 1;
        } else if (opts.verbose) {
          process.stdout.write(r.message + "\n");
        } else if (opts.changes && r.changed) {
          process.stdout.write(r.message + "\n");
        }
      }
    } catch (err: unknown) {
      if (!opts.silent) {
        if (err instanceof Error) {
          process.stderr.write(`chown: ${err.message}\n`);
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
