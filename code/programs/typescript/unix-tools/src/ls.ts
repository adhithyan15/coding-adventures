/**
 * ls -- list directory contents.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `ls` utility in TypeScript.
 * It lists information about files and directories. By default, it
 * lists the contents of the current directory, sorted alphabetically.
 *
 * === How ls Works ===
 *
 *     ls                =>  list current directory
 *     ls /tmp           =>  list contents of /tmp
 *     ls -l             =>  long format (permissions, size, etc.)
 *     ls -a             =>  include hidden files (starting with .)
 *     ls -la            =>  long format with hidden files
 *
 * === Hidden Files ===
 *
 * Files starting with a dot (.) are "hidden" by convention. By
 * default, ls skips them. Two flags control this:
 *
 *     -a (--all):        Show ALL entries, including . and ..
 *     -A (--almost-all): Show hidden entries, but NOT . and ..
 *
 * === Sorting ===
 *
 * By default, ls sorts entries alphabetically (case-insensitive on
 * some systems). Several flags change the sort order:
 *
 *     -S   Sort by file size (largest first)
 *     -t   Sort by modification time (newest first)
 *     -X   Sort by extension (alphabetically)
 *     -U   No sorting (directory order)
 *     -r   Reverse the sort order
 *
 * === Long Format ===
 *
 * The -l flag enables long format, which shows:
 *
 *     permissions  links  owner  group  size  date  name
 *     -rw-r--r--   1     user   staff  1234  Mar 22 10:30  file.txt
 *
 * === Human-Readable Sizes ===
 *
 * With -h (--human-readable), file sizes are shown with unit suffixes:
 *
 *     1023      =>  1023
 *     1024      =>  1.0K
 *     1048576   =>  1.0M
 *     1073741824 => 1.0G
 *
 * @module ls
 */

import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { fileURLToPath } from "node:url";

// ---------------------------------------------------------------------------
// Import CLI Builder.
// ---------------------------------------------------------------------------

import { Parser } from "@coding-adventures/cli-builder";

// ---------------------------------------------------------------------------
// Locate the JSON spec file.
// ---------------------------------------------------------------------------

const __filename_ls = fileURLToPath(import.meta.url);
const __dirname_ls = path.dirname(__filename_ls);
const SPEC_FILE = path.resolve(__dirname_ls, "..", "ls.json");

// ---------------------------------------------------------------------------
// Types: Options that control ls's behavior.
// ---------------------------------------------------------------------------

/**
 * Configuration options for listing operations.
 */
export interface ListOptions {
  /** Show all entries including hidden (-a). */
  all: boolean;
  /** Show hidden entries except . and .. (-A). */
  almostAll: boolean;
  /** Use long listing format (-l). */
  long: boolean;
  /** Show sizes in human-readable format (-h). */
  humanReadable: boolean;
  /** Reverse sort order (-r). */
  reverse: boolean;
  /** Sort by size (-S). */
  sortBySize: boolean;
  /** Sort by modification time (-t). */
  sortByTime: boolean;
  /** Sort by extension (-X). */
  sortByExtension: boolean;
  /** Do not sort (-U). */
  unsorted: boolean;
  /** Append indicator to entries (-F). */
  classify: boolean;
  /** One entry per line (-1). */
  onePerLine: boolean;
}

// ---------------------------------------------------------------------------
// Types: A directory entry with metadata.
// ---------------------------------------------------------------------------

/**
 * Represents a single entry returned by listDirectory.
 *
 * We gather all metadata upfront so formatting functions don't need
 * to call `fs.statSync` again.
 */
export interface DirEntry {
  /** The entry name (basename only). */
  name: string;
  /** Full path for stat operations. */
  fullPath: string;
  /** File size in bytes. */
  size: number;
  /** Modification time. */
  mtime: Date;
  /** Whether this entry is a directory. */
  isDirectory: boolean;
  /** Whether this entry is a symbolic link. */
  isSymlink: boolean;
  /** Unix permission mode (e.g. 0o755). */
  mode: number;
  /** Number of hard links. */
  nlink: number;
  /** Owner UID. */
  uid: number;
  /** Group GID. */
  gid: number;
}

// ---------------------------------------------------------------------------
// Business Logic: List directory contents.
// ---------------------------------------------------------------------------

/**
 * List the contents of a directory.
 *
 * This function reads the directory entries, filters them according
 * to the visibility options, gathers stat metadata for each entry,
 * and sorts them according to the sort options.
 *
 * @param dirPath  The directory to list.
 * @param opts     Listing options.
 * @returns        Array of DirEntry objects, sorted and filtered.
 * @throws         If the path doesn't exist or isn't readable.
 *
 * @example
 * ```ts
 * const entries = listDirectory("/tmp", { all: false, almostAll: false, long: false,
 *   humanReadable: false, reverse: false, sortBySize: false, sortByTime: false,
 *   sortByExtension: false, unsorted: false, classify: false, onePerLine: false });
 * ```
 */
