// ============================================================================
// GrammarLexer.swift — Grammar-Driven Lexer with Pattern Groups
// ============================================================================
//
// The Grammar-Driven Lexer: Tokenization from .tokens Files
// ==========================================================
//
// The hand-written tokenize functions in many compilers hardcode which
// characters map to which tokens. That works well for a single language,
// but what if you want to tokenize Python *and* Ruby *and* JavaScript
// with the same codebase? You would need to rewrite the character-
// dispatching logic for each language.
//
// This module takes a different approach, inspired by classic tools like
// Lex (https://en.wikipedia.org/wiki/Lex_(software)) and
// Flex (https://en.wikipedia.org/wiki/Flex_(lexical_analyser_generator)).
// Instead of hardcoding patterns in Swift, we read token definitions
// from a TokenGrammar (parsed from a .tokens file by grammar-tools)
// and use those definitions to drive tokenization at runtime.
//
// The algorithm is straightforward:
//
//     1. Compile each token definition's pattern into an NSRegularExpression.
//     2. At each position in the source, try each regex in definition order.
//     3. First match wins -- emit a Token with the matched type and value.
//     4. If nothing matches, throw a LexerError.
//
// Extensions for Starlark/Python-like Languages
// ----------------------------------------------
//
// Beyond basic regex-driven tokenization, this module supports:
//
// - **Skip patterns**: Whitespace and comment patterns that are consumed
//   without producing tokens.
// - **Type aliases**: A token definition like `STRING_DQ -> STRING` emits
//   tokens with type "STRING" instead of "STRING_DQ".
// - **Reserved keywords**: Identifiers that must not appear in source code
//   (e.g., `class` and `import` in Starlark). Raises LexerError on match.
// - **Indentation mode**: For Python-like languages, tracks indentation
//   levels and emits synthetic INDENT/DEDENT/NEWLINE tokens.
//
// Pattern Groups and On-Token Callbacks
// --------------------------------------
//
// Pattern groups enable **context-sensitive lexing**. A grammar can define
// named groups of patterns (e.g., a "tag" group for XML attributes) that
// are only active when the group is at the top of the lexer's group stack.
//
// The lexer maintains a stack of group names, starting with "default".
// An **on-token callback** can push/pop groups, emit synthetic tokens,
// suppress the current token, or toggle skip pattern processing. This
// enables lexing of context-sensitive languages like XML/HTML where
// different parts of the input require different token patterns.
//
// New Extensions (beyond the base TypeScript implementation)
// ----------------------------------------------------------
//
// - **_lastEmittedToken**: Tracks the most recently emitted token for
//   `previousToken()` lookbehind in callbacks.
// - **_bracketDepths**: Tracks `()`, `[]`, `{}` nesting independently
//   for `bracketDepth()` in callbacks.
// - **_contextKeywordSet**: Words that are sometimes keywords and sometimes
//   identifiers, emitted with TOKEN_CONTEXT_KEYWORD flag.
// - **precededByNewline()**: Detects line breaks between consecutive tokens
//   for automatic semicolon insertion support.
//
// ============================================================================

import Foundation
import GrammarTools

// ---------------------------------------------------------------------------
// Bracket Kind Enum
// ---------------------------------------------------------------------------

/// The three bracket types tracked by the lexer.
///
/// Each opener/closer pair is tracked independently so callbacks can
/// query the nesting depth of a specific bracket type. This is critical
/// for template literal interpolation where `}` at brace-depth 0 has
/// different semantics than `}` at brace-depth > 0.
///
///     Bracket Type | Opener | Closer
///     -------------|--------|-------
///     paren        |   (    |   )
///     bracket      |   [    |   ]
///     brace        |   {    |   }
///
public enum BracketKind: Sendable {
    case paren
    case bracket
    case brace
}

// ---------------------------------------------------------------------------
// Compiled Pattern — Internal
// ---------------------------------------------------------------------------

/// A compiled token pattern -- ready for regex matching.
///
/// Each compiled pattern pairs a token name (like "NUMBER" or "TAG_NAME")
/// with an NSRegularExpression. The optional `alias` field maps the
/// definition name to a different token type for emission.
///
/// Example: if the .tokens file defines `STRING_DQ -> STRING`, then:
///   - name = "STRING_DQ"
///   - alias = "STRING"
///   - The emitted token type will be "STRING" (the alias).
///
private struct CompiledPattern {
    let name: String
    let regex: NSRegularExpression
    let alias: String?
}

// ---------------------------------------------------------------------------
// On-Token Callback Type
// ---------------------------------------------------------------------------

/// Signature for on-token callbacks.
///
/// The callback receives the matched token and a `LexerContext` that
/// provides controlled access to group stack manipulation, token emission,
/// suppression, lookahead, and skip pattern toggling.
///
/// The callback is NOT invoked for:
/// - Skip pattern matches (they produce no tokens)
/// - Tokens emitted via `ctx.emit()` (prevents infinite loops)
/// - The EOF token
///
public typealias OnTokenCallback = (Token, LexerContext) -> Void

// ---------------------------------------------------------------------------
// Escape Helper for Literal Patterns
// ---------------------------------------------------------------------------

