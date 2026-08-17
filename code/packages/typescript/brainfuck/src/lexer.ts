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
 * Grammar Source
 * --------------
 *
 * The token grammar is compiled ahead of time from `brainfuck.tokens` (in
 * `code/grammars/brainfuck/`) into `./_token_grammar.ts`, a native
 * TypeScript object literal. This avoids reading and parsing a grammar
 * file from disk at runtime -- which would break once this package is
 * published, since a published npm package never ships the monorepo's
 * `code/grammars/` tree.
 *
 * (This package also has a `parser.ts` that needs a compiled *parser*
 * grammar. Since both live in this same `src/` directory, the generated
 * files use distinct names -- `_token_grammar.ts` here and
 * `_parser_grammar.ts` for the parser -- to avoid a collision.)
 */

import { grammarTokenize } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";

import { TOKEN_GRAMMAR } from "./_token_grammar.js";

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
   * Run the generic grammar-driven tokenizer against the pre-compiled
   * TOKEN_GRAMMAR constant. This is the same engine used for JSON,
   * Starlark, Python, Ruby, and other languages -- the only thing that
   * changes between languages is the grammar.
   */
  return grammarTokenize(source, TOKEN_GRAMMAR);
}