export function listDirectory(
  dirPath: string,
  opts: ListOptions
): DirEntry[] {
  // --- Step 1: Read directory entries ------------------------------------

  let names: string[];

  try {
    names = fs.readdirSync(dirPath);
  } catch {
    throw new Error(
      `ls: cannot access '${dirPath}': No such file or directory`
    );
  }

  // --- Step 2: Filter hidden entries -------------------------------------
  // By default, entries starting with '.' are hidden.
  // -a shows everything including . and ..
  // -A shows hidden entries but not . and ..

  if (opts.all) {
    // Add . and .. explicitly (readdirSync doesn't include them).
    names = [".", "..", ...names];
  } else if (!opts.almostAll) {
    // Filter out hidden entries.
    names = names.filter((name) => !name.startsWith("."));
  }
  // If almostAll is true, we keep hidden entries but don't add . and ..
  // (readdirSync already excludes them).

  // --- Step 3: Gather metadata for each entry ----------------------------

  const entries: DirEntry[] = [];

  for (const name of names) {
    const fullPath =
      name === "." || name === ".."
        ? dirPath
        : path.join(dirPath, name);

    try {
      // Use lstat to detect symlinks (stat follows them).
      const lstat = fs.lstatSync(fullPath);

      entries.push({
        name,
        fullPath,
        size: lstat.size,
        mtime: lstat.mtime,
        isDirectory: lstat.isDirectory(),
        isSymlink: lstat.isSymbolicLink(),
        mode: lstat.mode,
        nlink: lstat.nlink,
        uid: lstat.uid,
        gid: lstat.gid,
      });
    } catch {
      // If we can't stat an entry, skip it (permission denied, etc.).
      continue;
    }
  }

  // --- Step 4: Sort entries -----------------------------------------------

  if (!opts.unsorted) {
    entries.sort((a, b) => {
      let cmp = 0;

      if (opts.sortBySize) {
        // Largest first.
        cmp = b.size - a.size;
      } else if (opts.sortByTime) {
        // Newest first.
        cmp = b.mtime.getTime() - a.mtime.getTime();
      } else if (opts.sortByExtension) {
        // Alphabetical by extension.
        const extA = path.extname(a.name).toLowerCase();
        const extB = path.extname(b.name).toLowerCase();
        cmp = extA.localeCompare(extB);
      } else {
        // Default: alphabetical by name.
        cmp = a.name.localeCompare(b.name);
      }

      return opts.reverse ? -cmp : cmp;
    });
  }

  return entries;
}

// ---------------------------------------------------------------------------
// Business Logic: Format a single entry.
// ---------------------------------------------------------------------------

/**
 * Format a human-readable file size.
 *
 * Converts bytes into K, M, G, T, P units using powers of 1024.
 * Values less than 1024 are shown without a unit suffix.
 *
 * @param bytes  The size in bytes.
 * @returns      A human-readable string like "1.0K" or "234M".
 *
 * @example
 * ```ts
 * formatSize(1024);      // => "1.0K"
 * formatSize(1048576);   // => "1.0M"
 * formatSize(500);       // => "500"
 * ```
 */
export function formatSize(bytes: number): string {
  const units = ["", "K", "M", "G", "T", "P"];
  let unitIndex = 0;
  let size = bytes;

  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex++;
  }

  if (unitIndex === 0) {
    return String(bytes);
  }

  return `${size.toFixed(1)}${units[unitIndex]}`;
}

// ---------------------------------------------------------------------------
// Business Logic: Format permission string.
// ---------------------------------------------------------------------------

/**
 * Convert a Unix mode number to a permission string like "rwxr-xr--".
 *
 * The mode is a 12-bit value where:
 *   - Bits 8-6: owner permissions (rwx)
 *   - Bits 5-3: group permissions (rwx)
 *   - Bits 2-0: other permissions (rwx)
 *
 * We also prepend a file type character:
 *   d = directory, l = symlink, - = regular file
 *
 * @param mode        The Unix mode integer.
 * @param isDir       Whether the entry is a directory.
 * @param isSymlink   Whether the entry is a symbolic link.
 * @returns           A 10-character permission string.
 *
 * @example
 * ```ts
 * formatPermissions(0o755, true, false);   // => "drwxr-xr-x"
 * formatPermissions(0o644, false, false);  // => "-rw-r--r--"
 * ```
 */