/// Escape special regex characters in a literal pattern string.
///
/// When a .tokens file defines a literal pattern like `"+"`, we need to
/// escape the `+` so it is treated as a literal character in the regex,
/// not as a quantifier. This function handles all regex-special characters.
///
/// Characters escaped: . * + ? ^ $ { } ( ) | [ ] \
///
private func escapeRegExp(_ s: String) -> String {
    let specialChars = CharacterSet(charactersIn: ".*+?^${}()|[]\\")
    var result = ""
    for char in s.unicodeScalars {
        if specialChars.contains(char) {
            result.append("\\")
        }
        result.append(Character(char))
    }
    return result
}

// ---------------------------------------------------------------------------
// Escape Sequence Processing
// ---------------------------------------------------------------------------

/// Process escape sequences in a string value.
///
/// Handles the standard escape sequences: `\n` (newline), `\t` (tab),
/// `\\` (literal backslash), `\"` (literal double quote). Unknown escape
/// sequences pass through the escaped character (e.g., `\x` becomes `x`).
///
/// Escape sequence table:
///
///     Input     | Output
///     ----------|--------
///     \n        | newline
///     \t        | tab
///     \\        | \
///     \"        | "
///     \x        | x (unknown, pass through)
///
private func processEscapes(_ s: String) -> String {
    let escapeMap: [Character: Character] = [
        "n": "\n",
        "t": "\t",
        "\\": "\\",
        "\"": "\"",
    ]

    var result = ""
    var iterator = s.makeIterator()

    while let char = iterator.next() {
        if char == "\\" {
            if let nextChar = iterator.next() {
                result.append(escapeMap[nextChar] ?? nextChar)
            }
        } else {
            result.append(char)
        }
    }

    return result
}

// ---------------------------------------------------------------------------
// Token Type Resolution
// ---------------------------------------------------------------------------

/// Resolve a grammar token name and matched value to a token type string.
///
/// The resolution follows a priority order:
///
///     1. Reserved keyword check: If NAME and value is reserved, throw error.
///     2. Keyword detection: If NAME and value is a keyword, return "KEYWORD".
///     3. Alias resolution: If the definition has an alias, use it.
///     4. Direct name: Use the definition name as-is.
///
/// This priority ensures that keywords always take precedence over aliases,
/// and reserved words always cause errors regardless of other configuration.
///
private func resolveTokenType(
    tokenName: String,
    value: String,
    keywordSet: Set<String>,
    reservedSet: Set<String>,
    alias: String?,
    line: Int,
    column: Int
) throws -> String {
    // Reserved keyword check -- these are hard errors
    if tokenName == "NAME" && reservedSet.contains(value) {
        throw LexerError(
            "Reserved keyword '\(value)' cannot be used as an identifier",
            line: line,
            column: column
        )
    }

    // Regular keyword check -- promote NAME to KEYWORD
    if tokenName == "NAME" && keywordSet.contains(value) {
        return "KEYWORD"
    }

    // Alias takes precedence over raw name
    if let alias = alias {
        return alias
    }

    return tokenName
}

// ---------------------------------------------------------------------------
// The Grammar-Driven Lexer (Class-Based)
// ---------------------------------------------------------------------------

/// A lexer driven by a `TokenGrammar` (parsed from a .tokens file).
///
/// Instead of hardcoded character-matching logic, this lexer:
///
/// 1. Compiles each token definition's pattern into an NSRegularExpression.
/// 2. At each position, tries each regex in definition order (first match wins).
/// 3. Emits a `Token` with the matched type and value.
///
/// The `GrammarLexer` class extends basic grammar tokenization with:
///
/// - **Pattern groups**: Named sets of patterns that can be activated/
///   deactivated via a stack. This enables context-sensitive lexing where
///   different parts of the input use different token patterns.
///
/// - **On-token callbacks**: A hook that fires after each token match,
///   allowing external code to push/pop groups, emit synthetic tokens,
///   suppress tokens, or toggle skip pattern processing.
///
/// - **Bracket depth tracking**: Automatic tracking of `()`, `[]`, `{}`
///   nesting for template literal interpolation and similar constructs.
///
/// - **Token lookbehind**: The most recently emitted token is available
///   via `LexerContext.previousToken()` for context-sensitive decisions.
///
/// - **Context keywords**: Words that are sometimes keywords and sometimes
///   identifiers, emitted as NAME with TOKEN_CONTEXT_KEYWORD flag.
///
/// Usage:
///
///     import GrammarTools
///
///     let grammar = parseTokenGrammar(source)
///     let lexer = GrammarLexer(source: "<div class=\"main\">hello</div>", grammar: grammar)
///     lexer.setOnToken { token, ctx in
///         if token.type == "OPEN_TAG" { ctx.pushGroup("tag") }
///         if token.type == "TAG_CLOSE" { ctx.popGroup() }
///     }
///     let tokens = lexer.tokenize()
///
public final class GrammarLexer {

    // -- Source and position tracking --

    /// The complete source code string being tokenized.
    /// May be modified by pre-tokenize hooks before lexing begins.
    private var _source: String

    /// The source as an array of characters for O(1) indexed access.
    /// Rebuilt whenever _source changes (pre-tokenize hooks).
    private var _chars: [Character]

