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
 * Extensions for Starlark/Python-like Languages
 * ----------------------------------------------
 *
 * Beyond basic regex-driven tokenization, this module supports:
 *
 * - **Skip patterns**: Whitespace and comment patterns that are consumed
 *   without producing tokens (defined in the `skip:` section of .tokens).
 * - **Type aliases**: A token definition like `STRING_DQ -> STRING` emits
 *   tokens with type "STRING" instead of "STRING_DQ".
 * - **Reserved keywords**: Identifiers that must not appear in source code
 *   (e.g., `class` and `import` in Starlark). Raises LexerError on match.
 * - **Indentation mode**: For Python-like languages, tracks indentation
 *   levels and emits synthetic INDENT/DEDENT/NEWLINE tokens.
 *
 * Pattern Groups and On-Token Callbacks
 * --------------------------------------
 *
 * Pattern groups enable **context-sensitive lexing**. A grammar can define
 * named groups of patterns (e.g., a "tag" group for XML attributes) that
 * are only active when the group is at the top of the lexer's group stack.
 *
 * The lexer maintains a stack of group names, starting with "default".
 * An **on-token callback** can push/pop groups, emit synthetic tokens,
 * suppress the current token, or toggle skip pattern processing. This
 * enables lexing of context-sensitive languages like XML/HTML where
 * different parts of the input require different token patterns.
 *
 * Example — XML-like lexing:
 *
 *     // Grammar defines "default" patterns (TEXT, OPEN_TAG) and a
 *     // "tag" group (TAG_NAME, EQUALS, VALUE, TAG_CLOSE).
 *     const lexer = new GrammarLexer(source, grammar);
 *     lexer.setOnToken((token, ctx) => {
 *       if (token.type === "OPEN_TAG") ctx.pushGroup("tag");
 *       if (token.type === "TAG_CLOSE") ctx.popGroup();
 *     });
 *     const tokens = lexer.tokenize();
 */

import type { TokenGrammar } from "@coding-adventures/grammar-tools";

import type { Token, Trivia } from "./token.js";
import { TOKEN_CONTEXT_KEYWORD } from "./token.js";
import { LexerError } from "./tokenizer.js";

// ---------------------------------------------------------------------------
// Compiled Pattern
// ---------------------------------------------------------------------------

/**
 * A compiled token pattern — ready for regex matching.
 *
 * Each compiled pattern pairs a token name (like "NUMBER" or "TAG_NAME")
 * with a RegExp object. The optional `alias` field maps the definition
 * name to a different token type for emission (e.g., STRING_DQ -> STRING).
 */
interface CompiledPattern {
  readonly name: string;
  readonly pattern: RegExp;
  readonly alias?: string;
}

// ---------------------------------------------------------------------------
// Escape helper for literal patterns
// ---------------------------------------------------------------------------

/**
 * Escape special regex characters in a literal pattern string.
 *
 * When a `.tokens` file defines a literal pattern like `"+"`, we need to
 * escape the `+` so it is treated as a literal character in the regex,
 * not as a quantifier. This function handles all regex-special characters.
 */
function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

// ---------------------------------------------------------------------------
// Token Type Resolution
// ---------------------------------------------------------------------------

/**
 * Resolve a grammar token name and matched value to a token type string.
 *
 * The resolution follows a priority order:
 *
 * 1. **Reserved keyword check**: If the token is a NAME and the value is
 *    a reserved keyword, throw a LexerError immediately.
 * 2. **Keyword detection**: If the token is a NAME and the value is a
 *    keyword, return "KEYWORD" (promoting the NAME to a keyword token).
 * 3. **Alias resolution**: If the definition has an alias, use it.
 * 4. **Direct name**: Use the definition name as-is.
 */
function resolveTokenType(
  tokenName: string,
  value: string,
  keywordSet: ReadonlySet<string>,
  reservedSet: ReadonlySet<string>,
  alias: string | undefined,
  line: number,
  column: number,
): string {
  // Reserved keyword check — these are hard errors
  if (tokenName === "NAME" && reservedSet.has(value)) {
    throw new LexerError(
      `Reserved keyword '${value}' cannot be used as an identifier`,
      line,
      column,
    );
  }

  // Regular keyword check — promote NAME to KEYWORD
  if (tokenName === "NAME" && keywordSet.has(value)) {
    return "KEYWORD";
  }

  // Alias takes precedence
  if (alias) {
    return alias;
  }

  return tokenName;
}

// ---------------------------------------------------------------------------
// Escape Sequence Processing
// ---------------------------------------------------------------------------

/**
 * Process escape sequences in a string value.
 *
 * Handles the standard escape sequences: `\n` (newline), `\t` (tab),
 * `\\` (literal backslash), `\"` (literal double quote). Unknown escape
 * sequences pass through the escaped character (e.g., `\x` becomes `x`).
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
// On-Token Callback Type
// ---------------------------------------------------------------------------

/**
 * Signature for on-token callbacks.
 *
 * The callback receives the matched token and a `LexerContext` that
 * provides controlled access to group stack manipulation, token emission,
 * suppression, lookahead, and skip pattern toggling.
 *
 * The callback is NOT invoked for:
 * - Skip pattern matches (they produce no tokens)
 * - Tokens emitted via `ctx.emit()` (prevents infinite loops)
 * - The EOF token
 */
export type OnTokenCallback = (token: Token, ctx: LexerContext) => void;

export interface GrammarLexerOptions {
  readonly preserveSourceInfo?: boolean;
}

// ---------------------------------------------------------------------------
// Lexer Context — Callback Interface for Group Transitions
// ---------------------------------------------------------------------------

/**
 * Interface that on-token callbacks use to control the lexer.
 *
 * When a callback is registered via `GrammarLexer.setOnToken()`, it
 * receives a `LexerContext` on every token match. The context provides
 * controlled access to the group stack, token emission, and skip control.
 *
 * Methods that modify state (push/pop/emit/suppress) take effect **after**
 * the callback returns — they do not interrupt the current match.
 *
 * Think of the context as a "request form" that the callback fills out.
 * The lexer's main loop reads the form after the callback returns and
 * applies the requested actions in order:
 *
 * 1. Suppress the current token (if requested)
 * 2. Append any emitted synthetic tokens
 * 3. Apply group stack changes (push/pop)
 * 4. Toggle skip processing (if requested)
 *
 * Example — XML lexer callback:
 *
 *     function xmlHook(token: Token, ctx: LexerContext): void {
 *       if (token.type === "OPEN_TAG_START") {
 *         ctx.pushGroup("tag");
 *       } else if (token.type === "TAG_CLOSE" || token.type === "SELF_CLOSE") {
 *         ctx.popGroup();
 *       }
 *     }
 */
