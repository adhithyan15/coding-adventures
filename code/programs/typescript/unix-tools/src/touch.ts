/**
 * touch -- change file timestamps.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `touch` utility in TypeScript. It
 * updates the access and modification times of each FILE to the current
 * time. If the file does not exist, it is created (unless -c is given).
 *
 * === How touch Works ===
 *
 * touch is most commonly used to create empty files:
 *
 *     touch newfile.txt         =>   creates newfile.txt (empty)
 *     touch existing.txt        =>   updates timestamps
 *     touch -c noexist.txt      =>   does nothing (file doesn't exist)
 *
 * === Time Selection ===
 *
 * By default, touch updates both access time (atime) and modification
 * time (mtime). You can select just one:
 *
 *     touch -a file.txt         =>   update only access time
 *     touch -m file.txt         =>   update only modification time
 *
 * === Custom Timestamps ===
 *
 * Instead of the current time, you can specify a time with:
 * - `-d STRING`: Parse a date string (e.g., "2024-01-15 10:30:00")
 * - `-r FILE`: Use another file's timestamps
 * - `-t STAMP`: Use [[CC]YY]MMDDhhmm[.ss] format
 *
 * These three options are mutually exclusive.
 *
 * @module touch
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
const SPEC_FILE = path.resolve(__dirname, "..", "touch.json");

// ---------------------------------------------------------------------------
// Business Logic: Parse timestamp formats.
// ---------------------------------------------------------------------------

/**
 * Parse a -t timestamp in [[CC]YY]MMDDhhmm[.ss] format.
 *
 * The format is positional:
 *   - CC: century (e.g., 20)
 *   - YY: year within century (e.g., 24)
 *   - MM: month (01-12)
 *   - DD: day (01-31)
 *   - hh: hour (00-23)
 *   - mm: minute (00-59)
 *   - .ss: seconds (00-59), optional
 *
 * If CC is omitted, the century is inferred from YY:
 *   - YY in 69-99 => 19YY
 *   - YY in 00-68 => 20YY
 *
 * If CCYY is omitted entirely, the current year is used.
 */
function parseTimestamp(stamp: string): Date {
  // Separate seconds if present.
  let mainPart = stamp;
  let seconds = 0;

  const dotIndex = stamp.indexOf(".");
  if (dotIndex !== -1) {
    seconds = parseInt(stamp.substring(dotIndex + 1), 10);
    mainPart = stamp.substring(0, dotIndex);
  }

  let year: number;
  let month: number;
  let day: number;
  let hour: number;
  let minute: number;

  if (mainPart.length === 8) {
    // MMDDhhmm -- no year, use current year.
    year = new Date().getFullYear();
    month = parseInt(mainPart.substring(0, 2), 10);
    day = parseInt(mainPart.substring(2, 4), 10);
    hour = parseInt(mainPart.substring(4, 6), 10);
    minute = parseInt(mainPart.substring(6, 8), 10);
  } else if (mainPart.length === 10) {
    // YYMMDDhhmm -- two-digit year.
    const yy = parseInt(mainPart.substring(0, 2), 10);
    year = yy >= 69 ? 1900 + yy : 2000 + yy;
    month = parseInt(mainPart.substring(2, 4), 10);
    day = parseInt(mainPart.substring(4, 6), 10);
    hour = parseInt(mainPart.substring(6, 8), 10);
    minute = parseInt(mainPart.substring(8, 10), 10);
  } else if (mainPart.length === 12) {
    // CCYYMMDDhhmm -- four-digit year.
    year = parseInt(mainPart.substring(0, 4), 10);
    month = parseInt(mainPart.substring(4, 6), 10);
    day = parseInt(mainPart.substring(6, 8), 10);
    hour = parseInt(mainPart.substring(8, 10), 10);
    minute = parseInt(mainPart.substring(10, 12), 10);
  } else {
    throw new Error(`invalid timestamp format: '${stamp}'`);
  }

  return new Date(year, month - 1, day, hour, minute, seconds);
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then touch files.
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
        process.stderr.write(`touch: ${error.message}\n`);
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

  const accessOnly = !!flags["access_only"];
  const modificationOnly = !!flags["modification_only"];
  const noCreate = !!flags["no_create"];
  const dateStr = flags["date"] as string | undefined;
  const referenceFile = flags["reference"] as string | undefined;
  const timestampStr = flags["timestamp"] as string | undefined;

  // Normalize file list.
  let files = args["files"] as string[] | string;
  if (typeof files === "string") {
    files = [files];
  }

  // --- Step 4: Determine the timestamp to use ------------------------------
  // By default, use the current time.
  // -d, -r, and -t are mutually exclusive alternatives.

  let targetTime = new Date();

  if (dateStr) {
    targetTime = new Date(dateStr);
    if (isNaN(targetTime.getTime())) {
      process.stderr.write(`touch: invalid date format '${dateStr}'\n`);
      process.exit(1);
    }
  } else if (referenceFile) {
    try {
      const stat = fs.statSync(referenceFile);
      targetTime = stat.mtime;
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      process.stderr.write(`touch: ${referenceFile}: ${message}\n`);
      process.exit(1);
    }
  } else if (timestampStr) {
    try {
      targetTime = parseTimestamp(timestampStr);
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      process.stderr.write(`touch: ${message}\n`);
      process.exit(1);
    }
  }

  // --- Step 5: Touch each file ---------------------------------------------

  for (const file of files) {
    const exists = fs.existsSync(file);

    if (!exists) {
      if (noCreate) {
        // -c: don't create, just skip.
        continue;
      }

      // Create the file.
      try {
        fs.writeFileSync(file, "");
      } catch (err: unknown) {
        const message = err instanceof Error ? err.message : String(err);
        process.stderr.write(`touch: ${file}: ${message}\n`);
        process.exitCode = 1;
        continue;
      }
    }

    // Update timestamps.
    try {
      const stat = fs.statSync(file);

      // Determine atime and mtime.
      // If neither -a nor -m is given, update both.
      // If -a is given, update only atime (keep mtime).
      // If -m is given, update only mtime (keep atime).
      const newAtime = accessOnly || (!accessOnly && !modificationOnly)
        ? targetTime
        : stat.atime;
      const newMtime = modificationOnly || (!accessOnly && !modificationOnly)
        ? targetTime
        : stat.mtime;

      fs.utimesSync(file, newAtime, newMtime);
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      process.stderr.write(`touch: ${file}: ${message}\n`);
      process.exitCode = 1;
    }
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

main();