    /// Current position (index) in the _chars array.
    private var _pos: Int = 0

    /// Current line number (1-based), for error reporting.
    private var _line: Int = 1

    /// Current column number (1-based), for error reporting.
    private var _column: Int = 1

    // -- Grammar metadata --

    /// The TokenGrammar that defines which tokens to recognize.
    private let _grammar: TokenGrammar

    /// Pre-computed set of keywords for O(1) lookup.
    private let _keywordSet: Set<String>

    /// Reserved keywords that cause lex errors.
    private let _reservedSet: Set<String>

    /// Whether the grammar has skip patterns defined.
    private let _hasSkipPatterns: Bool

    /// Whether indentation mode is active.
    private let _indentationMode: Bool

    /// Whether Haskell-style layout post-processing is active.
    private let _layoutMode: Bool

    /// Whether keyword matching is case-insensitive.
    ///
    /// When true, NAME tokens are checked against the keyword set using their
    /// uppercased form, and keyword tokens are emitted with their value normalized
    /// to uppercase. Non-keyword identifiers retain their original casing.
    private let _caseInsensitive: Bool

    // -- Compiled patterns --

    /// Default group compiled patterns, in priority order.
    private let _patterns: [CompiledPattern]

    /// Compiled skip patterns (comments, whitespace).
    private let _skipPatterns: [NSRegularExpression]

    /// Compiled patterns per group. "default" + named groups.
    private var _groupPatterns: [String: [CompiledPattern]]

    /// Maps definition names to their aliases (e.g., STRING_DQ -> STRING).
    private var _aliasMap: [String: String]

    // -- Group stack and callback --

    /// The group stack. Bottom is always "default". Top is the active
    /// group whose patterns are tried during token matching.
    private var _groupStack: [String] = ["default"]

    /// On-token callback -- nil means no callback (zero overhead).
    /// When set, fires after each token match with a LexerContext.
    private var _onToken: OnTokenCallback? = nil

    /// Skip enabled flag -- can be toggled by callbacks for groups
    /// where whitespace is significant (e.g., CDATA, raw text).
    private var _skipEnabled: Bool = true

    // -- Extension: Token lookbehind --

    /// The most recently emitted token, for lookbehind in callbacks.
    /// Updated after each token push (including callback-emitted tokens).
    /// Reset to nil on each tokenize() call.
    private var _lastEmittedToken: Token? = nil

    // -- Extension: Bracket depth tracking --

    /// Per-type bracket nesting depth counters.
    ///
    /// Tracks `()`, `[]`, and `{}` independently. Updated after each
    /// token match in both standard and indentation modes. Exposed to
    /// callbacks via `LexerContext.bracketDepth()`.
    private var _bracketDepths: (paren: Int, bracket: Int, brace: Int) = (0, 0, 0)

    // -- Extension: Context keywords --

    /// Pre-computed set of context-sensitive keywords for O(1) lookup.
    /// Words in this set are emitted as NAME with TOKEN_CONTEXT_KEYWORD flag.
    private let _contextKeywordSet: Set<String>

    /// Pre-computed set of layout introducer keywords for O(1) lookup.
    private let _layoutKeywordSet: Set<String>

    // -- Pre/post tokenize hooks --

    /// Pre-tokenize hooks: transform source text before lexing.
    private var _preTokenizeHooks: [(String) -> String] = []

    /// Post-tokenize hooks: transform token list after lexing.
    private var _postTokenizeHooks: [([Token]) -> [Token]] = []

    /// The escape mode from the grammar (controls string processing).
    private let _escapeMode: String?

    // -----------------------------------------------------------------------
    // Initialization
    // -----------------------------------------------------------------------