export class LexerContext {
  /** @internal Reference to the lexer (for reading group stack state). */
  private readonly _lexer: GrammarLexer;

  /** @internal The full source string being tokenized. */
  private readonly _source: string;

  /** @internal Position in the source immediately after the current token. */
  private readonly _posAfter: number;

  /** @internal Whether the current token should be suppressed from output. */
  _suppressed: boolean = false;

  /** @internal Synthetic tokens to inject after the current one. */
  _emitted: Token[] = [];

  /** @internal Group stack actions recorded by the callback: ("push", name) or ("pop", ""). */
  _groupActions: Array<[string, string]> = [];

  /** @internal New skip-enabled state, or null if unchanged. */
  _skipEnabled: boolean | null = null;

  /** @internal The most recently emitted token (for lookbehind). */
  private readonly _previousToken: Token | null;

  /** @internal The current token's line number (for newline detection). */
  private readonly _currentTokenLine: number;

  constructor(
    lexer: GrammarLexer,
    source: string,
    posAfterToken: number,
    previousToken: Token | null,
    currentTokenLine: number,
  ) {
    this._lexer = lexer;
    this._source = source;
    this._posAfter = posAfterToken;
    this._previousToken = previousToken;
    this._currentTokenLine = currentTokenLine;
  }

  /**
   * Push a pattern group onto the stack.
   *
   * The pushed group becomes active for the **next** token match.
   * Throws an Error if the group name is not defined in the grammar.
   *
   * Multiple pushes in a single callback are applied in order, so you
   * can stack multiple groups if needed (though this is rare).
   */
  pushGroup(groupName: string): void {
    if (!this._lexer.hasGroup(groupName)) {
      throw new Error(
        `Unknown pattern group: '${groupName}'. ` +
        `Available groups: ${this._lexer.availableGroups().sort().join(", ")}`,
      );
    }
    this._groupActions.push(["push", groupName]);
  }

  /**
   * Pop the current group from the stack.
   *
   * If only the "default" group remains (stack depth = 1), this is a
   * no-op. The default group is the floor and cannot be popped — this
   * prevents accidental stack underflow in recursive structures.
   */
  popGroup(): void {
    this._groupActions.push(["pop", ""]);
  }

  /**
   * Return the name of the currently active group.
   *
   * The active group is the top of the group stack. When no groups
   * have been pushed, this is always "default".
   */
  activeGroup(): string {
    return this._lexer.activeGroup();
  }

  /**
   * Return the depth of the group stack (always >= 1).
   *
   * A depth of 1 means only the "default" group is on the stack.
   * A depth of 2 means one group has been pushed on top of default.
   */
  groupStackDepth(): number {
    return this._lexer.groupStackDepth();
  }

  /**
   * Inject a synthetic token after the current one.
   *
   * Emitted tokens do NOT trigger the callback (this prevents infinite
   * loops — a callback that emits tokens which trigger the callback
   * which emits more tokens...). Multiple `emit()` calls produce
   * tokens in call order.
   */
  emit(token: Token): void {
    this._emitted.push(token);
  }

  /**
   * Suppress the current token — do not include it in output.
   *
   * Combined with `emit()`, this enables **token replacement**: suppress
   * the original token and emit a modified version in its place.
   */
  suppress(): void {
    this._suppressed = true;
  }

  /**
   * Peek at a source character past the current token.
   *
   * This provides lookahead capability without advancing the lexer's
   * position. Useful for making group-switching decisions based on
   * what comes next in the source.
   *
   * @param offset - Number of characters ahead (1 = immediately after token).
   * @returns The character, or empty string if past EOF.
   */
  peek(offset: number = 1): string {
    const idx = this._posAfter + offset - 1;
    if (idx >= 0 && idx < this._source.length) {
      return this._source[idx];
    }
    return "";
  }

  /**
   * Peek at the next `length` characters past the current token.
   *
   * Returns a substring starting immediately after the current token.
   * If fewer than `length` characters remain, returns whatever is left.
   */
  peekStr(length: number): string {
    return this._source.slice(this._posAfter, this._posAfter + length);
  }

  /**
   * Toggle skip pattern processing.
   *
   * When disabled, skip patterns (whitespace, comments) are not tried.
   * This is useful for groups where whitespace is significant — for
   * example, CDATA sections in XML where spaces must be preserved
   * as part of the content rather than being silently consumed.
   */
  setSkipEnabled(enabled: boolean): void {
    this._skipEnabled = enabled;
  }

  // -----------------------------------------------------------------------
  // Extension: Token Lookbehind
  // -----------------------------------------------------------------------

  /**
   * Return the most recently emitted token, or null at the start of input.
   *
   * "Emitted" means the token actually made it into the output list —
   * suppressed tokens are not counted. This provides **lookbehind**
   * capability for context-sensitive decisions.
   *
   * For example, in JavaScript `/` is a regex literal after `=`, `(`
   * or `,` but a division operator after `)`, `]`, identifiers, or
   * numbers. The callback can check `ctx.previousToken()?.type` to
   * decide which interpretation to use.
   *
   * @returns The last token in the output list, or null if no tokens
   *          have been emitted yet.
   */
  previousToken(): Token | null {
    return this._previousToken;
  }

  // -----------------------------------------------------------------------
  // Extension: Bracket Depth Tracking
  // -----------------------------------------------------------------------

  /**
   * Return the current nesting depth for a specific bracket type,
   * or the total depth across all types if no argument is given.
   *
   * Depth starts at 0 and increments on each opener (`(`, `[`, `{`),
   * decrements on each closer (`)`, `]`, `}`). The count never goes
   * below 0 — unmatched closers are clamped.
   *
   * This is essential for template literal interpolation in languages
   * like JavaScript, Kotlin, and Ruby, where `}` at brace-depth 0
   * closes the interpolation rather than being part of a nested
   * expression.
   *
   * @param kind - Optional bracket type to query. If omitted, returns
   *               the sum of all three depths.
   */
  bracketDepth(kind?: "paren" | "bracket" | "brace"): number {
    return this._lexer.bracketDepth(kind);
  }

