/**
 * Dartmouth BASIC Lexer -- tokenizes the 1964 Dartmouth BASIC language.
 *
 * This module is a **thin wrapper** around the generic `GrammarLexer` class
 * from the `@coding-adventures/lexer` package. It loads the
 * `dartmouth_basic.tokens` grammar file and applies two post-tokenize hooks
 * to handle the language's two special lexical challenges:
 *
 *   1. **LINE_NUM disambiguation** — a bare integer at the start of a line
 *      is a line label, not a numeric expression.
 *   2. **REM suppression** — everything after the keyword REM on a line is
 *      a comment and must be discarded.
 *
 * What Was Dartmouth BASIC?
 * -------------------------
 *
 * Dartmouth BASIC (Beginner's All-purpose Symbolic Instruction Code) was
 * created by John G. Kemeny and Thomas E. Kurtz at Dartmouth College in 1964.
 * It ran on a GE-225 mainframe and was accessed through teletypes — printers
 * with keyboards, no screens. The paper tape scrolled as you typed.
 *
 * Design goals:
 *
 *   - **Approachable for non-science students** — BASIC was the first language
 *     designed to be learned in hours, not semesters.
 *   - **Interactive** — a student could type a program and immediately run it.
 *     Most 1964 computing was batch-processed; Dartmouth BASIC was time-shared.
 *   - **Forgiving** — every variable is pre-initialised to 0. No declarations
 *     needed. No type errors (everything is a number).
 *
 * The language's influence is enormous: it eventually spawned Microsoft BASIC
 * (Gates and Allen's first product, 1975), Applesoft BASIC, GW-BASIC, and
 * hundreds of home computer BASICs of the 1970s–80s. Millions of people learned
 * to program on BASIC dialects.
 *
 * Lexical Structure
 * -----------------
 *
 * Every BASIC program is a sequence of **numbered lines**:
 *
 *     10 LET X = 5
 *     20 PRINT X
 *     30 GOTO 10
 *
 * The line number serves two roles:
 *
 *   1. **Ordering** — lines are sorted by number before execution. You could
 *      type them in any order and the BASIC system would sort them for you.
 *   2. **Addressing** — GOTO 30 means "jump to line 30". Line numbers are
 *      the only branching mechanism in the original 1964 spec.
 *
 * Key differences from modern languages:
 *
 *   - **Uppercase only** — the GE-225 teletypes had no lowercase keys.
 *     The grammar uses `@case_insensitive true` to normalise input.
 *   - **No strings as data** — strings appear only in PRINT and DATA
 *     statements. There are no string variables.
 *   - **Single-letter variables** — variable names are one uppercase letter
 *     (A–Z) optionally followed by one digit (A0–Z9). That's 286 total.
 *   - **All numbers are floats** — even `42` is stored as `42.0` internally.
 *   - **REM for comments** — `10 REM THIS IS A COMMENT`. Everything after
 *     REM on the same line is ignored.
 *   - **NEWLINE is significant** — it terminates each statement. Unlike most
 *     languages, you cannot split a BASIC statement across lines.
 *
 * Token Types
 * -----------
 *
 * The final token stream contains these types:
 *
 *   | Type        | Example          | Description                               |
 *   |-------------|------------------|-------------------------------------------|
 *   | LINE_NUM    | "10", "999"      | Integer at the start of a line (relabeled)|
 *   | NUMBER      | "3.14", "1.5E3"  | Numeric literal in an expression          |
 *   | STRING      | "\"HELLO\""      | Double-quoted string literal              |
 *   | KEYWORD     | "PRINT", "LET"   | Reserved word (always uppercase)          |
 *   | BUILTIN_FN  | "SIN", "LOG"     | One of the 11 built-in math functions     |
 *   | USER_FN     | "FNA", "FNZ"     | User-defined function (FN + one letter)   |
 *   | NAME        | "X", "A1", "B9"  | Variable name (one letter + optional digit)|
 *   | PLUS        | "+"              | Addition                                  |
 *   | MINUS       | "-"              | Subtraction / unary negation              |
 *   | STAR        | "*"              | Multiplication                            |
 *   | SLASH       | "/"              | Division                                  |
 *   | CARET       | "^"              | Exponentiation (right-associative)        |
 *   | EQ          | "="              | Assignment (LET) and equality (IF)        |
 *   | LT          | "<"              | Less than                                 |
 *   | GT          | ">"              | Greater than                              |
 *   | LE          | "<="             | Less-than-or-equal                        |
 *   | GE          | ">="             | Greater-than-or-equal                     |
 *   | NE          | "<>"             | Not-equal                                 |
 *   | LPAREN      | "("              | Open parenthesis                          |
 *   | RPAREN      | ")"              | Close parenthesis                         |
 *   | COMMA       | ","              | List separator / PRINT zone separator     |
 *   | SEMICOLON   | ";"              | PRINT tight separator (no spaces)         |
 *   | NEWLINE     | "\n" or "\r\n"   | Statement terminator (significant!)       |
 *   | EOF         | ""               | End of input                              |
 *
 * The Two Post-Tokenize Hooks
 * ---------------------------
 *
 * **Hook 1: relabelLineNumbers**
 *
 * Problem: the grammar file defines both LINE_NUM and NUMBER with the regex
 * `/[0-9]+/`. The engine matches the first pattern that fits, so bare integers
 * always come out as LINE_NUM from the DFA. But we want:
 *
 *     10 LET X = 5
 *            └─── NUMBER (not LINE_NUM)
 *     ^^
 *     └────────── LINE_NUM
 *
 * Solution: walk the token list and re-label:
 *   - The first non-whitespace token after a NEWLINE (or at position 0) that
 *     has type LINE_NUM stays as LINE_NUM.
 *   - All other LINE_NUM tokens (i.e., numbers in expression position) become
 *     NUMBER tokens.
 *
 * Wait — actually in the grammar, LINE_NUM comes before NUMBER, so integer
 * literals throughout the program would initially be classified as LINE_NUM.
 * The hook reassigns: only the first token at line-start keeps LINE_NUM;
 * all others are relabeled to NUMBER. See the hook implementation for details.
 *
 * **Hook 2: suppressRemContent**
 *
 * Problem: REM introduces a comment that should be invisible to the parser.
 * After a KEYWORD("REM") token, everything up to (but not including) the
 * next NEWLINE token should be dropped from the stream.
 *
 * Solution: walk the token list; once a KEYWORD("REM") is seen, suppress
 * all subsequent tokens until the next NEWLINE.
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `dartmouth_basic.tokens` grammar lives in `code/grammars/` at the
 * repository root. We navigate from this module's compiled location up to
 * that directory:
 *
 *     src/tokenizer.ts
 *         -> dartmouth-basic-lexer/   (one ..)
 *         -> typescript/              (two ..)
 *         -> packages/                (three ..)
 *         -> code/                    (four ..)
 *         + grammars/                 = code/grammars/
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseTokenGrammar } from "@coding-adventures/grammar-tools";
import { GrammarLexer } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";

/**
 * Resolve __dirname for ESM modules.
 *
 * In CommonJS (Node.js pre-ESM), `__dirname` is a built-in global that gives
 * the directory of the current file. In ESM (which TypeScript compiles to),
 * `__dirname` does not exist — only `import.meta.url` is available.
 *
 * `import.meta.url` is a full file URL, e.g.:
 *   "file:///Users/alice/code/packages/typescript/dartmouth-basic-lexer/src/tokenizer.ts"
 *
 * We convert it to a filesystem path with `fileURLToPath`, then take the
 * directory with `dirname`. The result is equivalent to the old `__dirname`.
 */
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