    /// Create a new GrammarLexer from source code and a TokenGrammar.
    ///
    /// The grammar defines which token patterns to recognize, in what order,
    /// and which identifiers are keywords. Pattern compilation happens here
    /// (once) so the hot path in `tokenize()` only does regex matching.
    ///
    /// - Parameters:
    ///   - source: The raw source code text to tokenize.
    ///   - grammar: A TokenGrammar (parsed from a .tokens file).
    ///
    public init(source: String, grammar: TokenGrammar) {
        self._grammar = grammar
        self._caseInsensitive = grammar.caseInsensitive
        self._escapeMode = grammar.escapeMode

        // Store source (case handling).
        //
        // Two orthogonal case directives exist in .tokens files:
        //
        //   case_sensitive: false     — the source is lowercased before matching,
        //                               so lowercase patterns like /[a-z]+/ can match
        //                               uppercase input like "PRINT".
        //
        //   # @case_insensitive true  — keyword lookup compares the uppercased form
        //                               of each NAME against the keyword set, and emits
        //                               KEYWORD tokens with their uppercase value.
        //
        // A grammar may set BOTH directives (e.g. dartmouth_basic.tokens). In that
        // case we must STILL lowercase the source: the patterns are lowercase-only,
        // so skipping the lowercasing step would leave uppercase input unmatched.
        //
        // The `_caseInsensitive` flag therefore only controls keyword-promotion
        // behaviour (lines 1009–1027), not source lowercasing. Source lowercasing
        // is driven solely by `case_sensitive: false`.
        let caseSensitive = grammar.caseSensitive ?? true
        if !caseSensitive {
            self._source = source.lowercased()
        } else {
            self._source = source
        }
        self._chars = Array(self._source)

        // Build keyword set. When case-insensitive, store uppercase for comparison.
        if _caseInsensitive {
            self._keywordSet = Set(grammar.keywords.map { $0.uppercased() })
        } else {
            self._keywordSet = Set(grammar.keywords)
        }

        self._reservedSet = Set(grammar.reservedKeywords ?? [])
        self._contextKeywordSet = Set(grammar.contextKeywords ?? [])
        self._indentationMode = grammar.mode == "indentation"
        self._layoutMode = grammar.mode == "layout"
        self._layoutKeywordSet = Set(grammar.layoutKeywords ?? [])
        self._hasSkipPatterns = !(grammar.skipDefinitions ?? []).isEmpty

        // Build alias map: definition name -> alias name.
        self._aliasMap = [:]
        for defn in grammar.definitions {
            if let alias = defn.alias {
                self._aliasMap[defn.name] = alias
            }
        }

        // Compile token patterns into NSRegularExpression objects.
        // Order matters -- patterns are tried in the order they appear in the
        // .tokens file. This is the "first match wins" rule from Lex/Flex.
        self._patterns = grammar.definitions.compactMap { defn in
            let patternSource = defn.isRegex ? defn.pattern : escapeRegExp(defn.pattern)
            // Anchor to start of remaining text with \A
            guard let regex = try? NSRegularExpression(pattern: "\\A(?:\(patternSource))") else {
                return nil
            }
            return CompiledPattern(name: defn.name, regex: regex, alias: defn.alias)
        }

        // Compile skip patterns.
        self._skipPatterns = (grammar.skipDefinitions ?? []).compactMap { defn in
            let patternSource = defn.isRegex ? defn.pattern : escapeRegExp(defn.pattern)
            return try? NSRegularExpression(pattern: "\\A(?:\(patternSource))")
        }

        // Compile per-group patterns.
        self._groupPatterns = ["default": _patterns]

        if let groups = grammar.groups {
            for (groupName, group) in groups {
                let compiled: [CompiledPattern] = group.definitions.compactMap { defn in
                    let patternSource = defn.isRegex ? defn.pattern : escapeRegExp(defn.pattern)
                    if let alias = defn.alias {
                        self._aliasMap[defn.name] = alias
                    }
                    guard let regex = try? NSRegularExpression(pattern: "\\A(?:\(patternSource))") else {
                        return nil
                    }
                    return CompiledPattern(name: defn.name, regex: regex, alias: defn.alias)
                }
                self._groupPatterns[groupName] = compiled
            }
        }
    }

    // -----------------------------------------------------------------------
    // Public API: Callback Registration
    // -----------------------------------------------------------------------

    /// Register a callback that fires on every token match.
    ///
    /// The callback receives the matched token and a `LexerContext`. It can
    /// use the context to push/pop groups, emit extra tokens, suppress the
    /// current token, or toggle skip processing.
    ///
    /// Only one callback can be registered at a time. Pass `nil` to clear.
    ///
    /// The callback is NOT invoked for:
    /// - Skip pattern matches (they produce no tokens)
    /// - Tokens emitted via `ctx.emit()` (prevents infinite loops)
    /// - The EOF token
    ///
    /// - Parameter callback: The on-token callback, or nil to clear.
    ///
    public func setOnToken(_ callback: OnTokenCallback?) {
        self._onToken = callback
    }

    // -----------------------------------------------------------------------
    // Public API: Group Introspection (used by LexerContext)
    // -----------------------------------------------------------------------

    /// Check whether a group name is defined in the grammar.
    public func hasGroup(_ groupName: String) -> Bool {
        return _groupPatterns[groupName] != nil
    }

    /// Return all available group names.
    public func availableGroups() -> [String] {
        return Array(_groupPatterns.keys)
    }

    /// Return the name of the currently active group (top of stack).
    public func activeGroup() -> String {
        return _groupStack.last!
    }

    /// Return the depth of the group stack (always >= 1).
    public func groupStackDepth() -> Int {
        return _groupStack.count
    }

    // -----------------------------------------------------------------------
    // Extension: Bracket Depth (public for LexerContext)
    // -----------------------------------------------------------------------

    /// Return the current nesting depth for a specific bracket type,
    /// or the total depth across all types if no argument is given.
    ///
    /// - Parameter kind: Optional bracket type. If nil, returns the sum.
    /// - Returns: The bracket nesting depth.
    ///
    public func bracketDepth(kind: BracketKind? = nil) -> Int {
        guard let kind = kind else {
            return _bracketDepths.paren + _bracketDepths.bracket + _bracketDepths.brace
        }
        switch kind {
        case .paren: return _bracketDepths.paren
        case .bracket: return _bracketDepths.bracket
        case .brace: return _bracketDepths.brace
        }
    }

    // -----------------------------------------------------------------------
    // Hook Registration
    // -----------------------------------------------------------------------