  // -----------------------------------------------------------------------
  // Extension: Newline Detection
  // -----------------------------------------------------------------------

  /**
   * Return true if a newline appeared between the previous token
   * and the current token (i.e., they are on different lines).
   *
   * This is used by languages with automatic semicolon insertion
   * (JavaScript, Go) to detect line breaks that trigger implicit
   * statement termination. The lexer exposes this as a convenience
   * so callbacks and post-tokenize hooks can set the
   * `TOKEN_PRECEDED_BY_NEWLINE` flag on tokens that need it.
   *
   * Returns false if there is no previous token (start of input).
   */
  precededByNewline(): boolean {
    if (this._previousToken === null) return false;
    return this._previousToken.line < this._currentTokenLine;
  }
}

// ---------------------------------------------------------------------------
// The Grammar-Driven Lexer (Class-Based)
// ---------------------------------------------------------------------------

/**
 * A lexer driven by a `TokenGrammar` (parsed from a `.tokens` file).
 *
 * Instead of hardcoded character-matching logic, this lexer:
 *
 * 1. Compiles each token definition's pattern into a regex.
 * 2. At each position, tries each regex in definition order (first match wins).
 * 3. Emits a `Token` with the matched type and value.
 *
 * The `GrammarLexer` class extends the basic `grammarTokenize` function
 * with two powerful features:
 *
 * - **Pattern groups**: Named sets of patterns that can be activated/
 *   deactivated via a stack. This enables context-sensitive lexing where
 *   different parts of the input use different token patterns.
 *
 * - **On-token callbacks**: A hook that fires after each token match,
 *   allowing external code to push/pop groups, emit synthetic tokens,
 *   suppress tokens, or toggle skip pattern processing.
 *
 * Usage:
 *
 *     import { parseTokenGrammar } from "@coding-adventures/grammar-tools";
 *     import { GrammarLexer } from "@coding-adventures/lexer";
 *
 *     const grammar = parseTokenGrammar(source);
 *     const lexer = new GrammarLexer("<div class=\"main\">hello</div>", grammar);
 *     lexer.setOnToken((token, ctx) => {
 *       if (token.type === "OPEN_TAG") ctx.pushGroup("tag");
 *       if (token.type === "TAG_CLOSE") ctx.popGroup();
 *     });
 *     const tokens = lexer.tokenize();
 */
export class GrammarLexer {
  // -- Source and position tracking --

  /** The complete source code string being tokenized. */
  private _source: string;

  /** Current position (index) in the source string. */
  private _pos: number = 0;

  /** Current line number (1-based), for error reporting. */
  private _line: number = 1;

  /** Current column number (1-based), for error reporting. */
  private _column: number = 1;

  // -- Grammar metadata --

  /** The TokenGrammar that defines which tokens to recognize. */
  private readonly _grammar: TokenGrammar;

  /** Pre-computed set of keywords for O(1) lookup. */
  private readonly _keywordSet: ReadonlySet<string>;

  /** Reserved keywords that cause lex errors. */
  private readonly _reservedSet: ReadonlySet<string>;

  /** Whether the grammar has skip patterns defined. */
  private readonly _hasSkipPatterns: boolean;

  /** Whether indentation mode is active. */
  private readonly _indentationMode: boolean;

  /** Whether Haskell-style layout mode is active. */
  private readonly _layoutMode: boolean;

  /**
   * Whether token matching is case-sensitive.
   *
   * When true (the default), the source is matched as-is. When false,
   * the source is lowercased before matching so that patterns written
   * in lowercase will match input regardless of case.
   */
  private readonly _caseSensitive: boolean;

  /**
   * Whether keyword matching is case-insensitive (from `grammar.caseInsensitive`).
   *
   * When true, NAME tokens are checked against the keyword set using their
   * uppercased form, and keyword tokens are emitted with their value normalized
   * to uppercase. Non-keyword identifiers retain their original casing.
   */
  private readonly _caseInsensitive: boolean;

  // -- Compiled patterns --

  /** Default group compiled patterns, in priority order. */
  private readonly _patterns: CompiledPattern[];

  /** Compiled skip patterns (comments, whitespace). */
  private readonly _skipPatterns: CompiledPattern[];

  /** Compiled patterns per group. "default" + named groups. */
  private readonly _groupPatterns: Record<string, CompiledPattern[]>;

  /** Maps definition names to their aliases (e.g., STRING_DQ -> STRING). */
  private readonly _aliasMap: Record<string, string>;

  // -- Group stack and callback --

  /**
   * The group stack. Bottom is always "default". Top is the active
   * group whose patterns are tried during token matching.
   */
  private _groupStack: string[] = ["default"];

  /**
   * On-token callback — null means no callback (zero overhead).
   * When set, fires after each token match with a LexerContext.
   */
  private _onToken: OnTokenCallback | null = null;

  /**
   * Skip enabled flag — can be toggled by callbacks for groups
   * where whitespace is significant (e.g., CDATA, raw text).
   */
  private _skipEnabled: boolean = true;

  // -- Extension: Token lookbehind --

  /**
   * The most recently emitted token, for lookbehind in callbacks.
   * Updated after each token push (including callback-emitted tokens).
   * Reset to null on each tokenize() call.
   */
  private _lastEmittedToken: Token | null = null;

  // -- Extension: Bracket depth tracking --

  /**
   * Per-type bracket nesting depth counters.
   *
   * Tracks `()`, `[]`, and `{}` independently. Updated after each
   * token match in both standard and indentation modes. Exposed to
   * callbacks via `LexerContext.bracketDepth()`.
   *
   * This enables context-sensitive lexing for template literals,
   * string interpolation, and other constructs where bracket nesting
   * determines how to tokenize subsequent input.
   */
  private _bracketDepths = { paren: 0, bracket: 0, brace: 0 };

  // -- Extension: Context keywords --

