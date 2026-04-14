/**
 * Tokenizer — Breaking Source Code into Tokens
 * =============================================
 *
 * This module implements a *lexer* (also called a *tokenizer* or *scanner*),
 * which is the very first phase of understanding a programming language.
 * Before a computer can execute code like `x = 1 + 2`, it needs to break
 * that raw text into meaningful chunks. Those chunks are called **tokens**.
 *
 * Think of it like reading a sentence in English. When you see:
 *
 *     The cat sat on the mat.
 *
 * Your brain automatically groups the letters into words: "The", "cat",
 * "sat", "on", "the", "mat", and the period ".". You don't think about
 * individual letters — you think about *words* and *punctuation*. A lexer
 * does the same thing for source code.
 *
 * Given the input `x = 1 + 2`, the lexer produces:
 *
 *     NAME("x")  EQUALS("=")  NUMBER("1")  PLUS("+")  NUMBER("2")  EOF
 *
 * Each of these is a **Token** — a small labeled piece of text. The label
 * (like NAME or NUMBER) is called the **token type**, and the text itself
 * (like "x" or "1") is called the **token value**.
 *
 * Why is this useful?
 * -------------------
 *
 * The lexer simplifies everything that comes after it. The *parser* (the
 * next stage) doesn't have to worry about whitespace, or whether a number
 * is one digit or five digits. It just sees a clean stream of tokens to
 * work with. This separation of concerns is a fundamental principle of
 * compiler design, established in the earliest days of computing.
 *
 * Design: Language-Agnostic
 * -------------------------
 *
 * This lexer is designed to be **language-agnostic**. The core logic —
 * reading numbers, reading names, recognizing operators — is the same
 * across many programming languages. The only thing that changes is *which
 * words are keywords*. In Python, `if` is a keyword. In Ruby, `elsif` is
 * a keyword (instead of Python's `elif`). By making the keyword list
 * configurable via `LexerConfig`, we can reuse the same lexer for multiple
 * languages.
 */

import type { Token } from "./token.js";
import { classifyChar, newTokenizerDFA } from "./tokenizer-dfa.js";

// ---------------------------------------------------------------------------
// Lexer Configuration
// ---------------------------------------------------------------------------

/**
 * Configuration that makes the lexer adaptable to different languages.
 *
 * The key insight is that most programming languages share the same *kinds*
 * of tokens — numbers, strings, operators, identifiers — but differ in
 * which words are **keywords**. By externalizing the keyword list into a
 * config object, we can reuse the same lexer engine for Python, Ruby,
 * JavaScript, or any other language.
 *
 * Example — Python configuration:
 *
 *     const pythonConfig: LexerConfig = {
 *       keywords: [
 *         "if", "else", "elif", "while", "for", "def", "return",
 *         "class", "import", "from", "as", "True", "False", "None",
 *       ],
 *     };
 *
 * Example — Ruby configuration:
 *
 *     const rubyConfig: LexerConfig = {
 *       keywords: [
 *         "if", "else", "elsif", "end", "while", "for", "def", "return",
 *         "class", "require", "puts", "true", "false", "nil",
 *       ],
 *     };
 */
export interface LexerConfig {
  /**
   * A list of words that should be classified as KEYWORD tokens instead
   * of NAME tokens. The lexer checks every identifier against this list.
   * If no config is provided to the Lexer, no words are treated as
   * keywords (everything is a NAME).
   */
  readonly keywords: readonly string[];
}

// ---------------------------------------------------------------------------
// Lexer Error
// ---------------------------------------------------------------------------

/**
 * An error encountered during tokenization.
 *
 * When the lexer encounters something it doesn't understand — like an
 * unterminated string `"hello` or an unexpected character `@` — it throws
 * this error with a helpful message that includes the line and column
 * where the problem occurred.
 */
export class LexerError extends Error {
  public readonly line: number;
  public readonly column: number;

  constructor(message: string, line: number, column: number) {
    super(`Lexer error at ${line}:${column}: ${message}`);
    this.name = "LexerError";
    this.line = line;
    this.column = column;
  }
}

// ---------------------------------------------------------------------------
// Simple Token Lookup Table
// ---------------------------------------------------------------------------

/**
 * A mapping from single characters to their token types.
 *
 * This table handles "simple" tokens — characters that always mean the
 * same thing regardless of context. We look up the character in this
 * map to instantly know what token type it is.
 *
 * Note that `=` is NOT in this table because it requires lookahead
 * (it could be `=` or `==`).
 */