    /// Register a text transform to run before tokenization.
    ///
    /// The hook receives the raw source string and returns a (possibly
    /// modified) source string. Multiple hooks compose left-to-right.
    ///
    public func addPreTokenize(_ hook: @escaping (String) -> String) {
        _preTokenizeHooks.append(hook)
    }

    /// Register a token transform to run after tokenization.
    ///
    /// The hook receives the full token list (including EOF) and returns
    /// a (possibly modified) token list. Multiple hooks compose left-to-right.
    ///
    public func addPostTokenize(_ hook: @escaping ([Token]) -> [Token]) {
        _postTokenizeHooks.append(hook)
    }

    // -----------------------------------------------------------------------
    // Main Tokenization Entry Point
    // -----------------------------------------------------------------------

    /// Tokenize the source code using the grammar's token definitions.
    ///
    /// Dispatches to the appropriate tokenization method based on whether
    /// indentation mode is active. Resets the group stack and skip flag
    /// at the end so the lexer can be reused for multiple `tokenize()` calls.
    ///
    /// Pre-tokenize hooks transform the source text before lexing begins.
    /// Post-tokenize hooks transform the token list after lexing completes.
    ///
    /// - Returns: A list of Token objects, always ending with an EOF token.
    /// - Throws: LexerError if an unexpected character is encountered, a
    ///           reserved keyword is used, or indentation is inconsistent.
    ///
    public func tokenize() throws -> [Token] {
        // Stage 1: Pre-tokenize hooks transform the source text.
        if !_preTokenizeHooks.isEmpty {
            var source = _source
            for hook in _preTokenizeHooks {
                source = hook(source)
            }
            _source = source
            _chars = Array(source)
        }

        // Reset extension state for reuse.
        _lastEmittedToken = nil
        _bracketDepths = (0, 0, 0)
        _pos = 0
        _line = 1
        _column = 1

        // Stage 2: Core tokenization.
        var tokens: [Token]
        if _indentationMode {
            tokens = try _tokenizeIndentation()
        } else if _layoutMode {
            tokens = try _tokenizeLayout()
        } else {
            tokens = try _tokenizeStandard()
        }

        // Stage 3: Post-tokenize hooks transform the token list.
        for hook in _postTokenizeHooks {
            tokens = hook(tokens)
        }

        return tokens
    }

    // -----------------------------------------------------------------------
    // Standard (non-indentation) Tokenization
    // -----------------------------------------------------------------------

    /// Tokenize without indentation tracking.
    ///
    /// The algorithm:
    ///
    ///     1. While there are characters left:
    ///        a. If skip patterns exist and skip is enabled, try them.
    ///        b. If no skip patterns, use default whitespace skip.
    ///        c. If the current character is a newline, emit NEWLINE.
    ///        d. Try active group's token patterns (first match wins).
    ///        e. If callback registered, invoke it and process actions.
    ///        f. If nothing matches, raise LexerError.
    ///     2. Append EOF.
    ///
    private func _tokenizeStandard() throws -> [Token] {
        var tokens: [Token] = []

        while _pos < _chars.count {
            let char = _chars[_pos]

            // --- Skip patterns (grammar-defined) ---
            if _hasSkipPatterns {
                if _skipEnabled && _trySkip() {
                    continue
                }
            } else {
                // --- Default whitespace skip ---
                if char == " " || char == "\t" || char == "\r" {
                    _advance()
                    continue
                }
            }

            // --- Newlines become NEWLINE tokens ---
            if char == "\n" {
                let newlineTok = Token(type: "NEWLINE", value: "\\n", line: _line, column: _column)
                tokens.append(newlineTok)
                _lastEmittedToken = newlineTok
                _advance()
                continue
            }

            // --- Try active group's token patterns (first match wins) ---
            let activeGroupName = _groupStack.last!
            if let token = try _tryMatchTokenInGroup(activeGroupName) {
                // Update bracket depth tracking.
                _updateBracketDepth(token.value)

                // --- Invoke on-token callback ---
                if let onToken = _onToken {
                    let ctx = LexerContext(
                        lexer: self,
                        source: _source,
                        posAfterToken: _pos,
                        previousToken: _lastEmittedToken,
                        currentTokenLine: token.line
                    )
                    onToken(token, ctx)

                    // Apply suppression.
                    if !ctx.suppressed {
                        tokens.append(token)
                        _lastEmittedToken = token
                    }

                    // Append any tokens emitted by the callback.
                    for emitted in ctx.emitted {
                        tokens.append(emitted)
                        _lastEmittedToken = emitted
                    }

                    // Apply group stack actions in order.
                    for (action, groupName) in ctx.groupActions {
                        if action == "push" {
                            _groupStack.append(groupName)
                        } else if action == "pop" && _groupStack.count > 1 {
                            _groupStack.removeLast()
                        }
                    }

                    // Apply skip toggle if the callback changed it.
                    if let skipOverride = ctx.skipEnabledOverride {
                        _skipEnabled = skipOverride
                    }
                } else {
                    tokens.append(token)
                    _lastEmittedToken = token
                }
                continue
            }

            throw LexerError(
                "Unexpected character: \"\(char)\"",
                line: _line,
                column: _column
            )
        }

        // --- Append EOF sentinel ---
        tokens.append(Token(type: "EOF", value: "", line: _line, column: _column))

        // Reset group stack and skip flag for reuse.
        _groupStack = ["default"]
        _skipEnabled = true

        return tokens
    }