/**
 * Navigate from `src/` four levels up to `code/`, then into `grammars/`.
 *
 * Directory structure relative to this file:
 *
 *     __dirname  = .../dartmouth-basic-lexer/src/
 *     ..         = .../dartmouth-basic-lexer/
 *     ../..      = .../typescript/
 *     ../../..   = .../packages/
 *     ../../../.. = .../code/
 *     + grammars = .../code/grammars/
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const TOKENS_PATH = join(GRAMMARS_DIR, "dartmouth_basic.tokens");

// ---------------------------------------------------------------------------
// Post-Tokenize Hook 0: normalizeCase
// ---------------------------------------------------------------------------

/**
 * Upcase the values of KEYWORD, BUILTIN_FN, USER_FN, and NAME tokens.
 *
 * The `dartmouth_basic.tokens` grammar uses `case_sensitive: false`, which
 * causes the `GrammarLexer` to lowercase the entire source string before
 * matching. This is necessary so that `PRINT`, `Print`, and `print` all
 * match the regex `/print/`. However, the emitted token values are the
 * lowercased matched strings — `"let"`, `"sin"`, `"x"` — not the uppercase
 * forms expected by the public API.
 *
 * This hook normalises them back to uppercase:
 *
 *   KEYWORD("let")     → KEYWORD("LET")
 *   BUILTIN_FN("sin")  → BUILTIN_FN("SIN")
 *   USER_FN("fna")     → USER_FN("FNA")
 *   NAME("x")          → NAME("X")
 *
 * Other token types (NUMBER, STRING, operators, NEWLINE, EOF) are left
 * unchanged — their values are not alphabetic identifiers.
 *
 * Why upcase?
 *
 * The 1964 Dartmouth BASIC teletypes only had uppercase characters. The
 * canonical form of all identifiers is uppercase. The public API should
 * return `KEYWORD("PRINT")`, not `KEYWORD("print")`, so that callers can
 * compare token values with simple string equality against uppercase constants.
 *
 * @param tokens - The raw token list from the grammar engine (with lowercase values).
 * @returns A new token list with identifier values uppercased.
 */