const SIMPLE_TOKENS: ReadonlyMap<string, string> = new Map([
  ["+", "PLUS"],
  ["-", "MINUS"],
  ["*", "STAR"],
  ["/", "SLASH"],
  ["(", "LPAREN"],
  [")", "RPAREN"],
  [",", "COMMA"],
  [":", "COLON"],
  [";", "SEMICOLON"],
  ["{", "LBRACE"],
  ["}", "RBRACE"],
  ["[", "LBRACKET"],
  ["]", "RBRACKET"],
  [".", "DOT"],
  ["!", "BANG"],
]);

// ---------------------------------------------------------------------------
// Character Classification Helpers
// ---------------------------------------------------------------------------

/**
 * Check if a character is a digit (0-9).
 *
 * We use a simple character code comparison instead of a regex because
 * this function is called for every character in the input, and the
 * comparison is faster than creating and matching a regex.
 */
function isDigit(ch: string): boolean {
  return ch >= "0" && ch <= "9";
}

/**
 * Check if a character is a letter (a-z, A-Z) or underscore.
 *
 * This determines whether a character can *start* an identifier.
 * Identifiers in most languages begin with a letter or underscore.
 */
function isAlpha(ch: string): boolean {
  return (ch >= "a" && ch <= "z") || (ch >= "A" && ch <= "Z") || ch === "_";
}

/**
 * Check if a character is alphanumeric (letter, digit, or underscore).
 *
 * This determines whether a character can *continue* an identifier.
 * After the first character, identifiers allow digits too.
 */
function isAlphaNumeric(ch: string): boolean {
  return isAlpha(ch) || isDigit(ch);
}

// ---------------------------------------------------------------------------
// The Lexer
// ---------------------------------------------------------------------------

/**
 * The main lexer — reads source code character by character and produces tokens.
 *
 * How It Works — The Big Picture
 * ------------------------------
 *
 * Imagine you're reading a book one letter at a time, with your finger
 * pointing at the current letter. The lexer works the same way:
 *
 * 1. It maintains a **position** (like your finger) that points to the
 *    current character in the source code.
 * 2. It looks at the current character and decides what kind of token
 *    is starting here.
 * 3. It reads as many characters as needed to complete that token.
 * 4. It records the token and moves on.
 * 5. It repeats until it reaches the end of the input.
 *
 * For example, given `x = 42`:
 *
 * - Position 0: sees `x` (a letter) -> reads an identifier -> emits NAME("x")
 * - Position 1: sees ` ` (a space) -> skips whitespace
 * - Position 2: sees `=` -> peeks ahead, next is ` ` (not `=`) -> emits EQUALS("=")
 * - Position 3: sees ` ` -> skips whitespace
 * - Position 4: sees `4` (a digit) -> reads all digits -> emits NUMBER("42")
 * - Position 6: end of input -> emits EOF
 *
 * The "peek ahead" step for `=` is important. When the lexer sees `=`,
 * it doesn't know yet whether this is `=` (assignment) or `==` (comparison).
 * It needs to look at the *next* character without consuming it. This is
 * called **lookahead**, and it's one of the fundamental techniques in lexer
 * design.
 *
 * Usage:
 *
 *     const tokens = tokenize("x = 1 + 2");
 *
 * With a language-specific configuration:
 *
 *     const config = { keywords: ["if", "else", "while"] };
 *     const tokens = tokenize("if x == 1", config);
 *     // The word "if" will be a KEYWORD token, not a NAME token
 */

/**
 * Tokenize source code into a list of tokens.
 *
 * This is the main entry point. It scans the source code character by
 * character, deciding what kind of token starts at each position and
 * delegating to the appropriate reading logic.
 *
 * The algorithm is a classic **dispatch on first character**:
 *
 * 1. If the character is a space or tab -> skip whitespace
 * 2. If the character is a newline -> emit a NEWLINE token
 * 3. If the character is a digit -> read a number
 * 4. If the character is a letter or underscore -> read a name/keyword
 * 5. If the character is a double quote -> read a string
 * 6. If the character is `=` -> peek ahead to distinguish `=` from `==`
 * 7. If the character is a simple operator/delimiter -> look it up in the table
 * 8. Otherwise -> raise an error (unexpected character)
 *
 * After all characters are processed, an EOF token is appended.
 *
 * @param source - The raw source code text to tokenize.
 * @param config - Optional configuration specifying language-specific keywords.
 *     If omitted, no words will be treated as keywords.
 * @returns A list of Token objects, always ending with an EOF token.
 * @throws LexerError if an unexpected character is encountered, or if a
 *     string literal is not properly terminated.
 */
