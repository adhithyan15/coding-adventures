/**
 * sleep -- delay for a specified amount of time.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `sleep` utility in TypeScript.
 * It pauses execution for the specified duration. Multiple durations
 * can be given and they are summed.
 *
 * === Duration Suffixes ===
 *
 * Each duration argument is a number optionally followed by a suffix:
 *
 *     Suffix    Meaning     Multiplier
 *     ------    -------     ----------
 *     s         Seconds     1           (default)
 *     m         Minutes     60
 *     h         Hours       3600
 *     d         Days        86400
 *
 * If no suffix is given, seconds are assumed.
 *
 * === Multiple Arguments ===
 *
 * When multiple durations are given, they are summed:
 *
 *     sleep 1m 30s    =>    sleeps for 90 seconds
 *     sleep 1h 30m    =>    sleeps for 5400 seconds
 *
 * === Floating Point Support ===
 *
 * Unlike some implementations, GNU sleep supports fractional values:
 *
 *     sleep 0.5       =>    sleeps for half a second
 *     sleep 1.5m      =>    sleeps for 90 seconds
 *
 * @module sleep
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
const SPEC_FILE = path.resolve(__dirname, "..", "sleep.json");

// ---------------------------------------------------------------------------
// Business Logic: Parse duration strings.
// ---------------------------------------------------------------------------

/**
 * Suffix multipliers -- how many seconds each suffix represents.
 *
 * This table maps each valid suffix character to its multiplier in
 * seconds. The absence of a suffix implies seconds (multiplier = 1).
 *
 *     "30"   => 30 * 1    = 30 seconds
 *     "30s"  => 30 * 1    = 30 seconds
 *     "2m"   => 2  * 60   = 120 seconds
 *     "1h"   => 1  * 3600 = 3600 seconds
 *     "0.5d" => 0.5 * 86400 = 43200 seconds
 */
const SUFFIX_MULTIPLIERS: Record<string, number> = {
  s: 1,
  m: 60,
  h: 3600,
  d: 86400,
};

/**
 * Parse a single duration string into seconds.
 *
 * A duration string is a number optionally followed by a suffix letter
 * (s, m, h, d). If no suffix is present, seconds are assumed.
 *
 * === Parsing Strategy ===
 *
 * We check if the last character is a letter. If so, it's the suffix
 * and we slice it off. The remaining string is parsed as a float.
 *
 * @param s The duration string to parse (e.g., "30", "2m", "1.5h").
 * @returns The duration in seconds.
 * @throws Error if the string cannot be parsed.
 *
 * @example
 *   parseDuration("30")    =>  30
 *   parseDuration("30s")   =>  30
 *   parseDuration("2m")    =>  120
 *   parseDuration("1.5h")  =>  5400
 *   parseDuration("0.5d")  =>  43200
 */
export function parseDuration(s: string): number {
  if (s.length === 0) {
    throw new Error(`invalid time interval '${s}'`);
  }

  // Check if the last character is a suffix letter.
  const lastChar = s[s.length - 1].toLowerCase();
  let numberPart: string;
  let multiplier: number;

  if (lastChar in SUFFIX_MULTIPLIERS) {
    // The last character is a recognized suffix.
    numberPart = s.slice(0, -1);
    multiplier = SUFFIX_MULTIPLIERS[lastChar];
  } else {
    // No suffix -- assume seconds.
    numberPart = s;
    multiplier = 1;
  }

  const value = parseFloat(numberPart);

  if (isNaN(value) || numberPart.trim().length === 0) {
    throw new Error(`invalid time interval '${s}'`);
  }

  if (value < 0) {
    throw new Error(`invalid time interval '${s}'`);
  }

  return value * multiplier;
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then sleep.
// ---------------------------------------------------------------------------

/**
 * Entry point.
 *
 * The flow is:
 * 1. Parse arguments with CLI Builder.
 * 2. Handle --help and --version.
 * 3. Parse each duration argument and sum them.
 * 4. Sleep for the total duration using setTimeout wrapped in a Promise.
 */
async function main(): Promise<void> {
  // --- Step 1: Parse arguments ---------------------------------------------

  let result;

  try {
    const parser = new Parser(SPEC_FILE, process.argv);
    result = parser.parse();
  } catch (err: unknown) {
    if (err && typeof err === "object" && "errors" in err) {
      const errors = (err as { errors: Array<{ message: string }> }).errors;
      for (const error of errors) {
        process.stderr.write(`sleep: ${error.message}\n`);
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

  // --- Step 3: Parse durations and compute total ---------------------------

  const args = (result as { arguments: Record<string, unknown> }).arguments;
  let durations = args["duration"] as string[] | string;

  if (typeof durations === "string") {
    durations = [durations];
  }

  let totalSeconds = 0;

  for (const dur of durations) {
    try {
      totalSeconds += parseDuration(dur);
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      process.stderr.write(`sleep: ${message}\n`);
      process.exit(1);
    }
  }

  // --- Step 4: Sleep -------------------------------------------------------
  // We convert seconds to milliseconds for setTimeout. The Promise wrapper
  // makes this awaitable so the process doesn't exit prematurely.

  if (totalSeconds > 0) {
    await new Promise<void>((resolve) => {
      setTimeout(resolve, totalSeconds * 1000);
    });
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------
// Guard against running during tests. The VITEST env var is set by vitest.

if (!process.env.VITEST) {
  main();
}
