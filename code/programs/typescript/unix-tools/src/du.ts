/**
 * du -- estimate file space usage.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `du` utility in TypeScript. It
 * estimates and reports the disk space used by files and directories.
 *
 * === How du Works ===
 *
 * By default, du recursively walks a directory tree and reports the
 * cumulative size of each directory:
 *
 *     $ du /tmp
 *     4       /tmp/subdir
 *     12      /tmp
 *
 * === Key Flags ===
 *
 * - `-s` (summarize): Only show a total for each argument.
 * - `-h` (human-readable): Print sizes with K, M, G suffixes.
 * - `-a` (all): Show sizes for individual files, not just directories.
 * - `-c` (total): Print a grand total at the end.
 * - `-d N` (max-depth): Limit output to N levels of depth.
 *
 * === Size Calculation ===
 *
 * du reports the actual disk usage, not the apparent file size. In this
 * implementation, we use `fs.statSync().size` (apparent size) since
 * Node.js doesn't expose block-level information portably. We convert
 * bytes to 1K-blocks (rounded up) to match traditional du output.
 *
 * @module du
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

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const SPEC_FILE = path.resolve(__dirname, "..", "du.json");

// ---------------------------------------------------------------------------
// Types: Options and result entries.
// ---------------------------------------------------------------------------

/**
 * Options controlling how du traverses and reports.
 */
export interface DuOptions {
  /** Show individual files, not just directories. */
  all: boolean;
  /** Format sizes as human-readable (K, M, G). */
  humanReadable: boolean;
  /** Use powers of 1000 instead of 1024. */
  si: boolean;
  /** Only show totals for each argument. */
  summarize: boolean;
  /** Show a grand total at the end. */
  total: boolean;
  /** Maximum depth to show (-1 for unlimited). */
  maxDepth: number;
  /** Follow symlinks. */
  dereference: boolean;
  /** Patterns to exclude. */
  exclude: string[];
  /** End lines with NUL instead of newline. */
  nullTerminated: boolean;
}

/**
 * A single entry in the du output.
 *
 * Each entry represents a file or directory with its cumulative size.
 */
export interface DuEntry {
  /** Size in bytes. */
  sizeBytes: number;
  /** Path to the file or directory. */
  path: string;
}

// ---------------------------------------------------------------------------
// Business Logic: Format size.
// ---------------------------------------------------------------------------

/**
 * Format a byte count for display.
 *
 * In default mode, du shows sizes in 1K-blocks (bytes / 1024, rounded up).
 * In human-readable mode, it uses K/M/G/T suffixes.
 *
 * @param bytes         - Size in bytes.
 * @param humanReadable - Whether to use human-readable format.
 * @param si            - Whether to use powers of 1000.
 * @returns Formatted size string.
 */
export function formatDuSize(
  bytes: number,
  humanReadable: boolean,
  si: boolean
): string {
  if (!humanReadable) {
    // Show in 1K-blocks (rounded up).
    return String(Math.ceil(bytes / 1024));
  }

  const base = si ? 1000 : 1024;
  const units = ["B", "K", "M", "G", "T", "P"];

  if (bytes < base) return bytes + "B";

  let value = bytes;
  let unitIndex = 0;

  while (value >= base && unitIndex < units.length - 1) {
    value /= base;
    unitIndex++;
  }

  if (value < 10) {
    return value.toFixed(1) + units[unitIndex];
  }
  return Math.round(value) + units[unitIndex];
}

// ---------------------------------------------------------------------------
// Business Logic: Check if a path matches an exclude pattern.
// ---------------------------------------------------------------------------

/**
 * Check if a filename matches any of the exclude patterns.
 *
 * We do a simple substring/glob match. For simplicity, we support
 * basic patterns: exact match and prefix/suffix wildcards.
 *
 * @param name     - The file or directory name (not full path).
 * @param patterns - Array of exclude patterns.
 * @returns True if the name should be excluded.
 */
export function shouldExclude(name: string, patterns: string[]): boolean {
  for (const pattern of patterns) {
    // Simple glob: if pattern starts/ends with *, do substring match.
    if (pattern.startsWith("*") && pattern.endsWith("*")) {
      if (name.includes(pattern.slice(1, -1))) return true;
    } else if (pattern.startsWith("*")) {
      if (name.endsWith(pattern.slice(1))) return true;
    } else if (pattern.endsWith("*")) {
      if (name.startsWith(pattern.slice(0, -1))) return true;
    } else {
      if (name === pattern) return true;
    }
  }
  return false;
}

