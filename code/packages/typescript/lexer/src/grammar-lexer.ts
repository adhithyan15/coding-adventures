/**
 * Grammar-Driven Lexer — Tokenization from .tokens Files
 * =======================================================
 *
 * The hand-written `tokenize` function in `tokenizer.ts` hardcodes which
 * characters map to which tokens. That works well for a single language,
 * but what if you want to tokenize Python *and* Ruby *and* JavaScript
 * with the same codebase? You would need to rewrite the character-
 * dispatching logic for each language.
 *
 * This module takes a different approach, inspired by classic tools like
 * [Lex](https://en.wikipedia.org/wiki/Lex_(software)) and
 * [Flex](https://en.wikipedia.org/wiki/Flex_(lexical_analyser_generator)).
 * Instead of hardcoding patterns in TypeScript, we read token definitions
 * from a `.tokens` file (parsed by the `grammar-tools` package) and use
 * those definitions to drive tokenization at runtime.
 *
 * How It Works — The Big Picture
 * ------------------------------
 *
 * A `.tokens` file looks like this:
 *
 *     NAME   = /[a-zA-Z_][a-zA-Z0-9_]* /
 *     NUMBER = /[0-9]+/
 *     PLUS   = "+"
 *     MINUS  = "-"
 *
 *     keywords:
 *       if
 *       else
 *
 * Each line defines a token: a name and a pattern. The pattern is either a
 * regex (`/.../`) or a literal string (`"..."`). The `grammar-tools`
 * package parses this file into a `TokenGrammar` object — a structured list
 * of `TokenDefinition` objects plus a keyword list.
 *
 * The `grammarTokenize` function takes that `TokenGrammar` and does the
 * following:
 *
 * 1. **Compile** each token definition into a JavaScript `RegExp` object.
 *    Literal patterns are escaped so that characters like `+` and `*` are
 *    treated as literal characters, not regex operators.
 *
 * 2. **At each position** in the source code, try each compiled pattern in
 *    order (first match wins). This is the "priority" mechanism — if two
 *    patterns could match at the same position, the one that appears first
 *    in the `.tokens` file wins.
 *
 * 3. **Emit a Token** with the matched type and value, using the same
 *    `Token` interface as the hand-written lexer.
 *
 * Because both lexers produce identical `Token` objects, downstream
 * consumers (the parser, the evaluator) do not care which lexer generated
 * the tokens. You can swap one for the other freely.
 *
 * Why Two Lexers?
 * ---------------
 *
 * The hand-written `tokenize` is the **reference implementation** — clear,
 * well-documented, and easy to step through in a debugger. The
 * `grammarTokenize` is the **grammar-driven alternative** — flexible,
 * language-agnostic, and data-driven. Having both lets us:
 *
 * - Verify correctness by comparing their outputs
 * - Demonstrate two fundamentally different approaches to the same problem
 * - Use the hand-written lexer for teaching and the grammar-driven one
 *   for production grammar work
 */

import type { TokenGrammar } from "@coding-adventures/grammar-tools";

import type { Token } from "./token.js";
import { LexerError } from "./tokenizer.js";

// ---------------------------------------------------------------------------
// Compiled Pattern
// ---------------------------------------------------------------------------

/**
 * A pre-compiled regex pattern paired with its token name.
 *
 * During initialization, each token definition from the grammar is compiled
 * into a RegExp. We store the compiled pattern alongside the token name
 * so that during tokenization we can match and emit in one step.
 */
interface CompiledPattern {
  readonly name: string;
  readonly pattern: RegExp;
}

// ---------------------------------------------------------------------------
// Escape helper for literal patterns
// ---------------------------------------------------------------------------

/**
 * Escape a string so that all regex-special characters are treated literally.
 *
 * For example, "+" becomes "\\+", which matches a literal + character.
 * This is equivalent to Python's `re.escape()` or Go's `regexp.QuoteMeta()`.
 *
 * Why do we need this? Because literal patterns in .tokens files like `"+"`
 * are meant to match the exact character `+`, not be interpreted as the
 * regex quantifier "one or more of the preceding element".
 */
function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

// ---------------------------------------------------------------------------
// Token Type Resolution
// ---------------------------------------------------------------------------

/**
 * Map of known token names to their canonical type strings.
 *
 * This mirrors the Python `TokenType` enum: we map grammar token names
 * (like "PLUS") to the type strings used in Token objects. If a token
 * name from the grammar isn't in this map, we fall back to "NAME" as
 * a safe default.
 */
