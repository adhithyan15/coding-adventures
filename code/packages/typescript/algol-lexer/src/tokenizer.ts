/**
 * ALGOL 60 Lexer -- tokenizes ALGOL 60 source text using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `grammarTokenize` function
 * from the `@coding-adventures/lexer` package. It loads the `algol.tokens` grammar
 * file and delegates all tokenization work to the generic engine.
 *
 * What Is ALGOL 60?
 * -----------------
 *
 * ALGOL 60 (ALGOrithmic Language, 1960) is one of the most historically important
 * programming languages ever created. Although few programmers write ALGOL today,
 * nearly every modern language descends from it:
 *
 *   - **Pascal** -- direct descendent; Niklaus Wirth designed it after working on ALGOL 68
 *   - **C** -- ALGOL syntax adapted for systems programming
 *   - **Ada** -- ALGOL-family language for safety-critical systems
 *   - **Simula** -- ALGOL 60 extended with classes; the first OOP language
 *   - **Java, Rust, Go, Swift** -- indirect descendants through C, Pascal, and Simula
 *
 * ALGOL 60 Lexical Rules
 * ----------------------
 *
 * ALGOL 60's lexical conventions differ notably from C-family languages:
 *
 *   - **Case-insensitive keywords** -- BEGIN, Begin, begin all produce the same token.
 *     The lexer normalizes keywords to lowercase before reclassification.
 *   - **Word-based boolean operators** -- `and`, `or`, `not`, `impl`, `eqv` are
 *     keywords, not symbols like &&, ||, !.
 *   - **Keyword-based arithmetic** -- `div` and `mod` are keywords (integer division
 *     and modulo), not symbols.
 *   - **Single-quoted strings** -- unlike C/Java, ALGOL strings use ' ' delimiters.
 *     No escape sequences -- a quote cannot appear inside a string literal.
 *   - **Assignment with :=** -- borrowed from mathematics. The = symbol means equality
 *     comparison, avoiding the C mistake of confusing assignment and equality.
 *   - **Two exponentiation syntaxes** -- `**` (Fortran convention) and `^` (caret).
 *     The original ALGOL report used ↑ which could not be typed on hardware of the era.
 *   - **Comments** -- `comment <text>;` -- the comment keyword triggers comment-skip
 *     mode; everything up to the next semicolon is silently consumed.
 *
 * Token Types
 * -----------
 *
 * From algol.tokens (value tokens):
 *
 *   | Token       | Example      | Description                                        |
 *   |-------------|--------------|-----------------------------------------------------|
 *   | REAL_LIT    | 3.14, 1.5E3  | Real number (decimal point or exponent or both)     |
 *   | INTEGER_LIT | 0, 42, 1000  | Integer (digits only, no decimal point)             |
 *   | STRING_LIT  | 'hello'      | Single-quoted string (no escapes)                   |
 *   | IDENT       | x, sum, A1   | Identifier (reclassified to keyword if in list)     |
 *
 * Operators and delimiters:
 *
 *   | Token     | Example | Description                                           |
 *   |-----------|---------|-------------------------------------------------------|
 *   | ASSIGN    | :=      | Assignment (must match before COLON to avoid ambiguity) |
 *   | POWER     | **      | Exponentiation (must match before STAR to avoid ambiguity) |
 *   | LEQ       | <=      | Less-than-or-equal (ASCII for ≤)                      |
 *   | GEQ       | >=      | Greater-than-or-equal (ASCII for ≥)                   |
 *   | NEQ       | !=      | Not-equal (ASCII for ≠)                               |
 *   | PLUS      | +       | Addition                                              |
 *   | MINUS     | -       | Subtraction / unary negation                          |
 *   | STAR      | *       | Multiplication                                        |
 *   | SLASH     | /       | Division                                              |
 *   | CARET     | ^       | Exponentiation (alternate to **)                      |
 *   | EQ        | =       | Equality test (not assignment!)                       |
 *   | LT        | <       | Less than                                             |
 *   | GT        | >       | Greater than                                          |
 *   | LPAREN    | (       | Open parenthesis                                      |
 *   | RPAREN    | )       | Close parenthesis                                     |
 *   | LBRACKET  | [       | Open bracket (array subscript)                        |
 *   | RBRACKET  | ]       | Close bracket                                         |
 *   | SEMICOLON | ;       | Statement separator / comment terminator              |
 *   | COMMA     | ,       | List separator                                        |
 *   | COLON     | :       | Bound pair separator in array declarations            |
 *
 * Keywords (reclassified from IDENT):
 *
 *   begin, end, if, then, else, for, do, step, until, while, goto, switch,
 *   procedure, own, array, label, value, integer, real, boolean, string,
 *   true, false, not, and, or, impl, eqv, div, mod, comment
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `algol.tokens` file lives in `code/grammars/` at the repository root.
 * We navigate from this module's location up to that directory:
 *
 *     src/tokenizer.ts -> algol-lexer/ -> typescript/ -> packages/ -> code/ -> grammars/
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
 * In CommonJS, __dirname is a global. In ESM, it does not exist — we must
 * derive it from import.meta.url, which gives the file URL of the current
 * module (e.g., "file:///path/to/tokenizer.ts"). The fileURLToPath + dirname
 * pattern converts this to a directory path.
 */