function normalizeCase(tokens: Token[]): Token[] {
  /**
   * Token types whose values should be normalised to uppercase.
   * These are the alphabetic identifier types. NUMBER, STRING, operators,
   * NEWLINE, and EOF contain non-alphabetic values and are left as-is.
   */
  const UPCASE_TYPES = new Set(["KEYWORD", "BUILTIN_FN", "USER_FN", "NAME"]);

  return tokens.map((tok) => {
    if (UPCASE_TYPES.has(tok.type)) {
      return { ...tok, value: tok.value.toUpperCase() };
    }
    return tok;
  });
}

// ---------------------------------------------------------------------------
// Post-Tokenize Hook 1: relabelLineNumbers
// ---------------------------------------------------------------------------

/**
 * Relabel the first NUMBER token on each source line as LINE_NUM.
 *
 * In Dartmouth BASIC, integers serve two distinct roles:
 *
 *   1. **Line labels** — `10 LET X = 5` — the `10` names the line.
 *   2. **Literals** — `LET X = 42` — the `42` is an expression value.
 *   3. **GOTO targets** — `GOTO 30` — the `30` is a branch destination.
 *
 * The grammar cannot distinguish them by pattern alone — both use the same
 * regex `/[0-9]+/`. Instead, we use a **post-tokenize hook** to walk the
 * completed token list and relabel based on position:
 *
 *   - A NUMBER token that appears at the very start of the source (before any
 *     other non-whitespace token) OR immediately after a NEWLINE token gets
 *     relabeled as LINE_NUM.
 *   - All other NUMBER (and LINE_NUM) tokens remain (or become) NUMBER.
 *
 * Why does this work? Because BASIC programs always start a new statement at
 * the beginning of a physical line. The line number is always the first token
 * on each line. By tracking "are we at the start of a line?", we can
 * unambiguously classify integer tokens.
 *
 * Algorithm:
 *
 *   1. Start with `atLineStart = true` (the first token is always at line start).
 *   2. For each token:
 *      - If `atLineStart` and token type is "NUMBER" or "LINE_NUM":
 *          Relabel to LINE_NUM. Set `atLineStart = false`.
 *      - If `atLineStart` and token type is something else (e.g., empty line):
 *          Set `atLineStart = false`. Emit unchanged.
 *      - If token type is "NEWLINE":
 *          Set `atLineStart = true` for the next token.
 *      - Otherwise: emit unchanged.
 *
 * @param tokens - The raw token list from the grammar engine.
 * @returns A new token list with LINE_NUM tokens correctly placed.
 *
 * @example
 *     // "10 LET X = 5"
 *     // Before hook: [NUMBER("10"), KEYWORD("LET"), NAME("X"), EQ("="), LINE_NUM("5"), ...]
 *     // Wait — grammar emits LINE_NUM first. So initial list has LINE_NUM for ALL integers.
 *     // Hook corrects: first on each line keeps LINE_NUM, rest become NUMBER.
 *
 * @example
 *     // "10 GOTO 30\n20 LET X = 1"
 *     // After hook: [LINE_NUM("10"), KEYWORD("GOTO"), NUMBER("30"), NEWLINE,
 *     //              LINE_NUM("20"), KEYWORD("LET"), NAME("X"), EQ, NUMBER("1"), ...]
 */
