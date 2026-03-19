/**
 * token-grammar.ts — Parser and validator for .tokens files.
 *
 * A .tokens file is a declarative description of the lexical grammar of a
 * programming language. It lists every token the lexer should recognize, in
 * priority order (first match wins), along with an optional keywords section
 * for reserved words.
 *
 * This module solves the "front half" of the grammar-tools pipeline: it reads
 * a plain-text token specification and produces a structured TokenGrammar
 * object that downstream tools (lexer generators, cross-validators) can
 * consume.
 *
 * File format overview
 * --------------------
 *
 * Each non-blank, non-comment line in a .tokens file has one of three forms:
 *
 *   TOKEN_NAME = /regex_pattern/      — a regex-based token
 *   TOKEN_NAME = "literal_string"     — a literal-string token
 *   keywords:                         — begins the keywords section
 *
 * Lines starting with # are comments. Blank lines are ignored.
 *
 * The keywords section lists one reserved word per line (indented). Keywords
 * are identifiers that the lexer recognizes as NAME tokens but then
 * reclassifies. For instance, `if` matches the NAME pattern but is promoted
 * to an IF keyword.
 *
 * Design decisions
 * ----------------
 *
 * Why hand-parse instead of using regex or a parser library? Because the
 * format is simple enough that a line-by-line parser is clearer, faster, and
 * produces better error messages than any generic tool would. Every error
 * includes the line number where the problem occurred, which matters a lot
 * when users are writing grammars by hand.
 *
 * Why interfaces instead of classes? Because we want lightweight, plain data
 * objects that are easy to serialize, compare, and test. TypeScript interfaces
 * give us structural typing with zero runtime overhead — the compiler checks
 * the shape at build time, and at runtime they are just plain objects.
 */

// ---------------------------------------------------------------------------
// Exceptions
// ---------------------------------------------------------------------------

/**
 * Thrown when a .tokens file cannot be parsed.
 *
 * Properties:
 *   message: Human-readable description of the problem.
 *   lineNumber: 1-based line number where the error occurred.
 */
export class TokenGrammarError extends Error {
  public readonly lineNumber: number;

  constructor(message: string, lineNumber: number) {
    super(`Line ${lineNumber}: ${message}`);
    this.lineNumber = lineNumber;
    this.name = "TokenGrammarError";
  }
}

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

/**
 * A single token rule from a .tokens file.
 *
 * Properties:
 *   name: The token name, e.g. "NUMBER" or "PLUS".
 *   pattern: The pattern string — either a regex source (without delimiters)
 *       or a literal string (without quotes). Regex patterns are stored as
 *       strings, not compiled RegExp objects, so the grammar remains a pure
 *       data structure that is easy to serialize and inspect.
 *   isRegex: True if the pattern was written as /regex/, false if it
 *       was written as "literal".
 *   lineNumber: The 1-based line number where this definition appeared.
 */
export interface TokenDefinition {
  readonly name: string;
  readonly pattern: string;
  readonly isRegex: boolean;
  readonly lineNumber: number;
}

/**
 * The complete contents of a parsed .tokens file.
 *
 * Properties:
 *   definitions: Ordered list of token definitions. Order matters
 *       because the lexer uses first-match-wins semantics.
 *   keywords: List of reserved words from the keywords: section.
 */
export interface TokenGrammar {
  readonly definitions: readonly TokenDefinition[];
  readonly keywords: readonly string[];
}

// ---------------------------------------------------------------------------
// Helper: extract all token names from a grammar
// ---------------------------------------------------------------------------

/**
 * Return the set of all defined token names.
 *
 * This is useful for cross-validation: the parser grammar references
 * tokens by name, and we need to check that every referenced token
 * actually exists.
 */
