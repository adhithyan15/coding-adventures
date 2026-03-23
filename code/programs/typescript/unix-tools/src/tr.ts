/**
 * tr -- translate or delete characters.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the GNU `tr` utility in TypeScript. It
 * reads from standard input, translates, squeezes, or deletes characters,
 * and writes the result to standard output.
 *
 * === How tr Works ===
 *
 * tr operates on individual characters, not strings or words. It reads
 * stdin one character at a time, transforms it according to SET1 and SET2,
 * and writes the result:
 *
 *     echo "hello" | tr 'l' 'r'         =>   "herro"
 *     echo "hello" | tr 'a-z' 'A-Z'     =>   "HELLO"
 *     echo "hello" | tr -d 'l'          =>   "heo"
 *     echo "aabbcc" | tr -s 'a-c'       =>   "abc"
 *
 * === Character Sets ===
 *
 * SET1 and SET2 are strings of characters. Special notations:
 *
 * - `a-z`: Character range from 'a' to 'z' (inclusive).
 * - `\n`, `\t`, `\\`: Escape sequences for newline, tab, backslash.
 *
 * === Operation Modes ===
 *
 * 1. **Translate** (default): Each character in SET1 is replaced by the
 *    corresponding character in SET2.
 * 2. **Delete** (`-d`): Characters in SET1 are removed from the input.
 * 3. **Squeeze** (`-s`): Consecutive duplicate characters from the last
 *    SET are compressed to a single occurrence.
 * 4. **Complement** (`-c`): Use the complement of SET1 (all characters
 *    NOT in SET1).
 *
 * @module tr
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
const SPEC_FILE = path.resolve(__dirname, "..", "tr.json");

// ---------------------------------------------------------------------------
// Business Logic: Parse character set notation.
// ---------------------------------------------------------------------------

/**
 * Expand a character set specification into an array of characters.
 *
 * Supports:
 * - Literal characters: "abc" => ['a', 'b', 'c']
 * - Ranges: "a-z" => ['a', 'b', ..., 'z']
 * - Escape sequences: "\\n" => ['\n'], "\\t" => ['\t']
 *
 * === Range Expansion ===
 *
 * A range "X-Y" expands to all characters with code points from X to Y
 * inclusive. The range must be ascending (X <= Y). For example:
 *
 *     "a-f"   => ['a', 'b', 'c', 'd', 'e', 'f']
 *     "0-9"   => ['0', '1', ..., '9']
 *     "A-Z"   => ['A', 'B', ..., 'Z']
 */
function expandSet(setSpec: string): string[] {
  const chars: string[] = [];
  let i = 0;

  while (i < setSpec.length) {
    // Handle escape sequences.
    if (setSpec[i] === "\\" && i + 1 < setSpec.length) {
      const next = setSpec[i + 1];
      switch (next) {
        case "n":
          chars.push("\n");
          break;
        case "t":
          chars.push("\t");
          break;
        case "r":
          chars.push("\r");
          break;
        case "\\":
          chars.push("\\");
          break;
        case "a":
          chars.push("\x07");
          break;
        case "b":
          chars.push("\b");
          break;
        case "f":
          chars.push("\f");
          break;
        case "v":
          chars.push("\v");
          break;
        default:
          chars.push(next);
          break;
      }
      i += 2;
      continue;
    }

    // Handle ranges: "X-Y" where X and Y are single characters.
    // A range is detected when the current character is followed by "-" and
    // then another character. For example, "a-z" expands to all lowercase
    // letters from 'a' to 'z'.
    if (i + 2 < setSpec.length && setSpec[i + 1] === "-") {
      const startCode = setSpec.charCodeAt(i);
      const endCode = setSpec.charCodeAt(i + 2);
      if (startCode <= endCode) {
        for (let code = startCode; code <= endCode; code++) {
          chars.push(String.fromCharCode(code));
        }
        i += 3;
        continue;
      }
    }

    // Plain character.
    chars.push(setSpec[i]);
    i++;
  }

  return chars;
}

/**
 * Compute the complement of a character set.
 *
 * The complement contains all characters (0-127 for ASCII) that are NOT
 * in the given set. This is used with the -c flag.
 */