// ---------------------------------------------------------------------------
// Business Logic: Calculate disk usage recursively.
// ---------------------------------------------------------------------------

/**
 * Calculate disk usage for a path, recursively traversing directories.
 *
 * This function walks the file tree depth-first. For each directory, it
 * sums the sizes of all contained files and subdirectories. The results
 * are collected into the `entries` array.
 *
 * === Walk Algorithm ===
 *
 * 1. If the path is a file, return its size.
 * 2. If the path is a directory:
 *    a. Recursively process each child.
 *    b. Sum all child sizes to get the directory's total.
 *    c. Add the directory entry (if not suppressed by depth/summarize).
 *
 * @param dirPath  - Path to measure.
 * @param opts     - Du options.
 * @param entries  - Output array to collect entries.
 * @param depth    - Current recursion depth (0 = argument itself).
 * @returns Total size in bytes of the path.
 */
export function diskUsage(
  dirPath: string,
  opts: DuOptions,
  entries: DuEntry[],
  depth: number = 0
): number {
  let stat: fs.Stats;
  try {
    stat = opts.dereference
      ? fs.statSync(dirPath)
      : fs.lstatSync(dirPath);
  } catch {
    return 0;
  }

  const name = path.basename(dirPath);

  // Check exclusion.
  if (shouldExclude(name, opts.exclude)) {
    return 0;
  }

  // If it's a file (not a directory), return its size.
  if (!stat.isDirectory()) {
    const size = stat.size;
    // In -a mode (or if this is a top-level argument), report the file.
    if (opts.all && !opts.summarize && (opts.maxDepth < 0 || depth <= opts.maxDepth)) {
      entries.push({ sizeBytes: size, path: dirPath });
    }
    return size;
  }

  // It's a directory. Walk its contents.
  let totalSize = 0;

  try {
    const children = fs.readdirSync(dirPath);
    for (const child of children) {
      const childPath = path.join(dirPath, child);
      totalSize += diskUsage(childPath, opts, entries, depth + 1);
    }
  } catch {
    // Permission denied or other error -- skip silently.
  }

  // Add the directory's own size (metadata).
  totalSize += stat.size;

  // Report this directory if appropriate.
  if (!opts.summarize || depth === 0) {
    if (opts.maxDepth < 0 || depth <= opts.maxDepth) {
      entries.push({ sizeBytes: totalSize, path: dirPath });
    }
  }

  return totalSize;
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then estimate disk usage.
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
        process.stderr.write(`du: ${error.message}\n`);
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

  const flags = (result as { flags: Record<string, unknown> }).flags;
  const args = (result as { arguments: Record<string, unknown> }).arguments;

  const opts: DuOptions = {
    all: !!flags["all"],
    humanReadable: !!flags["human_readable"],
    si: !!flags["si"],
    summarize: !!flags["summarize"],
    total: !!flags["total"],
    maxDepth: flags["max_depth"] !== undefined ? (flags["max_depth"] as number) : -1,
    dereference: !!flags["dereference"],
    exclude: ((flags["exclude"] as string[]) ?? []),
    nullTerminated: !!flags["null"],
  };

  // If summarize is set, it's like max_depth=0.
  if (opts.summarize) {
    opts.maxDepth = 0;
  }

  const lineEnd = opts.nullTerminated ? "\0" : "\n";

  let files = args["files"] as string[] | string | undefined;
  if (!files) {
    files = ["."];
  } else if (typeof files === "string") {
    files = [files];
  }

  let grandTotal = 0;

  for (const file of files) {
    const entries: DuEntry[] = [];
    const total = diskUsage(file, opts, entries, 0);
    grandTotal += total;

    for (const entry of entries) {
      const size = formatDuSize(entry.sizeBytes, opts.humanReadable, opts.si);
      process.stdout.write(`${size}\t${entry.path}${lineEnd}`);
    }
  }

  if (opts.total) {
    const size = formatDuSize(grandTotal, opts.humanReadable, opts.si);
    process.stdout.write(`${size}\ttotal${lineEnd}`);
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

if (!process.env.VITEST) {
  main();
}