export function tokenNames(grammar: TokenGrammar): Set<string> {
  return new Set(grammar.definitions.map((d) => d.name));
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

/**
 * Parse the text of a .tokens file into a TokenGrammar.
 *
 * The parser operates line-by-line. It has two modes:
 *
 * 1. **Definition mode** (default) — each line is either a comment, a
 *    blank, or a token definition of the form `NAME = /pattern/` or
 *    `NAME = "literal"`.
 *
 * 2. **Keywords mode** — entered when the parser encounters a line
 *    matching `keywords:`. Each subsequent indented line is treated as
 *    a keyword until a non-indented, non-blank, non-comment line is found
 *    (or EOF).
 *
 * @param source - The full text content of a .tokens file.
 * @returns A TokenGrammar containing all parsed definitions and keywords.
 * @throws TokenGrammarError if any line cannot be parsed.
 */
export function parseTokenGrammar(source: string): TokenGrammar {
  const lines = source.split("\n");
  const definitions: TokenDefinition[] = [];
  const keywords: string[] = [];
  let inKeywords = false;

  // The identifier pattern: must start with a letter or underscore,
  // followed by letters, digits, or underscores.
  const identifierPattern = /^[a-zA-Z_][a-zA-Z0-9_]*$/;

  for (let i = 0; i < lines.length; i++) {
    const lineNumber = i + 1;

    // Strip trailing whitespace but preserve leading whitespace
    // (we need it to detect keyword entries).
    const line = lines[i].replace(/\s+$/, "");

    // --- Blank lines and comments are always skipped ---
    const stripped = line.trim();
    if (stripped === "" || stripped.startsWith("#")) {
      continue;
    }

    // --- Keywords section header ---
    if (stripped === "keywords:" || stripped === "keywords :") {
      inKeywords = true;
      continue;
    }

    // --- Inside keywords section ---
    if (inKeywords) {
      // Keywords are indented lines. A non-indented line that isn't
      // blank or a comment means we've left the keywords section.
      if (line[0] === " " || line[0] === "\t") {
        if (stripped) {
          keywords.push(stripped);
        }
        continue;
      } else {
        // We've exited the keywords section. Fall through to
        // parse this line as a normal definition.
        inKeywords = false;
      }
    }

    // --- Token definition ---
    // Expected format: NAME = /pattern/  or  NAME = "literal"
    // We split on the first '=' to separate name from pattern.
    const eqIndex = line.indexOf("=");
    if (eqIndex === -1) {
      throw new TokenGrammarError(
        `Expected token definition (NAME = pattern), got: '${stripped}'`,
        lineNumber
      );
    }

    const namePart = line.slice(0, eqIndex).trim();
    const patternPart = line.slice(eqIndex + 1).trim();

    // Validate that we got a name.
    if (!namePart) {
      throw new TokenGrammarError(
        "Missing token name before '='",
        lineNumber
      );
    }

    // Validate the name looks like an identifier.
    if (!identifierPattern.test(namePart)) {
      throw new TokenGrammarError(
        `Invalid token name: '${namePart}' ` +
          "(must be an identifier like NAME or PLUS_EQUALS)",
        lineNumber
      );
    }

    // Parse the pattern: either /regex/ or "literal".
    if (!patternPart) {
      throw new TokenGrammarError(
        `Missing pattern after '=' for token '${namePart}'`,
        lineNumber
      );
    }

    if (patternPart.startsWith("/") && patternPart.endsWith("/")) {
      // Regex pattern — strip the delimiters.
      const regexBody = patternPart.slice(1, -1);
      if (!regexBody) {
        throw new TokenGrammarError(
          `Empty regex pattern for token '${namePart}'`,
          lineNumber
        );
      }
      definitions.push({
        name: namePart,
        pattern: regexBody,
        isRegex: true,
        lineNumber,
      });
    } else if (patternPart.startsWith('"') && patternPart.endsWith('"')) {
      // Literal pattern — strip the quotes.
      const literalBody = patternPart.slice(1, -1);
      if (!literalBody) {
        throw new TokenGrammarError(
          `Empty literal pattern for token '${namePart}'`,
          lineNumber
        );
      }
      definitions.push({
        name: namePart,
        pattern: literalBody,
        isRegex: false,
        lineNumber,
      });
    } else {
      throw new TokenGrammarError(
        `Pattern for token '${namePart}' must be /regex/ or ` +
          `"literal", got: '${patternPart}'`,
        lineNumber
      );
    }
  }

  return { definitions, keywords };
}

// ---------------------------------------------------------------------------
// Validator
// ---------------------------------------------------------------------------

/**
 * Check a parsed TokenGrammar for common problems.
 *
 * This is a *lint* pass, not a parse pass — the grammar has already been
 * parsed successfully. We are looking for semantic issues that would cause
 * problems downstream:
 *
 * - **Duplicate token names**: Two definitions with the same name. The
 *   second would shadow the first, which is almost certainly a mistake.
 * - **Invalid regex patterns**: A pattern written as /regex/ that the
 *   JavaScript RegExp constructor cannot compile. Caught here rather than
 *   at lexer-generation time so the user gets an early, clear error.
 * - **Empty patterns**: Should have been caught during parsing, but we
 *   double-check here for safety.
 * - **Non-UPPER_CASE names**: By convention, token names are UPPER_CASE.
 *   This helps distinguish them from parser rule names (lowercase) in
 *   .grammar files.
 *
 * @param grammar - A parsed TokenGrammar to validate.
 * @returns A list of warning/error strings. An empty list means no issues.
 */
export function validateTokenGrammar(grammar: TokenGrammar): string[] {
  const issues: string[] = [];
  const seenNames = new Map<string, number>();

  for (const defn of grammar.definitions) {
    // --- Duplicate check ---
    const firstLine = seenNames.get(defn.name);
    if (firstLine !== undefined) {
      issues.push(
        `Line ${defn.lineNumber}: Duplicate token name ` +
          `'${defn.name}' (first defined on line ${firstLine})`
      );
    } else {
      seenNames.set(defn.name, defn.lineNumber);
    }

    // --- Empty pattern check ---
    if (!defn.pattern) {
      issues.push(
        `Line ${defn.lineNumber}: Empty pattern for token '${defn.name}'`
      );
    }

    // --- Invalid regex check ---
    if (defn.isRegex) {
      try {
        new RegExp(defn.pattern);
      } catch (e: unknown) {
        const message = e instanceof Error ? e.message : String(e);
        issues.push(
          `Line ${defn.lineNumber}: Invalid regex for token ` +
            `'${defn.name}': ${message}`
        );
      }
    }

    // --- Naming convention check ---
    if (defn.name !== defn.name.toUpperCase()) {
      issues.push(
        `Line ${defn.lineNumber}: Token name '${defn.name}' ` +
          `should be UPPER_CASE`
      );
    }
  }

  return issues;
}
