/**
 * Brainfuck Lexer -- tokenizes Brainfuck source using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `grammarTokenize` function
 * from the `@coding-adventures/lexer` package. It loads the `brainfuck.tokens`
 * grammar file and delegates all tokenization work to the generic engine.
 *
 * What Is Brainfuck?
 * ------------------
 *
 * Brainfuck is a minimalist esoteric programming language created by Urban
 * Mueller in 1993. It has exactly **eight** meaningful characters:
 *
 *   | Character | Token      | Meaning                                       |
 *   |-----------|------------|-----------------------------------------------|
 *   | `>`       | RIGHT      | Move the data pointer one cell to the right   |
 *   | `<`       | LEFT       | Move the data pointer one cell to the left    |
 *   | `+`       | INC        | Increment the byte at the data pointer        |
 *   | `-`       | DEC        | Decrement the byte at the data pointer        |
 *   | `.`       | OUTPUT     | Output the byte at the data pointer as ASCII  |
 *   | `,`       | INPUT      | Read one byte from input into the current cell|
 *   | `[`       | LOOP_START | Jump past matching `]` if current cell is zero|
 *   | `]`       | LOOP_END   | Jump back to matching `[` if current cell != 0|
 *
 * Everything else is treated as a **comment** and silently discarded. There is
 * no dedicated comment syntax -- any character that isn't a command is a comment.
 * This is an intentional feature of Brainfuck: programmers annotate their code
 * by writing normal prose directly in the source, knowing the 8 command
 * characters are unambiguous.
 *
 * Brainfuck vs. JSON for Tokenization
 * ------------------------------------
 *
 * Brainfuck is even simpler to tokenize than JSON:
 *
 *   - **No strings** -- no quoted sequences to handle
 *   - **No numbers** -- no numeric literals to parse
 *   - **No keywords** -- no identifier reclassification needed
 *   - **Single-character tokens only** -- every meaningful token is exactly
 *     one character, with no multi-character tokens at all
 *   - **Universal comment syntax** -- anything that isn't a command is noise
 *
 * The simplicity makes Brainfuck an excellent second grammar (after JSON) for
 * validating that the grammar-driven infrastructure works for radically
 * different language shapes.
 *
 * Token Types
 * -----------
 *
 * The `brainfuck.tokens` file defines these token types:
 *
 *   | Token      | Example | Description                              |
 *   |------------|---------|------------------------------------------|
 *   | RIGHT      | `>`     | Move data pointer right                  |
 *   | LEFT       | `<`     | Move data pointer left                   |
 *   | INC        | `+`     | Increment cell                           |
 *   | DEC        | `-`     | Decrement cell                           |
 *   | OUTPUT     | `.`     | Output cell as ASCII                     |
 *   | INPUT      | `,`     | Read input into cell                     |
 *   | LOOP_START | `[`     | Begin loop                               |
 *   | LOOP_END   | `]`     | End loop                                 |
 *   | EOF        | (synth) | Synthetic end-of-input marker            |
 *
 * Skip Patterns
 * -------------
 *
 * The grammar defines two skip patterns:
 *   - WHITESPACE `/[ \t\r\n]+/` -- handles line endings (updates line counter)
 *   - COMMENT `/[^><+\-.,\[\] \t\r\n]+/` -- absorbs non-command, non-whitespace
 *
 * The two-pattern split ensures that line/column tracking remains accurate.
 * If a single COMMENT pattern consumed newlines, the lexer's internal line
 * counter would drift. By routing `\n` through WHITESPACE, the engine sees
 * every newline and increments the line counter correctly.
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `brainfuck.tokens` file lives in `code/grammars/` at the repository root.
 * We navigate from this module's location up to that directory:
 *
 *     src/lexer.ts -> brainfuck/ -> typescript/ -> packages/ -> code/ -> grammars/
 *
 * That is 4 levels up from `src/`:
 *   __dirname       = .../brainfuck/src/
 *   ..              = .../brainfuck/
 *   ../..           = .../typescript/
 *   ../../..        = .../packages/
 *   ../../../..     = .../code/
 *   + grammars      = .../code/grammars/
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseTokenGrammar } from "@coding-adventures/grammar-tools";
import { grammarTokenize } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";

/**
 * Resolve __dirname for ESM modules.
 *
 * In CommonJS, __dirname is a global. In ESM, it does not exist -- we must
 * derive it from import.meta.url, which gives the file URL of the current
 * module (e.g., "file:///path/to/lexer.ts"). The fileURLToPath + dirname
 * pattern converts this to a directory path string.
 */