  /**
   * Pre-computed set of context-sensitive keywords for O(1) lookup.
   * Words in this set are emitted as NAME with TOKEN_CONTEXT_KEYWORD flag.
   */
  private readonly _contextKeywordSet: ReadonlySet<string>;

  /** Layout introducer keywords used when layout mode is active. */
  private readonly _layoutKeywordSet: ReadonlySet<string>;

  /** Pre-tokenize hooks: transform source text before lexing. */
  private _preTokenizeHooks: Array<(source: string) => string> = [];

  /** Post-tokenize hooks: transform token list after lexing. */
  private _postTokenizeHooks: Array<(tokens: Token[]) => Token[]> = [];

  /** Whether token/trivia source metadata should be preserved. */
  private readonly _preserveSourceInfo: boolean;

  /** Trivia collected since the previous emitted token. */
  private _pendingTrivia: Trivia[] = [];

  /** Sequential token index assigned in emission order. */
  private _nextTokenIndex: number = 0;

  constructor(source: string, grammar: TokenGrammar, options?: GrammarLexerOptions) {
    this._grammar = grammar;
    this._preserveSourceInfo = options?.preserveSourceInfo === true;
    this._caseInsensitive = grammar.caseInsensitive === true;
    this._caseSensitive = grammar.caseSensitive !== false && !this._caseInsensitive;
    // Only lowercase the source for the legacy caseSensitive:false pattern-level mode.
    // For caseInsensitive keyword mode we keep the original source and normalize per-token
    // during keyword lookup, so non-keyword identifiers preserve their original casing.
    this._source =
      !this._caseSensitive && !this._caseInsensitive
        ? source.toLowerCase()
        : source;
    // When case-insensitive, store keywords in uppercase so lookups can use value.toUpperCase().
    this._keywordSet = new Set(
      this._caseInsensitive
        ? grammar.keywords.map((k) => k.toUpperCase())
        : grammar.keywords,
    );
    this._reservedSet = new Set(grammar.reservedKeywords ?? []);
    this._contextKeywordSet = new Set(grammar.contextKeywords ?? []);
    this._indentationMode = grammar.mode === "indentation";
    this._layoutMode = grammar.mode === "layout";
    this._layoutKeywordSet = new Set(grammar.layoutKeywords ?? []);
    this._hasSkipPatterns = (grammar.skipDefinitions ?? []).length > 0;

    // Build alias map: definition name -> alias name.
    // For example, STRING_DQ -> STRING. When we match STRING_DQ, we
    // emit the token type as STRING (the alias).
    this._aliasMap = {};
    for (const defn of grammar.definitions) {
      if (defn.alias) {
        this._aliasMap[defn.name] = defn.alias;
      }
    }

    // When case-insensitive mode is active, compile all regexes with the
    // "i" flag so that patterns written with lowercase character classes
    // (e.g. /[a-z]+/) also match uppercase input. This mirrors the Rust
    // lexer's approach of lowercasing the entire source, but preserves
    // original casing in token values.
    const reFlags = this._caseInsensitive ? "i" : "";

    // Compile token patterns into regex objects.
    // Order matters — patterns are tried in the order they appear in the
    // .tokens file. This is the "first match wins" rule from Lex/Flex.
    this._patterns = grammar.definitions.map((defn) => {
      const patternSource = defn.isRegex ? defn.pattern : escapeRegExp(defn.pattern);
      return {
        name: defn.name,
        pattern: new RegExp(patternSource, reFlags),
        alias: defn.alias,
      };
    });

    // Compile skip patterns (comments, whitespace, etc.).
    // These are tried before token patterns at each position.
    this._skipPatterns = (grammar.skipDefinitions ?? []).map((defn) => {
      const patternSource = defn.isRegex ? defn.pattern : escapeRegExp(defn.pattern);
      return {
        name: defn.name,
        pattern: new RegExp(patternSource, reFlags),
      };
    });

    // --- Pattern groups ---
    // Compile per-group patterns. The "default" group uses the top-level
    // definitions. Named groups use their own definitions. When no groups
    // are defined, _groupPatterns has only "default".
    this._groupPatterns = {
      default: [...this._patterns],
    };

    if (grammar.groups) {
      for (const [groupName, group] of Object.entries(grammar.groups)) {
        const compiled: CompiledPattern[] = group.definitions.map((defn) => {
          const patternSource = defn.isRegex ? defn.pattern : escapeRegExp(defn.pattern);
          // Register aliases from group definitions
          if (defn.alias) {
            this._aliasMap[defn.name] = defn.alias;
          }
          return {
            name: defn.name,
            pattern: new RegExp(patternSource, reFlags),
            alias: defn.alias,
          };
        });
        this._groupPatterns[groupName] = compiled;
      }
    }
  }

  // -- Public API: callback registration --

  /**
   * Register a callback that fires on every token match.
   *
   * The callback receives the matched token and a `LexerContext`. It can
   * use the context to push/pop groups, emit extra tokens, suppress the
   * current token, or toggle skip processing.
   *
   * Only one callback can be registered at a time. Pass `null` to clear.
   *
   * The callback is NOT invoked for:
   * - Skip pattern matches (they produce no tokens)
   * - Tokens emitted via `ctx.emit()` (prevents infinite loops)
   * - The EOF token
   */
  setOnToken(callback: OnTokenCallback | null): void {
    this._onToken = callback;
  }

  // -- Public API: group introspection (used by LexerContext) --

  /** Check whether a group name is defined in the grammar. */
  hasGroup(groupName: string): boolean {
    return groupName in this._groupPatterns;
  }

  /** Return all available group names. */
  availableGroups(): string[] {
    return Object.keys(this._groupPatterns);
  }

  /** Return the name of the currently active group (top of stack). */
  activeGroup(): string {
    return this._groupStack[this._groupStack.length - 1];
  }

  /** Return the depth of the group stack (always >= 1). */
  groupStackDepth(): number {
    return this._groupStack.length;
  }

  // -- Extension: Bracket depth --

