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
 * Each non-blank, non-comment line in a .tokens file has one of these forms:
 *
 *   TOKEN_NAME = /regex_pattern/           — a regex-based token
 *   TOKEN_NAME = "literal_string"          — a literal-string token
 *   TOKEN_NAME = /regex/ -> ALIAS          — emits token type ALIAS instead
 *   TOKEN_NAME = "literal" -> ALIAS        — same for literals
 *   mode: indentation                      — sets the lexer mode
 *   keywords:                              — begins the keywords section
 *   reserved:                              — begins the reserved keywords section
 *   skip:                                  — begins the skip patterns section
 *   group NAME:                            — begins a named pattern group
 *
 * Lines starting with # are comments. Blank lines are ignored.
 *
 * The keywords section lists one reserved word per line (indented). Keywords
 * are identifiers that the lexer recognizes as NAME tokens but then
 * reclassifies. For instance, `if` matches the NAME pattern but is promoted
 * to an IF keyword.
 *
 * Pattern groups (group NAME:) enable context-sensitive lexing: the lexer
 * maintains a stack of active groups and only tries patterns from the group
 * on top of the stack. Language-specific callback code pushes/pops groups
 * in response to matched tokens. For example, an XML lexer pushes a "tag"
 * group when it sees `<` and pops it on `>`, so attribute-related patterns
 * are only active inside tags. Patterns outside any group section belong
 * to the implicit "default" group.
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
  readonly alias?: string;
}

/**
 * A named set of token definitions that are active together.
 *
 * When this group is at the top of the lexer's group stack, only these
 * patterns are tried during token matching. Skip patterns are global
 * and always tried regardless of the active group.
 *
 * Pattern groups enable context-sensitive lexing. For example, an XML
 * lexer defines a "tag" group with patterns for attribute names, equals
 * signs, and attribute values. These patterns are only active inside
 * tags — the callback pushes the "tag" group when `<` is matched and
 * pops it when `>` is matched.
 *
 * Properties:
 *   name: The group name, e.g. "tag" or "cdata". Must be a lowercase
 *       identifier matching [a-z_][a-z0-9_]*.
 *   definitions: Ordered list of token definitions in this group.
 *       Order matters (first-match-wins), just like the top-level
 *       definitions list.
 */
export interface PatternGroup {
  readonly name: string;
  readonly definitions: readonly TokenDefinition[];
}

/**
 * The complete contents of a parsed .tokens file.
 *
 * Properties:
 *   definitions: Ordered list of token definitions. Order matters
 *       because the lexer uses first-match-wins semantics.
 *   keywords: List of reserved words from the keywords: section.
 *   mode: Optional lexer mode (e.g. "indentation").
 *   escapeMode: Controls how STRING tokens are processed.
 *   skipDefinitions: Patterns matched and consumed without producing tokens.
 *   reservedKeywords: Keywords that are syntax errors if used as identifiers.
 *   groups: Named pattern groups for context-sensitive lexing. Each group
 *       contains an ordered list of token definitions that are only active
 *       when the group is at the top of the lexer's group stack.
 *   version: Grammar file version number, from `# @version N` magic comment.
 *       Defaults to 0 (meaning "latest" or "unversioned").
 *   caseInsensitive: Whether the lexer should match tokens case-insensitively,
 *       from `# @case_insensitive true` magic comment. Defaults to false.
 *
 * Magic comments
 * --------------
 *
 * Lines beginning with `# @key value` are "magic comments." They look like
 * ordinary comments but carry structured metadata for tooling. This is the
 * same convention used by many languages (Python's `# type:`, PHP's
 * `// @var`, etc.) — keeping metadata in comments means the file stays
 * human-readable and backward-compatible with tools that just skip comments.
 *
 * Supported magic comments:
 *   # @version N             — integer schema version (default 0 = latest)
 *   # @case_insensitive true — enable case-insensitive matching (default false)
 *
 * Unknown keys are silently ignored, making it easy to add new metadata in
 * the future without breaking older parsers.
 */