const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Navigate from src/ up to the grammars/ directory.
 *
 * The path traversal:
 *   __dirname    = .../algol-lexer/src/
 *   ..           = .../algol-lexer/
 *   ../..        = .../typescript/
 *   ../../..     = .../packages/
 *   ../../../..  = .../code/
 *   + grammars   = .../code/grammars/
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const VALID_VERSIONS = new Set(["algol60"]);

function resolveTokensPath(version = "algol60"): string {
  if (!VALID_VERSIONS.has(version)) {
    const valid = Array.from(VALID_VERSIONS).sort().join(", ");
    throw new Error(`Unknown ALGOL version ${JSON.stringify(version)}. Valid versions: ${valid}`);
  }
  return join(GRAMMARS_DIR, "algol", `${version}.tokens`);
}

/**
 * Tokenize ALGOL 60 source text and return an array of tokens.
 *
 * The function reads the `algol.tokens` grammar file, parses it into a
 * `TokenGrammar` object (which contains regex patterns, literal patterns,
 * keyword lists, and skip patterns), then passes the source text and grammar
 * to the generic `grammarTokenize` engine.
 *
 * The generic engine handles:
 *   - Pattern matching (regexes and literals)
 *   - Keyword reclassification (IDENT tokens are reclassified to keyword types
 *     when the identifier text matches a keyword in the grammar)
 *   - Skip patterns (whitespace and comments)
 *   - Position tracking (line and column for each token)
 *
 * Comment handling:
 *   The `algol.tokens` grammar defines a COMMENT skip pattern:
 *     /comment[^;]*;/
 *   This matches `comment` followed by any text up to the next semicolon.
 *   The entire comment (including the word "comment" and the final ";") is
 *   consumed silently. No COMMENT token is emitted.
 *
 * @param source - The ALGOL 60 source text to tokenize.
 * @returns An array of Token objects. The last token is always EOF.
 *
 * @example
 *     const tokens = tokenizeAlgol("begin integer x; x := 42 end");
 *     // [Token(begin, "begin"), Token(integer, "integer"), Token(IDENT, "x"),
 *     //  Token(SEMICOLON, ";"), Token(IDENT, "x"), Token(ASSIGN, ":="),
 *     //  Token(INTEGER_LIT, "42"), Token(end, "end"), Token(EOF, "")]
 *
 * @example
 *     const tokens = tokenizeAlgol("x := 1 + 2 * 3");
 *     // [Token(IDENT, "x"), Token(ASSIGN, ":="), Token(INTEGER_LIT, "1"),
 *     //  Token(PLUS, "+"), Token(INTEGER_LIT, "2"), Token(STAR, "*"),
 *     //  Token(INTEGER_LIT, "3"), Token(EOF, "")]
 *
 * @example
 *     // Comments are silently consumed:
 *     const tokens = tokenizeAlgol("comment this is ignored; x := 1");
 *     // [Token(IDENT, "x"), Token(ASSIGN, ":="), Token(INTEGER_LIT, "1"),
 *     //  Token(EOF, "")]
 */
export function tokenizeAlgol(source: string, version = "algol60"): Token[] {
  /**
   * Read the grammar file from disk. In a production system, you would
   * cache this — but for an educational codebase, reading on every call
   * keeps the code simple and makes the data flow obvious.
   */
  const grammarText = readFileSync(resolveTokensPath(version), "utf-8");

  /**
   * Parse the grammar text into a structured TokenGrammar object.
   * This extracts:
   *   - Token patterns (regex and literal)
   *   - Keywords (for reclassification of IDENT tokens)
   *   - Skip patterns (whitespace, comments)
   *
   * Unlike JSON, ALGOL has keywords, boolean word-operators, and comment syntax.
   * All of these are specified in algol.tokens; the generic engine handles them.
   */
  const grammar = parseTokenGrammar(grammarText);

  /**
   * Run the generic grammar-driven tokenizer. The ALGOL grammar is richer than
   * JSON, but the same engine handles both. The keyword list in the grammar
   * causes IDENT tokens (e.g., "begin", "integer") to be reclassified to their
   * specific keyword types after matching.
   *
   * Post-processing: the generic grammarTokenize engine emits "KEYWORD" as the
   * type for all keyword tokens. ALGOL 60 has many distinct keywords and callers
   * expect the specific keyword name as the type (e.g. "begin" not "KEYWORD").
   * We reclassify here so that the ALGOL lexer public API returns the lowercase
   * keyword word as the token type, matching the per-language convention.
   */
  /**
   * Build a set of keyword strings for case-insensitive reclassification.
   *
   * The generic grammarTokenize engine does case-sensitive keyword lookup:
   * "begin" (lowercase) matches and is returned as type "KEYWORD", but
   * "BEGIN" or "Begin" do not match and stay as "NAME". ALGOL 60 keywords
   * are case-insensitive by convention, so we handle both cases here:
   *
   *   1. "KEYWORD" tokens: already reclassified — just lowercase the type.
   *   2. "NAME" tokens whose lowercase value is in the keyword set: reclassify
   *      to the lowercase keyword name (handles "BEGIN", "Begin", "bEgIn", etc.)
   */
  const keywordSet = new Set(grammar.keywords);
  return grammarTokenize(source, grammar).map((token) => {
    if (token.type === "KEYWORD") {
      return { ...token, type: token.value.toLowerCase() };
    }
    if (token.type === "NAME" && keywordSet.has(token.value.toLowerCase())) {
      return { ...token, type: token.value.toLowerCase() };
    }
    return token;
  });
}