  /**
   * Return the current nesting depth for a specific bracket type,
   * or the total depth across all types if no argument is given.
   *
   * This is the public API used by LexerContext to expose bracket
   * depth to callbacks. Language packages use this for template
   * literal interpolation and similar nested constructs.
   */
  bracketDepth(kind?: "paren" | "bracket" | "brace"): number {
    if (kind === undefined) {
      return (
        this._bracketDepths.paren +
        this._bracketDepths.bracket +
        this._bracketDepths.brace
      );
    }
    return this._bracketDepths[kind];
  }

  // -- Hook registration --

  /**
   * Register a text transform to run before tokenization.
   *
   * The hook receives the raw source string and returns a (possibly
   * modified) source string. Multiple hooks compose left-to-right.
   */
  addPreTokenize(hook: (source: string) => string): void {
    this._preTokenizeHooks.push(hook);
  }

  /**
   * Register a token transform to run after tokenization.
   *
   * The hook receives the full token list (including EOF) and returns
   * a (possibly modified) token list. Multiple hooks compose left-to-right.
   */
  addPostTokenize(hook: (tokens: Token[]) => Token[]): void {
    this._postTokenizeHooks.push(hook);
  }

  // -- Main tokenization entry point --

  /**
   * Tokenize the source code using the grammar's token definitions.
   *
   * Dispatches to the appropriate tokenization method based on whether
   * indentation mode is active. Resets the group stack and skip flag
   * at the end so the lexer can be reused for multiple `tokenize()` calls.
   *
   * Pre-tokenize hooks transform the source text before lexing begins.
   * Post-tokenize hooks transform the token list after lexing completes.
   *
   * @returns A list of Token objects, always ending with an EOF token.
   * @throws LexerError if an unexpected character is encountered, a
   *         reserved keyword is used, or indentation is inconsistent.
   */
  tokenize(): Token[] {
    // Stage 1: Pre-tokenize hooks transform the source text.
    if (this._preTokenizeHooks.length > 0) {
      let source = this._source;
      for (const hook of this._preTokenizeHooks) {
        source = hook(source);
      }
      this._source = source;
    }

    // Reset extension state for reuse.
    this._lastEmittedToken = null;
    this._bracketDepths = { paren: 0, bracket: 0, brace: 0 };
    this._pendingTrivia = [];
    this._nextTokenIndex = 0;

    // Stage 2: Core tokenization.
    let tokens: Token[];
    if (this._indentationMode) {
      tokens = this._tokenizeIndentation();
    } else if (this._layoutMode) {
      tokens = this._tokenizeLayout();
    } else {
      tokens = this._tokenizeStandard();
    }

    // Stage 3: Post-tokenize hooks transform the token list.
    for (const hook of this._postTokenizeHooks) {
      tokens = hook(tokens);
    }

    return tokens;
  }

  // -- Standard (non-indentation) tokenization --

  /**
   * Tokenize without indentation tracking.
   *
   * The algorithm:
   *
   * 1. While there are characters left:
   *    a. If skip patterns exist and skip is enabled, try them.
   *    b. If no skip patterns, use default whitespace skip.
   *    c. If the current character is a newline, emit NEWLINE.
   *    d. Try active group's token patterns (first match wins).
   *    e. If callback registered, invoke it and process actions.
   *    f. If nothing matches, raise LexerError.
   * 2. Append EOF.
   *
   * When pattern groups are active, the lexer uses `_groupStack[-1]`
   * to determine which set of patterns to try. When a callback is
   * registered via `setOnToken()`, it fires after each token match
   * and can push/pop groups, emit extra tokens, or suppress the
   * current token.
   */
  private _tokenizeStandard(): Token[] {
    const tokens: Token[] = [];

    while (this._pos < this._source.length) {
      const char = this._source[this._pos];

      // --- Skip patterns (grammar-defined) ---
      // When the grammar has skip patterns AND skip is enabled, they
      // take over whitespace handling. The callback can disable skip
      // processing for groups where whitespace is significant (CDATA).
      if (this._hasSkipPatterns) {
        if (this._skipEnabled && this._trySkip()) {
          continue;
        }
      } else {
        // --- Default whitespace skip ---
        // Without skip patterns, use the hardcoded behavior: skip
        // spaces, tabs, carriage returns silently.
        if (char === " " || char === "\t" || char === "\r") {
          this._consumeDefaultWhitespace();
          continue;
        }
      }

      // --- Newlines become NEWLINE tokens ---
      // Newlines are structural — they mark line boundaries.
      if (char === "\n") {
        const newlineTok: Token = {
          type: "NEWLINE",
          value: "\\n",
          line: this._line,
          column: this._column,
        };
        const startOffset = this._pos;
        this._advance();
        this._emitToken(tokens, this._withOptionalSourceInfo(newlineTok, startOffset));
        continue;
      }

      // --- Try active group's token patterns (first match wins) ---
      // The active group is the top of the group stack. When no
      // groups are defined, this is always "default" (the top-level
      // definitions), preserving backward compatibility.
      const activeGroupName = this._groupStack[this._groupStack.length - 1];
      const token = this._tryMatchTokenInGroup(activeGroupName);
      if (token !== null) {
        // Update bracket depth tracking.
        this._updateBracketDepth(token.value);

        // --- Invoke on-token callback ---
        // The callback can push/pop groups, emit extra tokens,
        // suppress the current token, or toggle skip processing.
        // Emitted tokens do NOT re-trigger the callback.
        if (this._onToken !== null) {
          const ctx = new LexerContext(
            this,
            this._source,
            this._pos,
            this._lastEmittedToken,
            token.line,
          );
          this._onToken(token, ctx);

          // Apply suppression: if the callback suppressed this
          // token, don't add it to the output.
          if (!ctx._suppressed) {
            this._emitToken(tokens, token);
          }

          // Append any tokens emitted by the callback.
          for (const emitted of ctx._emitted) {
            this._emitToken(tokens, emitted);
          }

          // Apply group stack actions in order.
          for (const [action, groupName] of ctx._groupActions) {
            if (action === "push") {
              this._groupStack.push(groupName);
            } else if (action === "pop" && this._groupStack.length > 1) {
              this._groupStack.pop();
            }
          }

          // Apply skip toggle if the callback changed it.
          if (ctx._skipEnabled !== null) {
            this._skipEnabled = ctx._skipEnabled;
          }
        } else {
          this._emitToken(tokens, token);
        }
        continue;
      }

      throw new LexerError(
        `Unexpected character: ${JSON.stringify(char)}`,
        this._line,
        this._column,
      );
    }

    // --- Append EOF sentinel ---
    const eof: Token = {
      type: "EOF",
      value: "",
      line: this._line,
      column: this._column,
    };
    this._emitToken(tokens, this._withOptionalSourceInfo(eof, this._pos));

    // Reset group stack and skip flag for reuse (in case tokenize is
    // called again on the same instance).
    this._groupStack = ["default"];
    this._skipEnabled = true;

    return tokens;
  }

