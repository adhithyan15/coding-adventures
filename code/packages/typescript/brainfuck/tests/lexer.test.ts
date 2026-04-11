/**
 * Tests for the Brainfuck Lexer (TypeScript).
 *
 * These tests verify that the grammar-driven lexer correctly tokenizes Brainfuck
 * source text when loaded with the `brainfuck.tokens` grammar file.
 *
 * Brainfuck is one of the simplest languages to tokenize: exactly 8 single-character
 * command tokens, and everything else is silently discarded as a comment. There are
 * no strings, no numbers, no keywords, and no multi-character tokens.
 *
 * Test Categories
 * ---------------
 *
 *   1. **All 8 command tokens** -- each of ><+-.,[] produces the correct token type
 *   2. **Comment skipping** -- non-command characters are silently discarded
 *   3. **Position tracking** -- line and column numbers are recorded correctly
 *   4. **Empty source** -- empty string produces only EOF
 *   5. **Canonical "++[>+<-]"** -- the classic loop pattern
 *   6. **Mixed commands and comments** -- realistic Brainfuck programs with annotations
 *   7. **Multi-line programs** -- newlines are skipped and line counter advances
 *   8. **EOF is always last** -- the last token in every result is EOF
 */

import { describe, it, expect } from "vitest";
import { tokenizeBrainfuck } from "../src/lexer.js";
import type { Token } from "@coding-adventures/lexer";

/**
 * Helper: extract just the token types from a Brainfuck source string.
 * Excludes EOF to make assertions on command sequences more concise.
 */
function commandTypes(source: string): string[] {
  return tokenizeBrainfuck(source)
    .map((t) => t.type)
    .filter((type) => type !== "EOF");
}

/**
 * Helper: extract just the token values from a Brainfuck source string.
 * Excludes EOF.
 */
function commandValues(source: string): string[] {
  return tokenizeBrainfuck(source)
    .filter((t) => t.type !== "EOF")
    .map((t) => t.value);
}

describe("all 8 command tokens", () => {
  it("tokenizes the RIGHT command: >", () => {
    /**
     * `>` moves the data pointer one cell to the right.
     * This is the simplest possible Brainfuck source: a single command.
     * The lexer should produce exactly one command token (RIGHT) plus EOF.
     */
    const tokens = tokenizeBrainfuck(">");
    expect(tokens[0].type).toBe("RIGHT");
    expect(tokens[0].value).toBe(">");
  });

  it("tokenizes the LEFT command: <", () => {
    /**
     * `<` moves the data pointer one cell to the left.
     */
    const tokens = tokenizeBrainfuck("<");
    expect(tokens[0].type).toBe("LEFT");
    expect(tokens[0].value).toBe("<");
  });

  it("tokenizes the INC command: +", () => {
    /**
     * `+` increments the byte at the current data pointer position.
     * After 256 increments, the cell wraps back to 0 (unsigned byte arithmetic).
     */
    const tokens = tokenizeBrainfuck("+");
    expect(tokens[0].type).toBe("INC");
    expect(tokens[0].value).toBe("+");
  });

  it("tokenizes the DEC command: -", () => {
    /**
     * `-` decrements the byte at the current data pointer position.
     * After 256 decrements from 0, the cell wraps to 255.
     */
    const tokens = tokenizeBrainfuck("-");
    expect(tokens[0].type).toBe("DEC");
    expect(tokens[0].value).toBe("-");
  });

  it("tokenizes the OUTPUT command: .", () => {
    /**
     * `.` outputs the byte at the current data pointer as an ASCII character.
     * For example, if the cell contains 65, the output is 'A'.
     */
    const tokens = tokenizeBrainfuck(".");
    expect(tokens[0].type).toBe("OUTPUT");
    expect(tokens[0].value).toBe(".");
  });

  it("tokenizes the INPUT command: ,", () => {
    /**
     * `,` reads one byte from the input stream into the current cell.
     */
    const tokens = tokenizeBrainfuck(",");
    expect(tokens[0].type).toBe("INPUT");
    expect(tokens[0].value).toBe(",");
  });

  it("tokenizes the LOOP_START command: [", () => {
    /**
     * `[` marks the start of a loop. If the current cell is zero, execution
     * jumps to the matching `]`. Otherwise execution continues into the loop body.
     *
     * Note: `[` alone is unmatched and would cause a parser error, but the
     * *lexer* only tokenizes -- it does not check bracket matching.
     */
    const tokens = tokenizeBrainfuck("[");
    expect(tokens[0].type).toBe("LOOP_START");
    expect(tokens[0].value).toBe("[");
  });

  it("tokenizes the LOOP_END command: ]", () => {
    /**
     * `]` marks the end of a loop. If the current cell is nonzero, execution
     * jumps back to the matching `[`. Otherwise execution continues past the loop.
     *
     * The `[-]` idiom (clear-cell loop) is idiomatic Brainfuck.
     */
    const tokens = tokenizeBrainfuck("]");
    expect(tokens[0].type).toBe("LOOP_END");
    expect(tokens[0].value).toBe("]");
  });
});

