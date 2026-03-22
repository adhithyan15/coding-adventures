/**
 * seq -- print a sequence of numbers.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `seq` utility in TypeScript. It
 * prints a sequence of numbers from FIRST to LAST, in steps of INCREMENT.
 *
 * === How seq Works ===
 *
 * seq accepts one, two, or three positional arguments:
 *
 *     seq LAST              =>   1, 2, 3, ..., LAST
 *     seq FIRST LAST        =>   FIRST, FIRST+1, ..., LAST
 *     seq FIRST INCR LAST   =>   FIRST, FIRST+INCR, FIRST+2*INCR, ..., LAST
 *
 * === Floating Point Support ===
 *
 * seq handles both integers and floating-point numbers:
 *
 *     seq 0.5 0.5 2.5   =>   0.5, 1.0, 1.5, 2.0, 2.5
 *
 * === Equal Width Mode (-w) ===
 *
 * With `-w`, all numbers are padded with leading zeroes to the same width:
 *
 *     seq -w 8 12   =>   08, 09, 10, 11, 12
 *
 * === Custom Separator (-s) ===
 *
 * By default, numbers are separated by newlines. With `-s STRING`:
 *
 *     seq -s ', ' 5   =>   1, 2, 3, 4, 5
 *
 * @module seq
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
const SPEC_FILE = path.resolve(__dirname, "..", "seq.json");

// ---------------------------------------------------------------------------
// Business Logic: Generate the number sequence.
// ---------------------------------------------------------------------------

/**
 * Determine how many decimal places a number string has.
 *
 * This is needed for proper formatting: if the user writes `seq 0.5 2`,
 * we should output `0.5`, `1.0`, `1.5`, `2.0` -- preserving the decimal
 * precision from the input.
 *
 *     "3"    => 0
 *     "1.5"  => 1
 *     "0.25" => 2
 */
function decimalPlaces(numStr: string): number {
  const dot = numStr.indexOf(".");
  if (dot === -1) return 0;
  return numStr.length - dot - 1;
}

/**
 * Format a number with a specific number of decimal places.
 *
 * JavaScript's `toFixed` handles this nicely, but we need to be careful
 * about floating-point representation artifacts. For example, 0.1 + 0.2
 * is not exactly 0.3 in IEEE 754. `toFixed` rounds correctly for display
 * purposes.
 */
function formatNumber(value: number, precision: number): string {
  return value.toFixed(precision);
}

/**
 * Generate the sequence of numbers.
 *
 * We compute the number of steps using the formula:
 *   steps = floor((last - first) / increment)
 *
 * Then we generate each number as `first + i * increment` for
 * i = 0, 1, ..., steps. This avoids accumulating floating-point
 * errors from repeated addition.
 *
 * === Why multiply instead of add? ===
 *
 * If we wrote `current += increment` in a loop, floating-point errors
 * would accumulate. After 1000 steps of 0.1, we might get 99.99999...
 * instead of 100.0. By computing `first + i * increment`, each value
 * has at most one multiplication's worth of error.
 */
function generateSequence(
  first: number,
  increment: number,
  last: number,
  precision: number
): string[] {
  const results: string[] = [];

  if (increment > 0) {
    for (let i = 0; first + i * increment <= last + 1e-10; i++) {
      results.push(formatNumber(first + i * increment, precision));
    }
  } else if (increment < 0) {
    for (let i = 0; first + i * increment >= last - 1e-10; i++) {
      results.push(formatNumber(first + i * increment, precision));
    }
  }
  // If increment is 0, we'd loop forever. GNU seq prints an error;
  // we just return nothing.

  return results;
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then print the sequence.
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
        process.stderr.write(`seq: ${error.message}\n`);
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

  const separator = (flags["separator"] as string) ?? "\n";
  const equalWidth = !!flags["equal_width"];

  // Parse the positional arguments.
  // seq accepts 1, 2, or 3 numbers:
  //   seq LAST            =>   first=1, increment=1, last=LAST
  //   seq FIRST LAST      =>   first=FIRST, increment=1, last=LAST
  //   seq FIRST INCR LAST =>   first=FIRST, increment=INCR, last=LAST
  let numbers = args["numbers"] as string[] | string;
  if (typeof numbers === "string") {
    numbers = [numbers];
  }

  let firstStr: string, incrStr: string, lastStr: string;

  if (numbers.length === 1) {
    firstStr = "1";
    incrStr = "1";
    lastStr = numbers[0];
  } else if (numbers.length === 2) {
    firstStr = numbers[0];
    incrStr = "1";
    lastStr = numbers[1];
  } else {
    firstStr = numbers[0];
    incrStr = numbers[1];
    lastStr = numbers[2];
  }

  const first = parseFloat(firstStr);
  const increment = parseFloat(incrStr);
  const last = parseFloat(lastStr);

  // Validate inputs.
  if (isNaN(first) || isNaN(increment) || isNaN(last)) {
    process.stderr.write("seq: invalid floating point argument\n");
    process.exit(1);
  }

  if (increment === 0) {
    process.stderr.write("seq: invalid Zero increment value\n");
    process.exit(1);
  }

  // --- Step 4: Determine precision -----------------------------------------
  // The output precision is the maximum decimal places across all inputs.

  const precision = Math.max(
    decimalPlaces(firstStr),
    decimalPlaces(incrStr),
    decimalPlaces(lastStr)
  );

  // --- Step 5: Generate and output the sequence ----------------------------

  let sequence = generateSequence(first, increment, last, precision);

  // Equal-width mode: pad all numbers to the same width with leading zeroes.
  if (equalWidth && sequence.length > 0) {
    const maxWidth = Math.max(...sequence.map((s) => s.length));
    sequence = sequence.map((s) => s.padStart(maxWidth, "0"));
  }

  if (sequence.length > 0) {
    process.stdout.write(sequence.join(separator) + "\n");
  }
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

main();