    private func _tokenizeLayout() throws -> [Token] {
        return _applyLayout(try _tokenizeStandard())
    }

    private func _applyLayout(_ tokens: [Token]) -> [Token] {
        var result: [Token] = []
        var layoutStack: [Int] = []
        var pendingLayouts = 0
        var suppressDepth = 0

        for (index, token) in tokens.enumerated() {
            if token.type == "NEWLINE" {
                result.append(token)

                if suppressDepth == 0, let nextToken = _nextLayoutToken(tokens, startIndex: index + 1) {
                    while let current = layoutStack.last, nextToken.column < current {
                        result.append(_virtualLayoutToken(type: "VIRTUAL_RBRACE", value: "}", anchor: nextToken))
                        layoutStack.removeLast()
                    }

                    if let current = layoutStack.last,
                       nextToken.type != "EOF",
                       nextToken.value != "}",
                       nextToken.column == current {
                        result.append(_virtualLayoutToken(type: "VIRTUAL_SEMICOLON", value: ";", anchor: nextToken))
                    }
                }
                continue
            }

            if token.type == "EOF" {
                while !layoutStack.isEmpty {
                    result.append(_virtualLayoutToken(type: "VIRTUAL_RBRACE", value: "}", anchor: token))
                    layoutStack.removeLast()
                }
                result.append(token)
                continue
            }

            if pendingLayouts > 0 {
                if token.value == "{" {
                    pendingLayouts -= 1
                } else {
                    for _ in 0..<pendingLayouts {
                        layoutStack.append(token.column)
                        result.append(_virtualLayoutToken(type: "VIRTUAL_LBRACE", value: "{", anchor: token))
                    }
                    pendingLayouts = 0
                }
            }

            result.append(token)

            if !_isVirtualLayoutToken(token) {
                if token.value == "(" || token.value == "[" || token.value == "{" {
                    suppressDepth += 1
                } else if (token.value == ")" || token.value == "]" || token.value == "}") && suppressDepth > 0 {
                    suppressDepth -= 1
                }
            }

            if _isLayoutKeyword(token) {
                pendingLayouts += 1
            }
        }

        return result
    }

    private func _nextLayoutToken(_ tokens: [Token], startIndex: Int) -> Token? {
        guard startIndex < tokens.count else { return nil }
        for token in tokens[startIndex...] where token.type != "NEWLINE" {
            return token
        }
        return nil
    }

    private func _virtualLayoutToken(type: String, value: String, anchor: Token) -> Token {
        return Token(type: type, value: value, line: anchor.line, column: anchor.column)
    }

    private func _isVirtualLayoutToken(_ token: Token) -> Bool {
        return token.type.hasPrefix("VIRTUAL_")
    }

    private func _isLayoutKeyword(_ token: Token) -> Bool {
        guard !_layoutKeywordSet.isEmpty else { return false }
        let value = token.value
        return _layoutKeywordSet.contains(value) || _layoutKeywordSet.contains(value.lowercased())
    }

    // -----------------------------------------------------------------------
    // Extension: Bracket Depth Tracking Helper
    // -----------------------------------------------------------------------

    /// Update bracket depth counters based on a token's value.
    ///
    /// Called after each token match in both standard and indentation modes.
    /// Only single-character values are checked -- multi-character tokens
    /// cannot be brackets.
    ///
    ///     Character | Action
    ///     ----------|------------------
    ///     (         | paren depth + 1
    ///     )         | paren depth - 1 (clamped to 0)
    ///     [         | bracket depth + 1
    ///     ]         | bracket depth - 1 (clamped to 0)
    ///     {         | brace depth + 1
    ///     }         | brace depth - 1 (clamped to 0)
    ///
    private func _updateBracketDepth(_ value: String) {
        guard value.count == 1 else { return }
        let ch = value.first!
        switch ch {
        case "(": _bracketDepths.paren += 1
        case ")": if _bracketDepths.paren > 0 { _bracketDepths.paren -= 1 }
        case "[": _bracketDepths.bracket += 1
        case "]": if _bracketDepths.bracket > 0 { _bracketDepths.bracket -= 1 }
        case "{": _bracketDepths.brace += 1
        case "}": if _bracketDepths.brace > 0 { _bracketDepths.brace -= 1 }
        default: break
        }
    }

    // -----------------------------------------------------------------------
    // Indentation Mode Tokenization
    // -----------------------------------------------------------------------