  // -- Extension: Bracket depth tracking helper --

  /**
   * Update bracket depth counters based on a token's value.
   *
   * Called after each token match in both standard and indentation modes.
   * Only single-character values are checked — multi-character tokens
   * cannot be brackets.
   */
  private _updateBracketDepth(value: string): void {
    if (value.length !== 1) return;
    switch (value) {
      case "(": this._bracketDepths.paren++; break;
      case ")": if (this._bracketDepths.paren > 0) this._bracketDepths.paren--; break;
      case "[": this._bracketDepths.bracket++; break;
      case "]": if (this._bracketDepths.bracket > 0) this._bracketDepths.bracket--; break;
      case "{": this._bracketDepths.brace++; break;
      case "}": if (this._bracketDepths.brace > 0) this._bracketDepths.brace--; break;
    }
  }

  // -- Indentation mode tokenization --

  /**
   * Tokenize with Python-style indentation tracking.
   *
   * This method implements the full indentation algorithm: it maintains
   * an indent stack, tracks bracket depth for implicit line joining,
   * and emits synthetic INDENT/DEDENT/NEWLINE tokens.
   */
  private _tokenizeIndentation(): Token[] {
    const tokens: Token[] = [];
    const indentStack: number[] = [0];
    let bracketDepth = 0;
    let atLineStart = true;

    while (this._pos < this._source.length) {
      // Process line start (indentation)
      if (atLineStart && bracketDepth === 0) {
        const result = this._processLineStart(indentStack);
        if (result === "skip") {
          continue;
        }
        for (const token of result) {
          this._emitToken(tokens, token);
        }
        atLineStart = false;
        if (this._pos >= this._source.length) {
          break;
        }
      }

      const char = this._source[this._pos];

      // Newline handling
      if (char === "\n") {
        if (bracketDepth === 0) {
          const newlineTok: Token = {
            type: "NEWLINE",
            value: "\\n",
            line: this._line,
            column: this._column,
          };
          const startOffset = this._pos;
          this._advance();
          this._emitToken(tokens, this._withOptionalSourceInfo(newlineTok, startOffset));
        } else {
          this._advance();
        }
        atLineStart = true;
        continue;
      }

      // Inside brackets: skip whitespace
      if (
        bracketDepth > 0 &&
        (char === " " || char === "\t" || char === "\r")
      ) {
        this._consumeDefaultWhitespace();
        continue;
      }

      // Try skip patterns
      if (this._trySkip()) {
        continue;
      }

      // Try token patterns (always uses default group for indentation mode)
      const tok = this._tryMatchTokenInGroup("default");
      if (tok !== null) {
        // Track bracket depth (local for INDENT/DEDENT logic)
        if (tok.value === "(" || tok.value === "[" || tok.value === "{") {
          bracketDepth++;
        } else if (
          tok.value === ")" ||
          tok.value === "]" ||
          tok.value === "}"
        ) {
          bracketDepth--;
        }
        // Track bracket depth (shared for callback access)
        this._updateBracketDepth(tok.value);
        this._emitToken(tokens, tok);
        continue;
      }

      throw new LexerError(
        `Unexpected character: ${JSON.stringify(char)}`,
        this._line,
        this._column,
      );
    }

    // EOF: emit remaining DEDENTs
    while (indentStack.length > 1) {
      indentStack.pop();
      this._emitToken(tokens, this._withOptionalSourceInfo({
        type: "DEDENT",
        value: "",
        line: this._line,
        column: this._column,
      }, this._pos));
    }

    // Final NEWLINE if needed
    if (
      tokens.length === 0 ||
      tokens[tokens.length - 1].type !== "NEWLINE"
    ) {
      this._emitToken(tokens, this._withOptionalSourceInfo({
        type: "NEWLINE",
        value: "\\n",
        line: this._line,
        column: this._column,
      }, this._pos));
    }

    this._emitToken(tokens, this._withOptionalSourceInfo({
      type: "EOF",
      value: "",
      line: this._line,
      column: this._column,
    }, this._pos));

    // Reset group stack for reuse.
    this._groupStack = ["default"];
    this._skipEnabled = true;

    return tokens;
  }

  private _tokenizeLayout(): Token[] {
    return this._applyLayout(this._tokenizeStandard());
  }

  private _applyLayout(tokens: Token[]): Token[] {
    const result: Token[] = [];
    const layoutStack: number[] = [];
    let pendingLayouts = 0;
    let suppressDepth = 0;

    for (let index = 0; index < tokens.length; index++) {
      const token = tokens[index];
      const typeName = token.typeName ?? token.type;

      if (typeName === "NEWLINE") {
        result.push(token);
        const nextToken = this._nextLayoutToken(tokens, index + 1);
        if (suppressDepth === 0 && nextToken !== null) {
          while (layoutStack.length > 0 && nextToken.column < layoutStack[layoutStack.length - 1]) {
            result.push(this._virtualLayoutToken("VIRTUAL_RBRACE", "}", nextToken));
            layoutStack.pop();
          }

          if (
            layoutStack.length > 0 &&
            (nextToken.typeName ?? nextToken.type) !== "EOF" &&
            nextToken.value !== "}" &&
            nextToken.column === layoutStack[layoutStack.length - 1]
          ) {
            result.push(this._virtualLayoutToken("VIRTUAL_SEMICOLON", ";", nextToken));
          }
        }
        continue;
      }

      if (typeName === "EOF") {
        while (layoutStack.length > 0) {
          result.push(this._virtualLayoutToken("VIRTUAL_RBRACE", "}", token));
          layoutStack.pop();
        }
        result.push(token);
        continue;
      }

      if (pendingLayouts > 0) {
        if (token.value === "{") {
          pendingLayouts -= 1;
        } else {
          for (let count = 0; count < pendingLayouts; count++) {
            layoutStack.push(token.column);
            result.push(this._virtualLayoutToken("VIRTUAL_LBRACE", "{", token));
          }
          pendingLayouts = 0;
        }
      }

      result.push(token);

      if (!this._isVirtualLayoutToken(token)) {
        if (token.value === "(" || token.value === "[" || token.value === "{") {
          suppressDepth += 1;
        } else if ((token.value === ")" || token.value === "]" || token.value === "}") && suppressDepth > 0) {
          suppressDepth -= 1;
        }
      }

      if (this._isLayoutKeyword(token)) {
        pendingLayouts += 1;
      }
    }

    return result;
  }