export function formatPermissions(
  mode: number,
  isDir: boolean,
  isSymlink: boolean
): string {
  const typeChar = isSymlink ? "l" : isDir ? "d" : "-";

  const perms = [
    mode & 0o400 ? "r" : "-",
    mode & 0o200 ? "w" : "-",
    mode & 0o100 ? "x" : "-",
    mode & 0o040 ? "r" : "-",
    mode & 0o020 ? "w" : "-",
    mode & 0o010 ? "x" : "-",
    mode & 0o004 ? "r" : "-",
    mode & 0o002 ? "w" : "-",
    mode & 0o001 ? "x" : "-",
  ];

  return typeChar + perms.join("");
}

// ---------------------------------------------------------------------------
// Business Logic: Format a date for ls -l output.
// ---------------------------------------------------------------------------

/**
 * Format a date for ls long format.
 *
 * If the date is within the last 6 months, show "Mon DD HH:MM".
 * Otherwise, show "Mon DD  YYYY".
 *
 * @param date  The modification date.
 * @returns     Formatted date string.
 */
export function formatDate(date: Date): string {
  const months = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun",
    "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
  ];

  const month = months[date.getMonth()];
  const day = String(date.getDate()).padStart(2, " ");
  const now = new Date();
  const sixMonthsAgo = new Date(now.getTime() - 180 * 24 * 60 * 60 * 1000);

  if (date > sixMonthsAgo) {
    const hours = String(date.getHours()).padStart(2, "0");
    const minutes = String(date.getMinutes()).padStart(2, "0");
    return `${month} ${day} ${hours}:${minutes}`;
  } else {
    const year = String(date.getFullYear()).padStart(5, " ");
    return `${month} ${day} ${year}`;
  }
}

// ---------------------------------------------------------------------------
// Business Logic: Format one entry for output.
// ---------------------------------------------------------------------------

/**
 * Format a directory entry for display.
 *
 * In short mode, returns just the name (optionally with a classifier).
 * In long mode, returns the full ls -l line.
 *
 * @param entry  The directory entry to format.
 * @param opts   Listing options.
 * @returns      Formatted string for display.
 *
 * @example
 * ```ts
 * formatEntry(entry, { long: true, humanReadable: true, classify: false, ... });
 * // => "-rw-r--r--  1 user  staff  1.0K Mar 22 10:30 file.txt"
 * ```
 */
export function formatEntry(entry: DirEntry, opts: ListOptions): string {
  let suffix = "";

  // Classify mode: append a character indicating file type.
  if (opts.classify) {
    if (entry.isDirectory) suffix = "/";
    else if (entry.isSymlink) suffix = "@";
    else if (entry.mode & 0o111) suffix = "*";
  }

  if (!opts.long) {
    return entry.name + suffix;
  }

  // Long format: permissions links owner group size date name
  const perms = formatPermissions(entry.mode, entry.isDirectory, entry.isSymlink);
  const links = String(entry.nlink).padStart(2, " ");

  // Try to resolve user/group names. Fall back to numeric IDs.
  let owner: string;
  try {
    owner = os.userInfo().username;
  } catch {
    owner = String(entry.uid);
  }
  const group = String(entry.gid);

  const size = opts.humanReadable
    ? formatSize(entry.size).padStart(5, " ")
    : String(entry.size).padStart(8, " ");

  const date = formatDate(entry.mtime);

  return `${perms} ${links} ${owner} ${group} ${size} ${date} ${entry.name}${suffix}`;
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then list directories.
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
        process.stderr.write(`ls: ${error.message}\n`);
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
  const files: string[] = result.args?.files || ["."];

  const opts: ListOptions = {
    all: flags.all || false,
    almostAll: flags.almost_all || false,
    long: flags.long || false,
    humanReadable: flags.human_readable || false,
    reverse: flags.reverse || false,
    sortBySize: flags.sort_by_size || false,
    sortByTime: flags.sort_by_time || false,
    sortByExtension: flags.sort_by_extension || false,
    unsorted: flags.unsorted || false,
    classify: flags.classify || false,
    onePerLine: flags.one_per_line || false,
  };

  try {
    for (const filePath of files) {
      if (files.length > 1) {
        process.stdout.write(`${filePath}:\n`);
      }

      const entries = listDirectory(filePath, opts);
      const formatted = entries.map((e) => formatEntry(e, opts));

      if (opts.long || opts.onePerLine) {
        for (const line of formatted) {
          process.stdout.write(line + "\n");
        }
      } else {
        process.stdout.write(formatted.join("  ") + "\n");
      }

      if (files.length > 1) {
        process.stdout.write("\n");
      }
    }
  } catch (err: unknown) {
    if (err instanceof Error) {
      process.stderr.write(err.message + "\n");
    }
    process.exit(1);
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

if (!process.env.VITEST) {
  main();
}
