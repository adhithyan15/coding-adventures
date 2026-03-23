/**
 * df -- report file system disk space usage.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `df` utility in TypeScript. It
 * displays information about file system disk space usage, showing the
 * total size, used space, available space, and usage percentage for each
 * mounted file system.
 *
 * === Default Output ===
 *
 * With no arguments, df shows all mounted file systems:
 *
 *     Filesystem     1K-blocks      Used Available Use% Mounted on
 *     /dev/disk1s1   976490576 234567890 741922686  24% /
 *
 * === Human-Readable Mode (-h) ===
 *
 * With `-h`, sizes are shown with SI suffixes:
 *
 *     Filesystem     Size  Used Avail Use% Mounted on
 *     /dev/disk1s1   932G  224G  708G  24% /
 *
 * === Implementation ===
 *
 * Node.js doesn't have a native API for listing mounted file systems.
 * We shell out to the system `df` command and parse its output. This
 * approach works on all POSIX systems (Linux, macOS, FreeBSD).
 *
 * For specific paths, we can also use `fs.statfsSync()` (Node 18.15+)
 * to get file system statistics without shelling out.
 *
 * @module df
 */

import * as path from "node:path";
import { execSync } from "node:child_process";
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
const SPEC_FILE = path.resolve(__dirname, "..", "df.json");

// ---------------------------------------------------------------------------
// Types: File system information.
// ---------------------------------------------------------------------------

/**
 * Information about a single mounted file system.
 *
 * All sizes are in 1K-blocks unless human-readable mode is active.
 */
export interface FsInfo {
  /** Device name (e.g., "/dev/disk1s1"). */
  filesystem: string;
  /** Total size in 1K-blocks (or formatted string). */
  size: string;
  /** Used space in 1K-blocks (or formatted string). */
  used: string;
  /** Available space in 1K-blocks (or formatted string). */
  available: string;
  /** Usage percentage (e.g., "24%"). */
  usePercent: string;
  /** Mount point (e.g., "/"). */
  mountedOn: string;
  /** File system type (e.g., "apfs", "ext4"). */
  fsType?: string;
}

// ---------------------------------------------------------------------------
// Business Logic: Format bytes in human-readable form.
// ---------------------------------------------------------------------------

/**
 * Format a number of 1K-blocks into a human-readable string.
 *
 * Uses powers of 1024 (like df -h):
 *   < 1024K     => show as K
 *   < 1024M     => show as M
 *   < 1024G     => show as G
 *   >= 1024G    => show as T
 *
 * @param kBlocks - Size in 1K-blocks.
 * @param si      - If true, use powers of 1000 instead of 1024.
 * @returns A formatted string like "1.2G".
 */
export function formatSize(kBlocks: number, si: boolean = false): string {
  const base = si ? 1000 : 1024;
  const bytes = kBlocks * 1024;

  if (bytes < base) return kBlocks + "K";

  const units = ["K", "M", "G", "T", "P", "E"];
  let value = kBlocks;
  let unitIndex = 0;

  while (value >= base && unitIndex < units.length - 1) {
    value /= base;
    unitIndex++;
  }

  // Use one decimal place if the value is less than 10.
  if (value < 10) {
    return value.toFixed(1) + units[unitIndex];
  }
  return Math.round(value) + units[unitIndex];
}

// ---------------------------------------------------------------------------
// Business Logic: Parse system df output.
// ---------------------------------------------------------------------------

/**
 * Run the system `df` command and parse its output.
 *
 * We use `df -Pk` for POSIX-compatible output with 1K-blocks. The
 * output format is:
 *
 *     Filesystem     1024-blocks      Used Available Capacity Mounted on
 *     /dev/disk1s1   976490576   234567890 741922686      24% /
 *
 * We skip the header line and parse each subsequent line.
 *
 * @param paths       - Optional paths to show info for.
 * @param humanReadable - Format sizes for humans.
 * @param si          - Use powers of 1000 instead of 1024.
 * @param showType    - Include file system type in output.
 * @returns Array of FsInfo objects.
 */