describe("comment skipping", () => {
  it("discards alphabetic comment text", () => {
    /**
     * In Brainfuck, any character that isn't one of the 8 commands is a comment.
     * Letters like "abc" are typical comment text and should produce no tokens.
     *
     * The COMMENT skip pattern in brainfuck.tokens matches:
     *   /[^><+\-.,\[\] \t\r\n]+/
     * which covers all non-command, non-whitespace characters.
     */
    const types = commandTypes("abc");
    expect(types).toEqual([]);
  });

  it("discards comments around commands", () => {
    /**
     * Brainfuck programmers typically annotate their code like:
     *   +++ set cell 0 to 3
     *
     * The text "set cell 0 to 3" should be silently discarded,
     * leaving only the 3 INC tokens.
     */
    const types = commandTypes("+++ set cell 0 to 3");
    expect(types).toEqual(["INC", "INC", "INC"]);
  });

  it("discards numeric comment text (digits)", () => {
    /**
     * Digits are not Brainfuck commands. `42` is just two comment characters.
     * This is a common gotcha: `+ 10 times` doesn't mean "run + 10 times";
     * the `10` is just comment text.
     */
    const types = commandTypes("42");
    expect(types).toEqual([]);
  });

  it("discards mixed comment text between commands", () => {
    /**
     * Commands interspersed with arbitrary comment text.
     * The sequence `> move right < move left` should tokenize as RIGHT, LEFT.
     */
    const types = commandTypes("> move right < move left");
    expect(types).toEqual(["RIGHT", "LEFT"]);
  });
});

describe("empty source", () => {
  it("returns only EOF for empty string", () => {
    /**
     * An empty Brainfuck program is valid -- it simply does nothing.
     * The lexer should return exactly one token: EOF.
     *
     * This is important because the parser must handle the empty-program
     * case: `program = { instruction }` allows zero instructions.
     */
    const tokens = tokenizeBrainfuck("");
    expect(tokens).toHaveLength(1);
    expect(tokens[0].type).toBe("EOF");
  });

  it("returns only EOF for whitespace-only source", () => {
    /**
     * A source containing only whitespace is also an empty Brainfuck program.
     * All whitespace is consumed by the WHITESPACE skip pattern.
     */
    const tokens = tokenizeBrainfuck("   \t\n  ");
    expect(tokens).toHaveLength(1);
    expect(tokens[0].type).toBe("EOF");
  });
});

describe("EOF is always last", () => {
  it("the last token is always EOF", () => {
    /**
     * The EOF token is a synthetic sentinel that the generic lexer always
     * appends. It tells the parser "there are no more tokens." The parser
     * uses EOF to verify it has consumed the entire input.
     */
    const tokens = tokenizeBrainfuck("+-");
    const lastToken = tokens[tokens.length - 1];
    expect(lastToken.type).toBe("EOF");
  });
});

