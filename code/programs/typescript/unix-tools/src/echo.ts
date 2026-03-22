/**
 * echo -- display a line of text.
 *
 * === What This Program Does ===
 *
 * This is a reimplementation of the `echo` utility in TypeScript. It
 * writes its arguments to standard output, separated by spaces, followed
 * by a newline (unless `-n` is specified).
 *
 * === How echo Works ===
 *
 * At its core, echo is simple:
 *
 *     echo hello world    =>    "hello world\n"
 *
 * The arguments are joined with spaces and a newline is appended.
 * Three flags modify this behavior:
 *
 * - `-n`: Suppress the trailing newline. Useful when building prompts
 *         or composing output with other commands.
 *
 * - `-e`: Enable interpretation of backslash escapes. Without this flag,
 *         `\n` is printed literally as two characters. With `-e`, it
 *         becomes an actual newline character.
 *
 * - `-E`: Disable escape interpretation (the default). This exists so
 *         you can explicitly override a previous `-e` in an alias or
 *         script.
 *
 * === Backslash Escape Table ===
 *
 * When `-e` is active, the following escape sequences are interpreted:
 *
 *     Escape    Meaning              ASCII Code
 *     ------    -------              ----------
 *     \\        Backslash            0x5C
 *     \a        Alert (bell)         0x07
 *     \b        Backspace            0x08
 *     \f        Form feed            0x0C
 *     \n        Newline              0x0A
 *     \r        Carriage return      0x0D
 *     \t        Horizontal tab       0x09
 *     \0NNN     Octal value          (up to 3 digits)
 *
 * === POSIX vs GNU ===
 *
 * POSIX echo has implementation-defined behavior for `-n` and escapes.
 * We follow the GNU coreutils convention: `-n` suppresses the newline,
 * `-e` enables escapes, `-E` disables them.
 *
 * @module echo
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
const SPEC_FILE = path.resolve(__dirname, "..", "echo.json");

// ---------------------------------------------------------------------------
// Business Logic: Escape Sequence Processing
// ---------------------------------------------------------------------------

/**
 * Interpret backslash escape sequences in a string.
 *
 * This function processes the escape sequences listed in the module
 * documentation. It walks through the string character by character,
 * replacing recognized escape sequences with their corresponding
 * characters.
 *
 * === How the Parser Works ===
 *
 * We use an index-based loop rather than a regex because octal escapes
 * (`\0NNN`) require looking ahead up to 3 characters, and we need to
 * handle the `\\` escape (which should not trigger further processing
 * of the second backslash).
 *
 * The algorithm:
 * 1. Walk through the string one character at a time.
 * 2. When we encounter `\`, look at the next character.
 * 3. If it's a recognized escape letter, emit the corresponding char.
 * 4. If it's `0`, consume up to 3 octal digits and emit the char.
 * 5. Otherwise, emit the backslash and the character literally.
 */
function interpretEscapes(input: string): string {
  // We build the output in an array for efficiency. String concatenation
  // in a loop creates O(n^2) intermediate strings; array.push + join is O(n).
  const output: string[] = [];
  let i = 0;

  while (i < input.length) {
    if (input[i] === "\\" && i + 1 < input.length) {
      // We found a backslash. Look at the next character to determine
      // which escape sequence this is.
      const next = input[i + 1];

      switch (next) {
        case "\\":
          output.push("\\");
          i += 2;
          break;
        case "a":
          // Alert (bell) -- the terminal beeps.
          output.push("\x07");
          i += 2;
          break;
        case "b":
          // Backspace -- moves the cursor back one position.
          output.push("\b");
          i += 2;
          break;
        case "f":
          // Form feed -- advances to the next "page" (rarely used today).
          output.push("\f");
          i += 2;
          break;
        case "n":
          // Newline -- the most common escape sequence.
          output.push("\n");
          i += 2;
          break;
        case "r":
          // Carriage return -- moves cursor to beginning of line.
          output.push("\r");
          i += 2;
          break;
        case "t":
          // Horizontal tab.
          output.push("\t");
          i += 2;
          break;
        case "0": {
          // Octal escape: \0 followed by up to 3 octal digits.
          // Examples: \0101 = 'A' (65 decimal), \012 = newline (10 decimal)
          let octal = "";
          let j = i + 2;
          while (j < input.length && j < i + 5 && input[j] >= "0" && input[j] <= "7") {
            octal += input[j];
            j++;
          }
          if (octal.length > 0) {
            output.push(String.fromCharCode(parseInt(octal, 8)));
          } else {
            // \0 with no digits = null character
            output.push("\0");
          }
          i = j;
          break;
        }
        default:
          // Unrecognized escape -- emit both characters literally.
          output.push("\\");
          output.push(next);
          i += 2;
          break;
      }
    } else {
      // Not a backslash -- emit the character as-is.
      output.push(input[i]);
      i++;
    }
  }

  return output.join("");
}

// ---------------------------------------------------------------------------
// Main: parse args via CLI Builder, then echo them.
// ---------------------------------------------------------------------------

/**
 * Entry point.
 *
 * The flow is:
 * 1. Parse arguments with CLI Builder.
 * 2. Handle --help and --version.
 * 3. Join the positional arguments with spaces.
 * 4. If -e is set, interpret escape sequences.
 * 5. If -n is NOT set, append a newline.
 * 6. Write to stdout.
 */
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
        process.stderr.write(`echo: ${error.message}\n`);
      }
      process.exit(1);
    }
    throw err;
  }

  // --- Step 2: Dispatch on result type -------------------------------------

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
  // Join all positional arguments with spaces, apply flags, and output.

  const flags = (result as { flags: Record<string, unknown> }).flags;
  const args = (result as { arguments: Record<string, unknown> }).arguments;

  // The "strings" argument is variadic, so it comes as an array.
  // If no arguments were given, it will be an empty array or undefined.
  const strings = (args["strings"] as string[]) || [];

  // Join all arguments with a single space, just like the real echo.
  let output = strings.join(" ");

  // If -e is set (and -E is not), interpret backslash escapes.
  // -E and -e are mutually exclusive (enforced by CLI Builder).
  if (flags["enable_escapes"]) {
    output = interpretEscapes(output);
  }

  // Append a trailing newline unless -n was specified.
  if (!flags["no_newline"]) {
    output += "\n";
  }

  process.stdout.write(output);
}

// ---------------------------------------------------------------------------
// Run the program.
// ---------------------------------------------------------------------------

main();