  private _nextLayoutToken(tokens: Token[], startIndex: number): Token | null {
    for (let index = startIndex; index < tokens.length; index++) {
      const token = tokens[index];
      if ((token.typeName ?? token.type) !== "NEWLINE") {
        return token;
      }
    }
    return null;
  }

  private _virtualLayoutToken(typeName: string, value: string, anchor: Token): Token {
    return this._withOptionalSourceInfo({
      type: typeName,
      typeName,
      value,
      line: anchor.line,
      column: anchor.column,
    }, anchor.startOffset ?? this._pos);
  }

  private _isVirtualLayoutToken(token: Token): boolean {
    return (token.typeName ?? token.type).startsWith("VIRTUAL_");
  }

  private _isLayoutKeyword(token: Token): boolean {
    if (this._layoutKeywordSet.size === 0) {
      return false;
    }
    const value = token.value ?? "";
    return this._layoutKeywordSet.has(value) || this._layoutKeywordSet.has(value.toLowerCase());
  }

  /**
   * Process indentation at the start of a logical line.
   *
   * Returns "skip" if the line should be skipped (blank/comment),
   * or an array of INDENT/DEDENT tokens.
   */
  private _processLineStart(indentStack: number[]): "skip" | Token[] {
    let indent = 0;
    const indentStartLine = this._line;
    const indentStartColumn = this._column;
    const indentStartOffset = this._pos;
    while (this._pos < this._source.length) {
      const char = this._source[this._pos];
      if (char === " ") {
        indent++;
        this._advance();
      } else if (char === "\t") {
        throw new LexerError(
          "Tab character in indentation (use spaces only)",
          this._line,
          this._column,
        );
      } else {
        break;
      }
    }

    if (indent > 0 && this._preserveSourceInfo) {
      this._pushTrivia(
        "WHITESPACE",
        this._source.slice(indentStartOffset, this._pos),
        indentStartLine,
        indentStartColumn,
        indentStartOffset,
      );
    }

    // Blank line or EOF
    if (this._pos >= this._source.length) {
      return "skip";
    }
    if (this._source[this._pos] === "\n") {
      const newlineStartLine = this._line;
      const newlineStartColumn = this._column;
      const newlineStartOffset = this._pos;
      this._advance(); // Consume newline to avoid infinite loop
      this._pushTrivia(
        "NEWLINE",
        "\n",
        newlineStartLine,
        newlineStartColumn,
        newlineStartOffset,
      );
      return "skip";
    }

    // Comment-only line — check skip patterns
    const remaining = this._source.slice(this._pos);
    for (const pat of this._skipPatterns) {
      const match = pat.pattern.exec(remaining);
      if (match !== null && match.index === 0) {
        const peekPos = this._pos + match[0].length;
        if (
          peekPos >= this._source.length ||
          this._source[peekPos] === "\n"
        ) {
          const triviaStartLine = this._line;
          const triviaStartColumn = this._column;
          const triviaStartOffset = this._pos;
          for (let i = 0; i < match[0].length; i++) {
            this._advance();
          }
          this._pushTrivia(
            pat.name,
            match[0],
            triviaStartLine,
            triviaStartColumn,
            triviaStartOffset,
          );
          if (
            this._pos < this._source.length &&
            this._source[this._pos] === "\n"
          ) {
            const newlineStartLine = this._line;
            const newlineStartColumn = this._column;
            const newlineStartOffset = this._pos;
            this._advance();
            this._pushTrivia(
              "NEWLINE",
              "\n",
              newlineStartLine,
              newlineStartColumn,
              newlineStartOffset,
            );
          }
          return "skip";
        }
      }
    }

    // Compare indent to current level
    const currentIndent = indentStack[indentStack.length - 1];
    const indentTokens: Token[] = [];

    if (indent > currentIndent) {
      indentStack.push(indent);
      indentTokens.push(this._withOptionalSourceInfo({
        type: "INDENT",
        value: "",
        line: this._line,
        column: 1,
      }, this._pos));
    } else if (indent < currentIndent) {
      while (
        indentStack.length > 1 &&
        indentStack[indentStack.length - 1] > indent
      ) {
        indentStack.pop();
        indentTokens.push(this._withOptionalSourceInfo({
          type: "DEDENT",
          value: "",
          line: this._line,
          column: 1,
        }, this._pos));
      }
      if (indentStack[indentStack.length - 1] !== indent) {
        throw new LexerError(
          "Inconsistent dedent",
          this._line,
          this._column,
        );
      }
    }

    return indentTokens;
  }

  // -- Shared helpers --

  /**
   * Try to match and consume a skip pattern at the current position.
   *
   * Skip patterns are defined in the `skip:` section of a .tokens file.
   * They match text that should be consumed without emitting a token —
   * typically comments and inline whitespace.
   *
   * @returns true if a skip pattern matched (text was consumed), false otherwise.
   */
  private _trySkip(): boolean {
    const remaining = this._source.slice(this._pos);
    for (const pat of this._skipPatterns) {
      const match = pat.pattern.exec(remaining);
      if (match !== null && match.index === 0) {
        const startLine = this._line;
        const startColumn = this._column;
        const startOffset = this._pos;
        for (let i = 0; i < match[0].length; i++) {
          this._advance();
        }
        this._pushTrivia(pat.name, match[0], startLine, startColumn, startOffset);
        return true;
      }
    }
    return false;
  }