function complementSet(set: string[]): string[] {
  const setChars = new Set(set);
  const result: string[] = [];
  for (let code = 0; code < 128; code++) {
    const ch = String.fromCharCode(code);
    if (!setChars.has(ch)) {
      result.push(ch);
    }
  }
  return result;
}

/**
 * Translate characters: replace each character in set1 with the
 * corresponding character in set2.
 *
 * If set2 is shorter than set1, the last character of set2 is used for
 * all remaining characters in set1 (unless -t is used to truncate set1).
 */
function translateChars(
  input: string,
  set1: string[],
  set2: string[],
  squeeze: boolean
): string {
  // Build a translation map for O(1) lookups.
  const translationMap = new Map<string, string>();

  for (let i = 0; i < set1.length; i++) {
    // If set2 is shorter, use its last character for remaining set1 chars.
    const replacement = i < set2.length ? set2[i] : set2[set2.length - 1];
    translationMap.set(set1[i], replacement);
  }

  // Build the squeeze set (characters in set2 for squeeze checking).
  const squeezeSet = new Set(set2);

  let output = "";
  let lastChar = "";

  for (const ch of input) {
    const translated = translationMap.get(ch) ?? ch;

    // If squeezing, skip consecutive duplicates of characters in the squeeze set.
    if (squeeze && translated === lastChar && squeezeSet.has(translated)) {
      continue;
    }

    output += translated;
    lastChar = translated;
  }

  return output;
}

/**
 * Delete characters: remove all characters that are in set1.
 */
function deleteChars(input: string, set1: string[], squeeze: boolean, squeezeSet: string[]): string {
  const deleteSet = new Set(set1);
  const squeezeLookup = new Set(squeezeSet);

  let output = "";
  let lastChar = "";

  for (const ch of input) {
    if (deleteSet.has(ch)) {
      continue;
    }

    // If squeezing with a squeeze set, compress consecutive duplicates.
    if (squeeze && ch === lastChar && squeezeLookup.has(ch)) {
      continue;
    }

    output += ch;
    lastChar = ch;
  }

  return output;
}

/**
 * Squeeze characters: replace consecutive duplicates in the set with
 * a single occurrence.
 */
function squeezeChars(input: string, set: string[]): string {
  const squeezeSet = new Set(set);
  let output = "";
  let lastChar = "";

  for (const ch of input) {
    if (ch === lastChar && squeezeSet.has(ch)) {
      continue;
    }
    output += ch;
    lastChar = ch;
  }

  return output;
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then translate/delete characters.
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
        process.stderr.write(`tr: ${error.message}\n`);
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

  const useComplement = !!flags["complement"];
  const deleteMode = !!flags["delete"];
  const squeeze = !!flags["squeeze_repeats"];
  const truncateSet1 = !!flags["truncate_set1"];

  const set1Spec = args["set1"] as string;
  const set2Spec = args["set2"] as string | undefined;

  // --- Step 4: Expand character sets ---------------------------------------

  let set1 = expandSet(set1Spec);
  const set2 = set2Spec ? expandSet(set2Spec) : [];

  // Apply complement if requested.
  if (useComplement) {
    set1 = complementSet(set1);
  }

  // Truncate set1 to length of set2 if requested.
  if (truncateSet1 && set2.length > 0) {
    set1 = set1.slice(0, set2.length);
  }

  // --- Step 5: Read stdin and transform ------------------------------------

  let input: string;
  try {
    input = fs.readFileSync(0, "utf-8");
  } catch {
    return;
  }

  let output: string;

  if (deleteMode) {
    // Delete mode: remove characters in set1. If -s is also given, squeeze
    // characters in set2.
    output = deleteChars(input, set1, squeeze, set2);
  } else if (squeeze && set2.length === 0) {
    // Squeeze-only mode (no translation): squeeze characters in set1.
    output = squeezeChars(input, set1);
  } else if (set2.length > 0) {
    // Translate mode: replace characters in set1 with set2.
    output = translateChars(input, set1, set2, squeeze);
  } else {
    // No operation -- just pass through.
    output = input;
  }

  process.stdout.write(output);
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

main();