function relabelLineNumbers(tokens: Token[]): Token[] {
  const result: Token[] = [];
  // We start at line-start because the first token in the source is always
  // at the beginning of the first line.
  let atLineStart = true;

  for (const tok of tokens) {
    if (atLineStart && tok.type === "NUMBER") {
      // A NUMBER (or LINE_NUM matched by regex) at line-start is a line label.
      // Relabel it to LINE_NUM and mark that we are no longer at line-start.
      result.push({ ...tok, type: "LINE_NUM" });
      atLineStart = false;
    } else if (atLineStart && tok.type === "LINE_NUM") {
      // The grammar emits LINE_NUM for all integers (since LINE_NUM regex
      // comes first). Keep those at line-start as LINE_NUM.
      result.push(tok);
      atLineStart = false;
    } else if (tok.type === "LINE_NUM") {
      // A LINE_NUM token NOT at line-start — this is actually a numeric
      // literal in expression position. Relabel it to NUMBER.
      result.push({ ...tok, type: "NUMBER" });
    } else {
      result.push(tok);
    }

    // After every NEWLINE, the next non-whitespace token begins a new line.
    if (tok.type === "NEWLINE") {
      atLineStart = true;
    }
  }

  return result;
}

// ---------------------------------------------------------------------------
// Post-Tokenize Hook 2: suppressRemContent
// ---------------------------------------------------------------------------

/**
 * Suppress all tokens between a REM keyword and the next NEWLINE.
 *
 * `REM` is Dartmouth BASIC's comment syntax:
 *
 *     10 REM THIS IS A COMMENT
 *     20 LET X = 1
 *
 * Everything after `REM` on the same line — "THIS IS A COMMENT" — is a
 * remark and should be invisible to the parser. Only the KEYWORD("REM")
 * token itself is kept (so the parser can recognise it as a valid statement).
 *
 * Why keep the REM token?
 *
 * The parser's grammar for a REM statement is:
 *
 *     rem_stmt := LINE_NUM KEYWORD("REM") NEWLINE
 *
 * If we dropped REM itself, the parser would see:
 *
 *     LINE_NUM NEWLINE
 *
 * which does not match any valid statement rule. Keeping REM lets the parser
 * handle remarks without a special case.
 *
 * Why suppression instead of a grammar rule?
 *
 * The `dartmouth_basic.tokens` grammar has no "consume until end of line"
 * rule. The generic `GrammarLexer` processes tokens one at a time; it does
 * not have a "rest of line" mode. The post-tokenize hook can see the complete
 * list and suppress in bulk — much simpler than extending the grammar engine.
 *
 * Algorithm:
 *
 *   1. Walk the token list with a `suppressing` flag (initially false).
 *   2. If not suppressing, emit the token.
 *   3. After emitting KEYWORD("REM"), set `suppressing = true`.
 *   4. When NEWLINE is seen while suppressing, set `suppressing = false`.
 *      (The NEWLINE itself is also suppressed — wait, no: re-read the spec.)
 *
 * Actually: the NEWLINE must be emitted so the parser knows the statement
 * ended. Let's check: the spec says "suppress all subsequent tokens until
 * (and not including) the next NEWLINE". So NEWLINE is NOT suppressed.
 *
 * Corrected algorithm:
 *   1. If suppressing and token is NEWLINE: stop suppressing and emit NEWLINE.
 *   2. If suppressing and token is not NEWLINE: drop it.
 *   3. If not suppressing: emit the token.
 *   4. After emitting KEYWORD("REM"): start suppressing.
 *
 * @param tokens - The token list after relabelLineNumbers.
 * @returns A new token list with REM comment bodies removed.
 *
 * @example
 *     // "10 REM THIS IS A COMMENT\n20 LET X = 1"
 *     // After hook:
 *     //   [LINE_NUM("10"), KEYWORD("REM"), NEWLINE,
 *     //    LINE_NUM("20"), KEYWORD("LET"), NAME("X"), EQ, NUMBER("1"), NEWLINE, EOF]
 *
 * @example
 *     // "10 REM" (no text after REM, just end of input)
 *     // After hook:
 *     //   [LINE_NUM("10"), KEYWORD("REM"), NEWLINE, EOF]
 *     //   (suppressing stops at NEWLINE; EOF comes after)
 */