const KNOWN_TOKEN_TYPES: ReadonlySet<string> = new Set([
  "NAME",
  "NUMBER",
  "STRING",
  "KEYWORD",
  "PLUS",
  "MINUS",
  "STAR",
  "SLASH",
  "EQUALS",
  "EQUALS_EQUALS",
  "LPAREN",
  "RPAREN",
  "COMMA",
  "COLON",
  "NEWLINE",
  "EOF",
]);

/**
 * Resolve a grammar token name and matched value to a token type string.
 *
 * This function handles two things:
 *
 * 1. **Keyword detection**: If the grammar token name is "NAME" and the
 *    matched value is in the keyword set, we return "KEYWORD" instead of
 *    "NAME". This is how `if` becomes a keyword while `iffy` stays a name.
 *
 * 2. **Name-to-type mapping**: We check if the token name is a known
 *    token type. If there is no match (which shouldn't happen with a
 *    well-formed `.tokens` file), we fall back to "NAME".
 *
 * @param tokenName - The token name from the grammar definition
 *     (e.g., "NAME", "PLUS", "NUMBER").
 * @param value - The actual matched text from the source code.
 * @param keywordSet - The set of keywords to check against.
 * @returns The appropriate token type string.
 */
function resolveTokenType(
  tokenName: string,
  value: string,
  keywordSet: ReadonlySet<string>,
): string {
  // Check if it's a NAME that should be reclassified as a KEYWORD.
  if (tokenName === "NAME" && keywordSet.has(value)) {
    return "KEYWORD";
  }

  // Map grammar token names to known type strings.
  if (KNOWN_TOKEN_TYPES.has(tokenName)) {
    return tokenName;
  }

  // If no direct mapping exists, default to NAME.
  // This provides a safe fallback for custom token names that
  // don't have a corresponding known type.
  return "NAME";
}

// ---------------------------------------------------------------------------
// Escape Sequence Processing
// ---------------------------------------------------------------------------

/**
 * Process escape sequences in a string value.
 *
 * This handles the same escape sequences as the hand-written lexer:
 *
 * - `\n` becomes a newline character
 * - `\t` becomes a tab character
 * - `\\` becomes a literal backslash
 * - `\"` becomes a literal double quote
 * - Any other `\X` becomes just `X` (unknown escapes pass through)
 *
 * This ensures that `grammarTokenize` produces identical string values
 * to the hand-written `tokenize`.
 *
 * @param s - The raw string content (after removing surrounding quotes).
 * @returns The string with escape sequences resolved.
 */
function processEscapes(s: string): string {
  const result: string[] = [];
  let i = 0;

  while (i < s.length) {
    if (s[i] === "\\" && i + 1 < s.length) {
      const escapeMap: Record<string, string> = {
        n: "\n",
        t: "\t",
        "\\": "\\",
        '"': '"',
      };
      const nextChar = s[i + 1];
      result.push(escapeMap[nextChar] ?? nextChar);
      i += 2;
    } else {
      result.push(s[i]);
      i += 1;
    }
  }

  return result.join("");
}

// ---------------------------------------------------------------------------
// The Grammar-Driven Lexer
// ---------------------------------------------------------------------------

/**
 * Tokenize source code using a grammar (parsed from a `.tokens` file).
 *
 * Instead of hardcoded character-matching logic, this lexer:
 *
 * 1. Compiles each token definition's pattern into a regex
 * 2. At each position, tries each regex in definition order (first match wins)
 * 3. Emits a Token with the matched type and value
 *
 * This is fundamentally different from the hand-written `tokenize`:
 *
 * - **Hand-written**: dispatch on first character, custom read methods
 * - **Grammar-driven**: regex matching in priority order
 *
 * Both produce the same Token objects, so the parser does not care
 * which lexer generated them.
 *
 * Usage:
 *
 *     import { parseTokenGrammar } from "@coding-adventures/grammar-tools";
 *     import { grammarTokenize } from "@coding-adventures/lexer";
 *
 *     const grammar = parseTokenGrammar(fs.readFileSync("python.tokens", "utf-8"));
 *     const tokens = grammarTokenize("x = 1 + 2", grammar);
 *
 * @param source - The raw source code text to tokenize.
 * @param grammar - A TokenGrammar object (typically parsed from a `.tokens`
 *     file using `parseTokenGrammar` from grammar-tools).
 * @returns A list of Token objects, always ending with an EOF token.
 * @throws LexerError if an unexpected character is encountered that does
 *     not match any token pattern.
 */