export function getFilesystemInfo(
  paths?: string[],
  humanReadable: boolean = false,
  si: boolean = false,
  showType: boolean = false
): FsInfo[] {
  // Build the df command.
  // -P: POSIX output format (single-line per filesystem).
  // -k: 1K-blocks.
  let cmd = "df -Pk";
  if (showType) {
    // On macOS, -T shows type in different format. Use -Y on macOS.
    // For simplicity, we'll parse type separately if needed.
    cmd = "df -Pk";
  }

  if (paths && paths.length > 0) {
    cmd += " " + paths.map((p) => `"${p}"`).join(" ");
  }

  let output: string;
  try {
    output = execSync(cmd, { encoding: "utf-8" });
  } catch {
    return [];
  }

  const lines = output.trim().split("\n");
  if (lines.length < 2) return [];

  const results: FsInfo[] = [];

  // Skip header line (line 0).
  for (let i = 1; i < lines.length; i++) {
    const parts = lines[i].split(/\s+/);
    if (parts.length < 6) continue;

    // The mount point may contain spaces, so rejoin everything after
    // the 5th field.
    const filesystem = parts[0];
    const sizeBlocks = parseInt(parts[1], 10);
    const usedBlocks = parseInt(parts[2], 10);
    const availBlocks = parseInt(parts[3], 10);
    const usePercent = parts[4];
    const mountedOn = parts.slice(5).join(" ");

    const info: FsInfo = {
      filesystem,
      size: humanReadable ? formatSize(sizeBlocks, si) : String(sizeBlocks),
      used: humanReadable ? formatSize(usedBlocks, si) : String(usedBlocks),
      available: humanReadable
        ? formatSize(availBlocks, si)
        : String(availBlocks),
      usePercent,
      mountedOn,
    };

    results.push(info);
  }

  return results;
}

// ---------------------------------------------------------------------------
// Business Logic: Format df output as a table.
// ---------------------------------------------------------------------------

/**
 * Format an array of FsInfo objects as a table string.
 *
 * Each column is right-aligned (except Filesystem and Mounted on,
 * which are left-aligned). Column widths are computed from the data.
 *
 * @param infos       - Array of file system info objects.
 * @param humanReadable - Whether sizes are human-readable.
 * @returns Formatted table string (with trailing newlines).
 */
export function formatDfTable(
  infos: FsInfo[],
  humanReadable: boolean = false
): string {
  if (infos.length === 0) return "";

  // Headers.
  const sizeHeader = humanReadable ? "Size" : "1K-blocks";
  const headers = ["Filesystem", sizeHeader, "Used", humanReadable ? "Avail" : "Available", "Use%", "Mounted on"];

  // Compute column widths.
  const widths = headers.map((h) => h.length);

  for (const info of infos) {
    widths[0] = Math.max(widths[0], info.filesystem.length);
    widths[1] = Math.max(widths[1], info.size.length);
    widths[2] = Math.max(widths[2], info.used.length);
    widths[3] = Math.max(widths[3], info.available.length);
    widths[4] = Math.max(widths[4], info.usePercent.length);
    widths[5] = Math.max(widths[5], info.mountedOn.length);
  }

  const formatRow = (row: string[]): string => {
    return [
      row[0].padEnd(widths[0]),
      row[1].padStart(widths[1]),
      row[2].padStart(widths[2]),
      row[3].padStart(widths[3]),
      row[4].padStart(widths[4]),
      row[5],
    ].join(" ");
  };

  const lines: string[] = [];
  lines.push(formatRow(headers));

  for (const info of infos) {
    lines.push(
      formatRow([
        info.filesystem,
        info.size,
        info.used,
        info.available,
        info.usePercent,
        info.mountedOn,
      ])
    );
  }

  return lines.join("\n") + "\n";
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then report disk space.
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
        process.stderr.write(`df: ${error.message}\n`);
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

  const humanReadable = !!flags["human_readable"];
  const si = !!flags["si"];
  const showType = !!flags["print_type"];

  let files = args["files"] as string[] | string | undefined;
  if (typeof files === "string") {
    files = [files];
  }

  const infos = getFilesystemInfo(
    files as string[] | undefined,
    humanReadable,
    si,
    showType
  );

  process.stdout.write(formatDfTable(infos, humanReadable || si));
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

if (!process.env.VITEST) {
  main();
}