    /// Tokenize with Python-style indentation tracking.
    ///
    /// This method implements the full indentation algorithm: it maintains
    /// an indent stack, tracks bracket depth for implicit line joining,
    /// and emits synthetic INDENT/DEDENT/NEWLINE tokens.
    ///
    /// The indentation algorithm:
    ///
    ///     1. At the start of each line, measure the indentation level.
    ///     2. Compare with the current indent level on the stack:
    ///        - If greater: push new level, emit INDENT.
    ///        - If less: pop levels until matching, emit DEDENT for each pop.
    ///        - If equal: no INDENT/DEDENT tokens.
    ///     3. Blank lines and comment-only lines are skipped entirely.
    ///     4. Inside brackets, newlines and indentation are ignored.
    ///     5. At EOF, emit DEDENT for each remaining level above 0.
    ///
    private func _tokenizeIndentation() throws -> [Token] {
        var tokens: [Token] = []
        var indentStack: [Int] = [0]
        var localBracketDepth = 0
        var atLineStart = true

        while _pos < _chars.count {
            // Process line start (indentation)
            if atLineStart && localBracketDepth == 0 {
                let result = try _processLineStart(&indentStack)
                switch result {
                case .skip:
                    continue
                case .tokens(let indentTokens):
                    tokens.append(contentsOf: indentTokens)
                    atLineStart = false
                    if _pos >= _chars.count {
                        break
                    }
                }
            }

            guard _pos < _chars.count else { break }
            let char = _chars[_pos]

            // Newline handling
            if char == "\n" {
                if localBracketDepth == 0 {
                    tokens.append(Token(type: "NEWLINE", value: "\\n", line: _line, column: _column))
                }
                _advance()
                atLineStart = true
                continue
            }

            // Inside brackets: skip whitespace
            if localBracketDepth > 0 && (char == " " || char == "\t" || char == "\r") {
                _advance()
                continue
            }

            // Try skip patterns
            if _trySkip() {
                continue
            }

            // Try token patterns (always uses default group for indentation mode)
            if let tok = try _tryMatchTokenInGroup("default") {
                // Track bracket depth (local for INDENT/DEDENT logic)
                if tok.value == "(" || tok.value == "[" || tok.value == "{" {
                    localBracketDepth += 1
                } else if tok.value == ")" || tok.value == "]" || tok.value == "}" {
                    localBracketDepth -= 1
                }
                // Track bracket depth (shared for callback access)
                _updateBracketDepth(tok.value)
                tokens.append(tok)
                _lastEmittedToken = tok
                continue
            }

            throw LexerError(
                "Unexpected character: \"\(char)\"",
                line: _line,
                column: _column
            )
        }

        // EOF: emit remaining DEDENTs
        while indentStack.count > 1 {
            indentStack.removeLast()
            tokens.append(Token(type: "DEDENT", value: "", line: _line, column: _column))
        }

        // Final NEWLINE if needed
        if tokens.isEmpty || tokens.last!.type != "NEWLINE" {
            tokens.append(Token(type: "NEWLINE", value: "\\n", line: _line, column: _column))
        }

        tokens.append(Token(type: "EOF", value: "", line: _line, column: _column))

        // Reset group stack for reuse.
        _groupStack = ["default"]
        _skipEnabled = true

        return tokens
    }

    /// Result from processing the start of a line in indentation mode.
    private enum LineStartResult {
        case skip
        case tokens([Token])
    }

    /// Process indentation at the start of a logical line.
    ///
    /// Returns `.skip` if the line should be skipped (blank/comment),
    /// or `.tokens` with an array of INDENT/DEDENT tokens.
    ///
    private func _processLineStart(_ indentStack: inout [Int]) throws -> LineStartResult {
        var indent = 0
        while _pos < _chars.count {
            let char = _chars[_pos]
            if char == " " {
                indent += 1
                _advance()
            } else if char == "\t" {
                throw LexerError(
                    "Tab character in indentation (use spaces only)",
                    line: _line,
                    column: _column
                )
            } else {
                break
            }
        }

        // Blank line or EOF
        if _pos >= _chars.count {
            return .skip
        }
        if _chars[_pos] == "\n" {
            _advance()
            return .skip
        }

        // Comment-only line -- check skip patterns
        let remaining = String(_chars[_pos...])
        for pat in _skipPatterns {
            let range = NSRange(remaining.startIndex..., in: remaining)
            if let match = pat.firstMatch(in: remaining, range: range) {
                let matchStr = (remaining as NSString).substring(with: match.range)
                let peekPos = _pos + matchStr.count
                if peekPos >= _chars.count || _chars[peekPos] == "\n" {
                    for _ in 0..<matchStr.count {
                        _advance()
                    }
                    if _pos < _chars.count && _chars[_pos] == "\n" {
                        _advance()
                    }
                    return .skip
                }
            }
        }

        // Compare indent to current level
        let currentIndent = indentStack.last!
        var indentTokens: [Token] = []

        if indent > currentIndent {
            indentStack.append(indent)
            indentTokens.append(Token(type: "INDENT", value: "", line: _line, column: 1))
        } else if indent < currentIndent {
            while indentStack.count > 1 && indentStack.last! > indent {
                indentStack.removeLast()
                indentTokens.append(Token(type: "DEDENT", value: "", line: _line, column: 1))
            }
            if indentStack.last! != indent {
                throw LexerError(
                    "Inconsistent dedent",
                    line: _line,
                    column: _column
                )
            }
        }

        return .tokens(indentTokens)
    }

    // -----------------------------------------------------------------------
    // Shared Helpers
    // -----------------------------------------------------------------------