export function tokenize(source: string, config?: LexerConfig): Token[] {
  // -- State variables --
  // These track the lexer's position as it scans through the source code.
  let pos = 0;
  let line = 1;
  let column = 1;
  const tokens: Token[] = [];

  // Pre-compute the keyword set once, not on every identifier check.
  // Using a Set gives us O(1) membership testing instead of O(n) with an array.
  const keywordSet: ReadonlySet<string> = new Set(config?.keywords ?? []);

  // -- Core character-reading functions --
  //
  // These are the low-level "machinery" of the lexer. They move the
  // position forward and tell us what character we're looking at.

  /**
   * Return the character at the current position, or null if at end.
   *
   * This is how the lexer "sees" the current character. Returning
   * null at the end (instead of throwing an error) makes it easy
   * to write loops like `while (currentChar() !== null)`.
   */
  function currentChar(): string | null {
    if (pos < source.length) {
      return source[pos];
    }
    return null;
  }

  /**
   * Look at the *next* character without advancing the position.
   *
   * This is the "lookahead" operation. It's essential for distinguishing
   * tokens that start the same way:
   * - `=` vs `==`
   *
   * The lexer sees `=` and thinks, "Is this assignment or comparison?"
   * It peeks at the next character to decide, without moving forward.
   * If the next character is `=`, it's `==`. Otherwise, it's just `=`.
   */
  function peek(): string | null {
    const peekPos = pos + 1;
    if (peekPos < source.length) {
      return source[peekPos];
    }
    return null;
  }

  /**
   * Consume the current character and move to the next one.
   *
   * This is the fundamental "step forward" operation. Every time the
   * lexer reads a character that belongs to the current token, it calls
   * advance() to move past it.
   *
   * The function also updates the line and column counters. When it sees
   * a newline character, it increments the line counter and resets the
   * column to 1 (the start of a new line). Otherwise, it just increments
   * the column.
   */
  function advance(): string {
    const char = source[pos];
    pos += 1;

    if (char === "\n") {
      line += 1;
      column = 1;
    } else {
      column += 1;
    }

    return char;
  }

  /**
   * Skip over spaces and tabs (but NOT newlines).
   *
   * Whitespace between tokens is meaningless in most contexts — `x=1`
   * and `x = 1` mean the same thing. The lexer skips over it silently.
   *
   * However, **newlines are NOT skipped** here. Newlines get their own
   * token (NEWLINE) because in some languages (like Python), newlines
   * are significant — they mark the end of a statement. The main
   * tokenize loop handles newlines explicitly.
   */
  function skipWhitespace(): void {
    while (currentChar() !== null && " \t\r".includes(currentChar()!)) {
      advance();
    }
  }

  // -- Token-reading functions --
  //
  // Each of these functions reads one specific kind of token. They are
  // called from the main loop when it identifies what kind of token is
  // starting at the current position.

  /**
   * Read an integer literal (a sequence of digits).
   *
   * When the main loop sees a digit character (0-9), it delegates to
   * this function. We keep reading characters as long as they are digits,
   * building up the number string.
   *
   * For example, if the source has `42 + 3`, and we're at position 0:
   * - Read '4', advance -> "4"
   * - Read '2', advance -> "42"
   * - See ' ' (space) — not a digit, so stop
   * - Emit NUMBER("42")
   */
  function readNumber(): Token {
    const startLine = line;
    const startColumn = column;
    const digits: string[] = [];

    while (currentChar() !== null && isDigit(currentChar()!)) {
      digits.push(advance());
    }

    return {
      type: "NUMBER",
      value: digits.join(""),
      line: startLine,
      column: startColumn,
    };
  }

  /**
   * Read an identifier or keyword.
   *
   * Identifiers (names) follow the same rules in almost every language:
   * - They start with a letter or underscore: a-z, A-Z, _
   * - They continue with letters, digits, or underscores: a-z, A-Z, 0-9, _
   *
   * After reading the full name, we check whether it's a **keyword** —
   * a reserved word with special meaning in the language. If it is, we
   * emit a KEYWORD token; otherwise, we emit a NAME token.
   *
   * For example, with Python keywords configured:
   * - `x` -> NAME("x")
   * - `if` -> KEYWORD("if")
   * - `if_condition` -> NAME("if_condition")  (not a keyword — extra chars)
   */
  function readName(): Token {
    const startLine = line;
    const startColumn = column;
    const chars: string[] = [];

    // Read the first character (must be a letter or underscore).
    // Read subsequent characters (letters, digits, or underscores).
    while (currentChar() !== null && isAlphaNumeric(currentChar()!)) {
      chars.push(advance());
    }

    const name = chars.join("");

    // Is this word a keyword in the configured language?
    const tokenType = keywordSet.has(name) ? "KEYWORD" : "NAME";

    return {
      type: tokenType,
      value: name,
      line: startLine,
      column: startColumn,
    };
  }

  /**
   * Read a double-quoted string literal.
   *
   * String literals are delimited by double quotes: `"Hello, World!"`.
   * The lexer reads everything between the opening and closing quotes,
   * handling **escape sequences** along the way.
   *
   * Escape sequences let you include special characters inside a string:
   * - `\"` -> a literal double-quote (without ending the string)
   * - `\\` -> a literal backslash
   * - `\n` -> a newline character
   * - `\t` -> a tab character
   *
   * For example, `"He said \"hi\""` produces the value: `He said "hi"`
   *
   * The opening quote has already been identified by the caller but NOT
   * consumed — this function consumes it and everything through the
   * closing quote.
   */
  function readString(): Token {
    const startLine = line;
    const startColumn = column;
    const chars: string[] = [];

    // Consume the opening double quote.
    advance();

    while (true) {
      const current = currentChar();

      if (current === null) {
        // We reached the end of input without finding a closing quote.
        // This is an error — the programmer forgot to close the string.
        throw new LexerError(
          "Unterminated string literal",
          startLine,
          startColumn,
        );
      }

      if (current === '"') {
        // Found the closing quote. Consume it and stop.
        advance();
        break;
      }

      if (current === "\\") {
        // Escape sequence — the backslash says "the next character
        // is special, don't treat it normally."
        advance(); // consume the backslash
        const escaped = currentChar();

        if (escaped === null) {
          throw new LexerError(
            "Unterminated string literal (ends with backslash)",
            startLine,
            startColumn,
          );
        }

        // Map escape codes to their actual characters.
        const escapeMap: Record<string, string> = {
          n: "\n",
          t: "\t",
          "\\": "\\",
          '"': '"',
        };

        chars.push(escapeMap[escaped] ?? escaped);
        advance();
      } else {
        // A regular character — just add it to the string.
        chars.push(current);
        advance();
      }
    }

    return {
      type: "STRING",
      value: chars.join(""),
      line: startLine,
      column: startColumn,
    };
  }

  // -- Main tokenization loop --
  //
  // The loop uses the TOKENIZER_DFA to classify the current character and
  // determine which sub-routine should handle it. After each sub-routine
  // finishes (emitting a token or skipping whitespace), the DFA is reset
  // to "start" and the loop repeats.

  const dfa = newTokenizerDFA();

  while (true) {
    const char = currentChar();
    const charClass = classifyChar(char);
    const nextState = dfa.process(charClass);

    if (nextState === "at_whitespace") {
      skipWhitespace();
    } else if (nextState === "at_newline") {
      const token: Token = {
        type: "NEWLINE",
        value: "\\n",
        line,
        column,
      };
      advance();
      tokens.push(token);
    } else if (nextState === "in_number") {
      tokens.push(readNumber());
    } else if (nextState === "in_name") {
      tokens.push(readName());
    } else if (nextState === "in_string") {
      tokens.push(readString());
    } else if (nextState === "in_equals") {
      const startLine = line;
      const startColumn = column;
      advance();

      if (currentChar() === "=") {
        advance();
        tokens.push({
          type: "EQUALS_EQUALS",
          value: "==",
          line: startLine,
          column: startColumn,
        });
      } else {
        tokens.push({
          type: "EQUALS",
          value: "=",
          line: startLine,
          column: startColumn,
        });
      }
    } else if (nextState === "in_operator") {
      const simpleType = SIMPLE_TOKENS.get(char!);
      const token: Token = {
        type: simpleType!,
        value: char!,
        line,
        column,
      };
      advance();
      tokens.push(token);
    } else if (nextState === "done") {
      break;
    } else if (nextState === "error") {
      throw new LexerError(
        `Unexpected character: ${JSON.stringify(char)}`,
        line,
        column,
      );
    }

    // Reset the DFA back to "start" for the next character.
    dfa.reset();
  }

  // --- End of input ---
  // Append the EOF token so the parser knows the input is finished.
  tokens.push({
    type: "EOF",
    value: "",
    line,
    column,
  });

  return tokens;
}