export interface TokenGrammar {
  readonly definitions: readonly TokenDefinition[];
  readonly keywords: readonly string[];
  readonly mode?: string;
  readonly escapeMode?: string;
  readonly skipDefinitions?: readonly TokenDefinition[];
  readonly reservedKeywords?: readonly string[];
  readonly groups?: Readonly<Record<string, PatternGroup>>;
  readonly layoutKeywords?: readonly string[];
  /** Controls whether the lexer matches case-sensitively. Defaults to true.
   *  When false, the lexer lowercases source text before matching. */
  readonly caseSensitive?: boolean;
  /** Grammar file version number from `# @version N` magic comment. Defaults to 0. */
  readonly version: number;
  /** Whether the lexer should match case-insensitively, from `# @case_insensitive true`. */
  readonly caseInsensitive: boolean;
  /**
   * Context-sensitive keywords — words that are keywords in some
   * syntactic positions but identifiers in others.
   *
   * These are emitted as NAME tokens with the TOKEN_CONTEXT_KEYWORD
   * flag set, leaving the final keyword-vs-identifier decision to
   * the language-specific parser or callback.
   *
   * Examples: JavaScript's `async`, `await`, `yield`, `get`, `set`.
   */
  readonly contextKeywords?: readonly string[];
  /**
   * Soft keywords — words that act as keywords only in specific syntactic
   * contexts, remaining ordinary identifiers everywhere else.
   *
   * Unlike contextKeywords (which set a flag on the token), soft keywords
   * produce plain NAME tokens with NO special flag. The lexer is completely
   * unaware of their keyword status — the parser handles disambiguation
   * entirely based on syntactic position.
   *
   * This distinction matters because:
   *   - contextKeywords: lexer hints to parser ("this NAME might be special")
   *   - softKeywords: lexer ignores them completely, parser owns the decision
   *
   * Examples:
   *   Python 3.10+: `match`, `case`, `_` (only keywords inside match statements)
   *   Python 3.12+: `type` (only a keyword in `type X = ...` statements)
   *
   * A `soft_keywords:` section in a .tokens file populates this field.
   */
  readonly softKeywords?: readonly string[];
}

function parseMagicComment(line: string): { key: string; value: string } | null {
  if (!line.startsWith("#")) return null;

  let index = 1;
  while (index < line.length && (line[index] === " " || line[index] === "\t")) {
    index++;
  }
  if (line[index] !== "@") return null;
  index++;

  const keyStart = index;
  while (index < line.length) {
    const ch = line[index];
    const isWordChar = (ch >= "a" && ch <= "z")
      || (ch >= "A" && ch <= "Z")
      || (ch >= "0" && ch <= "9")
      || ch === "_";
    if (!isWordChar) break;
    index++;
  }
  if (index === keyStart) return null;

  const key = line.slice(keyStart, index);
  while (index < line.length && (line[index] === " " || line[index] === "\t")) {
    index++;
  }
  return { key, value: line.slice(index).trim() };
}

// ---------------------------------------------------------------------------
// Helper: extract all token names from a grammar
// ---------------------------------------------------------------------------

/**
 * Return the set of all defined token names.
 *
 * When a definition has an alias, the alias is included in the set
 * (since that is the name the parser grammar references). The original
 * definition name is also included for completeness.
 *
 * Includes names from all pattern groups, since group tokens can
 * also appear in parser grammars.
 *
 * This is useful for cross-validation: the parser grammar references
 * tokens by name, and we need to check that every referenced token
 * actually exists.
 */
export function tokenNames(grammar: TokenGrammar): Set<string> {
  const names = new Set<string>();

  // Collect definitions from the top-level list plus all groups
  const allDefs: TokenDefinition[] = [...grammar.definitions];
  if (grammar.groups) {
    for (const group of Object.values(grammar.groups)) {
      allDefs.push(...group.definitions);
    }
  }

  for (const d of allDefs) {
    names.add(d.name);
    if (d.alias) {
      names.add(d.alias);
    }
  }
  return names;
}