export function grammarTokenize(source: string, grammar: TokenGrammar): Token[] {
  // -- State variables --
  let pos = 0;
  let line = 1;
  let column = 1;
  const tokens: Token[] = [];

  // Pre-compute keyword set for fast membership testing.
  // When the lexer matches a NAME token, it checks this set to decide
  // whether the value should be reclassified as a KEYWORD.
  const keywordSet: ReadonlySet<string> = new Set(grammar.keywords);

  // Compile token patterns into regex objects.
  // -------------------------------------------
  // Order matters here — patterns are tried in the order they appear
  // in the .tokens file. This is the "first match wins" rule that
  // Lex/Flex use. For example, if "==" is defined before "=", then
  // at a position where the source has "==", the "==" pattern will
  // match first, and we will never even try "=".
  //
  // For regex patterns (isRegex=true), we compile the pattern as-is.
  // For literal patterns (isRegex=false), we escape the pattern so
  // that characters like + and * are treated literally.
  const patterns: CompiledPattern[] = grammar.definitions.map((defn) => {
    const patternSource = defn.isRegex ? defn.pattern : escapeRegExp(defn.pattern);
    return {
      name: defn.name,
      pattern: new RegExp(patternSource),
    };
  });

  // -- Internal advance helper --

  /**
   * Move position forward by one character, tracking line and column.
   *
   * This is identical in spirit to the hand-written lexer's advance
   * function. When we encounter a newline character, we increment the
   * line counter and reset the column to 1. For all other characters,
   * we just increment the column.
   */
  function advance(): void {
    if (pos < source.length) {
      if (source[pos] === "\n") {
        line += 1;
        column = 1;
      } else {
        column += 1;
      }
      pos += 1;
    }
  }

  // -- Main tokenization loop --

  while (pos < source.length) {
    const char = source[pos];

    // --- Skip whitespace (spaces, tabs, carriage returns) ---
    // Just like the hand-written lexer, we skip horizontal whitespace
    // silently. Newlines are NOT whitespace here — they get their own
    // token because languages like Python care about line endings.
    if (char === " " || char === "\t" || char === "\r") {
      advance();
      continue;
    }

    // --- Newlines become NEWLINE tokens ---
    // We handle newlines specially (outside the pattern-matching loop)
    // because newlines are structural — they mark line boundaries.
    // The hand-written lexer does the same thing.
    if (char === "\n") {
      tokens.push({
        type: "NEWLINE",
        value: "\\n",
        line,
        column,
      });
      advance();
      continue;
    }

    // --- Try each pattern in priority order (first match wins) ---
    // This is the core of the grammar-driven approach. We take a
    // slice of the source from the current position to the end,
    // and try to match each pattern at the START of that slice
    // (using regex's exec() with the match anchored to the start).
    let matched = false;
    const remaining = source.slice(pos);

    for (const { name, pattern } of patterns) {
      // We need to match at the beginning of the remaining string.
      // JavaScript's regex doesn't have Python's match() (which
      // anchors to the start). We use a fresh exec() and check
      // that the match starts at index 0.
      const match = pattern.exec(remaining);
      if (match !== null && match.index === 0) {
        const value = match[0];
        const startLine = line;
        const startColumn = column;

        // Determine the token type for this match.
        const tokenType = resolveTokenType(name, value, keywordSet);

        // Handle STRING tokens specially: strip surrounding quotes
        // and process escape sequences, so the token value contains
        // the actual string content (matching the hand-written lexer).
        if (name === "STRING") {
          const inner = value.slice(1, -1); // strip quotes
          const processed = processEscapes(inner);
          tokens.push({
            type: tokenType,
            value: processed,
            line: startLine,
            column: startColumn,
          });
        } else {
          tokens.push({
            type: tokenType,
            value,
            line: startLine,
            column: startColumn,
          });
        }

        // Advance position by the number of characters matched.
        // We advance one character at a time so that line/column
        // tracking stays accurate (newlines inside strings, etc.).
        for (let i = 0; i < value.length; i++) {
          advance();
        }

        matched = true;
        break;
      }
    }

    if (!matched) {
      throw new LexerError(
        `Unexpected character: ${JSON.stringify(char)}`,
        line,
        column,
      );
    }
  }

  // --- Append the EOF sentinel ---
  // Just like the hand-written lexer, we always end with EOF so the
  // parser has a clean stop signal.
  tokens.push({
    type: "EOF",
    value: "",
    line,
    column,
  });

  return tokens;
}