const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Navigate from src/ up to the grammars/ directory.
 *
 * The path traversal:
 *   __dirname   = .../brainfuck/src/
 *   ..          = .../brainfuck/
 *   ../..       = .../typescript/
 *   ../../..    = .../packages/
 *   ../../../.. = .../code/
 *   + grammars  = .../code/grammars/
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const BF_TOKENS_PATH = join(GRAMMARS_DIR, "brainfuck.tokens");

/**
 * Tokenize Brainfuck source text and return an array of tokens.
 *
 * This function reads the `brainfuck.tokens` grammar file, parses it into a
 * `TokenGrammar` object, then passes the source text and grammar to the
 * generic `grammarTokenize` engine.
 *
 * The generic engine handles:
 *   - Pattern matching (literal single-character tokens)
 *   - Skip patterns (whitespace and comments)
 *   - Position tracking (line and column for each token)
 *
 * Only the 8 command characters produce tokens. All other characters
 * (letters, digits, punctuation, spaces, newlines) are silently consumed
 * by the two skip patterns defined in brainfuck.tokens.
 *
 * @param source - The Brainfuck source text to tokenize.
 * @returns An array of Token objects. The last token is always EOF.
 *
 * @example
 *     // Tokenize the "hello world" Brainfuck program's loop nucleus:
 *     const tokens = tokenizeBrainfuck("++[>+<-]");
 *     // Token(INC, "+"), Token(INC, "+"),
 *     // Token(LOOP_START, "["),
 *     //   Token(RIGHT, ">"), Token(INC, "+"), Token(LEFT, "<"), Token(DEC, "-"),
 *     // Token(LOOP_END, "]"),
 *     // Token(EOF, "")
 *
 * @example
 *     // Comments are silently discarded:
 *     const tokens = tokenizeBrainfuck("+ increment the cell");
 *     // [Token(INC, "+"), Token(EOF, "")]
 *     // "increment the cell" produces no tokens -- it's all comment text
 *
 * @example
 *     // All 8 commands in one pass:
 *     const tokens = tokenizeBrainfuck("><+-.,[]");
 *     // RIGHT, LEFT, INC, DEC, OUTPUT, INPUT, LOOP_START, LOOP_END, EOF
 */
export function tokenizeBrainfuck(source: string): Token[] {
  /**
   * Read the grammar file from disk. In a production system, you would
   * cache this -- but for an educational codebase, reading on every call
   * keeps the code simple and makes the data flow obvious.
   */
  const grammarText = readFileSync(BF_TOKENS_PATH, "utf-8");

  /**
   * Parse the grammar text into a structured TokenGrammar object.
   * This extracts:
   *   - Token patterns (8 single-character literals)
   *   - Skip patterns (WHITESPACE and COMMENT)
   *
   * Brainfuck has no keyword reclassification, no reserved words,
   * and no indentation mode -- the grammar is as simple as it gets.
   */
  const grammar = parseTokenGrammar(grammarText);

  /**
   * Run the generic grammar-driven tokenizer. This is the same engine
   * used for JSON, Starlark, Python, Ruby, and other languages -- the
   * only thing that changes between languages is the grammar file.
   */
  return grammarTokenize(source, grammar);
}