    /// Try to match and consume a skip pattern at the current position.
    ///
    /// Skip patterns are defined in the `skip:` section of a .tokens file.
    /// They match text that should be consumed without emitting a token --
    /// typically comments and inline whitespace.
    ///
    /// - Returns: true if a skip pattern matched (text was consumed), false otherwise.
    ///
    private func _trySkip() -> Bool {
        let remaining = String(_chars[_pos...])
        let nsRange = NSRange(remaining.startIndex..., in: remaining)

        for pat in _skipPatterns {
            if let match = pat.firstMatch(in: remaining, range: nsRange) {
                // \A anchor ensures the match is at position 0.
                let matchStr = (remaining as NSString).substring(with: match.range)
                for _ in 0..<matchStr.count {
                    _advance()
                }
                return true
            }
        }
        return false
    }

    /// Try to match a token pattern from a specific group.
    ///
    /// Tries each compiled pattern in the named group in priority order
    /// (first match wins). Handles keyword detection, reserved word
    /// checking, aliases, and string escape processing.
    ///
    /// - Parameter groupName: The pattern group to use (e.g., "default", "tag").
    /// - Returns: A Token if a pattern matched, nil otherwise.
    /// - Throws: LexerError if a reserved keyword is encountered.
    ///
    private func _tryMatchTokenInGroup(_ groupName: String) throws -> Token? {
        let remaining = String(_chars[_pos...])
        let nsRange = NSRange(remaining.startIndex..., in: remaining)
        let patterns = _groupPatterns[groupName] ?? _patterns

        for cp in patterns {
            if let match = cp.regex.firstMatch(in: remaining, range: nsRange) {
                var value = (remaining as NSString).substring(with: match.range)
                let startLine = _line
                let startColumn = _column

                // For case-insensitive grammars, normalize for keyword lookup.
                let lookupValue = _caseInsensitive ? value.uppercased() : value

                let tokenType = try resolveTokenType(
                    tokenName: cp.name,
                    value: lookupValue,
                    keywordSet: _keywordSet,
                    reservedSet: _reservedSet,
                    alias: cp.alias,
                    line: startLine,
                    column: startColumn
                )

                // Case-insensitive keyword normalization.
                if _caseInsensitive && tokenType == "KEYWORD" {
                    value = lookupValue
                }

                // Handle STRING tokens: strip quotes and process escapes.
                let effectiveName = _aliasMap[cp.name] ?? cp.name
                if effectiveName == "STRING" || cp.name == "STRING"
                    || cp.name.contains("STRING")
                    || (cp.alias?.contains("STRING") ?? false)
                {
                    if value.count >= 6 &&
                        (value.hasPrefix("\"\"\"") || value.hasPrefix("'''"))
                    {
                        let inner = String(value.dropFirst(3).dropLast(3))
                        value = _escapeMode == "none" ? inner : processEscapes(inner)
                    } else if value.count >= 2 {
                        let first = value.first!
                        if first == "\"" || first == "'" {
                            let inner = String(value.dropFirst().dropLast())
                            value = _escapeMode == "none" ? inner : processEscapes(inner)
                        }
                    }
                }

                // Context keyword detection.
                var flags: Int = 0
                if tokenType == "NAME" && !_contextKeywordSet.isEmpty
                    && _contextKeywordSet.contains(value)
                {
                    flags = TOKEN_CONTEXT_KEYWORD
                }

                let tok = Token(
                    type: tokenType,
                    value: value,
                    line: startLine,
                    column: startColumn,
                    flags: flags
                )

                // Advance past the match.
                let matchedStr = (remaining as NSString).substring(with: match.range)
                for _ in 0..<matchedStr.count {
                    _advance()
                }

                return tok
            }
        }
        return nil
    }

    /// Move position forward by one character, tracking line and column.
    ///
    /// When we encounter a newline character, we increment the line counter
    /// and reset the column to 1. For all other characters, we just increment
    /// the column. This is the same logic used in the TypeScript version.
    ///
    ///     Character | Effect
    ///     ----------|---------------------------
    ///     \n        | line += 1, column = 1
    ///     anything  | column += 1
    ///
    private func _advance() {
        if _pos < _chars.count {
            if _chars[_pos] == "\n" {
                _line += 1
                _column = 1
            } else {
                _column += 1
            }
            _pos += 1
        }
    }
}

// ---------------------------------------------------------------------------
// Convenience Function -- Backward-Compatible Wrapper
// ---------------------------------------------------------------------------

/// Tokenize source code using a grammar (parsed from a .tokens file).
///
/// This is a convenience wrapper around `GrammarLexer` that provides
/// backward compatibility with a function-based API. It creates a
/// `GrammarLexer` instance and calls `tokenize()` on it.
///
/// For advanced features like pattern groups and on-token callbacks, use
/// the `GrammarLexer` class directly.
///
/// - Parameters:
///   - source: The raw source code text to tokenize.
///   - grammar: A TokenGrammar object (parsed from a .tokens file).
/// - Returns: A list of Token objects, always ending with an EOF token.
/// - Throws: LexerError if tokenization fails.
///
public func grammarTokenize(source: String, grammar: TokenGrammar) throws -> [Token] {
    return try GrammarLexer(source: source, grammar: grammar).tokenize()
}