/**
 * Return the set of token names as the parser will see them.
 *
 * For definitions with aliases, this returns the alias (not the
 * definition name), because that is what the lexer will emit and
 * what the parser grammar references.
 *
 * For definitions without aliases, this returns the definition name.
 *
 * Includes names from all pattern groups.
 */
export function effectiveTokenNames(grammar: TokenGrammar): Set<string> {
  const allDefs: TokenDefinition[] = [...grammar.definitions];
  if (grammar.groups) {
    for (const group of Object.values(grammar.groups)) {
      allDefs.push(...group.definitions);
    }
  }
  return new Set(allDefs.map((d) => d.alias ?? d.name));
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
/**
 * Parse a single token definition line into a TokenDefinition.
 *
 * Handles both forms:
 *   NAME = /pattern/
 *   NAME = "literal"
 *   NAME = /pattern/ -> ALIAS
 *   NAME = "literal" -> ALIAS
 *
 * @param namePart - The token name (left side of =).
 * @param patternPart - Everything after the = sign.
 * @param lineNumber - The 1-based line number for error reporting.
 * @returns A TokenDefinition.
 */
function findClosingSlash(s: string): number {
  let inBracket = false;
  for (let i = 1; i < s.length; i++) {
    const ch = s[i];
    if (ch === "\\") {
      i++; // skip escaped character
      continue;
    }
    if (ch === "[" && !inBracket) {
      inBracket = true;
    } else if (ch === "]" && inBracket) {
      inBracket = false;
    } else if (ch === "/" && !inBracket) {
      return i;
    }
  }
  // Fallback: if bracket-aware scan found nothing (e.g. unclosed [),
  // try the last / as a best-effort parse.
  const last = s.lastIndexOf("/");
  return last > 0 ? last : -1;
}

function parseDefinition(
  namePart: string,
  patternPart: string,
  lineNumber: number,
): TokenDefinition {
  // Parse the pattern and optional alias. We must find the closing
  // delimiter FIRST, then check for "-> ALIAS" in the remainder.
  // A naive indexOf("->") would break when "->" appears inside a
  // regex pattern (e.g., /([^-]|-(?!->))+/ in XML's COMMENT_TEXT).

  if (!patternPart) {
    throw new TokenGrammarError(
      `Missing pattern after '=' for token '${namePart}'`,
      lineNumber,
    );
  }

  if (patternPart.startsWith("/")) {
    // Regex pattern — find the closing /
    // Use bracket-aware scan so that / inside [...] character classes
    // is not mistaken for the closing delimiter.
    const lastSlash = findClosingSlash(patternPart);
    if (lastSlash <= 0) {
      throw new TokenGrammarError(
        `Unclosed regex pattern for token '${namePart}'`,
        lineNumber,
      );
    }

    const regexBody = patternPart.slice(1, lastSlash);
    if (!regexBody) {
      throw new TokenGrammarError(
        `Empty regex pattern for token '${namePart}'`,
        lineNumber,
      );
    }

    // Check for -> ALIAS in the remainder after the closing /
    const remainder = patternPart.slice(lastSlash + 1).trim();
    let alias: string | undefined;
    if (remainder.startsWith("->")) {
      alias = remainder.slice(2).trim();
      if (!alias) {
        throw new TokenGrammarError(
          `Missing alias name after '->' for token '${namePart}'`,
          lineNumber,
        );
      }
    } else if (remainder) {
      throw new TokenGrammarError(
        `Unexpected text after regex pattern for token '${namePart}': '${remainder}'`,
        lineNumber,
      );
    }

    return { name: namePart, pattern: regexBody, isRegex: true, lineNumber, alias };
  } else if (patternPart.startsWith('"')) {
    // Literal pattern — find the closing "
    const closingQuote = patternPart.indexOf('"', 1);
    if (closingQuote === -1) {
      throw new TokenGrammarError(
        `Unclosed literal pattern for token '${namePart}'`,
        lineNumber,
      );
    }
    const literalBody = patternPart.slice(1, closingQuote);
    if (!literalBody) {
      throw new TokenGrammarError(
        `Empty literal pattern for token '${namePart}'`,
        lineNumber,
      );
    }

    // Check for -> ALIAS in the remainder after the closing "
    const litRemainder = patternPart.slice(closingQuote + 1).trim();
    let alias: string | undefined;
    if (litRemainder.startsWith("->")) {
      alias = litRemainder.slice(2).trim();
      if (!alias) {
        throw new TokenGrammarError(
          `Missing alias name after '->' for token '${namePart}'`,
          lineNumber,
        );
      }
    } else if (litRemainder) {
      throw new TokenGrammarError(
        `Unexpected text after literal pattern for token '${namePart}': '${litRemainder}'`,
        lineNumber,
      );
    }

    return { name: namePart, pattern: literalBody, isRegex: false, lineNumber, alias };
  } else {
    throw new TokenGrammarError(
      `Pattern for token '${namePart}' must be /regex/ or ` +
        `"literal", got: '${patternPart}'`,
      lineNumber,
    );
  }
}

export function parseTokenGrammar(source: string): TokenGrammar {
  const lines = source.split("\n");
  const definitions: TokenDefinition[] = [];
  const keywords: string[] = [];
  const contextKeywords: string[] = [];
  const softKeywords: string[] = [];
  const layoutKeywords: string[] = [];
  const skipDefinitions: TokenDefinition[] = [];
  const reservedKeywords: string[] = [];
  const groups: Record<string, PatternGroup> = {};
  let mode: string | undefined;
  let escapeMode: string | undefined;
  let caseSensitive: boolean = true;

  // Magic comment state. These are collected from `# @key value` lines
  // anywhere in the file before being used in the returned grammar.
  // We initialize to the defaults so callers always see well-typed values.
  let version = 0;
  let caseInsensitive = false;

  // Section tracking. We use a string to track which section we're in,
  // since sections are mutually exclusive and we can only be in one at
  // a time (or in no section = definition mode).
  //
  // For pattern groups, currentSection is "group:NAME" where NAME is
  // the group name. This distinguishes groups from other sections.
  let currentSection = "definitions";

  const identifierPattern = /^[a-zA-Z_][a-zA-Z0-9_]*$/;

  // Group name must be a lowercase identifier: letters, digits, underscores,
  // starting with a letter or underscore. No uppercase allowed — group names
  // are always lowercase by convention (e.g., "tag", "cdata", "string_body").
  const groupNamePattern = /^[a-z_][a-z0-9_]*$/;

  // Reserved names that cannot be used as group names. These collide with
  // built-in section names and the implicit "default" group.
  const reservedGroupNames = new Set([
    "default",
    "skip",
    "keywords",
    "reserved",
    "errors",
    "layout_keywords",
    "context_keywords",
    "soft_keywords",
  ]);

  for (let i = 0; i < lines.length; i++) {
    const lineNumber = i + 1;
    const line = lines[i].trimEnd();
    const stripped = line.trim();

    // Blank lines are always skipped.
    if (stripped === "") {
      continue;
    }

    // Lines starting with '#' are comments — but magic comments (`# @key value`)
    // carry structured metadata we need to extract before discarding the line.
    // We check for the magic pattern first; non-magic comments are skipped.
    if (stripped.startsWith("#")) {
      const magicMatch = parseMagicComment(stripped);
      if (magicMatch) {
        const { key, value } = magicMatch;
        if (key === "version") {
          // Parse the version as a decimal integer. NaN falls back to 0
          // so a malformed `# @version abc` is treated as "unversioned."
          const parsed = parseInt(value, 10);
          version = isNaN(parsed) ? 0 : parsed;
        } else if (key === "case_insensitive") {
          caseInsensitive = value === "true";
        }
        // Unknown keys are silently ignored — forward-compatible design.
      }
      continue;
    }

    // --- Mode directive ---
    if (stripped.startsWith("mode:") || stripped.startsWith("mode :")) {
      const modeValue = stripped.slice(stripped.indexOf(":") + 1).trim();
      if (!modeValue) {
        throw new TokenGrammarError(
          "Missing mode value after 'mode:'",
          lineNumber,
        );
      }
      mode = modeValue;
      currentSection = "definitions";
      continue;
    }

    // --- Escapes directive ---
    if (stripped.startsWith("escapes:") || stripped.startsWith("escapes :")) {
      const escapesValue = stripped.slice(stripped.indexOf(":") + 1).trim();
      if (!escapesValue) {
        throw new TokenGrammarError(
          "Missing escapes value after 'escapes:'",
          lineNumber,
        );
      }
      escapeMode = escapesValue;
      currentSection = "definitions";
      continue;
    }

    // --- Case-sensitive directive ---
    if (stripped.startsWith("case_sensitive:") || stripped.startsWith("case_sensitive :")) {
      const csValue = stripped.slice(stripped.indexOf(":") + 1).trim();
      if (!csValue) {
        throw new TokenGrammarError(
          "Missing value after 'case_sensitive:'",
          lineNumber,
        );
      }
      if (csValue.toLowerCase() === "true") {
        caseSensitive = true;
      } else if (csValue.toLowerCase() === "false") {
        caseSensitive = false;
      } else {
        throw new TokenGrammarError(
          `Invalid case_sensitive value: '${csValue}' (must be 'true' or 'false')`,
          lineNumber,
        );
      }
      currentSection = "definitions";
      continue;
    }

    // --- Group headers ---
    // Pattern groups are declared with "group NAME:" where NAME is
    // a lowercase identifier. All subsequent indented lines belong to
    // that group, just like skip: or errors: sections.
    if (stripped.startsWith("group ") && stripped.endsWith(":")) {
      const groupName = stripped.slice(6, -1).trim();
      if (!groupName) {
        throw new TokenGrammarError(
          "Missing group name after 'group'",
          lineNumber,
        );
      }
      if (!groupNamePattern.test(groupName)) {
        throw new TokenGrammarError(
          `Invalid group name: '${groupName}' ` +
            "(must be a lowercase identifier like 'tag' or 'cdata')",
          lineNumber,
        );
      }
      if (reservedGroupNames.has(groupName)) {
        throw new TokenGrammarError(
          `Reserved group name: '${groupName}' ` +
            `(cannot use ${[...reservedGroupNames].sort().join(", ")})`,
          lineNumber,
        );
      }
      if (groupName in groups) {
        throw new TokenGrammarError(
          `Duplicate group name: '${groupName}'`,
          lineNumber,
        );
      }
      groups[groupName] = { name: groupName, definitions: [] };
      currentSection = `group:${groupName}`;
      continue;
    }

    // --- Escapes directive ---
    if (stripped.startsWith("escapes:") || stripped.startsWith("escapes :")) {
      const escapesValue = stripped.slice(stripped.indexOf(":") + 1).trim();
      if (!escapesValue) {
        throw new TokenGrammarError(
          "Missing escapes value after 'escapes:'",
          lineNumber,
        );
      }
      escapeMode = escapesValue;
      continue;
    }

    // --- Section headers ---
    if (stripped === "keywords:" || stripped === "keywords :") {
      currentSection = "keywords";
      continue;
    }
    if (stripped === "skip:" || stripped === "skip :") {
      currentSection = "skip";
      continue;
    }
    if (stripped === "reserved:" || stripped === "reserved :") {
      currentSection = "reserved";
      continue;
    }
    if (stripped === "errors:" || stripped === "errors :") {
      currentSection = "errors";
      continue;
    }
    if (stripped === "context_keywords:" || stripped === "context_keywords :") {
      currentSection = "context_keywords";
      continue;
    }
    if (stripped === "layout_keywords:" || stripped === "layout_keywords :") {
      currentSection = "layout_keywords";
      continue;
    }
    if (stripped === "soft_keywords:" || stripped === "soft_keywords :") {
      currentSection = "soft_keywords";
      continue;
    }

    // --- Inside a section ---
    const isIndented = line[0] === " " || line[0] === "\t";

    if (isIndented && currentSection === "keywords") {
      if (stripped) {
        keywords.push(stripped);
      }
      continue;
    }

    if (isIndented && currentSection === "context_keywords") {
      if (stripped) {
        contextKeywords.push(stripped);
      }
      continue;
    }

    if (isIndented && currentSection === "layout_keywords") {
      if (stripped) {
        layoutKeywords.push(stripped);
      }
      continue;
    }

    if (isIndented && currentSection === "soft_keywords") {
      if (stripped) {
        softKeywords.push(stripped);
      }
      continue;
    }

    if (isIndented && currentSection === "reserved") {
      if (stripped) {
        reservedKeywords.push(stripped);
      }
      continue;
    }

    if (isIndented && currentSection === "errors") {
      // Error token definitions — parsed same as skip but stored separately.
      // Error tokens match invalid input patterns and are silently ignored
      // during tokenization (the lexer skips them like whitespace).
      const eqIndex = stripped.indexOf("=");
      if (eqIndex !== -1) {
        // We parse and discard error token definitions — they are informational
        // for documentation and future error-reporting features, not used yet.
        const _errName = stripped.slice(0, eqIndex).trim();
        const _errPattern = stripped.slice(eqIndex + 1).trim();
        void _errName;
        void _errPattern;
      }
      continue;
    }

    if (isIndented && currentSection === "skip") {
      // Skip definitions are indented token definitions
      const eqIndex = stripped.indexOf("=");
      if (eqIndex === -1) {
        throw new TokenGrammarError(
          `Expected skip definition (NAME = pattern), got: '${stripped}'`,
          lineNumber,
        );
      }
      const skipName = stripped.slice(0, eqIndex).trim();
      const skipPattern = stripped.slice(eqIndex + 1).trim();
      if (!skipPattern) {
        throw new TokenGrammarError(
          `Missing pattern after '=' for skip token '${skipName}'`,
          lineNumber,
        );
      }
      skipDefinitions.push(parseDefinition(skipName, skipPattern, lineNumber));
      continue;
    }

    // --- Inside a group section ---
    // Group sections contain indented token definitions, same format as
    // skip: sections. Each definition is parsed and added to the group's
    // definitions list.
    if (isIndented && currentSection.startsWith("group:")) {
      const groupName = currentSection.slice(6);
      const eqIndex = stripped.indexOf("=");
      if (eqIndex === -1) {
        throw new TokenGrammarError(
          `Expected token definition in group '${groupName}' ` +
            `(NAME = pattern), got: '${stripped}'`,
          lineNumber,
        );
      }
      const gName = stripped.slice(0, eqIndex).trim();
      const gPattern = stripped.slice(eqIndex + 1).trim();
      if (!gName || !gPattern) {
        throw new TokenGrammarError(
          `Incomplete definition in group '${groupName}': '${stripped}'`,
          lineNumber,
        );
      }
      const defn = parseDefinition(gName, gPattern, lineNumber);
      // Since interfaces are readonly, we build the definitions array
      // mutably here during parsing and freeze it when we return.
      (groups[groupName].definitions as TokenDefinition[]).push(defn);
      continue;
    }

    // Non-indented line exits any section
    if (!isIndented && currentSection !== "definitions") {
      currentSection = "definitions";
    }

    // --- Token definition ---
    const eqIndex = line.indexOf("=");
    if (eqIndex === -1) {
      throw new TokenGrammarError(
        `Expected token definition (NAME = pattern), got: '${stripped}'`,
        lineNumber,
      );
    }

    const namePart = line.slice(0, eqIndex).trim();
    const patternPart = line.slice(eqIndex + 1).trim();

    if (!namePart) {
      throw new TokenGrammarError(
        "Missing token name before '='",
        lineNumber,
      );
    }

    if (!identifierPattern.test(namePart)) {
      throw new TokenGrammarError(
        `Invalid token name: '${namePart}' ` +
          "(must be an identifier like NAME or PLUS_EQUALS)",
        lineNumber,
      );
    }

    definitions.push(parseDefinition(namePart, patternPart, lineNumber));
  }

  // Build the result. Only include optional fields when they have content,
  // keeping the interface clean for consumers that don't use those features.
  const hasGroups = Object.keys(groups).length > 0;

  return {
    definitions,
    keywords,
    mode,
    escapeMode,
    skipDefinitions: skipDefinitions.length > 0 ? skipDefinitions : undefined,
    reservedKeywords: reservedKeywords.length > 0 ? reservedKeywords : undefined,
    groups: hasGroups ? groups : undefined,
    layoutKeywords: layoutKeywords.length > 0 ? layoutKeywords : undefined,
    caseSensitive: caseSensitive ? undefined : false,
    version,
    caseInsensitive,
    contextKeywords: contextKeywords.length > 0 ? contextKeywords : undefined,
    softKeywords: softKeywords.length > 0 ? softKeywords : undefined,
  };
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
/**
 * Validate a single list of definitions for common problems.
 */
function validateDefinitions(
  defs: readonly TokenDefinition[],
  seenNames: Map<string, number>,
  issues: string[],
  label: string,
): void {
  for (const defn of defs) {
    const firstLine = seenNames.get(defn.name);
    if (firstLine !== undefined) {
      issues.push(
        `Line ${defn.lineNumber}: Duplicate ${label} name ` +
          `'${defn.name}' (first defined on line ${firstLine})`,
      );
    } else {
      seenNames.set(defn.name, defn.lineNumber);
    }

    if (!defn.pattern) {
      issues.push(
        `Line ${defn.lineNumber}: Empty pattern for ${label} '${defn.name}'`,
      );
    }

    if (defn.isRegex) {
      try {
        new RegExp(defn.pattern);
      } catch (e: unknown) {
        const message = e instanceof Error ? e.message : String(e);
        issues.push(
          `Line ${defn.lineNumber}: Invalid regex for ${label} ` +
            `'${defn.name}': ${message}`,
        );
      }
    }

    if (defn.name !== defn.name.toUpperCase()) {
      issues.push(
        `Line ${defn.lineNumber}: ${label} name '${defn.name}' ` +
          `should be UPPER_CASE`,
      );
    }
  }
}

export function validateTokenGrammar(grammar: TokenGrammar): string[] {
  const issues: string[] = [];
  const seenNames = new Map<string, number>();

  validateDefinitions(grammar.definitions, seenNames, issues, "token");

  if (grammar.skipDefinitions) {
    validateDefinitions(grammar.skipDefinitions, seenNames, issues, "skip token");
  }

  // Validate mode value
  if (
    grammar.mode !== undefined &&
    grammar.mode !== "indentation" &&
    grammar.mode !== "layout"
  ) {
    issues.push(`Unknown mode: '${grammar.mode}'`);
  }

  if (
    grammar.mode === "layout" &&
    (!grammar.layoutKeywords || grammar.layoutKeywords.length === 0)
  ) {
    issues.push("Layout mode requires a non-empty layoutKeywords list");
  }

  // Validate escapeMode value
  if (grammar.escapeMode !== undefined && grammar.escapeMode !== "none") {
    issues.push(`Unknown escapes mode: '${grammar.escapeMode}'`);
  }

  // Validate pattern groups
  if (grammar.groups) {
    const groupNamePattern = /^[a-z_][a-z0-9_]*$/;
    for (const [groupName, group] of Object.entries(grammar.groups)) {
      // Group name format check
      if (!groupNamePattern.test(groupName)) {
        issues.push(
          `Invalid group name '${groupName}' ` +
            `(must be a lowercase identifier)`,
        );
      }

      // Empty group warning — a group with no definitions is likely
      // a mistake (the author forgot to add patterns).
      if (group.definitions.length === 0) {
        issues.push(
          `Empty pattern group '${groupName}' ` +
            `(has no token definitions)`,
        );
      }

      // Validate definitions within the group using the same checks
      // as regular and skip definitions (duplicates, bad regex, naming).
      validateDefinitions(
        group.definitions,
        new Map<string, number>(),
        issues,
        `group '${groupName}' token`,
      );
    }
  }


  return issues;
}