function suppressRemContent(tokens: Token[]): Token[] {
  const result: Token[] = [];
  // True after we see a KEYWORD("REM") and before the next NEWLINE.
  let suppressing = false;

  for (const tok of tokens) {
    if (!suppressing) {
      // Normal mode: emit every token.
      result.push(tok);
    } else if (tok.type === "NEWLINE") {
      // End of the commented line: stop suppressing and emit the NEWLINE.
      // The parser needs the NEWLINE to terminate the REM statement.
      suppressing = false;
      result.push(tok);
    }
    // Otherwise: suppressing and not NEWLINE — drop the token silently.

    // Turn on suppression right after KEYWORD("REM") is emitted.
    // We check the type and value so that a hypothetical user variable
    // named REM (not possible in 1964 BASIC, but defensive coding) does
    // not trigger suppression.
    if (tok.type === "KEYWORD" && tok.value === "REM") {
      suppressing = true;
    }
  }

  return result;
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Create a configured `GrammarLexer` for Dartmouth BASIC source text.
 *
 * The returned lexer is pre-configured with the `dartmouth_basic.tokens`
 * grammar and the two post-tokenize hooks. Call `lexer.tokenize()` to run
 * tokenization and apply both hooks.
 *
 * This function is useful when you need to add your own post-tokenize hooks
 * on top of the standard ones, or when you want to inspect the lexer before
 * running it.
 *
 * @param source - The Dartmouth BASIC source text to tokenize.
 * @returns A `GrammarLexer` instance with hooks registered.
 *
 * @example
 *     const lex = createDartmouthBasicLexer("10 LET X = 5\n");
 *     lex.addPostTokenize(myExtraHook);
 *     const tokens = lex.tokenize();
 */
export function createDartmouthBasicLexer(source: string): GrammarLexer {
  /**
   * Read the grammar file from disk.
   *
   * The grammar file is read on each call here. In a production system,
   * you would cache the parsed grammar — but for an educational codebase,
   * reading it every time keeps the data flow explicit and easy to follow.
   *
   * The `readFileSync` call is synchronous and blocks the event loop while
   * the file is read. For tokenizing BASIC programs (which are small),
   * this is fine. A production system might use async I/O or a preloaded
   * grammar passed in as a parameter.
   */
  const grammarText = readFileSync(TOKENS_PATH, "utf-8");

  /**
   * Parse the grammar text into a structured `TokenGrammar` object.
   *
   * `parseTokenGrammar` (from `@coding-adventures/grammar-tools`) reads
   * the `.tokens` file format and extracts:
   *
   *   - Token patterns (both regex and literal string patterns)
   *   - The `keywords:` list (identifiers to reclassify as KEYWORD)
   *   - The `skip:` list (patterns to consume silently, like whitespace)
   *   - The `errors:` pattern (for unknown-character recovery)
   *   - Directives like `@case_insensitive true` and `@version 1`
   *
   * With `case_sensitive: false`, the grammar engine lowercases the entire
   * source string before matching. This means `print`, `Print`, and `PRINT`
   * all match the regex `/print/`. The `normalizeCase` hook then restores
   * token values to uppercase for the public API.
   */
  const grammar = parseTokenGrammar(grammarText);

  /**
   * Create the grammar-driven lexer with the Dartmouth BASIC grammar.
   *
   * `GrammarLexer` (from `@coding-adventures/lexer`) is the generic lexer
   * engine. It compiles the grammar patterns into an efficient matching
   * structure and provides the hook system we use here.
   */
  const lexer = new GrammarLexer(source, grammar);

  /**
   * Register Hook 0: Case normalisation — upcase identifier values.
   *
   * The grammar uses `case_sensitive: false`, which lowercases the source
   * before matching. As a result, all identifier tokens (KEYWORD, BUILTIN_FN,
   * USER_FN, NAME) have lowercase values. This hook restores them to uppercase
   * to match the canonical Dartmouth BASIC 1964 convention.
   *
   * This must run FIRST so that all subsequent hooks see uppercase values.
   * In particular:
   *   - relabelLineNumbers checks token.type (not value), so ordering vs
   *     normalizeCase is irrelevant.
   *   - suppressRemContent checks tok.type === "KEYWORD" && tok.value === "REM".
   *     After normalizeCase, the value is "REM" (uppercase), so this check
   *     works correctly. If suppressRemContent ran first, it would look for
   *     "REM" but find "rem" — and fail to suppress comments.
   *
   * Running normalizeCase first is therefore CRITICAL for correct REM suppression.
   */
  lexer.addPostTokenize(normalizeCase);

  /**
   * Register Hook 1: LINE_NUM disambiguation.
   *
   * This must run BEFORE suppressRemContent because it distinguishes
   * LINE_NUM from NUMBER — and suppressRemContent needs to see the
   * KEYWORD("REM") token cleanly (which it always does, since REM is not
   * a number). The ordering is actually irrelevant for correctness here,
   * but it is conventional to number-relabel first.
   */
  lexer.addPostTokenize(relabelLineNumbers);

  /**
   * Register Hook 2: REM comment suppression.
   *
   * This runs after normalizeCase and relabelLineNumbers. At this point, the
   * token list has uppercase values and correct LINE_NUM/NUMBER labels. The
   * suppression hook walks the list and drops everything between
   * KEYWORD("REM") and the next NEWLINE.
   */
  lexer.addPostTokenize(suppressRemContent);

  return lexer;
}

/**
 * Tokenize Dartmouth BASIC source text and return an array of tokens.
 *
 * This is the primary entry point for the Dartmouth BASIC lexer. It:
 *
 *   1. Loads the `dartmouth_basic.tokens` grammar from disk.
 *   2. Parses the grammar into a `TokenGrammar` object.
 *   3. Runs the `GrammarLexer` engine to produce a raw token list.
 *   4. Applies `relabelLineNumbers` to fix LINE_NUM vs NUMBER.
 *   5. Applies `suppressRemContent` to remove comment bodies.
 *   6. Returns the final cleaned token list.
 *
 * The returned list always ends with an EOF token.
 *
 * NEWLINE tokens are included in the output — they are significant in BASIC
 * (each NEWLINE terminates a statement). If you want to strip them (e.g.,
 * for display purposes), filter them out after calling this function.
 *
 * @param source - The Dartmouth BASIC source text to tokenize.
 *   May contain any mix of uppercase and lowercase (case-insensitive).
 *   May use Unix (`\n`) or Windows (`\r\n`) line endings.
 * @returns An array of `Token` objects ending with EOF.
 *
 * @example
 *     const tokens = tokenizeDartmouthBasic("10 LET X = 5");
 *     // [
 *     //   { type: "LINE_NUM", value: "10",  line: 1, column: 1  },
 *     //   { type: "KEYWORD",  value: "LET", line: 1, column: 4  },
 *     //   { type: "NAME",     value: "X",   line: 1, column: 8  },
 *     //   { type: "EQ",       value: "=",   line: 1, column: 10 },
 *     //   { type: "NUMBER",   value: "5",   line: 1, column: 12 },
 *     //   { type: "NEWLINE",  value: "\n",  line: 1, column: 13 },
 *     //   { type: "EOF",      value: "",    line: 1, column: 14 },
 *     // ]
 *
 * @example
 *     // Case-insensitive input:
 *     const tokens = tokenizeDartmouthBasic("10 print x");
 *     // KEYWORD("PRINT"), NAME("X") — same as "10 PRINT X"
 *
 * @example
 *     // REM comment:
 *     const tokens = tokenizeDartmouthBasic("10 REM THIS IS IGNORED\n20 END");
 *     // [LINE_NUM("10"), KEYWORD("REM"), NEWLINE,
 *     //  LINE_NUM("20"), KEYWORD("END"), NEWLINE, EOF]
 */
export function tokenizeDartmouthBasic(source: string): Token[] {
  const lex = createDartmouthBasicLexer(source);
  return lex.tokenize();
}