describe("position tracking", () => {
  it("tracks the correct column for commands on the first line", () => {
    /**
     * Every token includes position information: the line number and column
     * number where it starts. Both are 1-indexed (first line is 1, first
     * column is 1).
     *
     * For "><", the `>` is at column 1 and `<` is at column 2.
     */
    const tokens = tokenizeBrainfuck("><");
    expect(tokens[0].line).toBe(1);
    expect(tokens[0].column).toBe(1);
    expect(tokens[1].line).toBe(1);
    expect(tokens[1].column).toBe(2);
  });

  it("tracks line numbers across newlines", () => {
    /**
     * When commands appear on different lines, the line counter must advance.
     * The WHITESPACE skip pattern consumes newlines, which causes the lexer
     * engine to increment the line counter.
     *
     * Source:
     *   +     <- line 1
     *   -     <- line 2
     */
    const tokens = tokenizeBrainfuck("+\n-");
    expect(tokens[0].line).toBe(1);
    expect(tokens[1].line).toBe(2);
  });

  it("tracks column after whitespace gap", () => {
    /**
     * If commands are separated by spaces on the same line, the column
     * counter advances past the spaces. The `+` at position 1 and
     * `-` at position 5 (after 3 spaces).
     */
    const tokens = tokenizeBrainfuck("+   -");
    expect(tokens[0].column).toBe(1);
    expect(tokens[1].column).toBe(5);
  });
});

describe("canonical ++[>+<-] pattern", () => {
  it("tokenizes ++[>+<-] into exactly 8 command tokens plus EOF", () => {
    /**
     * The sequence `++[>+<-]` is a classic Brainfuck idiom:
     *
     *   ++   -- set cell 0 to 2
     *   [    -- loop while cell 0 is nonzero
     *     >  -- move to cell 1
     *     +  -- increment cell 1
     *     <  -- move back to cell 0
     *     -  -- decrement cell 0 (loop counter)
     *   ]    -- repeat until cell 0 is zero
     *
     * Result: cell 0 = 0, cell 1 = 2.  (Copies the value from cell 0 to cell 1
     * while decrementing cell 0 to zero.)
     *
     * This produces exactly 8 command tokens + EOF = 9 tokens total.
     */
    const tokens = tokenizeBrainfuck("++[>+<-]");
    expect(tokens).toHaveLength(9); // 8 commands + EOF
  });

  it("tokenizes ++[>+<-] into the correct token type sequence", () => {
    /**
     * Each of the 8 characters in `++[>+<-]` maps to a specific token type.
     * This test verifies the exact sequence.
     */
    const types = commandTypes("++[>+<-]");
    expect(types).toEqual([
      "INC",        // +
      "INC",        // +
      "LOOP_START", // [
      "RIGHT",      // >
      "INC",        // +
      "LEFT",       // <
      "DEC",        // -
      "LOOP_END",   // ]
    ]);
  });

  it("tokenizes ++[>+<-] with comments correctly", () => {
    /**
     * The same pattern with inline comments (as a Brainfuck programmer
     * might write it). Comments should be silently discarded, producing
     * the same 8 command tokens.
     */
    const source = "++ setup [>+<-] copy loop";
    const types = commandTypes(source);
    expect(types).toEqual([
      "INC", "INC", "LOOP_START", "RIGHT", "INC", "LEFT", "DEC", "LOOP_END",
    ]);
  });
});

describe("realistic programs", () => {
  it("tokenizes all 8 distinct commands in a single source", () => {
    /**
     * A source containing all 8 Brainfuck commands, each exactly once.
     * This verifies that the lexer can distinguish all 8 token types
     * in a single pass.
     *
     * ><+-.,[]  -- one of each command, in the order they appear in the grammar
     */
    const types = commandTypes("><+-.,[]");
    expect(types).toEqual([
      "RIGHT", "LEFT", "INC", "DEC", "OUTPUT", "INPUT", "LOOP_START", "LOOP_END",
    ]);
  });

  it("tokenizes a nested loop structure", () => {
    /**
     * Nested loops are common in Brainfuck. `[[]]` is a trivially nested
     * loop (inner loop is empty). The lexer doesn't check bracket matching --
     * that's the parser's job -- but it must produce the correct sequence.
     */
    const types = commandTypes("[[]]");
    expect(types).toEqual(["LOOP_START", "LOOP_START", "LOOP_END", "LOOP_END"]);
  });
});