  /**
   * Try to match a token pattern from a specific group.
   *
   * Tries each compiled pattern in the named group in priority order
   * (first match wins). Handles keyword detection, reserved word
   * checking, aliases, and string escape processing.
   *
   * @param groupName - The pattern group to use (e.g., "default", "tag").
   * @returns A Token if a pattern matched, null otherwise.
   */
  private _tryMatchTokenInGroup(groupName: string): Token | null {
    const remaining = this._source.slice(this._pos);
    const patterns = this._groupPatterns[groupName] ?? this._patterns;

    for (const { name, pattern, alias } of patterns) {
      const match = pattern.exec(remaining);
      if (match !== null && match.index === 0) {
        let value = match[0];
        const startLine = this._line;
        const startColumn = this._column;
        const startOffset = this._pos;

        // For case-insensitive grammars, normalize NAME tokens to uppercase for
        // keyword lookup. Keywords are stored uppercase in _keywordSet.
        const lookupValue = this._caseInsensitive ? value.toUpperCase() : value;

        const tokenType = resolveTokenType(
          name,
          lookupValue,
          this._keywordSet,
          this._reservedSet,
          alias,
          startLine,
          startColumn,
        );

        // Keyword tokens are emitted with their normalized (uppercase) value so
        // that `select`, `SELECT`, and `Select` all produce KEYWORD("SELECT").
        // Non-keyword identifiers always keep their original casing.
        if (this._caseInsensitive && tokenType === "KEYWORD") {
          value = lookupValue;
        }

        // Handle STRING tokens: strip quotes and optionally process escapes.
        // When escapeMode is "none", the lexer strips quotes but leaves escape
        // sequences as raw text.
        const effectiveName = this._aliasMap[name] ?? name;
        if (
          effectiveName === "STRING" ||
          name === "STRING" ||
          name.includes("STRING") ||
          (alias && alias.includes("STRING"))
        ) {
          // Multi-line strings use triple quotes, single-line use single quotes.
          if (
            value.length >= 6 &&
            (value.startsWith('"""') || value.startsWith("'''"))
          ) {
            const inner = value.slice(3, -3);
            value =
              this._grammar.escapeMode === "none"
                ? inner
                : processEscapes(inner);
          } else if (
            value.length >= 2 &&
            (value[0] === '"' || value[0] === "'")
          ) {
            const inner = value.slice(1, -1);
            value =
              this._grammar.escapeMode === "none"
                ? inner
                : processEscapes(inner);
          }
        }

        // Check if this NAME token is a context keyword — a word that
        // is sometimes a keyword and sometimes an identifier depending
        // on syntactic position. Context keywords are emitted as NAME
        // with the TOKEN_CONTEXT_KEYWORD flag, leaving the final
        // decision to the language-specific parser or callback.
        let flags: number | undefined;
        if (
          tokenType === "NAME" &&
          this._contextKeywordSet.size > 0 &&
          this._contextKeywordSet.has(value)
        ) {
          flags = TOKEN_CONTEXT_KEYWORD;
        }

        const tok: Token = flags !== undefined
          ? { type: tokenType, value, line: startLine, column: startColumn, flags }
          : { type: tokenType, value, line: startLine, column: startColumn };

        for (let i = 0; i < match[0].length; i++) {
          this._advance();
        }

        return this._withOptionalSourceInfo(tok, startOffset);
      }
    }
    return null;
  }

  private _consumeDefaultWhitespace(): void {
    const startLine = this._line;
    const startColumn = this._column;
    const startOffset = this._pos;
    while (this._pos < this._source.length) {
      const char = this._source[this._pos];
      if (char !== " " && char !== "\t" && char !== "\r") {
        break;
      }
      this._advance();
    }
    if (this._pos > startOffset) {
      this._pushTrivia(
        "WHITESPACE",
        this._source.slice(startOffset, this._pos),
        startLine,
        startColumn,
        startOffset,
      );
    }
  }

  private _pushTrivia(
    type: string,
    value: string,
    line: number,
    column: number,
    startOffset: number,
  ): void {
    if (!this._preserveSourceInfo) {
      return;
    }
    this._pendingTrivia.push({
      type,
      value,
      line,
      column,
      endLine: this._line,
      endColumn: this._column,
      startOffset,
      endOffset: this._pos,
    });
  }

  private _withOptionalSourceInfo(token: Token, startOffset: number): Token {
    if (!this._preserveSourceInfo) {
      return token;
    }
    return {
      ...token,
      startOffset,
      endOffset: this._pos,
      endLine: this._line,
      endColumn: this._column,
    };
  }

  private _emitToken(tokens: Token[], token: Token): void {
    let finalized = token;
    if (this._preserveSourceInfo) {
      finalized = {
        ...token,
        tokenIndex: this._nextTokenIndex++,
        ...(this._pendingTrivia.length > 0
          ? { leadingTrivia: [...this._pendingTrivia] }
          : {}),
      };
      this._pendingTrivia = [];
    }
    tokens.push(finalized);
    this._lastEmittedToken = finalized;
  }

  /**
   * Move position forward by one character, tracking line and column.
   *
   * When we encounter a newline character, we increment the line counter
   * and reset the column to 1. For all other characters, we just increment
   * the column.
   */
  private _advance(): void {
    if (this._pos < this._source.length) {
      if (this._source[this._pos] === "\n") {
        this._line += 1;
        this._column = 1;
      } else {
        this._column += 1;
      }
      this._pos += 1;
    }
  }
}

// ---------------------------------------------------------------------------
// Convenience function — backward-compatible wrapper
// ---------------------------------------------------------------------------

/**
 * Tokenize source code using a grammar (parsed from a `.tokens` file).
 *
 * This is a convenience wrapper around `GrammarLexer` that provides
 * backward compatibility with the original function-based API. It creates
 * a `GrammarLexer` instance and calls `tokenize()` on it.
 *
 * For advanced features like pattern groups and on-token callbacks, use
 * the `GrammarLexer` class directly.
 *
 * @param source - The raw source code text to tokenize.
 * @param grammar - A TokenGrammar object (parsed from a .tokens file).
 * @returns A list of Token objects, always ending with an EOF token.
 */
export function grammarTokenize(
  source: string,
  grammar: TokenGrammar,
  options?: GrammarLexerOptions,
): Token[] {
  return new GrammarLexer(source, grammar, options).tokenize();
}
